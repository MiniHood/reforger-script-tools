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
	languageClientRequests,
	languageClientServer,
} from '../extensionConfig/languageClient';
import { getManualScriptsFolderCandidate } from '../gameData/gameData';

let client: LanguageClient | undefined;

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): void {
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	const debugOutputChannel = vscode.window.createOutputChannel(languageClientIds.debugOutputName);
	context.subscriptions.push(outputChannel);
	context.subscriptions.push(debugOutputChannel);
	context.subscriptions.push(vscode.commands.registerCommand(
		languageClientCommands.debugHoverAtCursor,
		() => debugHoverAtCursor(context, debugOutputChannel),
	));

	void startLanguageClient(context, outputChannel);
}

export async function deactivateLanguageClient(): Promise<void> {
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
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		outputChannel.appendLine(`Language server failed to start: ${message}`);
		vscode.window.showWarningMessage(`Reforger language server failed to start: ${message}`);
	}
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
