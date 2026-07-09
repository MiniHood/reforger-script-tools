import * as vscode from 'vscode';
import { registerGameDataFeatures } from './gameData/gameData';

export function activate(context: vscode.ExtensionContext) {
	registerGameDataFeatures(context);
}

export function deactivate() {}
