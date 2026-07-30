import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { gameDataCommands, gameDataStorage } from '../extensionConfig/gameData';

/** Source authority is the Workbench-loaded add-on graph. This module never
 * searches disks for candidate game, Workbench, or downloaded add-on folders. */
export function registerGameDataFeatures(
	context: vscode.ExtensionContext,
	onGameDataSourceChanged?: () => Promise<void>,
): void {
	context.subscriptions.push(
		vscode.commands.registerCommand(gameDataCommands.refreshSources, async () => {
			await onGameDataSourceChanged?.();
		}),
		vscode.commands.registerCommand(gameDataCommands.openStorageFolder, async () => {
			const storageRoot = path.join(context.globalStorageUri.fsPath, gameDataStorage.rootFolder);
			await fs.mkdir(storageRoot, { recursive: true });
			await vscode.env.openExternal(vscode.Uri.file(storageRoot));
		}),
	);
}
