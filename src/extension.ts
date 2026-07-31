import * as vscode from 'vscode';
import { diagnostic, initializeDiagnostics } from './diagnostics/diagnostics';
import { registerGameDataFeatures } from './gameData/gameData';
import {
	beginLanguageClientStartupTimingSession,
	deactivateLanguageClient,
	logLanguageClientStartupTiming,
	registerLanguageClientFeatures,
} from './languageClient/languageClient';
import { registerMcpConfigurationCommand } from './mcp/mcpConfiguration';
import { registerWorkbenchCompilerFeatures } from './workbenchNetApi/compiler/workbenchCompiler';

export function activate(context: vscode.ExtensionContext) {
	initializeDiagnostics(context);
	beginLanguageClientStartupTimingSession();
	diagnostic('activationStart');
	logLanguageClientStartupTiming(context, 'activationStart', {
		extensionMode: extensionModeName(context.extensionMode),
		workspaceFolders: String(vscode.workspace.workspaceFolders?.length ?? 0),
	});
	const refreshLanguageClientGameData = registerLanguageClientFeatures(context);
	registerGameDataFeatures(context, refreshLanguageClientGameData);
	registerMcpConfigurationCommand(context);
	registerWorkbenchCompilerFeatures(context);
	logLanguageClientStartupTiming(context, 'activationEnd');
	diagnostic('activationEnd');
}

export function deactivate() {
	diagnostic('deactivation');
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
