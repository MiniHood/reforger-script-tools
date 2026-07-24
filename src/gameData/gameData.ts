import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { unzipSync } from 'fflate';
import { diagnostic } from '../diagnostics/diagnostics';
import {
	gameDataCommands,
	gameDataConfig,
	gameDataRepository,
	gameDataStateKeys,
	gameDataStorage,
	gameDataThresholds,
} from '../extensionConfig/gameData';

const repoUrl = `https://github.com/${gameDataRepository.owner}/${gameDataRepository.name}`;
const githubApiBase = `https://api.github.com/repos/${gameDataRepository.owner}/${gameDataRepository.name}`;

const downloadChoice = 'Download Game Data';
const setManualFolderChoice = 'Browse Game Data';

export interface GameDataMetadata {
	repoUrl: string;
	branch: string;
	commitSha: string;
	commitDate: string;
	commitMessage: string;
	downloadedAt: string;
	fileCount: number;
	byteCount: number;
}

export interface RemoteGameDataVersion {
	sha: string;
	date: string;
	message: string;
}

interface GitHubCommitResponse {
	sha: string;
	commit: {
		committer: {
			date: string;
		};
		message: string;
	};
}

interface ExtractResult {
	fileCount: number;
	byteCount: number;
}

export function isGameDataStale(local: GameDataMetadata | undefined, remote: RemoteGameDataVersion): boolean {
	return !local || local.commitSha !== remote.sha;
}

export function normalizeManualFolderInput(manualFolder: string): string {
	const trimmed = manualFolder.trim().replace(/^["']|["']$/g, '');
	return path.resolve(trimmed);
}

export function getManualScriptsFolderCandidate(manualFolder: string): string {
	const normalized = normalizeManualFolderInput(manualFolder);
	return path.basename(normalized).toLowerCase() === 'scripts'
		? normalized
		: path.join(normalized, 'scripts');
}

export function shouldWarnForLowScriptCount(
	scriptCount: number,
	manualFolder: string,
	warnedManualFolders: readonly string[],
): boolean {
	return scriptCount < gameDataThresholds.lowScriptCount && !warnedManualFolders.includes(normalizeManualFolderInput(manualFolder));
}

export function markManualFolderWarned(manualFolder: string, warnedManualFolders: readonly string[]): string[] {
	const normalized = normalizeManualFolderInput(manualFolder);
	return warnedManualFolders.includes(normalized)
		? [...warnedManualFolders]
		: [...warnedManualFolders, normalized];
}

export function registerGameDataFeatures(
	context: vscode.ExtensionContext,
	onGameDataSourceChanged?: () => Promise<void>,
): void {
	diagnostic('gameData.registration');
	context.subscriptions.push(
		vscode.commands.registerCommand(gameDataCommands.checkForUpdates, async () => {
			await runGameDataStartupCheck(context, true, onGameDataSourceChanged);
		}),
		vscode.commands.registerCommand(gameDataCommands.openStorageFolder, async () => {
			const storageRoot = getGameDataStorageRoot(context);
			await fs.mkdir(storageRoot, { recursive: true });
			await vscode.env.openExternal(vscode.Uri.file(storageRoot));
		}),
		vscode.commands.registerCommand(gameDataCommands.selectManualFolder, async () => {
			await promptAndSetManualFolder(context, onGameDataSourceChanged);
		}),
	);

	void runGameDataStartupCheck(context, false, onGameDataSourceChanged);
}

async function runGameDataStartupCheck(
	context: vscode.ExtensionContext,
	manualCommand: boolean,
	onGameDataSourceChanged?: () => Promise<void>,
): Promise<void> {
	const startedAt = Date.now();
	try {
		const manualFolder = getManualFolderSetting();
		if (manualFolder) {
			await validateManualFolder(context, manualFolder);
			if (manualCommand) {
				vscode.window.showInformationMessage('Manual Reforger game data folder is set. GitHub checks and downloads are skipped.');
			}
			diagnostic('gameData.check', { mode: 'manual', outcome: 'complete', elapsedMs: Date.now() - startedAt });
			return;
		}

		const remote = await fetchLatestRemoteVersion();
		const local = await readMetadata(context);

		if (!isGameDataStale(local, remote)) {
			if (manualCommand) {
				vscode.window.showInformationMessage(`Reforger game data is current: ${remote.message}.`);
			}
			diagnostic('gameData.check', { mode: 'downloaded', outcome: 'current', elapsedMs: Date.now() - startedAt });
			return;
		}

		const consent = context.globalState.get<boolean>(gameDataStateKeys.downloadAllowed, false);
		if (!consent) {
			const selection = await promptForGameDataSource();

			if (selection === setManualFolderChoice) {
				await promptAndSetManualFolder(context, onGameDataSourceChanged);
				return;
			}

			if (selection !== downloadChoice) {
				return;
			}

			await context.globalState.update(gameDataStateKeys.downloadAllowed, true);
		}

		await vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: 'Reforger game data',
				cancellable: false,
			},
			async progress => {
				await downloadAndInstallGameData(context, remote, progress);
			},
		);
		await onGameDataSourceChanged?.();
		vscode.window.showInformationMessage(`Reforger game data updated: ${remote.message}.`);
		diagnostic('gameData.check', { mode: 'downloaded', outcome: 'updated', elapsedMs: Date.now() - startedAt });
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		vscode.window.showWarningMessage(`Reforger game data update failed: ${message}`);
		diagnostic('gameData.check', { outcome: 'error', elapsedMs: Date.now() - startedAt });
	}
}

async function promptForGameDataSource(): Promise<string | undefined> {
	return vscode.window.showInformationMessage(
		'Reforger Script Tools needs Bohemia script data before language features can use game API context.',
		downloadChoice,
		setManualFolderChoice,
	);
}

async function promptAndSetManualFolder(
	context: vscode.ExtensionContext,
	onGameDataSourceChanged?: () => Promise<void>,
): Promise<void> {
	const selectedFolders = await vscode.window.showOpenDialog({
		canSelectFiles: false,
		canSelectFolders: true,
		canSelectMany: false,
		openLabel: 'Use as Game Data',
		title: 'Select Reforger game data folder or scripts folder',
	});

	const selectedFolder = selectedFolders?.[0]?.fsPath;
	if (!selectedFolder) {
		return;
	}

	await vscode.workspace
		.getConfiguration(gameDataConfig.section)
		.update(gameDataConfig.settings.manualFolder, selectedFolder, vscode.ConfigurationTarget.Global);
	await validateManualFolder(context, selectedFolder);
	await onGameDataSourceChanged?.();
	vscode.window.showInformationMessage('Manual Reforger game data folder set. GitHub game-data checks and downloads are skipped.');
}

async function validateManualFolder(context: vscode.ExtensionContext, manualFolder: string): Promise<void> {
	const scriptsFolder = await resolveManualScriptsFolder(manualFolder);
	const scriptCount = await countScriptFiles(scriptsFolder);
	const warnedFolders = context.globalState.get<string[]>(gameDataStateKeys.warnedLowScriptCountManualFolders, []);

	if (!shouldWarnForLowScriptCount(scriptCount, manualFolder, warnedFolders)) {
		return;
	}

	await context.globalState.update(gameDataStateKeys.warnedLowScriptCountManualFolders, markManualFolderWarned(manualFolder, warnedFolders));
	vscode.window.showWarningMessage(
		`Manual Reforger game data folder has ${scriptCount} .c script files. Expected at least ${gameDataThresholds.lowScriptCount}; language features may be incomplete.`,
	);
}

async function resolveManualScriptsFolder(manualFolder: string): Promise<string> {
	const normalized = normalizeManualFolderInput(manualFolder);
	const directScripts = path.basename(normalized).toLowerCase() === 'scripts';

	if (directScripts && await isDirectory(normalized)) {
		return normalized;
	}

	const nestedScripts = path.join(normalized, 'scripts');
	if (await isDirectory(nestedScripts)) {
		return nestedScripts;
	}

	return directScripts ? normalized : nestedScripts;
}

async function countScriptFiles(folder: string): Promise<number> {
	if (!await isDirectory(folder)) {
		return 0;
	}

	let count = 0;
	const entries = await fs.readdir(folder, { withFileTypes: true });

	for (const entry of entries) {
		const entryPath = path.join(folder, entry.name);
		if (entry.isDirectory()) {
			count += await countScriptFiles(entryPath);
		} else if (entry.isFile() && entry.name.toLowerCase().endsWith('.c')) {
			count += 1;
		}
	}

	return count;
}

async function fetchLatestRemoteVersion(): Promise<RemoteGameDataVersion> {
	const response = await fetch(`${githubApiBase}/commits/${gameDataRepository.branch}`, {
		headers: {
			'Accept': 'application/vnd.github+json',
			'User-Agent': 'reforger-script-tools',
		},
	});

	if (!response.ok) {
		throw new Error(`GitHub commit check failed: ${response.status} ${response.statusText}`);
	}

	const commit = await response.json() as GitHubCommitResponse;
	return {
		sha: commit.sha,
		date: commit.commit.committer.date,
		message: commit.commit.message,
	};
}

async function downloadAndInstallGameData(
	context: vscode.ExtensionContext,
	remote: RemoteGameDataVersion,
	progress: vscode.Progress<{ message?: string; increment?: number }>,
): Promise<void> {
	const storageRoot = getGameDataStorageRoot(context);
	const scriptsRoot = path.join(storageRoot, gameDataStorage.scriptsFolder);
	const stagingRoot = path.join(storageRoot, `${gameDataStorage.stagingPrefix}${Date.now()}`);

	await fs.mkdir(storageRoot, { recursive: true });
	await removeStaleStagingFolders(storageRoot);
	await fs.rm(stagingRoot, { recursive: true, force: true });

	try {
		progress.report({ message: 'Downloading archive' });
		const archive = await downloadArchive(remote.sha);

		progress.report({ message: 'Extracting scripts' });
		const result = await extractScriptsArchive(archive, stagingRoot);

		if (result.fileCount === 0) {
			throw new Error('Downloaded archive did not contain scripts.');
		}

		progress.report({ message: 'Finalizing game data' });
		await fs.rm(scriptsRoot, { recursive: true, force: true });
		await fs.rename(path.join(stagingRoot, gameDataStorage.scriptsFolder), scriptsRoot);
		await fs.rm(stagingRoot, { recursive: true, force: true });

		const metadata: GameDataMetadata = {
			repoUrl,
			branch: gameDataRepository.branch,
			commitSha: remote.sha,
			commitDate: remote.date,
			commitMessage: remote.message,
			downloadedAt: new Date().toISOString(),
			fileCount: result.fileCount,
			byteCount: result.byteCount,
		};
		await writeMetadata(context, metadata);
	} catch (error) {
		await fs.rm(stagingRoot, { recursive: true, force: true });
		throw error;
	}
}

async function removeStaleStagingFolders(storageRoot: string): Promise<void> {
	const entries = await fs.readdir(storageRoot, { withFileTypes: true });
	for (const entry of entries) {
		if (entry.isDirectory() && entry.name.startsWith(gameDataStorage.stagingPrefix)) {
			await fs.rm(path.join(storageRoot, entry.name), { recursive: true, force: true });
		}
	}
}

async function downloadArchive(commitSha: string): Promise<Uint8Array> {
	const response = await fetch(`https://codeload.github.com/${gameDataRepository.owner}/${gameDataRepository.name}/zip/${commitSha}`, {
		headers: {
			'User-Agent': 'reforger-script-tools',
		},
	});

	if (!response.ok) {
		throw new Error(`GitHub archive download failed: ${response.status} ${response.statusText}`);
	}

	return new Uint8Array(await response.arrayBuffer());
}

async function extractScriptsArchive(archive: Uint8Array, stagingRoot: string): Promise<ExtractResult> {
	const scriptsRoot = path.join(stagingRoot, gameDataStorage.scriptsFolder);
	const files = unzipSync(archive);
	let fileCount = 0;
	let byteCount = 0;

	await fs.mkdir(scriptsRoot, { recursive: true });

	for (const [archivePath, content] of Object.entries(files)) {
		if (archivePath.endsWith('/')) {
			continue;
		}

		const scriptsIndex = archivePath.indexOf('/scripts/');
		if (scriptsIndex < 0) {
			continue;
		}

		const relativePath = archivePath.slice(scriptsIndex + '/scripts/'.length);
		const pathParts = relativePath.split('/').filter(Boolean);
		if (pathParts.length === 0 || pathParts.includes('..')) {
			continue;
		}

		const targetPath = path.join(scriptsRoot, ...pathParts);
		await fs.mkdir(path.dirname(targetPath), { recursive: true });
		await fs.writeFile(targetPath, content);
		fileCount += 1;
		byteCount += content.byteLength;
	}

	return { fileCount, byteCount };
}

async function readMetadata(context: vscode.ExtensionContext): Promise<GameDataMetadata | undefined> {
	try {
		const metadataPath = getMetadataPath(context);
		const raw = await fs.readFile(metadataPath, 'utf8');
		return JSON.parse(raw) as GameDataMetadata;
	} catch {
		return undefined;
	}
}

async function writeMetadata(context: vscode.ExtensionContext, metadata: GameDataMetadata): Promise<void> {
	const metadataPath = getMetadataPath(context);
	await fs.mkdir(path.dirname(metadataPath), { recursive: true });
	await fs.writeFile(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`, 'utf8');
}

function getManualFolderSetting(): string | undefined {
	const value = vscode.workspace.getConfiguration(gameDataConfig.section).get<string>(gameDataConfig.settings.manualFolder);
	return value?.trim() ? value : undefined;
}

function getGameDataStorageRoot(context: vscode.ExtensionContext): string {
	return path.join(context.globalStorageUri.fsPath, gameDataStorage.rootFolder);
}

function getMetadataPath(context: vscode.ExtensionContext): string {
	return path.join(getGameDataStorageRoot(context), gameDataStorage.metadataFile);
}

async function isDirectory(targetPath: string): Promise<boolean> {
	try {
		return (await fs.stat(targetPath)).isDirectory();
	} catch {
		return false;
	}
}
