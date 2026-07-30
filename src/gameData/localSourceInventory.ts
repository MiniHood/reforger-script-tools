import * as fs from 'fs/promises';
import * as path from 'path';
import { createHash } from 'crypto';
import * as vscode from 'vscode';
import { gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';

export type AddonRootKind = 'base-game' | 'workbench' | 'user-addons';
export type AddonRootOrigin = 'configured' | 'discovered' | 'missing';

export interface ResolvedAddonRoot {
	kind: AddonRootKind;
	origin: AddonRootOrigin;
	path?: string;
}

export interface LocalSourceInventory {
	schema: 'reforger-addon-source-inventory-v1';
	roots: ResolvedAddonRoot[];
	addons: LocalAddonInventoryEntry[];
}

export interface LocalAddonInventoryEntry {
	rootKind: AddonRootKind;
	directoryName: string;
	path: string;
	projectFile?: string;
	packFiles: string[];
}

const rootKinds: readonly AddonRootKind[] = ['base-game', 'workbench', 'user-addons'];

export async function resolveAndWriteLocalSourceInventory(
	context: vscode.ExtensionContext,
): Promise<{ inventory: LocalSourceInventory; path: string }> {
	await removeLegacyRuntimeStorage(context.globalStorageUri.fsPath);
	const roots = await Promise.all(rootKinds.map(kind => resolveRoot(kind)));
	const inventory: LocalSourceInventory = {
		schema: 'reforger-addon-source-inventory-v1',
		roots,
		addons: (await Promise.all(roots.map(root => discoverAddons(root))))
			.flat()
			.sort((left, right) => addonKey(left).localeCompare(addonKey(right))),
	};
	const contents = `${JSON.stringify(inventory, null, 2)}\n`;
	const digest = createHash('sha256').update(contents).digest('hex');
	const inventoryPath = path.join(
		context.globalStorageUri.fsPath,
		gameDataStorage.rootFolder,
		`${gameDataStorage.inventoryPrefix}${digest}.json`,
	);
	await fs.mkdir(path.dirname(inventoryPath), { recursive: true });
	try {
		await fs.writeFile(inventoryPath, contents, { encoding: 'utf8', flag: 'wx' });
	} catch (error) {
		if (!isAlreadyExists(error)) {
			throw error;
		}
	}
	return { inventory, path: inventoryPath };
}

async function removeLegacyRuntimeStorage(globalStoragePath: string): Promise<void> {
	await Promise.all([
		fs.rm(path.join(globalStoragePath, 'game-data'), { recursive: true, force: true }),
		fs.rm(path.join(globalStoragePath, 'index-cache'), { recursive: true, force: true }),
		fs.rm(path.join(globalStoragePath, gameDataStorage.rootFolder, 'inventory-v1.json'), { force: true }),
	]);
}

function isAlreadyExists(error: unknown): boolean {
	return error instanceof Error && 'code' in error && error.code === 'EEXIST';
}

export function configuredRoot(kind: AddonRootKind): string | undefined {
	const setting = settingName(kind);
	const value = vscode.workspace.getConfiguration(gameDataConfig.section).get<string>(setting);
	return value?.trim() ? path.resolve(value.trim().replace(/^["']|["']$/g, '')) : undefined;
}

export function defaultRootCandidates(kind: AddonRootKind): string[] {
	const programFilesX86 = process.env['ProgramFiles(x86)'];
	const userProfile = process.env.USERPROFILE;
	switch (kind) {
		case 'base-game':
			return programFilesX86 ? [path.join(programFilesX86, 'Steam', 'steamapps', 'common', 'Arma Reforger', 'addons')] : [];
		case 'workbench':
			return programFilesX86 ? [path.join(programFilesX86, 'Steam', 'steamapps', 'common', 'Arma Reforger Tools', 'Workbench', 'addons')] : [];
		case 'user-addons':
			return userProfile ? [path.join(userProfile, 'Documents', 'My Games', 'ArmaReforger', 'addons')] : [];
	}
}

export function settingName(kind: AddonRootKind): string {
	switch (kind) {
		case 'base-game': return gameDataConfig.settings.baseGameAddonsFolder;
		case 'workbench': return gameDataConfig.settings.workbenchAddonsFolder;
		case 'user-addons': return gameDataConfig.settings.userAddonsFolder;
	}
}

async function resolveRoot(kind: AddonRootKind): Promise<ResolvedAddonRoot> {
	const configured = configuredRoot(kind);
	if (configured) {
		return (await isDirectory(configured))
			? { kind, origin: 'configured', path: configured }
			: { kind, origin: 'missing' };
	}
	for (const candidate of defaultRootCandidates(kind)) {
		if (await isDirectory(candidate)) {
			return { kind, origin: 'discovered', path: candidate };
		}
	}
	return { kind, origin: 'missing' };
}

async function isDirectory(candidate: string): Promise<boolean> {
	try { return (await fs.stat(candidate)).isDirectory(); } catch { return false; }
}

async function discoverAddons(root: ResolvedAddonRoot): Promise<LocalAddonInventoryEntry[]> {
	if (!root.path) {
		return [];
	}
	const children = await readDirectory(root.path);
	const addons = await Promise.all(children
		.filter(child => child.isDirectory())
		.map(async child => {
			const addonPath = path.join(root.path!, child.name);
			const files = await readDirectory(addonPath);
			const project = files.find(file => file.isFile() && file.name.toLowerCase() === 'addon.gproj');
			return {
				rootKind: root.kind,
				directoryName: child.name,
				path: addonPath,
				...(project ? { projectFile: path.join(addonPath, project.name) } : {}),
				packFiles: files
					.filter(file => file.isFile() && file.name.toLowerCase().endsWith('.pak'))
					.map(file => path.join(addonPath, file.name))
					.sort((left, right) => left.localeCompare(right)),
			};
		}));
	return addons;
}

function addonKey(addon: LocalAddonInventoryEntry): string {
	return `${addon.rootKind}\0${addon.directoryName.toLowerCase()}`;
}

async function readDirectory(directory: string): Promise<import('fs').Dirent[]> {
	try {
		return await fs.readdir(directory, { withFileTypes: true });
	} catch {
		return [];
	}
}
