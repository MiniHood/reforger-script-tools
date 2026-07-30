import * as fs from 'fs/promises';
import * as path from 'path';
import { createHash, randomUUID } from 'crypto';
import * as vscode from 'vscode';
import { gameDataConfig, gameDataStorage } from '../extensionConfig/gameData';

export type AddonRootKind = 'base-game' | 'workbench' | 'user-addons';
export type AddonRootOrigin = 'configured' | 'discovered' | 'missing';
export type AddonRootStatus = 'ready' | 'missing' | 'invalid';

export interface ResolvedAddonRoot {
	kind: AddonRootKind;
	origin: AddonRootOrigin;
	status: AddonRootStatus;
	candidates: string[];
	diagnostic: string;
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
	artifactIdentity: string;
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
			.sort((left, right) => ordinalCompare(addonKey(left), addonKey(right))),
	};
	const contents = `${JSON.stringify(inventory, null, 2)}\n`;
	const digest = createHash('sha256').update(contents).digest('hex');
	const inventoryPath = path.join(
		context.globalStorageUri.fsPath,
		gameDataStorage.rootFolder,
		`${gameDataStorage.inventoryPrefix}${digest}.json`,
	);
	await publishContentAddressedFile(inventoryPath, contents);
	return { inventory, path: inventoryPath };
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
			throw new Error(`Content-addressed add-on inventory is corrupt: ${targetPath}`);
		}
	} finally {
		await fs.rm(temporaryPath, { force: true });
	}
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
	return resolveRootFrom(kind, configured, defaultRootCandidates(kind));
}

export async function resolveRootFrom(
	kind: AddonRootKind,
	configured: string | undefined,
	candidates: string[],
	directoryProbe: (candidate: string) => Promise<boolean> = isDirectory,
): Promise<ResolvedAddonRoot> {
	if (configured) {
		return (await directoryProbe(configured))
			? readyRoot(kind, 'configured', configured, [configured])
			: {
				kind,
				origin: 'configured',
				status: 'invalid',
				path: configured,
				candidates: [configured],
				diagnostic: `Configured ${rootLabel(kind)} add-ons folder does not exist or is not a directory.`,
			};
	}
	for (const candidate of candidates) {
		if (await directoryProbe(candidate)) {
			return readyRoot(kind, 'discovered', candidate, candidates);
		}
	}
	return {
		kind,
		origin: 'missing',
		status: 'missing',
		candidates,
		diagnostic: `No ${rootLabel(kind)} add-ons folder was found.`,
	};
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
			const projectFile = project ? path.join(addonPath, project.name) : undefined;
			const packFiles = files
				.filter(file => file.isFile() && file.name.toLowerCase().endsWith('.pak'))
				.map(file => path.join(addonPath, file.name))
				.sort(ordinalCompare);
			return {
				rootKind: root.kind,
				directoryName: child.name,
				path: addonPath,
				...(projectFile ? { projectFile } : {}),
				packFiles,
				artifactIdentity: await artifactIdentity([...(projectFile ? [projectFile] : []), ...packFiles]),
			};
		}));
	return addons;
}

async function artifactIdentity(files: string[]): Promise<string> {
	const identities = await Promise.all(files.map(async file => {
		try {
			const stat = await fs.stat(file);
			return `${file}\0${stat.size}\0${stat.mtimeMs}`;
		} catch {
			return `${file}\0missing`;
		}
	}));
	return createHash('sha256').update(identities.join('\0')).digest('hex');
}

function addonKey(addon: LocalAddonInventoryEntry): string {
	return `${addon.rootKind}\0${addon.directoryName.toLowerCase()}`;
}

function ordinalCompare(left: string, right: string): number {
	return left < right ? -1 : left > right ? 1 : 0;
}

function readyRoot(
	kind: AddonRootKind,
	origin: Exclude<AddonRootOrigin, 'missing'>,
	resolvedPath: string,
	candidates: string[],
): ResolvedAddonRoot {
	return {
		kind,
		origin,
		status: 'ready',
		path: resolvedPath,
		candidates,
		diagnostic: `${rootLabel(kind)} add-ons folder is ready.`,
	};
}

function rootLabel(kind: AddonRootKind): string {
	switch (kind) {
		case 'base-game': return 'base-game';
		case 'workbench': return 'Workbench';
		case 'user-addons': return 'user';
	}
}

async function readDirectory(directory: string): Promise<import('fs').Dirent[]> {
	try {
		return await fs.readdir(directory, { withFileTypes: true });
	} catch {
		return [];
	}
}
