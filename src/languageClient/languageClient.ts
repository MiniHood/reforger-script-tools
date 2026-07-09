import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { LanguageClient, type LanguageClientOptions, type ServerOptions, TransportKind } from 'vscode-languageclient/node';
import { gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';
import {
	languageClientDocumentSelector,
	languageClientIds,
	languageClientLogs,
	languageClientServer,
} from '../extensionConfig/languageClient';
import { getManualScriptsFolderCandidate } from '../gameData/gameData';

let client: LanguageClient | undefined;

export function registerLanguageClientFeatures(context: vscode.ExtensionContext): void {
	const outputChannel = vscode.window.createOutputChannel(languageClientIds.name, { log: true });
	context.subscriptions.push(outputChannel);

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
	];
	const gameDataScripts = getGameDataScriptsPath(context);
	if (gameDataScripts) {
		serverArgs.push('--game-data-scripts', gameDataScripts);
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

	const devPath = path.join(context.extensionPath, ...languageClientServer.devBinaryRelativePath);
	if (await isFile(devPath)) {
		return devPath;
	}

	return undefined;
}

function getGameDataScriptsPath(context: vscode.ExtensionContext): string | undefined {
	const manualFolder = vscode.workspace
		.getConfiguration(gameDataConfig.section)
		.get<string>(gameDataConfig.settings.manualFolder);
	if (manualFolder?.trim()) {
		return getManualScriptsFolderCandidate(manualFolder);
	}

	return path.join(
		context.globalStorageUri.fsPath,
		gameDataStorage.rootFolder,
		gameDataStorage.scriptsFolder,
	);
}

async function isFile(targetPath: string): Promise<boolean> {
	try {
		return (await fs.stat(targetPath)).isFile();
	} catch {
		return false;
	}
}
