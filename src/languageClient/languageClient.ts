import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { LanguageClient, type LanguageClientOptions, type ServerOptions, TransportKind } from 'vscode-languageclient/node';
import { gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';
import {
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

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): void {
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugHoverAtCursor,
		() => debugHoverAtCursor(context, debugOutputChannel),
	));
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.openSymbolLocation,
		(args: unknown) => openSymbolLocation(args),
	));

	void startLanguageClient(context, outputChannel);
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
	const serverPath = await resolveServerPath(context);
	if (!serverPath) {
		outputChannel.appendLine('Language server binary was not found. Run npm run build-server during development.');
		return;
	}
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
		markdown: {
			isTrusted: true,
			supportHtml: true,
		},
		middleware: {
			provideHover: () => null,
		},
	};

	client = new LanguageClient(
		languageClientIds.id,
		languageClientIds.name,
		serverOptions,
		clientOptions,
	);

	try {
		await client.start();
		outputChannel.appendLine(`Language server started: ${serverPath}`);
		if (workspaceScriptRoots.length > 0) {
			outputChannel.appendLine(`Workspace script roots: ${workspaceScriptRoots.join('; ')}`);
		}
		clientDisposables.push(registerHtmlHoverProvider(client, outputChannel));
		clientDisposables.push(...registerWorkspaceScriptWatchers(context, client, outputChannel));
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Language server failed to start: ${message}`);
		vscode.window.showWarningMessage(`Reforger language server failed to start: ${message}`);
	}
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
	const pending = new Map<string, 'changed' | 'deleted'>();
	let timer: NodeJS.Timeout | undefined;

	const flush = (): void => {
		const entries = [...pending.entries()];
		pending.clear();
		timer = undefined;
		void Promise.all(entries.map(async ([filePath, kind]) => {
			if (kind === 'deleted') {
				activeClient.sendNotification(languageClientNotifications.workspaceFileDeleted, { path: filePath });
				return;
			}

			try {
				const text = await fs.readFile(filePath, 'utf8');
				activeClient.sendNotification(languageClientNotifications.workspaceFileChanged, { path: filePath, text });
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
		pending.set(uri.fsPath, kind);
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
