import * as path from 'path';
import * as vscode from 'vscode';
import { languageClientServer } from '../extensionConfig/languageClient';

let watcher: vscode.FileSystemWatcher | undefined;
let watchedPath: string | undefined;

/** Watches only the development server binary and publishes changes immediately. */
export function registerDevelopmentServerWatchBridge(
	context: vscode.ExtensionContext,
	serverPath: string,
	onBinaryChanged: () => void,
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

	context.subscriptions.push(
		watcher,
		watcher.onDidCreate(onBinaryChanged),
		watcher.onDidChange(onBinaryChanged),
	);
}

export function disposeDevelopmentServerWatchBridge(): void {
	watcher?.dispose();
	watcher = undefined;
	watchedPath = undefined;
}
