import * as fs from 'fs/promises';
import * as path from 'path';
import { createHash, randomUUID } from 'crypto';
import * as vscode from 'vscode';
import type { WorkbenchLoadedAddonGraph } from '../workbenchNetApi/gateway/workbenchGateway';
import { gameDataStorage } from '../extensionConfig/gameData';

export interface LoadedAddonSourceInventory {
	schema: 'reforger-workbench-loaded-addon-graph-v1';
	bridgeVersion: string;
	protocolVersion: 1;
	addons: WorkbenchLoadedAddonGraph['addons'];
}

/**
 * Publishes the exact graph which Workbench reports for this process. This
 * module deliberately never discovers add-on folders: a path not present in
 * Workbench's graph is not an index source.
 */
export async function writeLoadedAddonSourceInventory(
	context: vscode.ExtensionContext,
	graph: WorkbenchLoadedAddonGraph,
): Promise<string> {
	await removePreWorkbenchRuntimeStorage(context.globalStorageUri.fsPath);
	const inventory: LoadedAddonSourceInventory = {
		schema: 'reforger-workbench-loaded-addon-graph-v1',
		bridgeVersion: graph.bridgeVersion,
		protocolVersion: graph.protocolVersion,
		addons: graph.addons.map(addon => ({ ...addon })),
	};
	const contents = `${JSON.stringify(inventory, null, 2)}\n`;
	const digest = createHash('sha256').update(contents).digest('hex');
	const inventoryPath = path.join(
		context.globalStorageUri.fsPath,
		gameDataStorage.rootFolder,
		`${gameDataStorage.inventoryPrefix}${digest}.json`,
	);
	await publishContentAddressedFile(inventoryPath, contents);
	return inventoryPath;
}

export async function publishContentAddressedFile(targetPath: string, contents: string): Promise<void> {
	await fs.mkdir(path.dirname(targetPath), { recursive: true });
	const temporaryPath = path.join(
		path.dirname(targetPath),
		`.${path.basename(targetPath)}.${process.pid}.${randomUUID()}.tmp`,
	);
	const handle = await fs.open(temporaryPath, 'wx');
	try {
		await handle.writeFile(contents, { encoding: 'utf8' });
		await handle.sync();
	} finally {
		await handle.close();
	}
	try {
		await fs.link(temporaryPath, targetPath);
	} catch (error) {
		if (!isAlreadyExists(error)) {
			throw error;
		}
		const existing = await fs.readFile(targetPath, 'utf8');
		if (existing !== contents) {
			throw new Error(`Content-addressed Workbench add-on graph is corrupt: ${targetPath}`);
		}
	} finally {
		await fs.rm(temporaryPath, { force: true });
	}
}

async function removePreWorkbenchRuntimeStorage(globalStoragePath: string): Promise<void> {
	await Promise.all([
		fs.rm(path.join(globalStoragePath, 'game-data'), { recursive: true, force: true }),
		fs.rm(path.join(globalStoragePath, 'index-cache'), { recursive: true, force: true }),
		fs.rm(path.join(globalStoragePath, gameDataStorage.rootFolder, 'inventory-v1.json'), { force: true }),
	]);
}

function isAlreadyExists(error: unknown): boolean {
	return error instanceof Error && 'code' in error && error.code === 'EEXIST';
}
