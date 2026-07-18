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
import {
	languageClientCompletion,
	languageClientCrashHandling,
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
let deletionCompletionTimer: NodeJS.Timeout | undefined;
let insertionCompletionTimer: NodeJS.Timeout | undefined;
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
}

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): void {
	logLanguageClientStartupTiming(context, 'languageClientRegistrationStart');
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	const completionDebugOutputChannel = vscode.window.createOutputChannel(languageClientIds.completionDebugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(completionDebugOutputChannel);
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugHoverAtCursor,
		() => debugHoverAtCursor(context, debugOutputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugCompletionAtCursor,
		() => debugCompletionAtCursor(context, completionDebugOutputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.openSymbolLocation,
		(args: unknown) => openSymbolLocation(args),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.triggerSuggestAtSnippetPlaceholderEnd,
		() => triggerSuggestAtSnippetPlaceholderEnd(context, outputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.jumpToNextSnippetPlaceholderAndTriggerSuggest,
		() => jumpToNextSnippetPlaceholderAndTriggerSuggest(context, outputChannel),
	));
	context.subscriptions.push(registerCompletionRetriggerOnTextEdit(outputChannel));
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

function registerCompletionRetriggerOnTextEdit(outputChannel: vscode.LogOutputChannel): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}

		const changedDocumentUri = event.document.uri.toString();

		if (event.contentChanges.some(isDeletionChange)) {
			if (deletionCompletionTimer) {
				clearTimeout(deletionCompletionTimer);
			}
			deletionCompletionTimer = setTimeout(() => {
				deletionCompletionTimer = undefined;
				triggerCompletionWhenActive(
					changedDocumentUri,
					shouldRetriggerCompletionAfterDeletion,
					outputChannel,
					'deletion',
				);
			}, languageClientCompletion.deletionRetriggerDebounceMs);
		}

		if (event.contentChanges.some(isIdentifierInsertionChange)) {
			if (insertionCompletionTimer) {
				clearTimeout(insertionCompletionTimer);
			}
			insertionCompletionTimer = setTimeout(() => {
				insertionCompletionTimer = undefined;
				triggerCompletionWhenActive(
					changedDocumentUri,
					shouldRetriggerCompletionAfterInsertion,
					outputChannel,
					'identifier insertion',
				);
			}, languageClientCompletion.insertionRetriggerDebounceMs);
		}
	});
}

function isDeletionChange(change: vscode.TextDocumentContentChangeEvent): boolean {
	const rangeLength = change.rangeLength ?? change.range.end.character - change.range.start.character;
	return rangeLength > change.text.length || (change.text.length === 0 && !change.range.isEmpty);
}

function isIdentifierInsertionChange(change: vscode.TextDocumentContentChangeEvent): boolean {
	return change.range.isEmpty && /^[A-Za-z0-9_]$/.test(change.text);
}

function shouldRetriggerCompletionAfterDeletion(editor: vscode.TextEditor): boolean {
	const position = editor.selection.active;
	if (!editor.selection.isEmpty) {
		return false;
	}
	const linePrefix = editor.document.lineAt(position.line).text.slice(0, position.character);
	return /[A-Za-z0-9_\.]$/.test(linePrefix);
}

function shouldRetriggerCompletionAfterInsertion(editor: vscode.TextEditor): boolean {
	const position = editor.selection.active;
	if (!editor.selection.isEmpty) {
		return false;
	}
	const linePrefix = editor.document.lineAt(position.line).text.slice(0, position.character);
	const wordMatch = /[A-Za-z_][A-Za-z0-9_]*$/.exec(linePrefix);
	if (!wordMatch || wordMatch[0].length < 2) {
		return false;
	}
	return true;
}

function triggerCompletionWhenActive(
	documentUri: string,
	shouldTrigger: (editor: vscode.TextEditor) => boolean,
	outputChannel: vscode.LogOutputChannel,
	reason: string,
): void {
	const activeEditor = vscode.window.activeTextEditor;
	if (
		!activeEditor
		|| activeEditor.document.uri.toString() !== documentUri
		|| activeEditor.document.languageId !== languageClientLanguage.id
	) {
		return;
	}
	if (!shouldTrigger(activeEditor)) {
		return;
	}
	outputChannel.debug(`Triggering Enforce completion after ${reason}: ${documentUri}`);
	void vscode.commands.executeCommand('editor.action.triggerSuggest');
}

async function jumpToNextSnippetPlaceholderAndTriggerSuggest(
	context: vscode.ExtensionContext,
	outputChannel: vscode.LogOutputChannel,
): Promise<void> {
	try {
		outputChannel.debug('Completion follow-up: jump to next snippet placeholder and trigger suggest.');
		await delay(25);
		await vscode.commands.executeCommand('jumpToNextSnippetPlaceholder');
		await delay(25);
		const placeholderCount = orientSelectedEnumOwnerPlaceholderToActiveEnd();
		logLanguageClientStartupTiming(context, 'completionFollowupJumpAndSuggest', {
			enumOwnerPlaceholders: placeholderCount,
		});
		await delay(50);
		await vscode.commands.executeCommand('editor.action.triggerSuggest');
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.debug(`Completion follow-up failed: ${message}`);
		logLanguageClientStartupTiming(context, 'completionFollowupJumpAndSuggestFailed', {
			message,
		});
	}
}

async function triggerSuggestAtSnippetPlaceholderEnd(
	context: vscode.ExtensionContext,
	outputChannel: vscode.LogOutputChannel,
): Promise<void> {
	try {
		outputChannel.debug('Completion follow-up: trigger suggest at snippet placeholder end.');
		await delay(25);
		const placeholderCount = orientSelectedEnumOwnerPlaceholderToActiveEnd();
		logLanguageClientStartupTiming(context, 'completionFollowupTriggerSuggest', {
			enumOwnerPlaceholders: placeholderCount,
		});
		await delay(50);
		await vscode.commands.executeCommand('editor.action.triggerSuggest');
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.debug(`Completion follow-up failed: ${message}`);
		logLanguageClientStartupTiming(context, 'completionFollowupTriggerSuggestFailed', {
			message,
		});
	}
}

function orientSelectedEnumOwnerPlaceholderToActiveEnd(): number {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== languageClientLanguage.id) {
		return 0;
	}

	let placeholderCount = 0;
	const nextSelections = editor.selections.map(selection => {
		if (selection.isEmpty) {
			return selection;
		}
		const text = editor.document.getText(selection);
		if (!/^[A-Za-z_][A-Za-z0-9_]*\.$/.test(text)) {
			return selection;
		}
		placeholderCount += 1;
		return new vscode.Selection(selection.start, selection.end);
	});

	if (placeholderCount > 0) {
		editor.selections = nextSelections;
	}
	return placeholderCount;
}

function delay(ms: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, ms));
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
	disposeClientDisposables();
	devServerWatcher?.dispose();
	devServerWatcher = undefined;
	if (restartTimer) {
		clearTimeout(restartTimer);
		restartTimer = undefined;
	}
	if (deletionCompletionTimer) {
		clearTimeout(deletionCompletionTimer);
		deletionCompletionTimer = undefined;
	}
	if (insertionCompletionTimer) {
		clearTimeout(insertionCompletionTimer);
		insertionCompletionTimer = undefined;
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
			try {
				const hover = await activeClient.sendRequest<LspHoverResponse | null>(
					'textDocument/hover',
					{
						textDocument: { uri: document.uri.toString() },
						position: { line: position.line, character: position.character },
					},
					token,
				);
				return hover ? hoverFromLspResponse(hover) : null;
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`HTML hover request failed for ${document.uri.toString()}: ${message}`);
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
		pending.clear();
		timer = undefined;
		void Promise.all(entries.map(async ([, entry]) => {
			const { path: filePath, kind, sequence } = entry;
			if (kind === 'deleted') {
				activeClient.sendNotification(languageClientNotifications.workspaceFileDeleted, { path: filePath, sequence });
				return;
			}

			try {
				const text = await fs.readFile(filePath, 'utf8');
				activeClient.sendNotification(languageClientNotifications.workspaceFileChanged, { path: filePath, text, sequence });
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`Workspace script change skipped for ${filePath}: ${message}`);
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
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Hover debug request failed: ${message}`);
		outputChannel.show(true);
		vscode.window.showWarningMessage(`Hover debug request failed: ${message}`);
	}
}

async function debugCompletionAtCursor(
	context: vscode.ExtensionContext,
	outputChannel: vscode.OutputChannel,
): Promise<void> {
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
		const reportPath = await writeCompletionDebugReport(context, editor, position, report);
		outputChannel.clear();
		outputChannel.appendLine(`Completion debug report written to: ${reportPath}`);
		outputChannel.appendLine('');
		outputChannel.appendLine(report);
		outputChannel.show(true);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Completion debug request failed: ${message}`);
		outputChannel.show(true);
		vscode.window.showWarningMessage(`Completion debug request failed: ${message}`);
	}
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
