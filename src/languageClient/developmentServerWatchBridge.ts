import * as path from 'path';
import * as vscode from 'vscode';
import { languageClientServer } from '../extensionConfig/languageClient';

let watcher: vscode.FileSystemWatcher | undefined;
let watchedPath: string | undefined;
let restartTimer: NodeJS.Timeout | undefined;

/** Watches only the development server binary and requests a debounced restart. */
export function registerDevelopmentServerWatchBridge(
	context: vscode.ExtensionContext,
	serverPath: string,
	onRestart: () => void,
): void {
	if (context.extensionMode !== vscode.ExtensionMode.Development) {
		return;
	}

	const developmentPath = path.join(context.extensionPath, ...languageClientServer.devBinaryRelativePath);
	if (path.normalize(serverPath) !== path.normalize(developmentPath)
		|| (watchedPath === developmentPath && watcher)) {
		return;
	}

	watcher?.dispose();
	watchedPath = developmentPath;
	watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(
		path.dirname(developmentPath),
		path.basename(developmentPath),
	));
	const scheduleRestart = (): void => {
		if (restartTimer) {
			clearTimeout(restartTimer);
		}
		restartTimer = setTimeout(() => {
			restartTimer = undefined;
			onRestart();
		}, 500);
	};

	context.subscriptions.push(
		watcher,
		watcher.onDidCreate(scheduleRestart),
		watcher.onDidChange(scheduleRestart),
	);
}

export function disposeDevelopmentServerWatchBridge(): void {
	watcher?.dispose();
	watcher = undefined;
	watchedPath = undefined;
	if (restartTimer) {
		clearTimeout(restartTimer);
		restartTimer = undefined;
	}
}
