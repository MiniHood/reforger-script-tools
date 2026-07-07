import * as vscode from 'vscode';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import JSZip from 'jszip';
import { ExtensionLogger } from '../core/logger';
import type { EnforceSymbolIndex } from '../language/index/symbolIndex';

interface ExportSettings {
	manualGameScriptDataPath: string;
	checkGameScriptUpdates: boolean;
}

interface RefreshUi {
	notification: vscode.Progress<{ message?: string; increment?: number }>;
	statusBar: vscode.StatusBarItem;
}

interface ScriptSourceMetadata {
	source: 'github' | 'manual';
	repository: string;
	branch: string;
	archiveUrl: string;
	sourceSha?: string;
	downloadedAt: string;
	scriptCount: number;
}

export interface GameScriptDataStatus {
	available: boolean;
	changed: boolean;
	indexed?: boolean;
	skipped?: boolean;
}

export interface GitHubScriptArchiveEntry {
	name: string;
	dir: boolean;
	async(type: 'uint8array'): Promise<Uint8Array>;
}

const settingsSection = 'reforgerScriptTools';
const githubScriptRepository = 'BohemiaInteractive/Arma-Reforger-Script-Diff';
const githubScriptBranch = 'main';
const githubScriptApiBaseUrl = `https://api.github.com/repos/${githubScriptRepository}`;
const githubScriptArchiveUrl = `https://codeload.github.com/${githubScriptRepository}/zip/refs/heads/${githubScriptBranch}`;
const scriptWriteBatchSize = 200;
const minimumExpectedGameScriptFiles = 5000;
let gameDataPromptShown = false;

export function registerGameDataExportCommands(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	symbolIndex: EnforceSymbolIndex
): void {
	context.subscriptions.push(
		vscode.commands.registerCommand('reforger-script-tools.refreshGameData', async () => {
			await refreshGameData(context, output, logger, symbolIndex, true);
		})
	);
}

export async function ensureGameScriptData(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	symbolIndex: EnforceSymbolIndex
): Promise<GameScriptDataStatus> {
	const settings = getExportSettings(context);
	if (await hasExportedScriptData(context)) {
		logger.info('Downloaded Reforger game script data detected.');
		if (!settings.checkGameScriptUpdates) {
			return { available: true, changed: false };
		}
		return await ensureScriptDataCurrent(context, output, logger, symbolIndex);
	}
	if (settings.manualGameScriptDataPath) {
		const changed = await importManualScriptDataFolder(context, settings.manualGameScriptDataPath, output, logger);
		if (changed) {
			return { available: true, changed };
		}
	}

	gameDataPromptShown = true;
	logger.info('Downloaded Reforger game script data not detected during startup.');
	const choice = await vscode.window.showInformationMessage(
		'Official Reforger script data is not detected. Download it from Bohemia Interactive GitHub or select an existing scripts folder for completions, hover, and navigation.',
		'Download Script Data',
		'Select Folder'
	);

	if (choice === 'Download Script Data') {
		const changed = await refreshGameData(context, output, logger, symbolIndex);
		return { available: await hasExportedScriptData(context), changed, indexed: changed };
	}
	if (choice === 'Select Folder') {
		const changed = await selectManualScriptDataFolder(context, output, logger);
		return { available: await hasExportedScriptData(context), changed };
	}

	logger.warn('Script data setup skipped by user.');
	return { available: false, changed: false, skipped: true };
}

export async function refreshGameData(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	symbolIndex: EnforceSymbolIndex,
	showSuccessMessage = false
): Promise<boolean> {
	return vscode.window.withProgress(
		{
			location: vscode.ProgressLocation.Notification,
			cancellable: false,
		},
		async notification => {
			const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 95);
			const ui: RefreshUi = { notification, statusBar };
			let keepSuccessStatus = false;
			try {
				output.show(true);
				output.appendLine('Refresh Game Data started.');
				logger.info('Refresh Game Data started.');
				reportRefreshProgress(ui, 'Downloading official BI scripts...');
				const exported = await exportGameScriptData(context, output, logger, ui);
				if (!exported) {
					output.appendLine('Refresh Game Data stopped before index compilation.');
					return false;
				}

				logger.info('Compiling Reforger symbol index from downloaded BI scripts.');
				statusBar.dispose();
				const stats = await symbolIndex.refresh(false, {
					reason: 'export',
					forceGameDataRebuild: true,
					progress: message => notification.report({ message }),
				});
				if (!stats) {
					output.appendLine('Refresh Game Data stopped because index compilation failed.');
					return false;
				}
				output.appendLine('Refresh Game Data complete.');
				logger.info('Refresh Game Data complete.');
				if (showSuccessMessage) {
					vscode.window.showInformationMessage('Official BI Reforger script data downloaded and indexed.');
				}
				keepSuccessStatus = true;
				const successStatus = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 95);
				successStatus.text = '$(check) Reforger script data downloaded and indexed';
				successStatus.tooltip = 'Official BI Reforger script data refresh completed.';
				successStatus.show();
				setTimeout(() => successStatus.dispose(), 3000);
				return true;
			} finally {
				if (!keepSuccessStatus) {
					statusBar.dispose();
				}
			}
		}
	);
}

function reportRefreshProgress(ui: RefreshUi, message: string): void {
	ui.notification.report({ message });
	ui.statusBar.text = `$(sync~spin) ${message}`;
	ui.statusBar.tooltip = 'Refreshing Reforger script data.';
	ui.statusBar.show();
}

async function ensureScriptDataCurrent(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	symbolIndex: EnforceSymbolIndex
): Promise<GameScriptDataStatus> {
	const metadata = await readScriptSourceMetadata(context);
	const latestSha = await getLatestGitHubScriptsSha(logger);
	if (!latestSha || metadata?.sourceSha === latestSha) {
		logger.info(latestSha ? 'Downloaded BI script data is up to date.' : 'Could not check BI script updates; using existing downloaded script data.');
		return { available: true, changed: false };
	}

	logger.info(`Downloaded BI script data is stale. local=${metadata?.sourceSha ?? 'unknown'} latest=${latestSha}`);
	const choice = await vscode.window.showInformationMessage(
		'Official BI Reforger script data has an update available.',
		'Update Script Data',
		'Select Folder'
	);
	if (choice === 'Update Script Data') {
		const changed = await refreshGameData(context, output, logger, symbolIndex);
		return { available: await hasExportedScriptData(context), changed, indexed: changed };
	}
	if (choice === 'Select Folder') {
		const changed = await selectManualScriptDataFolder(context, output, logger);
		return { available: await hasExportedScriptData(context), changed };
	}
	return { available: true, changed: false, skipped: true };
}

async function exportGameScriptData(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	ui: RefreshUi
): Promise<boolean> {
	output.show(true);

	try {
		const exportPath = getConfiguredExportedGameDataPath(context);
		await fs.mkdir(exportPath, { recursive: true });
		output.appendLine(`GitHub source: ${githubScriptRepository}#${githubScriptBranch}`);
		output.appendLine(`Export folder: ${exportPath}`);
		logger.info(`Downloading script data from ${githubScriptArchiveUrl} to ${exportPath}`);

		const archiveBytes = await downloadGitHubScriptArchive(output, logger);
		reportRefreshProgress(ui, 'Reading downloaded script archive...');
		const zip = await JSZip.loadAsync(archiveBytes);
		const entries = getScriptArchiveEntries(zip);
		if (entries.length === 0) {
			throw new Error('Downloaded GitHub archive did not contain a scripts/ folder.');
		}

		logger.info(`GitHub archive script data file count=${entries.length}`);
		reportRefreshProgress(ui, 'Extracting downloaded BI script data...');
		output.appendLine('Extracting downloaded BI script data...');
		await writeScriptArchiveEntries(exportPath, entries);
		const latestSha = await getLatestGitHubScriptsSha(logger);
		await writeScriptSourceMetadata(exportPath, {
			source: 'github',
			repository: githubScriptRepository,
			branch: githubScriptBranch,
			archiveUrl: githubScriptArchiveUrl,
			sourceSha: latestSha,
			downloadedAt: new Date().toISOString(),
			scriptCount: entries.length,
		});
		output.appendLine('Downloaded official BI script data.');
		return true;
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		output.appendLine(`GitHub script download failed: ${message}`);
		logger.error(`GitHub script download failed: ${message}`);
		vscode.window.showErrorMessage('Reforger script data download failed. Check the Reforger Script Tools output.');
		return false;
	}
}

function getExportSettings(context: vscode.ExtensionContext): ExportSettings {
	const config = vscode.workspace.getConfiguration(settingsSection);
	return {
		manualGameScriptDataPath: config.get<string>('gameScriptDataPath', '') ?? '',
		checkGameScriptUpdates: config.get<boolean>('checkGameScriptUpdates', true),
	};
}

async function selectManualScriptDataFolder(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger
): Promise<boolean> {
	const selectedPath = await promptForManualScriptDataFolder();
	if (!selectedPath) {
		logger.warn('Manual script data folder selection cancelled.');
		return false;
	}
	const config = vscode.workspace.getConfiguration(settingsSection);
	await config.update('gameScriptDataPath', selectedPath, vscode.ConfigurationTarget.Global);
	return await importManualScriptDataFolder(context, selectedPath, output, logger);
}

async function promptForManualScriptDataFolder(): Promise<string | undefined> {
	const selection = await vscode.window.showOpenDialog({
		title: 'Select Reforger script data folder. Choose either the scripts folder or a folder containing scripts.',
		canSelectFiles: false,
		canSelectFolders: true,
		canSelectMany: false,
	});
	return selection?.[0]?.fsPath;
}

async function importManualScriptDataFolder(
	context: vscode.ExtensionContext,
	sourcePath: string,
	output: vscode.OutputChannel,
	logger: ExtensionLogger
): Promise<boolean> {
	const sourceScriptsPath = await resolveManualScriptsPath(sourcePath);
	if (!sourceScriptsPath) {
		vscode.window.showWarningMessage('Selected folder does not contain Reforger script data.');
		return false;
	}

	const exportPath = getConfiguredExportedGameDataPath(context);
	const targetScriptsPath = path.join(exportPath, 'scripts');
	const sourceScriptDataFileCount = await countEnforceScriptFiles(sourceScriptsPath);
	if (!isExpectedGameScriptDataFileCount(sourceScriptDataFileCount)) {
		const message = gameScriptDataNotExpectedMessage(sourceScriptDataFileCount);
		output.show(true);
		output.appendLine(message);
		logger.warn(message);
		vscode.window.showWarningMessage(message);
		return false;
	}

	output.show(true);
	output.appendLine(`Importing manual script data from ${sourceScriptsPath}`);
	output.appendLine(`Export folder: ${exportPath}`);
	logger.info(`Importing manual script data source=${sourceScriptsPath} export=${exportPath}`);
	await fs.mkdir(exportPath, { recursive: true });
	await fs.rm(targetScriptsPath, { recursive: true, force: true });
	const scriptDataFileCount = await copyScriptFolder(sourceScriptsPath, targetScriptsPath);
	await writeScriptSourceMetadata(exportPath, {
		source: 'manual',
		repository: githubScriptRepository,
		branch: githubScriptBranch,
		archiveUrl: githubScriptArchiveUrl,
		downloadedAt: new Date().toISOString(),
		scriptCount: scriptDataFileCount,
	});
	output.appendLine(`Imported ${scriptDataFileCount} manual script data file(s).`);
	return true;
}

async function downloadGitHubScriptArchive(output: vscode.OutputChannel, logger: ExtensionLogger): Promise<Uint8Array> {
	output.appendLine(`Downloading ${githubScriptArchiveUrl}`);
	const response = await fetch(githubScriptArchiveUrl, {
		headers: {
			'Accept': 'application/zip',
			'User-Agent': 'Reforger-Script-Tools',
		},
	});
	if (!response.ok) {
		throw new Error(`GitHub archive download failed: HTTP ${response.status}`);
	}

	const bytes = new Uint8Array(await response.arrayBuffer());
	output.appendLine(`Downloaded GitHub archive (${formatBytes(bytes.byteLength)}).`);
	logger.info(`Downloaded GitHub script archive bytes=${bytes.byteLength}`);
	return bytes;
}

export function getScriptArchiveEntries(zip: Pick<JSZip, 'forEach'>): GitHubScriptArchiveEntry[] {
	const entries: GitHubScriptArchiveEntry[] = [];
	zip.forEach((entryPath, entry) => {
		const scriptRelativePath = scriptPathFromArchiveEntry(entryPath);
		if (!scriptRelativePath || entry.dir) {
			return;
		}
		entries.push({
			name: scriptRelativePath,
			dir: false,
			async: type => entry.async(type),
		});
	});
	return entries.sort((left, right) => left.name.localeCompare(right.name));
}

export function scriptPathFromArchiveEntry(entryPath: string): string | undefined {
	const normalized = entryPath.replace(/\\/g, '/');
	const scriptsIndex = normalized.indexOf('/scripts/');
	const scriptPath = scriptsIndex >= 0
		? normalized.slice(scriptsIndex + 1)
		: normalized.startsWith('scripts/') ? normalized : undefined;
	if (!scriptPath) {
		return undefined;
	}
	if (scriptPath.split('/').some(part => part === '..' || part.length === 0)) {
		return undefined;
	}
	return scriptPath;
}

async function writeScriptArchiveEntries(
	exportPath: string,
	entries: readonly GitHubScriptArchiveEntry[]
): Promise<void> {
	const scriptsPath = path.join(exportPath, 'scripts');
	await fs.rm(scriptsPath, { recursive: true, force: true });
	await fs.mkdir(scriptsPath, { recursive: true });

	for (let index = 0; index < entries.length; index += scriptWriteBatchSize) {
		const batch = entries.slice(index, index + scriptWriteBatchSize);
		await Promise.all(batch.map(entry => writeScriptArchiveEntry(exportPath, entry)));
		await yieldToExtensionHost();
	}
}

async function writeScriptArchiveEntry(exportPath: string, entry: GitHubScriptArchiveEntry): Promise<void> {
	const outputPath = safeExportPath(exportPath, entry.name);
	await fs.mkdir(path.dirname(outputPath), { recursive: true });
	await fs.writeFile(outputPath, await entry.async('uint8array'));
}

function safeExportPath(exportPath: string, relativeScriptPath: string): string {
	const target = path.resolve(exportPath, relativeScriptPath);
	const root = path.resolve(exportPath);
	if (target !== root && !target.startsWith(`${root}${path.sep}`)) {
		throw new Error(`Archive entry attempted to write outside the export folder: ${relativeScriptPath}`);
	}
	return target;
}

async function writeScriptSourceMetadata(exportPath: string, metadata: ScriptSourceMetadata): Promise<void> {
	await fs.writeFile(
		path.join(exportPath, 'reforger-script-source.json'),
		`${JSON.stringify(metadata, null, 2)}\n`,
		'utf8'
	);
}

async function readScriptSourceMetadata(context: vscode.ExtensionContext): Promise<ScriptSourceMetadata | undefined> {
	try {
		const raw = await fs.readFile(path.join(getConfiguredExportedGameDataPath(context), 'reforger-script-source.json'), 'utf8');
		return JSON.parse(raw) as ScriptSourceMetadata;
	} catch {
		return undefined;
	}
}

async function hasExportedScriptData(context: vscode.ExtensionContext): Promise<boolean> {
	const exportPath = getConfiguredExportedGameDataPath(context);
	const scriptsPath = path.join(exportPath, 'scripts');
	return isExpectedGameScriptDataFileCount(await countEnforceScriptFiles(scriptsPath));
}

function getConfiguredExportedGameDataPath(context: vscode.ExtensionContext): string {
	return path.join(context.globalStorageUri.fsPath, 'exported-game-data');
}

async function resolveManualScriptsPath(sourcePath: string): Promise<string | undefined> {
	if (await containsEnforceScriptFile(sourcePath) && path.basename(sourcePath).toLowerCase() === 'scripts') {
		return sourcePath;
	}
	const nestedScriptsPath = path.join(sourcePath, 'scripts');
	if (await containsEnforceScriptFile(nestedScriptsPath)) {
		return nestedScriptsPath;
	}
	return await containsEnforceScriptFile(sourcePath) ? sourcePath : undefined;
}

async function copyScriptFolder(sourcePath: string, targetPath: string): Promise<number> {
	await fs.mkdir(targetPath, { recursive: true });
	const entries = await fs.readdir(sourcePath, { withFileTypes: true });
	let count = 0;
	for (const entry of entries) {
		const sourceEntry = path.join(sourcePath, entry.name);
		const targetEntry = path.join(targetPath, entry.name);
		if (entry.isDirectory()) {
			count += await copyScriptFolder(sourceEntry, targetEntry);
		} else if (entry.isFile()) {
			await fs.mkdir(path.dirname(targetEntry), { recursive: true });
			await fs.copyFile(sourceEntry, targetEntry);
			if (entry.name.toLowerCase().endsWith('.c')) {
				count++;
			}
		}
	}
	return count;
}

async function getLatestGitHubScriptsSha(logger: ExtensionLogger): Promise<string | undefined> {
	try {
		const response = await fetch(`${githubScriptApiBaseUrl}/commits/${githubScriptBranch}`, {
			headers: {
				'Accept': 'application/vnd.github+json',
				'User-Agent': 'Reforger-Script-Tools',
			},
		});
		if (!response.ok) {
			logger.warn(`GitHub update check failed: HTTP ${response.status}`);
			return undefined;
		}
		const payload = await response.json() as { sha?: string };
		return payload.sha;
	} catch (error) {
		logger.warn(`GitHub update check failed: ${error instanceof Error ? error.message : String(error)}`);
		return undefined;
	}
}

async function containsEnforceScriptFile(rootPath: string): Promise<boolean> {
	return (await countEnforceScriptFiles(rootPath, 1)) > 0;
}

async function countEnforceScriptFiles(rootPath: string, stopAfter = Number.POSITIVE_INFINITY): Promise<number> {
	let count = 0;
	try {
		const entries = await fs.readdir(rootPath, { withFileTypes: true });
		for (const entry of entries) {
			const fullPath = path.join(rootPath, entry.name);
			if (entry.isFile() && entry.name.toLowerCase().endsWith('.c')) {
				count++;
				if (count >= stopAfter) {
					return count;
				}
			}
			if (entry.isDirectory()) {
				count += await countEnforceScriptFiles(fullPath, stopAfter - count);
				if (count >= stopAfter) {
					return count;
				}
			}
		}
	} catch {
		return 0;
	}

	return count;
}

export function isExpectedGameScriptDataFileCount(fileCount: number): boolean {
	return fileCount >= minimumExpectedGameScriptFiles;
}

function gameScriptDataNotExpectedMessage(fileCount: number): string {
	return `Game script data not as expected: found ${fileCount} .c script file(s), expected at least ${minimumExpectedGameScriptFiles}. Refresh from GitHub or select the Reforger scripts folder.`;
}

function formatBytes(bytes: number): string {
	if (bytes < 1024) {
		return `${bytes} B`;
	}
	if (bytes < 1024 * 1024) {
		return `${(bytes / 1024).toFixed(1)} KB`;
	}
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function yieldToExtensionHost(): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, 0));
}
