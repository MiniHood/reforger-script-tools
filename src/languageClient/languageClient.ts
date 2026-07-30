import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import {
	CloseAction,
	ErrorAction,
	LanguageClient,
	type ErrorHandler,
	type LanguageClientOptions,
	NotificationType,
	type ServerOptions,
	State,
	TransportKind,
} from 'vscode-languageclient/node';
import {
	bracketColoringConfig,
	type BracketColoringMode,
	getBracketColoringMode,
} from '../extensionConfig/bracketColoring';
import {
	diagnostic,
	diagnosticsEnabled,
	languageServerDiagnosticPath,
} from '../diagnostics/diagnostics';
import {
	languageClientCrashHandling,
	languageClientCompletion,
	languageClientCommands,
	languageClientDocumentSelector,
	languageClientIndexCache,
	languageClientIds,
	languageClientLanguage,
	languageClientLogs,
	languageClientNotifications,
	languageClientRequests,
} from '../extensionConfig/languageClient';
import { resolveLocalSourceInventoryPath } from '../gameData/gameData';
import { registerHtmlHoverBridge } from './hoverBridge';
import {
	discoverWorkspaceScriptRoots,
	registerWorkspaceScriptWatchBridge,
} from './workspaceWatchBridge';
import { registerDebugCommandBridge } from './debugCommandBridge';
import {
	completionItemCount,
	completionPresentationMetadata,
	createCompletionMiddleware,
	isCompletionListIncomplete,
} from './completionMiddleware';
import { completionPresentationObservationForDocument, completionUiMiddlewareCallbacks, completionLifecycleTraceForDocument, registerCompletionUiBridge } from './completionUiBridge';
import { openSymbolLocation } from './symbolLocationBridge';
import {
	disposeDevelopmentServerWatchBridge,
	registerDevelopmentServerWatchBridge,
} from './developmentServerWatchBridge';
import {
	registerBlockCommentPair,
} from './typingAssistTransactionBridge';
import { registerControlHeaderEnter } from './controlHeaderEnterBridge';
import { registerActiveScopeDelimiterBridge } from './activeScopeDelimiterBridge';
import {
	applyBracketColoringEditorMode,
	bracketColoringServerArguments,
	usesCustomScopeDelimiterPresentation,
} from './bracketColoringBridge';
import { RestartCoordinator } from './restartCoordinator';
import { resolveLanguageServerPath } from './serverPath';
import { registerSemanticTokenTimingBridge } from './semanticTokenTimingBridge';
import { registerSemanticTokenBoundaryGuardBridge } from './semanticTokenBoundaryGuardBridge';

export { blockCommentPairPosition } from './typingAssistBridge';
export { ifSpaceCommitContractFromCommandArguments } from './completionUiBridge';

let client: LanguageClient | undefined;
let clientDisposables: vscode.Disposable[] = [];
const restartCoordinator = new RestartCoordinator();
let initialStartup: Promise<void> | undefined;
const workspaceWatcherDebounceMs = 250;
const startupTimingSessionStartMs = Date.now();
let firstDocumentOpenTimingLogged = false;
let firstSemanticTokenTimingLogged = false;

interface ExternalIndexProgressParams {
	phase: string;
	status?: string;
	gameDataFiles?: number;
}

type ExternalIndexProgress = vscode.Progress<{ message?: string; increment?: number }>;

interface ExternalIndexProgressSession {
	progress: ExternalIndexProgress;
}

let activeExternalIndexProgressSession: ExternalIndexProgressSession | undefined;

export function logLanguageClientStartupTiming(
	_context: vscode.ExtensionContext,
	event: string,
	fields: Record<string, string | number | boolean | undefined> = {},
): void {
	const elapsedMs = Date.now() - startupTimingSessionStartMs;
	diagnostic(`startup.${event}`, {
		elapsedMs,
		...sanitizeDiagnosticFields(fields),
	});
}

function sanitizeDiagnosticFields(fields: Record<string, string | number | boolean | undefined>): Record<string, string | number | boolean | undefined> {
	const safe: Record<string, string | number | boolean | undefined> = {};
	for (const [key, value] of Object.entries(fields)) {
		if (!key.toLowerCase().includes('path') && key !== 'uri' && key !== 'serverPath' && key !== 'message') {
			safe[key] = value;
		}
	}
	return safe;
}

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): () => Promise<void> {
	logLanguageClientStartupTiming(context, 'languageClientRegistrationStart');
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	const completionDebugOutputChannel = vscode.window.createOutputChannel(languageClientIds.completionDebugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(completionDebugOutputChannel);
	context.subscriptions.push(...registerControlHeaderEnter(() => client));
	context.subscriptions.push(registerBlockCommentPair(() => client));
	context.subscriptions.push(...registerCompletionUiBridge());
	context.subscriptions.push(...registerDebugCommandBridge(
		context,
		() => client,
		debugOutputChannel,
		completionDebugOutputChannel,
		completionLifecycleTraceForDocument,
		completionPresentationObservationForDocument,
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.openSymbolLocation,
		(args: unknown) => openSymbolLocation(args),
	));
	context.subscriptions.push(registerFirstDocumentOpenTiming(context));
	context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider(
		'reforger-pak',
		{
			provideTextDocumentContent: async uri => {
				if (!client) {
					throw new Error('Reforger language server is not ready.');
				}
				return client.sendRequest<string>(
					languageClientRequests.readPackSource,
					{ uri: uri.toString() },
				);
			},
		},
	));
	context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(event => {
		const setting = `${bracketColoringConfig.section}.${bracketColoringConfig.setting}`;
		if (event.affectsConfiguration(setting)) {
			void restartLanguageClient(context, outputChannel, 'bracket coloring changed');
		}
	}));

	const bracketColoring = getBracketColoringMode();
	const startup = synchronizeBracketColoringEditorMode(bracketColoring, outputChannel)
		.then(() => startLanguageClient(context, outputChannel, bracketColoring));
	initialStartup = startup;
	void startup.finally(() => {
		if (initialStartup === startup) {
			initialStartup = undefined;
		}
	});
	logLanguageClientStartupTiming(context, 'languageClientRegistrationEnd');
	return async () => {
		await vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: 'Reforger game data',
				cancellable: false,
			},
			async progress => {
				const session = { progress };
				activeExternalIndexProgressSession = session;
				progress.report({ message: 'Preparing script index' });
				try {
					await restartLanguageClient(
						context,
						outputChannel,
						'game-data source changed',
					);
				} finally {
					if (activeExternalIndexProgressSession === session) {
						activeExternalIndexProgressSession = undefined;
					}
				}
			},
		);
	};
}

function registerFirstDocumentOpenTiming(context: vscode.ExtensionContext): vscode.Disposable {
	for (const document of vscode.workspace.textDocuments) {
		if (document.languageId === languageClientLanguage.id) {
			logFirstDocumentOpened(context, document, 'alreadyOpen');
			break;
		}
	}

	return vscode.workspace.onDidOpenTextDocument(document => {
		if (document.languageId === languageClientLanguage.id) {
			logFirstDocumentOpened(context, document, 'didOpenEvent');
		}
	});
}

function logFirstDocumentOpened(
	context: vscode.ExtensionContext,
	document: vscode.TextDocument,
	source: string,
): void {
	if (firstDocumentOpenTimingLogged) {
		return;
	}
	firstDocumentOpenTimingLogged = true;
	logLanguageClientStartupTiming(context, 'firstDocumentOpened', {
		source,
		uri: document.uri.toString(),
		languageId: document.languageId,
		lineCount: document.lineCount,
		byteLength: Buffer.byteLength(document.getText(), 'utf8'),
	});
}


export async function deactivateLanguageClient(): Promise<void> {
	diagnostic('languageClient.deactivate');
	disposeClientDisposables();
	disposeDevelopmentServerWatchBridge();
	const activeClient = client;
	client = undefined;
	if (activeClient) {
		await activeClient.stop();
	}
}

async function startLanguageClient(
	context: vscode.ExtensionContext,
	outputChannel: vscode.LogOutputChannel,
	bracketColoring: BracketColoringMode,
	externalIndexProgress?: ExternalIndexProgress,
): Promise<void> {
	logLanguageClientStartupTiming(context, 'languageClientStartBegin');
	const serverPath = await resolveLanguageServerPath(context);
	if (!serverPath) {
		outputChannel.appendLine('Language server binary was not found. Run npm run build-server during development.');
		logLanguageClientStartupTiming(context, 'languageClientStartAborted', {
			reason: 'serverBinaryNotFound',
		});
		return;
	}
	logLanguageClientStartupTiming(context, 'languageServerPathResolved', {
		serverPath,
		extensionMode: extensionModeName(context.extensionMode),
	});
	registerDevelopmentServerWatchBridge(
		context,
		serverPath,
		() => {
			void restartLanguageClient(
				context,
				outputChannel,
				'development language-server binary changed',
			).catch(error => {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.appendLine(`Development language-server restart failed: ${message}`);
			});
		},
	);

	const sourceInventoryPath = await resolveLocalSourceInventoryPath(context);
	const serverArgs = [
		'--addon-index-storage',
		path.join(context.globalStorageUri.fsPath, languageClientIndexCache.rootFolder),
		'--addon-source-inventory', sourceInventoryPath,
		...bracketColoringServerArguments(bracketColoring),
	];
	if (diagnosticsEnabled()) {
		const logsRoot = path.join(context.globalStorageUri.fsPath, languageClientLogs.rootFolder);
		await fs.mkdir(logsRoot, { recursive: true });
		serverArgs.push(
			'--log',
			path.join(logsRoot, languageClientLogs.serverLogFile),
		);
	}
	const diagnosticPath = languageServerDiagnosticPath(context);
	if (diagnosticPath) {
		serverArgs.push('--diagnostic-log', diagnosticPath);
	}
	const workspaceScriptRoots = await discoverWorkspaceScriptRoots();
	for (const root of workspaceScriptRoots) {
		serverArgs.push('--workspace-scripts', root);
	}
	logLanguageClientStartupTiming(context, 'languageServerArgumentsReady', {
		hasAddonSourceInventory: true,
		workspaceScriptRoots: workspaceScriptRoots.length,
		serverArgs: serverArgs.length,
		bracketColoring,
	});

	const serverOptions: ServerOptions = {
		run: {
			command: serverPath,
			args: serverArgs,
			transport: TransportKind.stdio,
		},
		debug: {
			command: serverPath,
			args: serverArgs,
			transport: TransportKind.stdio,
		},
	};

	const semanticTokenTiming = registerSemanticTokenTimingBridge();
	clientDisposables.push(semanticTokenTiming);
	const semanticTokenBoundaryGuard = registerSemanticTokenBoundaryGuardBridge();
	clientDisposables.push(semanticTokenBoundaryGuard);
	const clientOptions: LanguageClientOptions = {
		documentSelector: [...languageClientDocumentSelector],
		outputChannel,
		errorHandler: createLanguageServerErrorHandler(),
		markdown: {
			isTrusted: true,
			supportHtml: true,
		},
		middleware: {
			provideHover: () => null,
			...createCompletionMiddleware(completionUiMiddlewareCallbacks),
			provideDocumentSemanticTokens: async (document, token, next) => {
				const timing = semanticTokenTiming.start(document);
				const startedAt = Date.now();
				const version = document.version;
				try {
					const result = await next(document, token);
					if (!token.isCancellationRequested && result) {
						semanticTokenBoundaryGuard.update(document, version, result);
					}
					semanticTokenTiming.complete(timing, 'ok', token.isCancellationRequested);
					logFirstSemanticTokenResponse(context, document, startedAt, 'ok');
					return result;
				} catch (error) {
					semanticTokenTiming.complete(timing, 'error', token.isCancellationRequested);
					logFirstSemanticTokenResponse(context, document, startedAt, 'error', error);
					throw error;
				}
			},
		},
	};

	logLanguageClientStartupTiming(context, 'languageClientCreateStart');
	client = new LanguageClient(
		languageClientIds.id,
		languageClientIds.name,
		serverOptions,
		clientOptions,
	);
	logLanguageClientStartupTiming(context, 'languageClientCreated');
	const externalIndexMonitor = externalIndexProgress
		? monitorExternalIndexProgress(client, externalIndexProgress)
		: undefined;
	if (externalIndexMonitor) {
		clientDisposables.push(externalIndexMonitor.disposable);
	}

	try {
		externalIndexProgress?.report({ message: 'Starting language server' });
		logLanguageClientStartupTiming(context, 'languageServerProcessSpawnRequested', {
			serverPath,
			transport: 'stdio',
		});
		await client.start();
		diagnostic('languageClient.started', { workspaceScriptRoots: workspaceScriptRoots.length });
		logLanguageClientStartupTiming(context, 'languageServerInitializeResponse', {
			serverPath,
		});
		outputChannel.appendLine(`Language server started: ${serverPath}`);
		if (workspaceScriptRoots.length > 0) {
			outputChannel.appendLine(`Workspace script roots: ${workspaceScriptRoots.join('; ')}`);
		}
		clientDisposables.push(registerHtmlHoverBridge(client, outputChannel));
		clientDisposables.push(...registerWorkspaceScriptWatchBridge(client, outputChannel));
		if (usesCustomScopeDelimiterPresentation(bracketColoring)) {
			clientDisposables.push(registerActiveScopeDelimiterBridge(client));
		}
		await externalIndexMonitor?.completion;
	} catch (error) {
		externalIndexMonitor?.disposable.dispose();
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Language server failed to start: ${message}`);
		logLanguageClientStartupTiming(context, 'languageClientStartFailed', {
			message,
		});
		vscode.window.showWarningMessage(`Reforger language server failed to start: ${message}`);
		diagnostic('languageClient.startFailed');
	}
}

function logFirstSemanticTokenResponse(
	context: vscode.ExtensionContext,
	document: vscode.TextDocument,
	startedAt: number,
	status: string,
	error?: unknown,
): void {
	if (firstSemanticTokenTimingLogged) {
		return;
	}
	firstSemanticTokenTimingLogged = true;
	logLanguageClientStartupTiming(context, 'firstSemanticTokenResponse', {
		status,
		uri: document.uri.toString(),
		languageId: document.languageId,
		elapsedMsForRequest: Date.now() - startedAt,
		message: error instanceof Error ? error.message : undefined,
	});
}

function createLanguageServerErrorHandler(): ErrorHandler {
	const restarts: number[] = [];
	return {
		error: (_error, _message, count) => {
			if (count !== undefined && count <= 3) {
				return { action: ErrorAction.Continue };
			}
			return { action: ErrorAction.Shutdown };
		},
		closed: () => {
			restarts.push(Date.now());
			if (restarts.length <= languageClientCrashHandling.maxRestartCount) {
				return { action: CloseAction.Restart };
			}

			const elapsed = restarts[restarts.length - 1] - restarts[0];
			if (elapsed <= languageClientCrashHandling.restartWindowMs) {
				void vscode.window.showErrorMessage(languageClientCrashHandling.finalCrashMessage);
				return {
					action: CloseAction.DoNotRestart,
					message: languageClientCrashHandling.finalCrashMessage,
					handled: true,
				};
			}

			restarts.shift();
			return { action: CloseAction.Restart };
		},
	};
}


async function restartLanguageClient(
	context: vscode.ExtensionContext,
	outputChannel: vscode.LogOutputChannel,
	reason: string,
): Promise<void> {
	await restartCoordinator.run(async () => {
		const startup = initialStartup;
		if (startup) {
			await startup;
		}
		outputChannel.appendLine(`Restarting language server: ${reason}`);
		disposeClientDisposables();
		const activeClient = client;
		client = undefined;
		try {
			if (activeClient) {
				await activeClient.stop();
			}
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			outputChannel.appendLine(`Language server stop during restart reported: ${message}`);
		}
		const bracketColoring = getBracketColoringMode();
		await synchronizeBracketColoringEditorMode(bracketColoring, outputChannel);
		await startLanguageClient(
			context,
			outputChannel,
			bracketColoring,
			activeExternalIndexProgressSession?.progress,
		);
	});
}

function monitorExternalIndexProgress(
	activeClient: LanguageClient,
	progress: ExternalIndexProgress,
): { completion: Promise<void>; disposable: vscode.Disposable } {
	let complete = false;
	let resolveCompletion: (() => void) | undefined;
	const completion = new Promise<void>(resolve => {
		resolveCompletion = resolve;
	});
	const finish = () => {
		if (!complete) {
			complete = true;
			resolveCompletion?.();
		}
	};
	const notification = activeClient.onNotification(
		new NotificationType<ExternalIndexProgressParams>(languageClientNotifications.externalIndexProgress),
		params => {
			progress.report({ message: externalIndexProgressMessage(params.phase, params.status) });
			if (params.phase === 'complete') {
				finish();
			}
		},
	);
	const stateChanges = activeClient.onDidChangeState(event => {
		if (event.newState === State.Stopped) {
			finish();
		}
	});
	return {
		completion,
		disposable: vscode.Disposable.from(notification, stateChanges, { dispose: finish }),
	};
}

export function externalIndexProgressMessage(phase: string, status?: string): string {
	switch (phase) {
		case 'pac-inspect-start':
			return 'Inspecting installed add-on packs';
		case 'pac-index-end':
			return 'Loading installed add-on index';
		case 'workspace-rebuild-start':
		case 'workspace-rebuild-end':
			return 'Indexing workspace scripts';
		case 'complete':
			if (status === 'ready') {
				return 'Script index ready';
			}
			if (status === 'failed') {
				return 'Script indexing failed';
			}
			return 'Script index unavailable';
		default:
			return 'Indexing scripts';
	}
}

async function synchronizeBracketColoringEditorMode(
	mode: BracketColoringMode,
	outputChannel: vscode.LogOutputChannel,
): Promise<void> {
	try {
		await applyBracketColoringEditorMode(mode);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Could not apply ${mode} bracket presentation: ${message}`);
		diagnostic('languageClient.bracketColoringConfigurationFailed', { mode, message });
	}
}


function disposeClientDisposables(): void {
	for (const disposable of clientDisposables) {
		disposable.dispose();
	}
	clientDisposables = [];
}

function extensionModeName(mode: vscode.ExtensionMode): string {
	switch (mode) {
		case vscode.ExtensionMode.Development:
			return 'development';
		case vscode.ExtensionMode.Production:
			return 'production';
		case vscode.ExtensionMode.Test:
			return 'test';
		default:
			return 'unknown';
	}
}
