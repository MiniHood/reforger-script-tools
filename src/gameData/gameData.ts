import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { diagnostic } from '../diagnostics/diagnostics';
import { gameDataCommands, gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';
import {
	type AddonRootKind,
	resolveAndWriteLocalSourceInventory,
	settingName,
} from './localSourceInventory';

export async function resolveLocalSourceInventoryPath(context: vscode.ExtensionContext): Promise<string> {
	return (await resolveAndWriteLocalSourceInventory(context)).path;
}

export function registerGameDataFeatures(
	context: vscode.ExtensionContext,
	onGameDataSourceChanged?: () => Promise<void>,
): void {
	context.subscriptions.push(
		vscode.commands.registerCommand(gameDataCommands.refreshSources, async () => {
			await refreshSources(context, onGameDataSourceChanged, true);
		}),
		vscode.commands.registerCommand(gameDataCommands.openStorageFolder, async () => {
			const storageRoot = path.join(context.globalStorageUri.fsPath, gameDataStorage.rootFolder);
			await fs.mkdir(storageRoot, { recursive: true });
			await vscode.env.openExternal(vscode.Uri.file(storageRoot));
		}),
		...rootCommands(context, onGameDataSourceChanged),
	);
	// The language client resolves the inventory before its first launch. This
	// second pass owns prompting only and must not restart a freshly started
	// server.
	void refreshSources(context, undefined, false);
}

function rootCommands(context: vscode.ExtensionContext, changed?: () => Promise<void>): vscode.Disposable[] {
	return [
		[gameDataCommands.selectBaseGameAddonsFolder, 'base-game'],
		[gameDataCommands.selectWorkbenchAddonsFolder, 'workbench'],
		[gameDataCommands.selectUserAddonsFolder, 'user-addons'],
	].map(([command, kind]) => vscode.commands.registerCommand(command, () => selectRoot(
		context,
		kind as AddonRootKind,
		changed,
	)));
}

async function refreshSources(context: vscode.ExtensionContext, changed?: () => Promise<void>, showResult = false): Promise<void> {
	const startedAt = Date.now();
	try {
		const result = await resolveAndWriteLocalSourceInventory(context);
		const base = result.inventory.roots.find(root => root.kind === 'base-game');
		if (!base?.path || base.status !== 'ready') {
			const choice = await vscode.window.showWarningMessage(
				base?.diagnostic ?? 'Reforger Script Tools could not find the Arma Reforger add-ons folder.',
				'Choose Base Game Add-ons Folder',
			);
			if (choice) {
				await selectRoot(context, 'base-game', changed);
			}
			return;
		}
		await changed?.();
		if (showResult) {
			vscode.window.showInformationMessage('Reforger local add-on sources refreshed.');
		}
		diagnostic('gameData.localSources', { outcome: 'complete', elapsedMs: Date.now() - startedAt });
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		vscode.window.showWarningMessage(`Reforger local source discovery failed: ${message}`);
		diagnostic('gameData.localSources', { outcome: 'error', elapsedMs: Date.now() - startedAt });
	}
}

async function selectRoot(context: vscode.ExtensionContext, kind: AddonRootKind, changed?: () => Promise<void>): Promise<void> {
	const selected = await vscode.window.showOpenDialog({
		canSelectFiles: false,
		canSelectFolders: true,
		canSelectMany: false,
		openLabel: 'Use Add-ons Folder',
		title: `Select ${rootTitle(kind)} add-ons folder`,
	});
	const folder = selected?.[0]?.fsPath;
	if (!folder) {
		return;
	}
	await vscode.workspace.getConfiguration(gameDataConfig.section).update(
		settingName(kind), folder, vscode.ConfigurationTarget.Global,
	);
	await refreshSources(context, changed, true);
}

function rootTitle(kind: AddonRootKind): string {
	switch (kind) {
		case 'base-game': return 'Arma Reforger base game';
		case 'workbench': return 'Arma Reforger Tools / Workbench';
		case 'user-addons': return 'Arma Reforger user';
	}
}
