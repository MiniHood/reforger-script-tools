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
import { createWorkbenchIntegration } from './workbenchNetApi/integration/workbenchIntegration';
import { resolveLanguageServerPath } from './languageClient/serverPath';

export function activate(context: vscode.ExtensionContext) {
	initializeDiagnostics(context);
	beginLanguageClientStartupTimingSession();
	diagnostic('activationStart');
	logLanguageClientStartupTiming(context, 'activationStart', {
		extensionMode: extensionModeName(context.extensionMode),
		workspaceFolders: String(vscode.workspace.workspaceFolders?.length ?? 0),
	});
	const integration = context.extensionMode === vscode.ExtensionMode.Test
		? undefined
		: createWorkbenchIntegration(context, resolveLanguageServerPath(context));
	const workbenchReady = integration?.start() ?? Promise.resolve(true);
	registerWorkbenchCompilerFeatures(context, integration);
	const refreshLanguageClientGameData = registerLanguageClientFeatures(context, workbenchReady);
	registerGameDataFeatures(context, refreshLanguageClientGameData);
	registerMcpConfigurationCommand(context);
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
