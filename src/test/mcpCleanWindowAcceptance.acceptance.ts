import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
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

		const skills = extension.packageJSON.contributes.chatSkills as Array<{ path: string }>;
		assert.deepStrictEqual(skills, [
			{ path: './skills/reforger/SKILL.md' },
			{ path: './skills/reforger-deep-dive/SKILL.md' },
			{ path: './skills/reforger-workbench-edit/SKILL.md' },
		]);
		for (const skill of skills) {
			await fs.access(path.join(extension.extensionPath, skill.path));
		}
		const commands = await vscode.commands.getCommands(true);
		assert.ok(commands.includes('workbench.action.chat.configure.skills'));
		const skillDiscovery = vscode.commands.executeCommand('workbench.action.chat.configure.skills');
		await new Promise(resolve => setTimeout(resolve, 250));
		assert.strictEqual(extension.isActive, false);
		await vscode.commands.executeCommand('workbench.action.closeQuickOpen');
		await Promise.race([
			Promise.resolve(skillDiscovery).catch(() => undefined),
			new Promise(resolve => setTimeout(resolve, 250)),
		]);
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

		await vscode.commands.executeCommand(
			'workbench.mcp.startServer',
			'*',
			{ waitForLiveTools: true },
		);
		const wikiStatus = vscode.lm.tools.find(tool =>
			tool.name.endsWith('official_wiki_status'));
		assert.ok(
			wikiStatus,
			`Discovered MCP tools did not include Official Wiki status: ${vscode.lm.tools
				.map(tool => tool.name)
				.join(', ')}`,
		);
		const wikiResult = await vscode.lm.invokeTool(
			wikiStatus.name,
			{ input: {}, toolInvocationToken: undefined },
		);
		const wikiText = wikiResult.content
			.filter((part): part is vscode.LanguageModelTextPart =>
				part instanceof vscode.LanguageModelTextPart)
			.map(part => part.value)
			.join('\n');
		assert.match(wikiText, /"available"\s*:\s*true/);

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
