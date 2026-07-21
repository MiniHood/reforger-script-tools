import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { diagnostic } from '../diagnostics/diagnostics';
import { languageClientNotifications } from '../extensionConfig/languageClient';

const workspaceWatcherDebounceMs = 250;

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

/** Owns debounced workspace-file notifications sent to the Rust index. */
export function registerWorkspaceScriptWatchBridge(
	client: LanguageClient,
	outputChannel: vscode.LogOutputChannel,
): vscode.Disposable[] {
	const folders = vscode.workspace.workspaceFolders ?? [];
	if (folders.length === 0) {
		return [];
	}
	const disposables: vscode.Disposable[] = [];
	const pending = new Map<string, { path: string; kind: 'changed' | 'deleted'; sequence: number }>();
	const sequences = new Map<string, number>();
	let timer: NodeJS.Timeout | undefined;
	const flush = (): void => {
		const entries = [...pending.entries()];
		diagnostic('workspaceWatcher.flush', { entries: entries.length });
		pending.clear();
		timer = undefined;
		void Promise.all(entries.map(async ([, entry]) => {
			const { path: filePath, kind, sequence } = entry;
			if (kind === 'deleted') {
				client.sendNotification(languageClientNotifications.workspaceFileDeleted, { path: filePath, sequence });
				diagnostic('workspaceWatcher.deleted', { sequence });
				return;
			}
			try {
				const text = await fs.readFile(filePath, 'utf8');
				client.sendNotification(languageClientNotifications.workspaceFileChanged, { path: filePath, text, sequence });
				diagnostic('workspaceWatcher.changed', { bytes: Buffer.byteLength(text, 'utf8'), sequence });
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`Workspace script change skipped for ${filePath}: ${message}`);
				diagnostic('workspaceWatcher.readFailed', { sequence });
			}
		}));
	};
	const schedule = (uri: vscode.Uri, kind: 'changed' | 'deleted'): void => {
		if (uri.scheme !== 'file') {
			return;
		}
		const key = workspaceWatcherPathKey(uri.fsPath);
		const sequence = (sequences.get(key) ?? 0) + 1;
		sequences.set(key, sequence);
		pending.set(key, { path: uri.fsPath, kind, sequence });
		if (timer) {
			clearTimeout(timer);
		}
		timer = setTimeout(flush, workspaceWatcherDebounceMs);
	};
	for (const folder of folders) {
		const folderName = path.basename(folder.uri.fsPath).toLowerCase();
		const pattern = new vscode.RelativePattern(folder, folderName === 'scripts' ? '**/*.c' : '**/{Scripts,scripts}/**/*.c');
		const watcher = vscode.workspace.createFileSystemWatcher(pattern);
		disposables.push(watcher, watcher.onDidCreate(uri => schedule(uri, 'changed')), watcher.onDidChange(uri => schedule(uri, 'changed')), watcher.onDidDelete(uri => schedule(uri, 'deleted')));
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
