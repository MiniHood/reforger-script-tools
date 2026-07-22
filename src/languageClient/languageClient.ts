import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import {
	CloseAction,
	ErrorAction,
	LanguageClient,
	type ErrorHandler,
	type LanguageClientOptions,
	type ServerOptions,
	TransportKind,
} from 'vscode-languageclient/node';
import { gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';
import { diagnostic, languageServerDiagnosticPath } from '../diagnostics/diagnostics';
import {
	languageClientCrashHandling,
	languageClientCompletion,
	languageClientCommands,
	languageClientDocumentSelector,
	languageClientIndexCache,
	languageClientIds,
	languageClientLanguage,
	languageClientLogs,
	languageClientRequests,
	languageClientServer,
} from '../extensionConfig/languageClient';
import { getManualScriptsFolderCandidate } from '../gameData/gameData';
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
import { completionUiMiddlewareCallbacks, completionLifecycleTraceForDocument, registerCompletionUiBridge } from './completionUiBridge';
import { openSymbolLocation } from './symbolLocationBridge';
import {
	disposeDevelopmentServerWatchBridge,
	registerDevelopmentServerWatchBridge,
} from './developmentServerWatchBridge';
import {
	registerBlockCommentPair,
	registerEnterTypingAssist,
} from './typingAssistTransactionBridge';

export { blockCommentPairPosition, enterAfterPosition, tabAfterPosition } from './typingAssistBridge';
export { isCurrentSingleTypingAssistCaret } from './typingAssistTransactionBridge';
export { ifSpaceCommitContractFromCommandArguments } from './completionUiBridge';

let client: LanguageClient | undefined;
let clientDisposables: vscode.Disposable[] = [];
let restartingClient = false;
let initialStartup: Promise<void> | undefined;
const workspaceWatcherDebounceMs = 250;
const startupTimingSessionStartMs = Date.now();
const startupTimingSessionId = `${startupTimingSessionStartMs}-${process.pid}`;
let startupTimingWriteQueue: Promise<void> = Promise.resolve();
let startupTimingLogPath: string | undefined;
let startupTimingLogDirectoryReady: Promise<void> | undefined;
let firstDocumentOpenTimingLogged = false;
let firstSemanticTokenTimingLogged = false;

export function logLanguageClientStartupTiming(
	context: vscode.ExtensionContext,
	event: string,
	fields: Record<string, string | number | boolean | undefined> = {},
): void {
	const elapsedMs = Date.now() - startupTimingSessionStartMs;
	const record = {
		timestamp: new Date().toISOString(),
		session: startupTimingSessionId,
		elapsedMs,
		event,
		...fields,
	};
	startupTimingLogPath ??= path.join(
		context.globalStorageUri.fsPath,
		languageClientLogs.rootFolder,
		languageClientLogs.startupTimingLogFile,
	);
	const logPath = startupTimingLogPath;
	startupTimingLogDirectoryReady ??= fs
		.mkdir(path.dirname(logPath), { recursive: true })
		.then(() => undefined);

	startupTimingWriteQueue = startupTimingWriteQueue
		.then(async () => {
			await startupTimingLogDirectoryReady;
			await fs.appendFile(logPath, `${JSON.stringify(record)}\n`, 'utf8');
		})
		.catch(() => undefined);
	diagnostic(`startup.${event}`, sanitizeDiagnosticFields(fields));
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

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): () => void {
	logLanguageClientStartupTiming(context, 'languageClientRegistrationStart');
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	const completionDebugOutputChannel = vscode.window.createOutputChannel(languageClientIds.completionDebugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(completionDebugOutputChannel);
	context.subscriptions.push(registerEnterTypingAssist(() => client));
	context.subscriptions.push(registerBlockCommentPair(() => client));
	context.subscriptions.push(...registerCompletionUiBridge());
	context.subscriptions.push(...registerDebugCommandBridge(
		context,
		() => client,
		debugOutputChannel,
		completionDebugOutputChannel,
		completionLifecycleTraceForDocument,
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.openSymbolLocation,
		(args: unknown) => openSymbolLocation(args),
	));
	context.subscriptions.push(registerFirstDocumentOpenTiming(context));

	const startup = startLanguageClient(context, outputChannel);
	initialStartup = startup;
	void startup.finally(() => {
		if (initialStartup === startup) {
			initialStartup = undefined;
		}
	});
	logLanguageClientStartupTiming(context, 'languageClientRegistrationEnd');
	return () => {
		void restartLanguageClient(context, outputChannel, 'game-data source changed');
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
): Promise<void> {
	logLanguageClientStartupTiming(context, 'languageClientStartBegin');
	const serverPath = await resolveServerPath(context);
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
	registerDevelopmentServerWatchBridge(context, serverPath, () => {
		void restartLanguageClient(context, outputChannel, 'development language-server binary changed');
	});

	const logsRoot = path.join(context.globalStorageUri.fsPath, languageClientLogs.rootFolder);
	await fs.mkdir(logsRoot, { recursive: true });

	const serverArgs = [
		'--log',
		path.join(logsRoot, languageClientLogs.serverLogFile),
		'--index-cache',
		path.join(context.globalStorageUri.fsPath, languageClientIndexCache.rootFolder, languageClientIndexCache.gameDataIndexFile),
	];
	const diagnosticPath = languageServerDiagnosticPath(context);
	if (diagnosticPath) {
		serverArgs.push('--diagnostic-log', diagnosticPath);
	}
	const gameDataPaths = getGameDataPaths(context);
	if (gameDataPaths.scripts) {
		serverArgs.push('--game-data-scripts', gameDataPaths.scripts);
	}
	if (gameDataPaths.metadata) {
		serverArgs.push('--game-data-metadata', gameDataPaths.metadata);
	}
	const workspaceScriptRoots = await discoverWorkspaceScriptRoots();
	for (const root of workspaceScriptRoots) {
		serverArgs.push('--workspace-scripts', root);
	}
	logLanguageClientStartupTiming(context, 'languageServerArgumentsReady', {
		hasGameDataScripts: Boolean(gameDataPaths.scripts),
		hasGameDataMetadata: Boolean(gameDataPaths.metadata),
		workspaceScriptRoots: workspaceScriptRoots.length,
		serverArgs: serverArgs.length,
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
				const startedAt = Date.now();
				try {
					const result = await next(document, token);
					logFirstSemanticTokenResponse(context, document, startedAt, 'ok');
					return result;
				} catch (error) {
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

	try {
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
	} catch (error) {
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
	if (restartingClient) {
		return;
	}

	restartingClient = true;
	try {
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
		await startLanguageClient(context, outputChannel);
	} finally {
		restartingClient = false;
	}
}


function disposeClientDisposables(): void {
	for (const disposable of clientDisposables) {
		disposable.dispose();
	}
	clientDisposables = [];
}

async function resolveServerPath(context: vscode.ExtensionContext): Promise<string | undefined> {
	const devPath = path.join(context.extensionPath, ...languageClientServer.devBinaryRelativePath);
	if (context.extensionMode === vscode.ExtensionMode.Development && await isFile(devPath)) {
		return devPath;
	}

	const packagedPath = path.join(
		context.extensionPath,
		'dist',
		languageClientServer.distFolder,
		`${process.platform}-${process.arch}`,
		languageClientServer.binaryName,
	);
	if (await isFile(packagedPath)) {
		return packagedPath;
	}

	if (await isFile(devPath)) {
		return devPath;
	}

	return undefined;
}


function getGameDataPaths(context: vscode.ExtensionContext): { scripts: string | undefined; metadata: string | undefined } {
	const manualFolder = vscode.workspace
		.getConfiguration(gameDataConfig.section)
		.get<string>(gameDataConfig.settings.manualFolder);
	if (manualFolder?.trim()) {
		return {
			scripts: getManualScriptsFolderCandidate(manualFolder),
			metadata: undefined,
		};
	}

	const gameDataRoot = path.join(context.globalStorageUri.fsPath, gameDataStorage.rootFolder);
	return {
		scripts: path.join(gameDataRoot, gameDataStorage.scriptsFolder),
		metadata: path.join(gameDataRoot, gameDataStorage.metadataFile),
	};
}

async function isFile(targetPath: string): Promise<boolean> {
	try {
		return (await fs.stat(targetPath)).isFile();
	} catch {
		return false;
	}
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
