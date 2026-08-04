import * as assert from 'node:assert';
import * as vscode from 'vscode';
import { mcpServer } from '../extensionConfig/mcp';
import {
	workbenchConfig,
	workbenchDefaults,
	workbenchTestCommands,
} from '../extensionConfig/workbench';
import type { WorkbenchCompilerObservation } from '../workbenchNetApi/compiler/workbenchCompiler';

suite('native MCP clean-window acceptance', () => {
	test('discovers the contribution on an MCP request without Enforce or Workbench activation', async () => {
		assert.strictEqual(vscode.workspace.workspaceFolders, undefined);
		assert.strictEqual(
			vscode.workspace.textDocuments.some(document => document.languageId === 'enforce'),
			false,
		);

		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-script-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		assert.strictEqual(extension.isActive, false);

		const discovery = vscode.commands.executeCommand('workbench.mcp.listServer');
		await waitUntil(() => extension.isActive);
		await vscode.commands.executeCommand('workbench.action.closeQuickOpen');
		await discovery;

		const providers = extension.packageJSON.contributes.mcpServerDefinitionProviders as Array<{
			id: string;
			label: string;
		}>;
		assert.deepStrictEqual(providers, [{
			id: mcpServer.providerId,
			label: mcpServer.label,
		}]);
		assert.strictEqual(
			vscode.workspace.getConfiguration(workbenchConfig.section).get(
				workbenchConfig.settings.enabled,
				workbenchDefaults.enabled,
			),
			false,
		);

		await new Promise(resolve => setTimeout(resolve, 250));
		const observation = await vscode.commands.executeCommand<WorkbenchCompilerObservation>(
			workbenchTestCommands.observeCompiler,
		);
		assert.strictEqual(observation.phase, 'disabled');
		assert.strictEqual(observation.statusVisible, false);
	});
});

async function waitUntil(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 100 && !predicate(); attempt += 1) {
		await new Promise(resolve => setTimeout(resolve, 50));
	}
	assert.strictEqual(predicate(), true, 'VS Code did not activate the contributed MCP provider');
}
