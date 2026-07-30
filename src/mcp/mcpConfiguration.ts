import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientIndexCache } from '../extensionConfig/languageClient';
import { mcpCommands, mcpServer } from '../extensionConfig/mcp';
import { resolveLanguageServerPath } from '../languageClient/serverPath';

export interface McpLaunch {
	command: string;
	args: string[];
}

export interface McpLaunchInputs {
	serverPath: string;
	indexCache: string;
}

type ConfigurationFormat = 'generic' | 'codex';

const genericChoice = 'Generic MCP JSON';
const codexChoice = 'Codex config.toml';

export function buildMcpLaunchConfiguration(inputs: McpLaunchInputs): McpLaunch {
	const args = ['mcp', '--index-cache', inputs.indexCache];
	return {
		command: inputs.serverPath,
		args,
	};
}

export function renderGenericMcpConfiguration(launch: McpLaunch): string {
	return `${JSON.stringify({
		mcpServers: {
			[mcpServer.name]: launch,
		},
	}, null, 2)}\n`;
}

export function renderCodexMcpConfiguration(launch: McpLaunch): string {
	return [
		`[mcp_servers.${mcpServer.name}]`,
		`command = ${tomlString(launch.command)}`,
		`args = [${launch.args.map(tomlString).join(', ')}]`,
		`startup_timeout_sec = ${mcpServer.initializationDeadlineSeconds}.0`,
		`tool_timeout_sec = ${mcpServer.toolTimeoutSeconds}.0`,
		'',
	].join('\n');
}

export function registerMcpConfigurationCommand(
	context: vscode.ExtensionContext,
): void {
	context.subscriptions.push(vscode.commands.registerCommand(
		mcpCommands.copyConfiguration,
		async (requestedFormat?: ConfigurationFormat) => {
			const serverPath = await resolveLanguageServerPath(context);
			if (!serverPath) {
				await vscode.window.showErrorMessage(
					'Reforger Script Tools could not find its packaged Rust runtime.',
				);
				return;
			}

			const format = requestedFormat ?? await selectConfigurationFormat();
			if (!format) {
				return;
			}
			const launch = buildMcpLaunchConfiguration({
				serverPath,
				indexCache: path.join(
					context.globalStorageUri.fsPath,
					languageClientIndexCache.rootFolder,
					languageClientIndexCache.gameDataIndexFile,
				),
			});
			const configuration = format === 'codex'
				? renderCodexMcpConfiguration(launch)
				: renderGenericMcpConfiguration(launch);

			await vscode.env.clipboard.writeText(configuration);
			await vscode.window.showInformationMessage(
				`${format === 'codex' ? codexChoice : genericChoice} copied to the clipboard.`,
			);
		},
	));
}

async function selectConfigurationFormat(): Promise<ConfigurationFormat | undefined> {
	const selected = await vscode.window.showQuickPick(
		[
			{
				label: codexChoice,
				description: 'Paste into Codex user or trusted project config.toml',
				format: 'codex' as const,
			},
			{
				label: genericChoice,
				description: 'Use with clients that accept the common mcpServers JSON shape',
				format: 'generic' as const,
			},
		],
		{
			placeHolder: 'Choose an MCP client configuration format',
			title: 'Copy Reforger Script Tools MCP Configuration',
		},
	);
	return selected?.format;
}

function tomlString(value: string): string {
	return JSON.stringify(value);
}
