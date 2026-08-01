import * as fs from 'fs/promises';
import * as path from 'path';
import { gameDataStorage } from '../extensionConfig/gameData';
import { languageClientIndexCache } from '../extensionConfig/languageClient';

const baseGameGuid = '58D0FB3206B6F859';

interface WorkbenchGraph {
	addons?: Array<{ guid?: string; sourceRoot?: string }>;
}

interface AddonCacheHeader {
	guid?: string;
	sourceRoot?: string;
	indexFile?: string;
}

/**
 * Resolve the parser-owned base-game cache selected by the current Workbench
 * graph. The fixed legacy path remains useful as the unavailable-cache
 * fallback when indexing has not published a cache yet.
 */
export async function resolveBaseGameIndexCache(globalStoragePath: string): Promise<string> {
	const cacheRoot = path.join(globalStoragePath, languageClientIndexCache.rootFolder);
	const fallback = path.join(
		globalStoragePath,
		languageClientIndexCache.rootFolder,
		languageClientIndexCache.baseGameIndexFile,
	);
	const graph = await readJson<WorkbenchGraph>(path.join(
		globalStoragePath,
		gameDataStorage.rootFolder,
		gameDataStorage.inventoryFile,
	));
	const graphBaseGame = graph?.addons?.find(addon => addon.guid?.toUpperCase() === baseGameGuid);
	const candidates = await readBaseGameCandidates(cacheRoot);
	const matchingGraphCandidate = graphBaseGame?.sourceRoot
		? candidates.find(candidate => samePath(candidate.sourceRoot, graphBaseGame.sourceRoot!))
		: undefined;
	if (matchingGraphCandidate) {
		return matchingGraphCandidate.indexPath;
	}
	if (!graphBaseGame && candidates.length === 1) {
		return candidates[0].indexPath;
	}
	return fallback;
}

async function readBaseGameCandidates(cacheRoot: string): Promise<Array<{
	indexPath: string;
	sourceRoot: string;
}>> {
	let entries: Array<{ name: string; isDirectory(): boolean }>;
	try {
		entries = await fs.readdir(cacheRoot, { withFileTypes: true });
	} catch {
		return [];
	}
	const candidates = await Promise.all(entries.filter(entry => entry.isDirectory()).map(async entry => {
		const directory = path.join(cacheRoot, entry.name);
		const header = await readJson<AddonCacheHeader>(path.join(directory, 'manifest-header.json'));
		if (
			header?.guid?.toUpperCase() !== baseGameGuid
			|| !header.sourceRoot
			|| header.indexFile !== 'symbols.bin'
		) {
			return undefined;
		}
		const indexPath = path.join(directory, header.indexFile);
		return await fileExists(indexPath) ? { indexPath, sourceRoot: header.sourceRoot } : undefined;
	}));
	return candidates.filter((candidate): candidate is {
		indexPath: string;
		sourceRoot: string;
	} => candidate !== undefined);
}

async function readJson<T>(file: string): Promise<T | undefined> {
	try {
		return JSON.parse(await fs.readFile(file, 'utf8')) as T;
	} catch {
		return undefined;
	}
}

async function fileExists(file: string): Promise<boolean> {
	try {
		await fs.access(file);
		return true;
	} catch {
		return false;
	}
}

function samePath(left: string, right: string): boolean {
	return normalizePath(left) === normalizePath(right);
}

function normalizePath(value: string): string {
	return value.replaceAll('\\', '/').replace(/^\/+\?\//, '').toLowerCase();
}
