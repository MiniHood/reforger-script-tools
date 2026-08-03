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
import { registerSearchUi } from './searchPrototype/searchUiPrototype';
import { registerWorkbenchCompilerFeatures } from './workbenchNetApi/compiler/workbenchCompiler';
import { createWorkbenchIntegration } from './workbenchNetApi/integration/workbenchIntegration';
import { resolveLanguageServerPath } from './languageClient/serverPath';

export async function activate(context: vscode.ExtensionContext) {
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
	const workbenchStartupGate = integration?.whenConsentSettled() ?? Promise.resolve(true);
	registerMcpConfigurationCommand(context);
	registerSearchUi(context);
	await workbenchStartupGate;
	let refreshLanguageClientGameData: ReturnType<typeof registerLanguageClientFeatures> | undefined;
	let workbenchConnectedBeforeLanguageClient = false;
	registerWorkbenchCompilerFeatures(context, integration, () => {
		if (refreshLanguageClientGameData) {
			void refreshLanguageClientGameData({ showProgress: false });
		} else {
			workbenchConnectedBeforeLanguageClient = true;
		}
	});
	refreshLanguageClientGameData = registerLanguageClientFeatures(
		context,
		workbenchReady,
		workbenchStartupGate,
	);
	if (workbenchConnectedBeforeLanguageClient) {
		void refreshLanguageClientGameData({ showProgress: false });
	}
	registerGameDataFeatures(context, () => refreshLanguageClientGameData?.());
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
