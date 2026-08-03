import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { diagnosticsLogs } from '../extensionConfig/diagnostics';
import { gameDataStorage } from '../extensionConfig/gameData';
import { languageClientIndexCache } from '../extensionConfig/languageClient';
import { workbenchConfig, workbenchDefaults } from '../extensionConfig/workbench';

interface WorkbenchGraph {
	addons: Array<{ guid: string; id: string; title: string; sourceRoot: string }>;
}

interface AddonCacheHeader {
	guid: string;
	displayId: string;
	sourceRoot: string;
	packCount: number;
	scriptCount: number;
	indexBytes: number;
}

interface DiagnosticRecord {
	timestamp?: string;
	event?: string;
	phase?: string;
	scopeAuthority?: string;
	guid?: string;
	displayId?: string;
	cacheStatus?: string;
	cacheDetail?: string;
	loadedInstances?: number;
	rebuiltInstances?: number;
	missingInstances?: number;
	workspaceExcludedInstances?: number;
	timingsMs?: { total?: number; indexLoad?: number; sourceInspection?: number; layerCompose?: number };
}

export async function writeAddonIndexReport(context: vscode.ExtensionContext): Promise<string> {
	const storageRoot = path.join(context.globalStorageUri.fsPath, gameDataStorage.rootFolder);
	const cacheRoot = path.join(context.globalStorageUri.fsPath, languageClientIndexCache.rootFolder);
	const logsRoot = path.join(context.globalStorageUri.fsPath, diagnosticsLogs.rootFolder);
	const diagnosticLogPath = path.join(logsRoot, diagnosticsLogs.serverFile);
	const graph = await readJson<WorkbenchGraph>(path.join(storageRoot, gameDataStorage.inventoryFile));
	const caches = await readAddonCacheHeaders(cacheRoot);
	const diagnostics = await readDiagnosticRecords(diagnosticLogPath);
	const mode = vscode.workspace.getConfiguration(workbenchConfig.section).get<string>(
		workbenchConfig.settings.externalIndexMode,
		workbenchDefaults.externalIndexMode,
	);

	const reportPath = path.join(storageRoot, gameDataStorage.indexReportFile);
	await fs.mkdir(storageRoot, { recursive: true });
	const contents = [
		'# Reforger Add-on Index Report',
		'',
		`- Generated: ${new Date().toISOString()}`,
		`- Configured external-index mode: \`${mode}\``,
		`- Cache root: \`${cacheRoot}\``,
		`- Diagnostic log: \`${diagnosticLogPath}\``,
		'',
		'## Current Workbench graph snapshot',
		'',
		graph?.addons?.length
			? markdownTable(['GUID', 'ID', 'Title', 'Source root', 'Matching cache'], graph.addons.map(addon => {
				const matching = caches.find(cache => cache.guid.toUpperCase() === addon.guid.toUpperCase() && samePath(cache.sourceRoot, addon.sourceRoot));
				return [addon.guid, addon.id, addon.title, addon.sourceRoot, matching ? `yes (${matching.scriptCount} scripts)` : 'no'];
			}))
			: 'No Workbench graph is currently published.',
		'',
		'## Cached add-on indexes',
		'',
		caches.length
			? markdownTable(['GUID', 'Display ID', 'Source root', 'Packs', 'Scripts', 'Index bytes'], caches.map(cache => [
				cache.guid,
				cache.displayId,
				cache.sourceRoot,
				String(cache.packCount),
				String(cache.scriptCount),
				String(cache.indexBytes),
			]))
			: 'No compatible add-on cache headers were found.',
		'',
		'## Recent indexing lifecycle',
		'',
		diagnostics.length
			? markdownTable(['Timestamp', 'Event', 'Phase/scope', 'Add-on', 'Outcome/counts', 'Timings'], diagnostics.slice(-80).map(record => [
				record.timestamp ?? '',
				record.event ?? '',
				[record.phase, record.scopeAuthority].filter(Boolean).join(' / '),
				[record.guid, record.displayId].filter(Boolean).join(' '),
				formatDiagnosticCounts(record),
				formatDiagnosticTimings(record),
			]))
			: 'No language-server diagnostic records were found. Enable `reforgerScriptTools.diagnostics.enabled`, reload VS Code, reproduce the startup, and regenerate this report.',
		'',
		'## Interpretation',
		'',
		'- The Workbench graph is the authoritative live membership snapshot when Workbench is connected.',
		'- The cached index table shows what can be used before Workbench is available.',
		'- `offline` records describe dependency-cache hydration; `workbench-reconciliation` records describe the later live graph delta.',
		'- A graph row with `Matching cache: no` explains why that add-on cannot be served from offline cache yet.',
		'',
	].join('\n');
	await fs.writeFile(reportPath, contents, 'utf8');
	return reportPath;
}

async function readAddonCacheHeaders(cacheRoot: string): Promise<AddonCacheHeader[]> {
	let entries: Array<{ name: string; isDirectory(): boolean }>;
	try {
		entries = await fs.readdir(cacheRoot, { withFileTypes: true });
	} catch {
		return [];
	}
	const headers = await Promise.all(entries.filter(entry => entry.isDirectory()).map(async entry => {
		const directory = path.join(cacheRoot, entry.name);
		return await readJson<AddonCacheHeader>(path.join(directory, 'manifest-header.json'))
			?? await readJson<AddonCacheHeader>(path.join(directory, 'manifest.json'));
	}));
	return headers.filter((header): header is AddonCacheHeader => Boolean(header?.guid && header.sourceRoot));
}

async function readDiagnosticRecords(file: string): Promise<DiagnosticRecord[]> {
	try {
		const contents = await fs.readFile(file, 'utf8');
		return contents.split(/\r?\n/).filter(Boolean).flatMap(line => {
			try {
				return [JSON.parse(line) as DiagnosticRecord];
			} catch {
				return [];
			}
		});
	} catch {
		return [];
	}
}

async function readJson<T>(file: string): Promise<T | undefined> {
	try {
		return JSON.parse(await fs.readFile(file, 'utf8')) as T;
	} catch {
		return undefined;
	}
}

function samePath(left: string, right: string): boolean {
	return left.replaceAll('\\', '/').replace(/^\\\\\?\//, '').toLowerCase()
		=== right.replaceAll('\\', '/').replace(/^\\\\\?\//, '').toLowerCase();
}

function formatDiagnosticCounts(record: DiagnosticRecord): string {
	const counts = [
		record.cacheStatus,
		record.cacheDetail,
		record.loadedInstances === undefined ? undefined : `loaded=${record.loadedInstances}`,
		record.rebuiltInstances === undefined ? undefined : `rebuilt=${record.rebuiltInstances}`,
		record.missingInstances === undefined ? undefined : `missing=${record.missingInstances}`,
		record.workspaceExcludedInstances === undefined ? undefined : `workspaceExcluded=${record.workspaceExcludedInstances}`,
	].filter(Boolean);
	return counts.join('; ');
}

function formatDiagnosticTimings(record: DiagnosticRecord): string {
	const timings = record.timingsMs;
	if (!timings) {
		return '';
	}
	return [
		timings.total === undefined ? undefined : `total=${timings.total}ms`,
		timings.indexLoad === undefined ? undefined : `cache=${timings.indexLoad}ms`,
		timings.sourceInspection === undefined ? undefined : `inspect=${timings.sourceInspection}ms`,
		timings.layerCompose === undefined ? undefined : `layer=${timings.layerCompose}ms`,
	].filter(Boolean).join('; ');
}

function markdownTable(headers: string[], rows: string[][]): string {
	return [
		`| ${headers.join(' | ')} |`,
		`| ${headers.map(() => '---').join(' | ')} |`,
		...rows.map(row => `| ${row.map(escapeMarkdown).join(' | ')} |`),
	].join('\n');
}

function escapeMarkdown(value: string): string {
	return value.replaceAll('|', '\\|').replaceAll('\n', ' ');
}
