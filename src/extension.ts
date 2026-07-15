import * as vscode from 'vscode';
import { registerGameDataFeatures } from './gameData/gameData';
import {
	deactivateLanguageClient,
	logLanguageClientStartupTiming,
	registerLanguageClientFeatures,
} from './languageClient/languageClient';

export function activate(context: vscode.ExtensionContext) {
	logLanguageClientStartupTiming(context, 'activationStart', {
		extensionMode: extensionModeName(context.extensionMode),
		workspaceFolders: String(vscode.workspace.workspaceFolders?.length ?? 0),
	});
	registerGameDataFeatures(context);
	registerLanguageClientFeatures(context);
	logLanguageClientStartupTiming(context, 'activationEnd');
}

export function deactivate() {
	return deactivateLanguageClient();
}

function extensionModeName(mode: vscode.ExtensionMode): string {
	switch (mode) {
		case vscode.ExtensionMode.Development:
			return 'development';
		case vscode.ExtensionMode.Production:
			return 'production';
		case vscode.ExtensionMode.Test:
			return 'test';
		default:
			return 'unknown';
	}
}
