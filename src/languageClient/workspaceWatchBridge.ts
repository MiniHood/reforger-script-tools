import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { diagnostic } from '../diagnostics/diagnostics';
import { languageClientNotifications } from '../extensionConfig/languageClient';

export async function discoverWorkspaceScriptRoots(): Promise<string[]> {
	const folders = vscode.workspace.workspaceFolders ?? [];
	const roots = new Set<string>();
	for (const folder of folders) {
		const folderPath = folder.uri.fsPath;
		if (path.basename(folderPath).toLowerCase() === 'scripts') {
			roots.add(folderPath);
			continue;
		}
		for (const childName of ['Scripts', 'scripts']) {
			const candidate = path.join(folderPath, childName);
			if (await isDirectory(candidate)) {
				roots.add(candidate);
			}
		}
	}
	return [...roots].sort();
}

/** Finds one unambiguous project descriptor per opened workspace folder. */
export async function discoverWorkspaceProjectFiles(): Promise<string[]> {
	const projectFiles: string[] = [];
	for (const folder of vscode.workspace.workspaceFolders ?? []) {
		const projectFile = await discoverWorkspaceProjectFile(folder.uri.fsPath);
		if (projectFile) {
			projectFiles.push(projectFile);
		}
	}
	return projectFiles;
}

export async function discoverWorkspaceProjectFile(folderPath: string): Promise<string | undefined> {
	try {
		const entries = await fs.readdir(folderPath, { withFileTypes: true });
		const candidates = entries
			.filter(entry => entry.isFile() && path.extname(entry.name).toLowerCase() === '.gproj')
			.map(entry => path.join(folderPath, entry.name))
			.sort();
		return candidates.length === 1 ? candidates[0] : undefined;
	} catch {
		return undefined;
	}
}

/** Publishes versioned workspace-file events to the Rust index immediately. */
export function registerWorkspaceScriptWatchBridge(
	client: LanguageClient,
	outputChannel: vscode.LogOutputChannel,
): vscode.Disposable[] {
	const folders = vscode.workspace.workspaceFolders ?? [];
	if (folders.length === 0) {
		return [];
	}
	const disposables: vscode.Disposable[] = [];
	const sequences = new Map<string, number>();
	const publish = (uri: vscode.Uri, kind: 'changed' | 'deleted'): void => {
		if (uri.scheme !== 'file') {
			return;
		}
		const key = workspaceWatcherPathKey(uri.fsPath);
		const sequence = (sequences.get(key) ?? 0) + 1;
		sequences.set(key, sequence);
		const filePath = uri.fsPath;
		if (kind === 'deleted') {
			client.sendNotification(languageClientNotifications.workspaceFileDeleted, { path: filePath, sequence });
			diagnostic('workspaceWatcher.deleted', { sequence });
			return;
		}
		void fs.readFile(filePath, 'utf8').then(
			text => {
				client.sendNotification(languageClientNotifications.workspaceFileChanged, { path: filePath, text, sequence });
				diagnostic('workspaceWatcher.changed', { bytes: Buffer.byteLength(text, 'utf8'), sequence });
			},
			error => {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`Workspace script change skipped for ${filePath}: ${message}`);
				diagnostic('workspaceWatcher.readFailed', { sequence });
			},
		);
	};
	for (const folder of folders) {
		const folderName = path.basename(folder.uri.fsPath).toLowerCase();
		const pattern = new vscode.RelativePattern(folder, folderName === 'scripts' ? '**/*.c' : '**/{Scripts,scripts}/**/*.c');
		const watcher = vscode.workspace.createFileSystemWatcher(pattern);
		disposables.push(watcher, watcher.onDidCreate(uri => publish(uri, 'changed')), watcher.onDidChange(uri => publish(uri, 'changed')), watcher.onDidDelete(uri => publish(uri, 'deleted')));
	}
	return disposables;
}

async function isDirectory(targetPath: string): Promise<boolean> {
	try { return (await fs.stat(targetPath)).isDirectory(); } catch { return false; }
}

function workspaceWatcherPathKey(filePath: string): string {
	const normalized = path.resolve(filePath).replace(/\\/g, '/');
	return process.platform === 'win32' ? normalized.toLowerCase() : normalized;
}
