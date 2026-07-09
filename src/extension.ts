import * as vscode from 'vscode';
import { registerGameDataFeatures } from './gameData/gameData';
import { deactivateLanguageClient, registerLanguageClientFeatures } from './languageClient/languageClient';

export function activate(context: vscode.ExtensionContext) {
	registerGameDataFeatures(context);
	registerLanguageClientFeatures(context);
}

export function deactivate() {
	return deactivateLanguageClient();
}
