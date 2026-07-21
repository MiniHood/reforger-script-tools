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
import {
	applyVersionedEditorEdits,
	isCurrentSingleCaret,
	type LspPosition,
	type LspTextEdit,
	type VersionedEditResponse,
} from './versionedEditorEdit';
import { registerHtmlHoverBridge } from './hoverBridge';
import {
	discoverWorkspaceScriptRoots,
	registerWorkspaceScriptWatchBridge,
} from './workspaceWatchBridge';
import { registerDebugCommandBridge } from './debugCommandBridge';
import { createCompletionMiddleware } from './completionMiddleware';
import { typingAssistRequest } from './typingAssistBridge';

let client: LanguageClient | undefined;
let clientDisposables: vscode.Disposable[] = [];
let devServerWatcher: vscode.FileSystemWatcher | undefined;
let watchedDevServerPath: string | undefined;
let restartTimer: NodeJS.Timeout | undefined;
let restartingClient = false;
let initialStartup: Promise<void> | undefined;
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
let pendingIfSpaceCommit: IfSpaceCommit | undefined;
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

interface IfSpaceCommit {
	documentUri: string;
	version: number;
	position: vscode.Position;
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

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): () => void {
	logLanguageClientStartupTiming(context, 'languageClientRegistrationStart');
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	const completionDebugOutputChannel = vscode.window.createOutputChannel(languageClientIds.completionDebugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(completionDebugOutputChannel);
	context.subscriptions.push(registerEnterTypingAssist());
	context.subscriptions.push(registerBlockCommentPair());
	context.subscriptions.push(registerEmptyCompletionRefresh());
	context.subscriptions.push(registerIfSpaceCommitCleanup());
	context.subscriptions.push(...registerDebugCommandBridge(
		context,
		() => client,
		debugOutputChannel,
		completionDebugOutputChannel,
		completionLifecycleTraceForDocument,
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
		languageClientCommands.normalizeIfSpaceCommit,
		(...args: unknown[]) => normalizeIfSpaceCommit(args),
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
			...createCompletionMiddleware({
				begin: (document, triggerKind) => {
					const transaction = pendingSnippetSuggestTransaction;
					recordCompletionLifecycle(document.uri.toString(), 'request', { requestVersion: document.version, triggerKind });
					return { transactionId: transaction?.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse ? transaction.id : undefined };
				},
				respond: (document, triggerKind, requestVersion, transactionId, result, elapsedMs) => {
					recordCompletionLifecycle(document.uri.toString(), 'response', { requestVersion, currentVersion: document.version, triggerKind, itemCount: completionItemCount(result), isIncomplete: isCompletionListIncomplete(result), elapsedMs });
					armEmptyCompletionRefresh(document, requestVersion, result);
					const transaction = pendingSnippetSuggestTransaction;
					if (transaction && transaction.id === transactionId && transaction.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse) {
						diagnostic('completion.transaction.response', { transactionId: transaction.id, triggerKind, itemCount: completionItemCount(result), elapsedMs, ...completionPresentationMetadata(result) });
						wrapBridgeCompletionCommands(result, transaction.id);
						advanceSnippetSuggestTransaction(transaction.id);
					}
				},
				fail: (document, triggerKind, requestVersion, transactionId, elapsedMs) => {
					recordCompletionLifecycle(document.uri.toString(), 'responseError', { requestVersion, triggerKind, elapsedMs });
					const transaction = pendingSnippetSuggestTransaction;
					if (transaction && transaction.id === transactionId && transaction.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse) {
						diagnostic('completion.transaction.responseError', { transactionId: transaction.id, triggerKind, elapsedMs });
						clearSnippetSuggestTransaction(transaction.id);
					}
				},
			}),
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

interface BlockCommentPairResponse extends VersionedEditResponse {}

interface EnterTypingAssistResponse extends VersionedEditResponse {}

function registerBlockCommentPair(): vscode.Disposable {
	let pending: BlockCommentPairTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.commentPair', { outcome: 'superseded', version: pending.version });
			pending = undefined;
		}
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}
		const position = blockCommentPairPosition(event.contentChanges);
		if (!position) {
			return;
		}
		const transaction: BlockCommentPairTransaction = {
			document: event.document,
			version: event.document.version,
			prePairPosition: event.contentChanges[0].range.start,
			position,
			caretReady: hasSingleEmptyCaretAt(event.document, position, event.document.version),
		};
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestBlockCommentPair(transaction, () => pending === transaction, () => {
					pending = undefined;
				});
			}
		});
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		const transaction = pending;
		if (!transaction || event.textEditor.document.uri.toString() !== transaction.document.uri.toString()) {
			return;
		}
		if (hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
			transaction.caretReady = true;
			void applyPendingBlockCommentPair(transaction, () => pending === transaction, () => {
				pending = undefined;
			});
			return;
		}
		if (transaction.document.version !== transaction.version
			|| !hasSingleEmptyCaretAt(transaction.document, transaction.prePairPosition, transaction.version)) {
			diagnostic('formatting.commentPair', { outcome: 'caretMoved', version: transaction.version });
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

interface BlockCommentPairTransaction {
	document: vscode.TextDocument;
	version: number;
	prePairPosition: vscode.Position;
	position: vscode.Position;
	caretReady: boolean;
	response?: BlockCommentPairResponse;
}

export function blockCommentPairPosition(
	changes: readonly vscode.TextDocumentContentChangeEvent[],
): vscode.Position | undefined {
	if (changes.length !== 1 || changes[0].rangeLength !== 0 || changes[0].text !== '**/') {
		return undefined;
	}
	const change = changes[0];
	return new vscode.Position(change.range.start.line, change.range.start.character + 1);
}

async function requestBlockCommentPair(
	transaction: BlockCommentPairTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	const activeClient = client;
	const editor = vscode.window.activeTextEditor;
	if (!activeClient || !editor || editor.document.uri.toString() !== transaction.document.uri.toString()
		|| transaction.document.version !== transaction.version) {
		diagnostic('formatting.commentPair', { outcome: 'rejectedEditorState', version: transaction.version });
		clear();
		return;
	}
	try {
		const response = await activeClient.sendRequest<BlockCommentPairResponse>(
			languageClientRequests.blockCommentPair,
			typingAssistRequest(transaction.document, transaction.position, editor),
		);
		if (!isCurrent() || transaction.document.version !== transaction.version || response.edits.length === 0) {
			diagnostic('formatting.commentPair', {
				outcome: response.edits.length === 0 ? 'noEdits' : 'staleResponse',
				version: transaction.version,
			});
			clear();
			return;
		}
		transaction.response = response;
		await applyPendingBlockCommentPair(transaction, isCurrent, clear);
	} catch {
		diagnostic('formatting.commentPair', { outcome: 'requestError', version: transaction.version });
		clear();
	}
}

async function applyPendingBlockCommentPair(
	transaction: BlockCommentPairTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	if (!transaction.response || !transaction.caretReady) {
		return;
	}
	if (!isCurrent() || transaction.document.version !== transaction.version
		|| !hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
		diagnostic('formatting.commentPair', { outcome: 'staleResponse', version: transaction.version });
		clear();
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		clear();
		return;
	}
	const applied = await applyVersionedEditorEdits(editor, transaction.response);
	diagnostic('formatting.commentPair', {
		outcome: applied ? 'applied' : 'editRejected',
		version: transaction.version,
		edits: transaction.response.edits.length,
	});
	clear();
}

function registerEnterTypingAssist(): vscode.Disposable {
	let pending: EnterTypingAssistTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.enter', { outcome: 'superseded', version: pending.version });
			pending = undefined;
		}
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}
		const editor = vscode.window.activeTextEditor;
		const enterPosition = enterAfterPosition(event.contentChanges);
		const tabPosition = editor && editor.document.uri.toString() === event.document.uri.toString()
			? tabAfterPosition(event.contentChanges)
			: undefined;
		const position = enterPosition ?? tabPosition;
		if (!position) {
			return;
		}
		const change = event.contentChanges[0];
		const transaction: EnterTypingAssistTransaction = {
			document: event.document,
			version: event.document.version,
			preEnterPosition: change.range.start,
			position,
			trigger: enterPosition ? '\n' : '\t',
			caretReady: hasSingleEmptyCaretAt(event.document, position, event.document.version),
		};
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestEnterTypingAssist(transaction, () => pending === transaction, () => {
					pending = undefined;
				});
			}
		});
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		const transaction = pending;
		if (!transaction || event.textEditor.document.uri.toString() !== transaction.document.uri.toString()) {
			return;
		}
		if (transaction.document.version !== transaction.version) {
			diagnostic('formatting.enter', { outcome: 'superseded', version: transaction.version });
			pending = undefined;
			return;
		}
		if (hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
			transaction.caretReady = true;
			void applyPendingEnterTypingAssist(transaction, () => pending === transaction, () => {
				pending = undefined;
			});
			return;
		}
		if (!hasSingleEmptyCaretAt(transaction.document, transaction.preEnterPosition, transaction.version)) {
			diagnostic('formatting.enter', { outcome: 'caretMoved', version: transaction.version });
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

interface EnterTypingAssistTransaction {
	document: vscode.TextDocument;
	version: number;
	preEnterPosition: vscode.Position;
	position: vscode.Position;
	trigger: '\n' | '\t';
	caretReady: boolean;
	response?: EnterTypingAssistResponse;
}

export function enterAfterPosition(
	changes: readonly vscode.TextDocumentContentChangeEvent[],
): vscode.Position | undefined {
	if (!isSinglePlainEnter(changes)) {
		return undefined;
	}
	const change = changes[0];
	const newline = change.text.lastIndexOf('\n');
	return new vscode.Position(change.range.start.line + 1, change.text.length - newline - 1);
}

function isSinglePlainEnter(changes: readonly vscode.TextDocumentContentChangeEvent[]): boolean {
	return changes.length === 1
		&& changes[0].rangeLength === 0
		&& /^\r?\n[\t ]*$/.test(changes[0].text);
}

async function requestEnterTypingAssist(
	transaction: EnterTypingAssistTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	const activeClient = client;
	const editor = vscode.window.activeTextEditor;
	if (!activeClient || !editor || editor.document.uri.toString() !== transaction.document.uri.toString()
		|| transaction.document.version !== transaction.version) {
		diagnostic('formatting.enter', { outcome: 'rejectedEditorState', version: transaction.version });
		clear();
		return;
	}
	diagnostic('formatting.enter', {
		outcome: 'admitted',
		version: transaction.version,
		line: transaction.position.line,
		character: transaction.position.character,
	});
	try {
		const response = await activeClient.sendRequest<EnterTypingAssistResponse>(
			languageClientRequests.enterTypingAssist,
			typingAssistRequest(transaction.document, transaction.position, editor, transaction.trigger),
		);
		if (!isCurrent() || transaction.document.version !== transaction.version) {
			diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'documentChanged' });
			clear();
			return;
		}
		if (response.edits.length === 0) {
			diagnostic('formatting.enter', { outcome: 'noEdits', version: transaction.version });
			clear();
			return;
		}
		transaction.response = response;
		if (!transaction.caretReady) {
			diagnostic('formatting.enter', { outcome: 'awaitingCaret', version: transaction.version });
		}
		await applyPendingEnterTypingAssist(transaction, isCurrent, clear);
	} catch {
		// A typing assist must never surface transport failures while the user edits.
		diagnostic('formatting.enter', { outcome: 'requestError', version: transaction.version });
		clear();
	}
}

async function applyPendingEnterTypingAssist(
	transaction: EnterTypingAssistTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	if (!transaction.response || !transaction.caretReady) {
		return;
	}
	if (!isCurrent() || transaction.document.version !== transaction.version
		|| !hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
		diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'caretMoved' });
		clear();
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		clear();
		return;
	}
	const applied = await applyVersionedEditorEdits(editor, transaction.response);
	diagnostic('formatting.enter', {
		outcome: applied ? 'applied' : 'editRejected',
		version: transaction.version,
		edits: transaction.response.edits.length,
	});
	clear();
}

export function isCurrentSingleTypingAssistCaret(
	documentVersion: number,
	expectedVersion: number,
	selectionCount: number,
	selectionIsEmpty: boolean,
	selectionActive: vscode.Position,
	expectedPosition: vscode.Position,
): boolean {
	return documentVersion === expectedVersion
		&& selectionCount === 1
		&& selectionIsEmpty
		&& selectionActive.isEqual(expectedPosition);
}

function hasSingleEmptyCaretAt(
	document: vscode.TextDocument,
	position: vscode.Position,
	expectedVersion: number,
): boolean {
	return isCurrentSingleCaret(document, expectedVersion, position);
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

export function tabAfterPosition(
	changes: readonly vscode.TextDocumentContentChangeEvent[],
): vscode.Position | undefined {
	if (changes.length !== 1 || changes[0].rangeLength !== 0) {
		return undefined;
	}
	const change = changes[0];
	if (change.text !== '\t') {
		return undefined;
	}
	return new vscode.Position(change.range.start.line, change.range.start.character + change.text.length);
}

/**
 * Removes only the commit character that VS Code appends after accepting the
 * Rust-authored `if ($0)` snippet with Space. This is a completion UI adapter,
 * not TypeScript syntax recognition: Rust attaches this command exclusively to
 * that item, and the exact caret-local postcondition is the whole admission
 * contract.
 */
async function normalizeIfSpaceCommit(args: readonly unknown[]): Promise<void> {
	const position = ifSpaceCommitPositionFromCommandArguments(args);
	if (!position) {
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor
		|| editor.document.languageId !== languageClientLanguage.id
		|| editor.selections.length !== 1
		|| !editor.selection.isEmpty) {
		return;
	}
	const deletion = new vscode.Range(position, position.translate(0, 1));
	if (editor.selection.active.isEqual(deletion.end)) {
		diagnostic('completion.ifSpaceCommit', { outcome: 'afterCommit' });
		await removeIfSpaceCommitCharacter(editor, deletion);
		return;
	}
	if (!editor.selection.active.isEqual(deletion.start)) {
		diagnostic('completion.ifSpaceCommit', { outcome: 'ignored' });
		return;
	}
	pendingIfSpaceCommit = {
		documentUri: editor.document.uri.toString(),
		version: editor.document.version,
		position: deletion.start,
	};
	diagnostic('completion.ifSpaceCommit', { outcome: 'awaitingCommitCharacter' });
}

export function ifSpaceCommitPositionFromCommandArguments(args: readonly unknown[]): vscode.Position | undefined {
	const [line, character] = args;
	if (typeof line !== 'string' || typeof character !== 'string'
		|| !/^\d+$/.test(line) || !/^\d+$/.test(character)) {
		return undefined;
	}
	const lineNumber = Number(line);
	const characterNumber = Number(character);
	if (!Number.isSafeInteger(lineNumber) || !Number.isSafeInteger(characterNumber)) {
		return undefined;
	}
	return new vscode.Position(lineNumber, characterNumber);
}

function registerIfSpaceCommitCleanup(): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		const pending = pendingIfSpaceCommit;
		if (!pending || event.document.uri.toString() !== pending.documentUri) {
			return;
		}
		pendingIfSpaceCommit = undefined;
		const [change] = event.contentChanges;
		if (event.document.version !== pending.version + 1
			|| event.contentChanges.length !== 1
			|| !change.range.isEmpty
			|| change.text !== ' '
			|| !change.range.start.isEqual(pending.position)) {
			diagnostic('completion.ifSpaceCommit', { outcome: 'unexpectedChange' });
			return;
		}
		const editor = vscode.window.activeTextEditor;
		if (!editor
			|| editor.document.uri.toString() !== pending.documentUri
			|| !editor.selection.active.isEqual(pending.position.translate(0, 1))) {
			diagnostic('completion.ifSpaceCommit', { outcome: 'postCommitShapeMissing' });
			return;
		}
		diagnostic('completion.ifSpaceCommit', { outcome: 'commitObserved' });
		void removeIfSpaceCommitCharacter(editor, new vscode.Range(pending.position, pending.position.translate(0, 1)));
	});
}

async function removeIfSpaceCommitCharacter(
	editor: vscode.TextEditor,
	deletion: vscode.Range,
): Promise<void> {
	const position = deletion.start;
	const applied = await editor.edit(edit => edit.delete(deletion), {
		undoStopBefore: false,
		undoStopAfter: false,
	});
	if (applied) {
		editor.selection = new vscode.Selection(position, position);
		diagnostic('completion.ifSpaceCommit', { outcome: 'normalized' });
	}
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
