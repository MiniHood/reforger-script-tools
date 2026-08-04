import * as assert from 'node:assert';
import { spawnSync } from 'node:child_process';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { mcpServer } from '../extensionConfig/mcp';
import { languageClientServer } from '../extensionConfig/languageClient';
import {
	buildMcpLaunchPolicy,
	renderGenericMcpConfiguration,
} from '../mcp/mcpConfiguration';
import {
	createMcpLaunchScopeChangeEvent,
	createExtensionMcpServerDefinitionProvider,
	createMcpServerDefinitionProvider,
} from '../mcp/mcpServerDefinitionProvider';

suite('MCP server definition provider', () => {
	test('publishes the shared launch as one versioned stdio server', async () => {
		const launch = {
			command: 'C:\\Extensions & Tools\\reforger_language_server.exe',
			args: [
				'mcp',
				'--workspace-scripts',
				'C:\\Projects\\My Addon (Local)\\Scripts',
			],
		};
		const provider = createMcpServerDefinitionProvider({
			extensionVersion: '2.0.1',
			resolveLaunch: async () => ({ launch, scopeIdentity: 'workspace-scope-42' }),
		});

		const definitions = await provider.provideMcpServerDefinitions(
			new vscode.CancellationTokenSource().token,
		);

		assert.ok(definitions);
		assert.strictEqual(definitions.length, 1);
		const definition = definitions[0];
		assert.ok(definition instanceof vscode.McpStdioServerDefinition);
		assert.strictEqual(definition.label, mcpServer.label);
		assert.strictEqual(definition.command, launch.command);
		assert.deepStrictEqual(definition.args, launch.args);
		assert.strictEqual(definition.version, '2.0.1:workspace-scope-42');
	});

	test('keeps the definition version stable for identical launch evidence', async () => {
		const policy = buildMcpLaunchPolicy({
			serverPath: 'C:\\Extensions & Tools\\reforger_language_server.exe',
			addonSourceInventory: 'C:\\Storage\\workbench graph.json',
			addonIndexStorage: 'C:\\Storage\\addon indexes',
			externalIndexMode: 'loaded',
			officialWikiRoot: 'C:\\Extensions & Tools\\data\\official-wiki',
			workspaceScripts: ['C:\\Projects\\My Addon (Local)\\Scripts'],
			dependencyProjects: [{
				path: 'C:\\Projects\\My Addon (Local)\\addon.gproj',
				contents: 'GameProject { ID "abc" }',
			}],
			addonSourceInventoryContents: '{"addons":[]}',
		});
		const first = createMcpServerDefinitionProvider({
			extensionVersion: '2.0.1',
			resolveLaunch: async () => policy,
		});
		const second = createMcpServerDefinitionProvider({
			extensionVersion: '2.0.1',
			resolveLaunch: async () => buildMcpLaunchPolicy({
				serverPath: 'C:\\Extensions & Tools\\reforger_language_server.exe',
				addonSourceInventory: 'C:\\Storage\\workbench graph.json',
				addonIndexStorage: 'C:\\Storage\\addon indexes',
				externalIndexMode: 'loaded',
				officialWikiRoot: 'C:\\Extensions & Tools\\data\\official-wiki',
				workspaceScripts: ['C:\\Projects\\My Addon (Local)\\Scripts'],
				dependencyProjects: [{
					path: 'C:\\Projects\\My Addon (Local)\\addon.gproj',
					contents: 'GameProject { ID "abc" }',
				}],
				addonSourceInventoryContents: '{"addons":[]}',
			}),
		});

		const firstDefinitions = await first.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token);
		const secondDefinitions = await second.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token);

		assert.strictEqual(firstDefinitions?.[0].version, secondDefinitions?.[0].version);
		assert.match(firstDefinitions?.[0].version ?? '', /^2\.0\.1:[a-f0-9]{64}$/);
	});

	test('changes the definition version when material launch evidence changes', async () => {
		const policy = (
			externalIndexMode: 'loaded' | 'none',
			descriptorContents: string,
			inventoryContents = '{"addons":[]}',
		) => buildMcpLaunchPolicy({
			serverPath: 'C:\\Extension\\reforger_language_server.exe',
			addonSourceInventory: 'C:\\Storage\\graph.json',
			addonIndexStorage: 'C:\\Storage\\addon-indexes',
			externalIndexMode,
			dependencyProjects: [{ path: 'C:\\Project\\addon.gproj', contents: descriptorContents }],
			addonSourceInventoryContents: inventoryContents,
		});
		const versionFor = async (
			externalIndexMode: 'loaded' | 'none',
			descriptorContents: string,
			inventoryContents?: string,
		) => {
			const provider = createMcpServerDefinitionProvider({
				extensionVersion: '2.0.1',
				resolveLaunch: async () => policy(externalIndexMode, descriptorContents, inventoryContents),
			});
			return (await provider.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token))?.[0].version;
		};

		const loaded = await versionFor('loaded', 'GameProject { ID "abc" }');
		assert.notStrictEqual(await versionFor('none', 'GameProject { ID "abc" }'), loaded);
		assert.notStrictEqual(await versionFor('loaded', 'GameProject { ID "changed" }'), loaded);
		assert.notStrictEqual(
			await versionFor('loaded', 'GameProject { ID "abc" }', '{"addons":[{"guid":"changed"}]}'),
			loaded,
		);
	});

	test('uses the extension launch policy for native and exported configurations', async () => {
		const root = await fs.mkdtemp(path.join(os.tmpdir(), 'rst-native-mcp-'));
		try {
			const extensionPath = path.join(root, 'Extension & Runtime');
			const storagePath = path.join(root, 'Global Storage');
			const serverPath = path.join(
				extensionPath,
				'dist',
				languageClientServer.distFolder,
				`${process.platform}-${process.arch}`,
				languageClientServer.binaryName,
			);
			await fs.mkdir(path.dirname(serverPath), { recursive: true });
			await fs.writeFile(serverPath, 'packaged runtime');
			const context = extensionContext(extensionPath, storagePath, '2.0.1');
			const provider = createExtensionMcpServerDefinitionProvider(context);

			const definitions = await provider.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token);
			assert.ok(definitions);
			const definition = definitions[0];
			const exported = JSON.parse(renderGenericMcpConfiguration({
				command: definition.command,
				args: definition.args,
			}));

			assert.deepStrictEqual(exported.mcpServers[mcpServer.name], {
				command: serverPath,
				args: definition.args,
			});
			assert.deepStrictEqual(definition.args.slice(0, 7), [
				'mcp',
				'--addon-source-inventory',
				path.join(context.globalStorageUri.fsPath, 'addon-sources', 'workbench-graph-v1.json'),
				'--addon-index-storage',
				path.join(context.globalStorageUri.fsPath, 'addon-indexes'),
				'--external-index-mode',
				'loaded',
			]);
			assert.deepStrictEqual(definition.args.slice(7, 9), [
				'--official-wiki-root',
				path.join(extensionPath, 'data', 'official-wiki'),
			]);
		} finally {
			await fs.rm(root, { recursive: true, force: true });
		}
	});

	test('reports an actionable failure when the packaged runtime is missing', async () => {
		const root = await fs.mkdtemp(path.join(os.tmpdir(), 'rst-missing-native-mcp-'));
		try {
			const provider = createExtensionMcpServerDefinitionProvider(
				extensionContext(path.join(root, 'extension'), path.join(root, 'storage'), '2.0.1'),
			);

			await assert.rejects(
				() => Promise.resolve(provider.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token)),
				/Reforger Script Tools could not find its bundled MCP Runtime\. Reinstall or update the extension\./,
			);
		} finally {
			await fs.rm(root, { recursive: true, force: true });
		}
	});

	test('initializes the native definition, lists tools, and reads Official Wiki status', async () => {
		const storagePath = await fs.mkdtemp(path.join(os.tmpdir(), 'rst-native-mcp-acceptance-'));
		try {
			const extensionPath = path.resolve(__dirname, '..', '..');
			const provider = createExtensionMcpServerDefinitionProvider(
				extensionContext(extensionPath, storagePath, '2.0.1'),
			);
			const definition = (await provider.provideMcpServerDefinitions(
				new vscode.CancellationTokenSource().token,
			))?.[0];
			assert.ok(definition);
			const requests = [
				{
					jsonrpc: '2.0',
					id: 1,
					method: 'initialize',
					params: {
						protocolVersion: '2025-11-25',
						capabilities: {},
						clientInfo: { name: 'native-provider-acceptance', version: '1' },
					},
				},
				{ jsonrpc: '2.0', method: 'notifications/initialized' },
				{ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} },
				{
					jsonrpc: '2.0',
					id: 3,
					method: 'tools/call',
					params: { name: 'official_wiki_status', arguments: {} },
				},
				{
					jsonrpc: '2.0',
					id: 4,
					method: 'tools/call',
					params: { name: 'workbench_status', arguments: {} },
				},
			];
			const processResult = spawnSync(
				definition.command,
				definition.args,
				{
					input: `${requests.map(request => JSON.stringify(request)).join('\n')}\n`,
					encoding: 'utf8',
					timeout: 15_000,
				},
			);
			assert.ifError(processResult.error);
			assert.strictEqual(processResult.status, 0, processResult.stderr);
			const responses = processResult.stdout
				.split(/\r?\n/)
				.filter(Boolean)
				.map(line => JSON.parse(line) as {
					id?: number;
					result?: {
						protocolVersion?: string;
						tools?: Array<{ name: string }>;
						isError?: boolean;
						structuredContent?: {
							available?: boolean;
							isRunning?: boolean;
							scriptsCompiled?: boolean;
							code?: string;
						};
					};
				});
			assert.strictEqual(responses.find(response => response.id === 1)?.result?.protocolVersion, '2025-11-25');
			assert.ok(responses.find(response => response.id === 2)?.result?.tools?.some(
				tool => tool.name === 'official_wiki_status',
			));
			assert.ok(responses.find(response => response.id === 2)?.result?.tools?.some(
				tool => tool.name === 'workbench_status',
			));
			assert.strictEqual(
				responses.find(response => response.id === 3)?.result?.structuredContent?.available,
				true,
			);
			const workbenchStatus = responses.find(response => response.id === 4)?.result;
			assert.ok(workbenchStatus);
			if (workbenchStatus.isError) {
				assert.match(workbenchStatus.structuredContent?.code ?? '', /^workbench_/);
			} else {
				assert.strictEqual(typeof workbenchStatus.structuredContent?.isRunning, 'boolean');
				assert.strictEqual(typeof workbenchStatus.structuredContent?.scriptsCompiled, 'boolean');
			}
		} finally {
			await fs.rm(storagePath, { recursive: true, force: true });
		}
	});

	test('resolves a retained definition against the current launch scope', async () => {
		let policy = buildMcpLaunchPolicy({
			serverPath: 'C:\\Extension\\reforger_language_server.exe',
			addonSourceInventory: 'C:\\Storage\\graph.json',
			addonIndexStorage: 'C:\\Storage\\addon-indexes',
			externalIndexMode: 'loaded',
		});
		const provider = createMcpServerDefinitionProvider({
			extensionVersion: '2.0.1',
			resolveLaunch: async () => policy,
		});
		const original = (await provider.provideMcpServerDefinitions(new vscode.CancellationTokenSource().token))?.[0];
		assert.ok(original);

		policy = buildMcpLaunchPolicy({
			serverPath: 'C:\\Extension\\reforger_language_server.exe',
			addonSourceInventory: 'C:\\Storage\\graph.json',
			addonIndexStorage: 'C:\\Storage\\addon-indexes',
			externalIndexMode: 'none',
		});
		const resolved = await provider.resolveMcpServerDefinition?.(
			original,
			new vscode.CancellationTokenSource().token,
		);

		assert.ok(resolved);
		assert.deepStrictEqual(resolved.args.slice(-2), ['--external-index-mode', 'none']);
		assert.notStrictEqual(resolved.version, original.version);
	});

	test('publishes only material launch-scope changes through the provider event', () => {
		const workspaceFolders = new vscode.EventEmitter<void>();
		const configuration = new vscode.EventEmitter<vscode.ConfigurationChangeEvent>();
		const workspaceEvidence = new vscode.EventEmitter<void>();
		const addonSourceInventory = new vscode.EventEmitter<void>();
		const changes = createMcpLaunchScopeChangeEvent({
			onDidChangeWorkspaceFolders: workspaceFolders.event,
			onDidChangeConfiguration: configuration.event,
			onDidChangeWorkspaceEvidence: workspaceEvidence.event,
			onDidChangeAddonSourceInventory: addonSourceInventory.event,
		});
		const provider = createMcpServerDefinitionProvider({
			extensionVersion: '2.0.1',
			resolveLaunch: async () => buildMcpLaunchPolicy({
				serverPath: 'server',
				addonSourceInventory: 'inventory',
				addonIndexStorage: 'indexes',
				externalIndexMode: 'loaded',
			}),
			onDidChangeMcpServerDefinitions: changes.event,
		});
		let changeCount = 0;
		const subscription = provider.onDidChangeMcpServerDefinitions?.(() => {
			changeCount += 1;
		});
		const configurationEvent = (affected: boolean): vscode.ConfigurationChangeEvent => ({
			affectsConfiguration: section => affected && section === 'reforgerScriptTools.workbench.externalIndexMode',
		});

		configuration.fire(configurationEvent(false));
		assert.strictEqual(changeCount, 0);
		configuration.fire(configurationEvent(true));
		workspaceFolders.fire();
		workspaceEvidence.fire();
		addonSourceInventory.fire();
		assert.strictEqual(changeCount, 4);

		subscription?.dispose();
		changes.dispose();
		workspaceFolders.dispose();
		configuration.dispose();
		workspaceEvidence.dispose();
		addonSourceInventory.dispose();
	});
});

function extensionContext(
	extensionPath: string,
	globalStoragePath: string,
	version: string,
): vscode.ExtensionContext {
	return {
		extensionPath,
		extensionMode: vscode.ExtensionMode.Production,
		globalStorageUri: vscode.Uri.file(globalStoragePath),
		extension: { packageJSON: { version } },
	} as vscode.ExtensionContext;
}
