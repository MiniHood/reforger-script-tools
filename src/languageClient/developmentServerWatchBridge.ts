import * as path from 'path';
import * as vscode from 'vscode';
import { languageClientServer } from '../extensionConfig/languageClient';

let watcher: vscode.FileSystemWatcher | undefined;
let watchedPath: string | undefined;
let restartInFlight: Promise<void> | undefined;
let restartRequested = false;
let watcherGeneration = 0;

/** Watches only the development server binary and serializes immediate restarts. */
export function registerDevelopmentServerWatchBridge(
	context: vscode.ExtensionContext,
	serverPath: string,
	onRestart: () => Promise<void>,
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
	const generation = ++watcherGeneration;
	watchedPath = developmentPath;
	watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(
		path.dirname(developmentPath),
		path.basename(developmentPath),
	));
	const requestRestart = (): void => {
		if (restartInFlight) {
			restartRequested = true;
			return;
		}
		const run = async (): Promise<void> => {
			do {
				restartRequested = false;
				await onRestart();
			} while (restartRequested && watcherGeneration === generation);
		};
		restartInFlight = run().finally(() => {
			restartInFlight = undefined;
		});
	};

	context.subscriptions.push(
		watcher,
		watcher.onDidCreate(requestRestart),
		watcher.onDidChange(requestRestart),
	);
}

export function disposeDevelopmentServerWatchBridge(): void {
	watcherGeneration += 1;
	watcher?.dispose();
	watcher = undefined;
	watchedPath = undefined;
	restartRequested = false;
}
