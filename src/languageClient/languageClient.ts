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
	languageClientNotifications,
	languageClientRequests,
	languageClientServer,
} from '../extensionConfig/languageClient';
import { getManualScriptsFolderCandidate } from '../gameData/gameData';

let client: LanguageClient | undefined;
let clientDisposables: vscode.Disposable[] = [];
let devServerWatcher: vscode.FileSystemWatcher | undefined;
let watchedDevServerPath: string | undefined;
let restartTimer: NodeJS.Timeout | undefined;
let restartingClient = false;
const workspaceWatcherDebounceMs = 250;
const devServerRestartDebounceMs = 500;
const startupTimingSessionStartMs = Date.now();
const startupTimingSessionId = `${startupTimingSessionStartMs}-${process.pid}`;
let startupTimingWriteQueue: Promise<void> = Promise.resolve();
let startupTimingLogPath: string | undefined;
let startupTimingLogDirectoryReady: Promise<void> | undefined;
let firstDocumentOpenTimingLogged = false;
let firstSemanticTokenTimingLogged = false;
let completionTransactionSequence = 0;
let pendingSnippetSuggestTransaction: SnippetSuggestTransaction | undefined;
let pendingEmptyCompletionRefresh: EmptyCompletionRefresh | undefined;
let latestEditorDocumentChange: EditorDocumentChange | undefined;
const completionLifecycleTraceLimit = 80;
const completionLifecycleTrace: CompletionLifecycleTraceEvent[] = [];

// TEMPORARY: release-gated forensic trace for the RplRpc multi-placeholder
// bridge. OpenSpec task 3.3 tracks removing this once live editor behavior is
// proven. It records only counts, lengths, and state transitions.
const snippetSuggestTraceVersion = 3;
const maxSnippetSuggestSelectionProbes = 8;

interface SnippetSuggestTransaction {
	id: number;
	documentUri: string;
	expectedSelectionTexts: readonly string[];
	nextPlaceholderIndex: number;
	selectionProbeCount: number;
	selectionListener: vscode.Disposable;
	cleanupTimer: ReturnType<typeof setTimeout>;
	suggestDispatchScheduled: boolean;
	awaitingCompletionResponse: boolean;
}

interface EmptyCompletionRefresh {
	documentUri: string;
	requestVersion: number;
}

interface EditorDocumentChange {
	documentUri: string;
	version: number;
	hasDeletion: boolean;
}

interface CompletionLifecycleTraceEvent {
	documentUri: string;
	event: string;
	fields: Record<string, string | number | boolean | undefined>;
}

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

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): void {
	logLanguageClientStartupTiming(context, 'languageClientRegistrationStart');
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	const completionDebugOutputChannel = vscode.window.createOutputChannel(languageClientIds.completionDebugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(completionDebugOutputChannel);
	context.subscriptions.push(registerSemicolonAfterEnter());
	context.subscriptions.push(registerEmptyCompletionRefresh());
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugHoverAtCursor,
		() => debugHoverAtCursor(context, debugOutputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugCompletionAtCursor,
		() => debugCompletionAtCursor(context, completionDebugOutputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.triggerSuggestAtSnippetPlaceholder,
		(...expectedSelectionTexts: unknown[]) => triggerSuggestAtSnippetPlaceholder(...expectedSelectionTexts),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.advanceSnippetPlaceholderAfterAccept,
		(transactionId: unknown, originalCommand: unknown) =>
			advanceSnippetPlaceholderAfterAccept(transactionId, originalCommand),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.openSymbolLocation,
		(args: unknown) => openSymbolLocation(args),
	));
	context.subscriptions.push(registerFirstDocumentOpenTiming(context));

	void startLanguageClient(context, outputChannel);
	logLanguageClientStartupTiming(context, 'languageClientRegistrationEnd');
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


interface OpenSymbolLocationArgs {
	uri: string;
	startByte: number;
	endByte: number;
}

async function openSymbolLocation(args: unknown): Promise<void> {
	if (!isOpenSymbolLocationArgs(args)) {
		vscode.window.showWarningMessage('Invalid Reforger symbol location.');
		return;
	}

	const uri = vscode.Uri.parse(args.uri, true);
	const document = await vscode.workspace.openTextDocument(uri);
	const editor = await vscode.window.showTextDocument(document);
	const range = rangeFromByteOffsets(document.getText(), args.startByte, args.endByte);
	editor.selection = new vscode.Selection(range.start, range.end);
	editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function isOpenSymbolLocationArgs(value: unknown): value is OpenSymbolLocationArgs {
	if (!value || typeof value !== 'object') {
		return false;
	}
	const candidate = value as Partial<OpenSymbolLocationArgs>;
	const startByte = candidate.startByte;
	const endByte = candidate.endByte;
	return typeof candidate.uri === 'string'
		&& Number.isInteger(startByte)
		&& Number.isInteger(endByte)
		&& startByte !== undefined
		&& endByte !== undefined
		&& startByte >= 0
		&& endByte >= startByte;
}

function rangeFromByteOffsets(text: string, startByte: number, endByte: number): vscode.Range {
	return new vscode.Range(
		positionFromByteOffset(text, startByte),
		positionFromByteOffset(text, endByte),
	);
}

function positionFromByteOffset(text: string, byteOffset: number): vscode.Position {
	let line = 0;
	let character = 0;
	let consumedBytes = 0;
	for (const char of text) {
		const charBytes = Buffer.byteLength(char, 'utf8');
		if (consumedBytes + charBytes > byteOffset) {
			break;
		}
		consumedBytes += charBytes;
		if (char === '\n') {
			line += 1;
			character = 0;
		} else {
			character += char.length;
		}
	}
	return new vscode.Position(line, character);
}

export async function deactivateLanguageClient(): Promise<void> {
	diagnostic('languageClient.deactivate');
	disposeClientDisposables();
	devServerWatcher?.dispose();
	devServerWatcher = undefined;
	if (restartTimer) {
		clearTimeout(restartTimer);
		restartTimer = undefined;
	}
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
	registerDevelopmentServerWatcher(context, serverPath, outputChannel);

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
			provideCompletionItem: async (document, position, completionContext, token, next) => {
				const transaction = pendingSnippetSuggestTransaction;
				const requestVersion = document.version;
				const startedAt = Date.now();
				recordCompletionLifecycle(document.uri.toString(), 'request', {
					requestVersion,
					triggerKind: completionContext.triggerKind,
				});
				try {
					const result = await next(document, position, completionContext, token);
					recordCompletionLifecycle(document.uri.toString(), 'response', {
						requestVersion,
						currentVersion: document.version,
						triggerKind: completionContext.triggerKind,
						itemCount: completionItemCount(result),
						isIncomplete: isCompletionListIncomplete(result),
						elapsedMs: Date.now() - startedAt,
					});
					armEmptyCompletionRefresh(document, requestVersion, result);
					if (transaction?.documentUri === document.uri.toString()
						&& transaction.awaitingCompletionResponse) {
						const presentation = completionPresentationMetadata(result);
						diagnostic('completion.transaction.response', {
							transactionId: transaction.id,
							triggerKind: completionContext.triggerKind,
							itemCount: completionItemCount(result),
							elapsedMs: Date.now() - startedAt,
							...presentation,
						});
						wrapBridgeCompletionCommands(result, transaction.id);
						advanceSnippetSuggestTransaction(transaction.id);
					}
					return result;
				} catch (error) {
					recordCompletionLifecycle(document.uri.toString(), 'responseError', {
						requestVersion,
						triggerKind: completionContext.triggerKind,
						elapsedMs: Date.now() - startedAt,
					});
					if (transaction?.documentUri === document.uri.toString()
						&& transaction.awaitingCompletionResponse) {
						diagnostic('completion.transaction.responseError', {
							transactionId: transaction.id,
							triggerKind: completionContext.triggerKind,
							elapsedMs: Date.now() - startedAt,
						});
						clearSnippetSuggestTransaction(transaction.id);
					}
					throw error;
				}
			},
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
		clientDisposables.push(registerHtmlHoverProvider(client, outputChannel));
		clientDisposables.push(...registerWorkspaceScriptWatchers(context, client, outputChannel));
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

function registerHtmlHoverProvider(
	activeClient: LanguageClient,
	outputChannel: vscode.LogOutputChannel,
): vscode.Disposable {
	return vscode.languages.registerHoverProvider(languageClientDocumentSelector, {
		provideHover: async (document, position, token) => {
			const startedAt = Date.now();
			try {
				const hover = await activeClient.sendRequest<LspHoverResponse | null>(
					'textDocument/hover',
					{
						textDocument: { uri: document.uri.toString() },
						position: { line: position.line, character: position.character },
					},
					token,
				);
			diagnostic('lsp.hover', { outcome: hover ? 'hit' : 'empty', elapsedMs: Date.now() - startedAt });
			return hover ? hoverFromLspResponse(hover) : null;
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
			outputChannel.debug(`HTML hover request failed for ${document.uri.toString()}: ${message}`);
			diagnostic('lsp.hover', { outcome: 'error', elapsedMs: Date.now() - startedAt });
				return null;
			}
		},
	});
}

interface LspHoverResponse {
	contents: LspMarkupContent | string | Array<LspMarkupContent | string>;
	range?: LspRange;
}

interface LspMarkupContent {
	kind?: string;
	value?: string;
}

interface LspRange {
	start: LspPosition;
	end: LspPosition;
}

interface LspPosition {
	line: number;
	character: number;
}

interface LspTextEdit {
	range: LspRange;
	newText: string;
}

function registerSemicolonAfterEnter(): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		if (event.document.languageId !== languageClientLanguage.id || !isSinglePlainEnter(event.contentChanges)) {
			return;
		}
		const editor = vscode.window.activeTextEditor;
		if (!editor || editor.document.uri.toString() !== event.document.uri.toString() || editor.selections.length !== 1
			|| !editor.selection.isEmpty) {
			return;
		}
		const version = event.document.version;
		const position = editor.selection.active;
		queueMicrotask(() => {
			void applySemicolonAfterEnter(event.document, version, position);
		});
	});
}

function isSinglePlainEnter(changes: readonly vscode.TextDocumentContentChangeEvent[]): boolean {
	return changes.length === 1
		&& changes[0].rangeLength === 0
		&& /^\r?\n[\t ]*$/.test(changes[0].text);
}

async function applySemicolonAfterEnter(
	document: vscode.TextDocument,
	version: number,
	position: vscode.Position,
): Promise<void> {
	const activeClient = client;
	const editor = vscode.window.activeTextEditor;
	if (!activeClient || !editor || document.version !== version || !hasSingleEmptyCaretAt(document, position)) {
		return;
	}
	try {
		const edits = await activeClient.sendRequest<LspTextEdit[]>(
			languageClientRequests.onTypeFormatting,
			{
				textDocument: { uri: document.uri.toString() },
				position: { line: position.line, character: position.character },
				ch: '\n',
				version,
				options: { tabSize: editor.options.tabSize, insertSpaces: editor.options.insertSpaces },
			},
		);
		if (document.version !== version || !hasSingleEmptyCaretAt(document, position)) {
			return;
		}
		await editor.edit(
			editBuilder => edits.forEach(edit => editBuilder.replace(rangeFromLsp(edit.range), edit.newText)),
			{ undoStopBefore: false, undoStopAfter: false },
		);
	} catch {
		// A typing assist must never surface transport failures while the user edits.
	}
}

function hasSingleEmptyCaretAt(document: vscode.TextDocument, position: vscode.Position): boolean {
	const editor = vscode.window.activeTextEditor;
	return editor?.document.uri.toString() === document.uri.toString()
		&& editor.selections.length === 1
		&& editor.selection.isEmpty
		&& editor.selection.active.isEqual(position);
}

function hoverFromLspResponse(hover: LspHoverResponse): vscode.Hover | null {
	const contents = Array.isArray(hover.contents) ? hover.contents : [hover.contents];
	const markdown = contents.map(content => htmlMarkdownContent(content));
	if (markdown.length === 0) {
		return null;
	}
	return new vscode.Hover(markdown, hover.range ? rangeFromLsp(hover.range) : undefined);
}

function htmlMarkdownContent(content: LspMarkupContent | string): vscode.MarkdownString {
	const markdown = new vscode.MarkdownString();
	markdown.isTrusted = true;
	markdown.supportHtml = true;

	if (typeof content === 'string') {
		markdown.appendMarkdown(content);
	} else if (content.kind === 'plaintext') {
		markdown.appendText(content.value ?? '');
	} else {
		markdown.appendMarkdown(content.value ?? '');
	}

	return markdown;
}

function rangeFromLsp(range: LspRange): vscode.Range {
	return new vscode.Range(
		new vscode.Position(range.start.line, range.start.character),
		new vscode.Position(range.end.line, range.end.character),
	);
}

function registerDevelopmentServerWatcher(
	context: vscode.ExtensionContext,
	serverPath: string,
	outputChannel: vscode.LogOutputChannel,
): void {
	if (context.extensionMode !== vscode.ExtensionMode.Development) {
		return;
	}

	const devPath = path.join(context.extensionPath, ...languageClientServer.devBinaryRelativePath);
	if (path.normalize(serverPath) !== path.normalize(devPath)) {
		return;
	}
	if (watchedDevServerPath === devPath && devServerWatcher) {
		return;
	}

	devServerWatcher?.dispose();
	watchedDevServerPath = devPath;

	const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(
		path.dirname(devPath),
		path.basename(devPath),
	));
	const scheduleRestart = (): void => {
		if (restartTimer) {
			clearTimeout(restartTimer);
		}
		restartTimer = setTimeout(() => {
			restartTimer = undefined;
			void restartLanguageClient(context, outputChannel, 'development language-server binary changed');
		}, devServerRestartDebounceMs);
	};

	context.subscriptions.push(
		watcher,
		watcher.onDidCreate(scheduleRestart),
		watcher.onDidChange(scheduleRestart),
	);
	devServerWatcher = watcher;
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

	try {
		await startLanguageClient(context, outputChannel);
	} finally {
		restartingClient = false;
	}
}

async function discoverWorkspaceScriptRoots(): Promise<string[]> {
	const folders = vscode.workspace.workspaceFolders ?? [];
	const roots = new Set<string>();

	for (const folder of folders) {
		const folderPath = folder.uri.fsPath;
		const folderName = path.basename(folderPath).toLowerCase();
		if (folderName === 'scripts') {
			roots.add(folderPath);
			continue;
		}

		for (const childName of ['Scripts', 'scripts']) {
			const candidate = path.join(folderPath, childName);
			if (await isDirectory(candidate)) {
				roots.add(candidate);
			}
		}
	}

	return [...roots].sort();
}

function registerWorkspaceScriptWatchers(
	context: vscode.ExtensionContext,
	activeClient: LanguageClient,
	outputChannel: vscode.LogOutputChannel,
): vscode.Disposable[] {
	const folders = vscode.workspace.workspaceFolders ?? [];
	if (folders.length === 0) {
		return [];
	}

	const disposables: vscode.Disposable[] = [];
	const pending = new Map<string, { path: string; kind: 'changed' | 'deleted'; sequence: number }>();
	const sequences = new Map<string, number>();
	let timer: NodeJS.Timeout | undefined;

	const flush = (): void => {
		const entries = [...pending.entries()];
		diagnostic('workspaceWatcher.flush', { entries: entries.length });
		pending.clear();
		timer = undefined;
		void Promise.all(entries.map(async ([, entry]) => {
			const { path: filePath, kind, sequence } = entry;
			if (kind === 'deleted') {
				activeClient.sendNotification(languageClientNotifications.workspaceFileDeleted, { path: filePath, sequence });
				diagnostic('workspaceWatcher.deleted', { sequence });
				return;
			}

			try {
				const text = await fs.readFile(filePath, 'utf8');
				activeClient.sendNotification(languageClientNotifications.workspaceFileChanged, { path: filePath, text, sequence });
				diagnostic('workspaceWatcher.changed', { bytes: Buffer.byteLength(text, 'utf8'), sequence });
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`Workspace script change skipped for ${filePath}: ${message}`);
				diagnostic('workspaceWatcher.readFailed', { sequence });
			}
		}));
	};

	const schedule = (uri: vscode.Uri, kind: 'changed' | 'deleted'): void => {
		if (uri.scheme !== 'file') {
			return;
		}
		const key = workspaceWatcherPathKey(uri.fsPath);
		const sequence = (sequences.get(key) ?? 0) + 1;
		sequences.set(key, sequence);
		pending.set(key, { path: uri.fsPath, kind, sequence });
		if (timer) {
			clearTimeout(timer);
		}
		timer = setTimeout(flush, workspaceWatcherDebounceMs);
	};

	for (const folder of folders) {
		const folderName = path.basename(folder.uri.fsPath).toLowerCase();
		const pattern = new vscode.RelativePattern(
			folder,
			folderName === 'scripts' ? '**/*.c' : '**/{Scripts,scripts}/**/*.c',
		);
		const watcher = vscode.workspace.createFileSystemWatcher(pattern);
		disposables.push(
			watcher,
			watcher.onDidCreate(uri => schedule(uri, 'changed')),
			watcher.onDidChange(uri => schedule(uri, 'changed')),
			watcher.onDidDelete(uri => schedule(uri, 'deleted')),
		);
	}

	return disposables;
}

function registerEmptyCompletionRefresh(): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		const documentUri = event.document.uri.toString();
		const hasDeletion = event.contentChanges.some(change => change.rangeLength > change.text.length);
		if (event.document.languageId === languageClientLanguage.id) {
			recordCompletionLifecycle(documentUri, 'documentChange', {
				version: event.document.version,
				changeCount: event.contentChanges.length,
				hasDeletion,
				insertedCharacters: event.contentChanges.reduce((total, change) => total + change.text.length, 0),
				deletedCharacters: event.contentChanges.reduce((total, change) => total + change.rangeLength, 0),
				activeDocument: isActiveEnforceDocument(event.document),
			});
		}
		latestEditorDocumentChange = {
			documentUri,
			version: event.document.version,
			hasDeletion,
		};

		const refresh = pendingEmptyCompletionRefresh;
		if (!refresh || refresh.documentUri !== documentUri || event.document.version <= refresh.requestVersion) {
			return;
		}
		pendingEmptyCompletionRefresh = undefined;
		if (!hasDeletion || !isActiveEnforceDocument(event.document)) {
			recordCompletionLifecycle(documentUri, 'emptyRefreshCancelled', {
				reason: hasDeletion ? 'inactiveDocument' : 'nonDeletion',
			});
			diagnostic('completion.emptyRefresh.cancelled', {
				reason: hasDeletion ? 'inactiveDocument' : 'nonDeletion',
			});
			return;
		}
		dispatchEmptyCompletionRefresh(event.document, 'deletion');
	});
}

function armEmptyCompletionRefresh(
	document: vscode.TextDocument,
	requestVersion: number,
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): void {
	if (!isRefreshableEmptyCompletion(result)) {
		if (pendingEmptyCompletionRefresh?.documentUri === document.uri.toString()) {
			pendingEmptyCompletionRefresh = undefined;
		}
		return;
	}

	const documentUri = document.uri.toString();
	const latestChange = latestEditorDocumentChange;
	if (latestChange?.documentUri === documentUri
		&& latestChange.version > requestVersion
		&& latestChange.hasDeletion
		&& isActiveEnforceDocument(document)) {
		dispatchEmptyCompletionRefresh(document, 'staleEmptyResponseAfterDeletion');
		return;
	}
	pendingEmptyCompletionRefresh = { documentUri, requestVersion };
	recordCompletionLifecycle(documentUri, 'emptyRefreshArmed', { requestVersion });
	diagnostic('completion.emptyRefresh.armed', { requestVersion });
}

function isRefreshableEmptyCompletion(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): result is vscode.CompletionList {
	return result !== null
		&& result !== undefined
		&& 'items' in result
		&& result.items.length === 0
		&& result.isIncomplete === true;
}

function isCompletionListIncomplete(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): boolean {
	return result !== null && result !== undefined && 'items' in result && result.isIncomplete === true;
}

function recordCompletionLifecycle(
	documentUri: string,
	event: string,
	fields: Record<string, string | number | boolean | undefined>,
): void {
	completionLifecycleTrace.push({ documentUri, event, fields });
	if (completionLifecycleTrace.length > completionLifecycleTraceLimit) {
		completionLifecycleTrace.shift();
	}
	diagnostic(`completion.lifecycle.${event}`, fields);
}

function isActiveEnforceDocument(document: vscode.TextDocument): boolean {
	return document.languageId === languageClientLanguage.id
		&& vscode.window.activeTextEditor?.document.uri.toString() === document.uri.toString();
}

function dispatchEmptyCompletionRefresh(document: vscode.TextDocument, source: 'deletion' | 'staleEmptyResponseAfterDeletion'): void {
	recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshDispatchRequested', { source });
	diagnostic('completion.emptyRefresh.dispatched', { source });
	queueMicrotask(() => {
		if (!isActiveEnforceDocument(document)) {
			recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshCancelled', { reason: 'activeEditorChanged' });
			diagnostic('completion.emptyRefresh.cancelled', { reason: 'activeEditorChanged' });
			return;
		}
		void vscode.commands.executeCommand('editor.action.triggerSuggest').then(
			() => {
				recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshSuggestDispatched', { source });
				diagnostic('completion.emptyRefresh.suggestDispatched', { source });
			},
			() => {
				recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshSuggestDispatchError', { source });
				diagnostic('completion.emptyRefresh.suggestDispatchError', { source });
			},
		);
	});
}

function triggerSuggestAtSnippetPlaceholder(...expectedSelectionTexts: unknown[]): void {
	diagnostic('completion.transaction.commandReceived', {
		traceVersion: snippetSuggestTraceVersion,
		placeholderCount: expectedSelectionTexts.length,
	});
	if (expectedSelectionTexts.length === 0
		|| expectedSelectionTexts.some(text => typeof text !== 'string' || text.length === 0)) {
		diagnostic('completion.transaction.ignored', { reason: 'invalidPlaceholderArgument' });
		return;
	}
	const expectedSelectionTextSequence = expectedSelectionTexts as string[];
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== languageClientLanguage.id) {
		diagnostic('completion.transaction.ignored', { reason: 'noActiveEnforceEditor' });
		return;
	}

	clearSnippetSuggestTransaction();
	const id = ++completionTransactionSequence;
	const documentUri = editor.document.uri.toString();
	const tryTrigger = (candidate: vscode.TextEditor, source: 'command' | 'selection'): void => {
		const transaction = pendingSnippetSuggestTransaction;
		if (!transaction || transaction.id !== id || candidate.document.uri.toString() !== documentUri) {
			return;
		}
		if (transaction.suggestDispatchScheduled || transaction.awaitingCompletionResponse) {
			return;
		}
		const expectedText = transaction.expectedSelectionTexts[transaction.nextPlaceholderIndex];
		const selectionCount = candidate.selections.length;
		const selectionLength = candidate.selection.end.character - candidate.selection.start.character;
		const matchesExpected = selectionCount === 1
			&& !candidate.selection.isEmpty
			&& candidate.document.getText(candidate.selection) === expectedText;
		if (!matchesExpected) {
			if (transaction.selectionProbeCount < maxSnippetSuggestSelectionProbes) {
				transaction.selectionProbeCount += 1;
				diagnostic('completion.transaction.selectionIgnored', {
					transactionId: id,
					source,
					placeholderIndex: transaction.nextPlaceholderIndex,
					selectionCount,
					selectionLength,
					expectedLength: expectedText.length,
					probeCount: transaction.selectionProbeCount,
				});
			}
			return;
		}
		diagnostic('completion.transaction.placeholderObserved', {
			transactionId: id,
			source,
			placeholderIndex: transaction.nextPlaceholderIndex,
			placeholderCount: transaction.expectedSelectionTexts.length,
			selectionLength,
		});
		transaction.suggestDispatchScheduled = true;
		transaction.awaitingCompletionResponse = true;
		resetSnippetSuggestTransactionTimeout(transaction, 'completionResponseNotObserved');
		queueMicrotask(() => {
			if (pendingSnippetSuggestTransaction?.id !== id) {
				return;
			}
			void vscode.commands.executeCommand('editor.action.triggerSuggest').then(
				() => diagnostic('completion.transaction.suggestDispatched', { transactionId: id }),
				() => diagnostic('completion.transaction.suggestDispatchError', { transactionId: id }),
			);
		});
	};

	const selectionListener = vscode.window.onDidChangeTextEditorSelection(event => {
		tryTrigger(event.textEditor, 'selection');
	});
	pendingSnippetSuggestTransaction = {
		id,
		documentUri,
		expectedSelectionTexts: expectedSelectionTextSequence,
		nextPlaceholderIndex: 0,
		selectionProbeCount: 0,
		selectionListener,
		cleanupTimer: setTimeout(() => undefined, 0),
		suggestDispatchScheduled: false,
		awaitingCompletionResponse: false,
	};
	resetSnippetSuggestTransactionTimeout(pendingSnippetSuggestTransaction, 'placeholderNotObserved');
	diagnostic('completion.transaction.armed', {
		transactionId: id,
		traceVersion: snippetSuggestTraceVersion,
		placeholderCount: expectedSelectionTextSequence.length,
	});
	tryTrigger(editor, 'command');
}

function wrapBridgeCompletionCommands(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
	transactionId: number,
): void {
	const items = !result ? [] : ('items' in result ? result.items : result);
	for (const item of items) {
		const originalCommand = item.command;
		item.command = {
			title: 'Advance enum snippet placeholder',
			command: languageClientCommands.advanceSnippetPlaceholderAfterAccept,
			arguments: [transactionId, originalCommand],
		};
	}
}

async function advanceSnippetPlaceholderAfterAccept(
	transactionId: unknown,
	originalCommand: unknown,
): Promise<void> {
	if (isVscodeCommand(originalCommand)) {
		await vscode.commands.executeCommand(
			originalCommand.command,
			...(originalCommand.arguments ?? []),
		);
	}

	const transaction = pendingSnippetSuggestTransaction;
	if (typeof transactionId !== 'number'
		|| transaction?.id !== transactionId
		|| transaction.nextPlaceholderIndex >= transaction.expectedSelectionTexts.length) {
		return;
	}

	diagnostic('completion.transaction.accepted', {
		transactionId,
		placeholderIndex: transaction.nextPlaceholderIndex - 1,
	});
	try {
		await vscode.commands.executeCommand('jumpToNextSnippetPlaceholder');
		diagnostic('completion.transaction.nextPlaceholderDispatched', {
			transactionId,
			placeholderIndex: transaction.nextPlaceholderIndex,
		});
	} catch {
		diagnostic('completion.transaction.nextPlaceholderDispatchError', {
			transactionId,
			placeholderIndex: transaction.nextPlaceholderIndex,
		});
	}
}

function isVscodeCommand(value: unknown): value is vscode.Command {
	return typeof value === 'object'
		&& value !== null
		&& 'command' in value
		&& typeof value.command === 'string';
}

function advanceSnippetSuggestTransaction(id: number): void {
	const transaction = pendingSnippetSuggestTransaction;
	if (!transaction || transaction.id !== id) {
		return;
	}
	transaction.suggestDispatchScheduled = false;
	transaction.awaitingCompletionResponse = false;
	transaction.nextPlaceholderIndex += 1;
	if (transaction.nextPlaceholderIndex >= transaction.expectedSelectionTexts.length) {
		clearSnippetSuggestTransaction(id);
		return;
	}
	resetSnippetSuggestTransactionTimeout(transaction, 'nextPlaceholderNotObserved');
	diagnostic('completion.transaction.awaitingNextPlaceholder', {
		transactionId: id,
		placeholderIndex: transaction.nextPlaceholderIndex,
		placeholderCount: transaction.expectedSelectionTexts.length,
	});
}

function resetSnippetSuggestTransactionTimeout(
	transaction: SnippetSuggestTransaction,
	reason: string,
): void {
	clearTimeout(transaction.cleanupTimer);
	transaction.cleanupTimer = setTimeout(() => {
		if (pendingSnippetSuggestTransaction?.id === transaction.id) {
			diagnostic('completion.transaction.abandoned', {
				transactionId: transaction.id,
				placeholderIndex: transaction.nextPlaceholderIndex,
				reason,
			});
			clearSnippetSuggestTransaction(transaction.id);
		}
	}, languageClientCompletion.snippetSuggestTransactionTimeoutMs);
}

function clearSnippetSuggestTransaction(expectedId?: number): void {
	const transaction = pendingSnippetSuggestTransaction;
	if (!transaction || (expectedId !== undefined && transaction.id !== expectedId)) {
		return;
	}
	transaction.selectionListener.dispose();
	clearTimeout(transaction.cleanupTimer);
	pendingSnippetSuggestTransaction = undefined;
}

function completionItemCount(result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined): number {
	if (!result) {
		return 0;
	}
	return 'items' in result ? result.items.length : result.length;
}

function completionPresentationMetadata(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): Record<string, string | number> {
	const items = !result ? [] : ('items' in result ? result.items : result);
	let plainRangeCount = 0;
	let insertReplaceRangeCount = 0;
	let invalidInsertReplaceRangeCount = 0;
	const firstRangeKinds: string[] = [];
	const firstFilterTextLengths: string[] = [];

	for (const item of items) {
		const range = item.range;
		if (!range) {
			continue;
		}
		if (range instanceof vscode.Range) {
			plainRangeCount += 1;
			if (firstRangeKinds.length < 3) {
				firstRangeKinds.push('plain');
				firstFilterTextLengths.push(String(item.filterText?.length ?? 0));
			}
			continue;
		}

		insertReplaceRangeCount += 1;
		if (!validInsertReplaceRange(range.inserting, range.replacing)) {
			invalidInsertReplaceRangeCount += 1;
		}
		if (firstRangeKinds.length < 3) {
			firstRangeKinds.push('insertReplace');
			firstFilterTextLengths.push(String(item.filterText?.length ?? 0));
		}
	}

	return {
		plainRangeCount,
		insertReplaceRangeCount,
		invalidInsertReplaceRangeCount,
		firstRangeKinds: firstRangeKinds.join(','),
		firstFilterTextLengths: firstFilterTextLengths.join(','),
	};
}

function validInsertReplaceRange(inserting: vscode.Range, replacing: vscode.Range): boolean {
	return inserting.start.isEqual(replacing.start)
		&& inserting.end.isBeforeOrEqual(replacing.end);
}

function workspaceWatcherPathKey(filePath: string): string {
	const normalized = path.resolve(filePath).replace(/\\/g, '/');
	return process.platform === 'win32' ? normalized.toLowerCase() : normalized;
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

async function debugHoverAtCursor(
	context: vscode.ExtensionContext,
	outputChannel: vscode.OutputChannel,
): Promise<void> {
	const startedAt = Date.now();
	diagnostic('command.debugHover.start');
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		vscode.window.showWarningMessage('Open an Enforce script file before running hover debug.');
		return;
	}
	if (editor.document.languageId !== languageClientLanguage.id) {
		vscode.window.showWarningMessage('Hover debug is only available for Enforce language files.');
		return;
	}

	const activeClient = client;
	if (!activeClient) {
		vscode.window.showWarningMessage('Reforger language server is not running.');
		return;
	}

	const position = editor.selection.active;
	const params = {
		textDocument: {
			uri: editor.document.uri.toString(),
		},
		position: {
			line: position.line,
			character: position.character,
		},
	};

	try {
		const report = await activeClient.sendRequest<string>(languageClientRequests.debugHover, params);
		const reportPath = await writeHoverDebugReport(context, editor, position, report);
		outputChannel.clear();
		outputChannel.appendLine(`Hover debug report written to: ${reportPath}`);
		outputChannel.appendLine('');
		outputChannel.appendLine(report);
		outputChannel.show(true);
		diagnostic('command.debugHover.complete', { elapsedMs: Date.now() - startedAt });
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Hover debug request failed: ${message}`);
		outputChannel.show(true);
		vscode.window.showWarningMessage(`Hover debug request failed: ${message}`);
		diagnostic('command.debugHover.error', { elapsedMs: Date.now() - startedAt });
	}
}

async function debugCompletionAtCursor(
	context: vscode.ExtensionContext,
	outputChannel: vscode.OutputChannel,
): Promise<void> {
	const startedAt = Date.now();
	diagnostic('command.debugCompletion.start');
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		vscode.window.showWarningMessage('Open an Enforce script file before running completion debug.');
		return;
	}
	if (editor.document.languageId !== languageClientLanguage.id) {
		vscode.window.showWarningMessage('Completion debug is only available for Enforce language files.');
		return;
	}

	const activeClient = client;
	if (!activeClient) {
		vscode.window.showWarningMessage('Reforger language server is not running.');
		return;
	}

	const position = editor.selection.active;
	const params = {
		textDocument: {
			uri: editor.document.uri.toString(),
		},
		position: {
			line: position.line,
			character: position.character,
		},
	};

	try {
		const report = await activeClient.sendRequest<string>(languageClientRequests.debugCompletion, params);
		const lifecycleTrace = completionLifecycleTraceForDocument(editor.document.uri.toString());
		const reportPath = await writeCompletionDebugReport(context, editor, position, `${lifecycleTrace}\n\n---\n\n${report}`);
		outputChannel.clear();
		outputChannel.appendLine(`Completion debug report written to: ${reportPath}`);
		outputChannel.appendLine('');
		outputChannel.appendLine(report);
		outputChannel.show(true);
		diagnostic('command.debugCompletion.complete', { elapsedMs: Date.now() - startedAt });
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Completion debug request failed: ${message}`);
		outputChannel.show(true);
		vscode.window.showWarningMessage(`Completion debug request failed: ${message}`);
		diagnostic('command.debugCompletion.error', { elapsedMs: Date.now() - startedAt });
	}
}

function completionLifecycleTraceForDocument(documentUri: string): string {
	const events = completionLifecycleTrace.filter(event => event.documentUri === documentUri);
	const lines = [
		'## Extension Completion Lifecycle Trace (temporary)',
		'',
		'Bounded to the latest 80 Enforce events in this extension host. It records no source text, cursor text, or completion payloads.',
		'',
	];
	if (events.length === 0) {
		lines.push('No lifecycle events were captured for this document.');
		return lines.join('\n');
	}
	lines.push('| Event | Fields |', '| --- | --- |');
	for (const event of events) {
		const fields = Object.entries(event.fields)
			.filter(([, value]) => value !== undefined)
			.map(([key, value]) => `${key}=${String(value)}`)
			.join(', ');
		lines.push(`| ${event.event} | ${fields || '<none>'} |`);
	}
	return lines.join('\n');
}

async function writeHoverDebugReport(
	context: vscode.ExtensionContext,
	editor: vscode.TextEditor,
	position: vscode.Position,
	report: string,
): Promise<string> {
	const folderPath = path.join(
		context.globalStorageUri.fsPath,
		languageClientLogs.rootFolder,
		languageClientLogs.hoverDebugFolder,
	);
	await fs.mkdir(folderPath, { recursive: true });

	const reportPath = path.join(folderPath, languageClientLogs.hoverDebugLatestFile);
	const prefix = [
		'# Reforger Hover Debug Log',
		'',
		`- Generated: ${new Date().toISOString()}`,
		`- Document URI: ${editor.document.uri.toString()}`,
		`- Document path: ${editor.document.uri.fsPath}`,
		`- Language ID: ${editor.document.languageId}`,
		`- Cursor: line ${position.line} character ${position.character} (UTF-16, zero-based)`,
		`- Source: VS Code command ${languageClientCommands.debugHoverAtCursor}`,
		'',
		'This file is overwritten by each hover-debug command run and is intentionally separate from the normal language-server runtime log.',
		'',
		'---',
		'',
	].join('\n');

	await fs.writeFile(reportPath, `${prefix}${report}\n`, 'utf8');
	return reportPath;
}

async function writeCompletionDebugReport(
	context: vscode.ExtensionContext,
	editor: vscode.TextEditor,
	position: vscode.Position,
	report: string,
): Promise<string> {
	const folderPath = path.join(
		context.globalStorageUri.fsPath,
		languageClientLogs.rootFolder,
		languageClientLogs.completionDebugFolder,
	);
	await fs.mkdir(folderPath, { recursive: true });

	const reportPath = path.join(folderPath, languageClientLogs.completionDebugLatestFile);
	const prefix = [
		'# Reforger Completion Debug Log',
		'',
		`- Generated: ${new Date().toISOString()}`,
		`- Document URI: ${editor.document.uri.toString()}`,
		`- Document path: ${editor.document.uri.fsPath}`,
		`- Language ID: ${editor.document.languageId}`,
		`- Cursor: line ${position.line} character ${position.character} (UTF-16, zero-based)`,
		`- Source: VS Code command ${languageClientCommands.debugCompletionAtCursor}`,
		'',
		'This file is overwritten by each completion-debug command run and is intentionally separate from the normal language-server runtime log.',
		'',
		'---',
		'',
	].join('\n');

	await fs.writeFile(reportPath, `${prefix}${report}\n`, 'utf8');
	return reportPath;
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

async function isDirectory(targetPath: string): Promise<boolean> {
	try {
		return (await fs.stat(targetPath)).isDirectory();
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
