import * as vscode from 'vscode';
import { mcpServer } from '../extensionConfig/mcp';
import { workbenchConfig } from '../extensionConfig/workbench';
import { resolveMcpLaunchPolicy, type McpLaunchPolicy } from './mcpConfiguration';

export interface McpServerDefinitionProviderOptions {
	extensionVersion: string;
	resolveLaunch(): Promise<McpLaunchPolicy>;
	onDidChangeMcpServerDefinitions?: vscode.Event<void>;
}

export interface McpLaunchScopeChangeSources {
	onDidChangeWorkspaceFolders: vscode.Event<unknown>;
	onDidChangeConfiguration: vscode.Event<vscode.ConfigurationChangeEvent>;
	onDidChangeProjectDescriptors: vscode.Event<unknown>;
}

export interface McpLaunchScopeChanges extends vscode.Disposable {
	readonly event: vscode.Event<void>;
}

export function createMcpServerDefinitionProvider(
	options: McpServerDefinitionProviderOptions,
): vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition> {
	const provideDefinition = async (): Promise<vscode.McpStdioServerDefinition> => {
		const { launch, scopeIdentity } = await options.resolveLaunch();
		return new vscode.McpStdioServerDefinition(
			mcpServer.label,
			launch.command,
			launch.args,
			{},
			`${options.extensionVersion}:${scopeIdentity}`,
		);
	};
	return {
		onDidChangeMcpServerDefinitions: options.onDidChangeMcpServerDefinitions,
		provideMcpServerDefinitions: async () => [await provideDefinition()],
		resolveMcpServerDefinition: provideDefinition,
	};
}

export function createMcpLaunchScopeChangeEvent(
	sources: McpLaunchScopeChangeSources,
): McpLaunchScopeChanges {
	const emitter = new vscode.EventEmitter<void>();
	const subscriptions = [
		sources.onDidChangeWorkspaceFolders(() => emitter.fire()),
		sources.onDidChangeProjectDescriptors(() => emitter.fire()),
		sources.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration(
				`${workbenchConfig.section}.${workbenchConfig.settings.externalIndexMode}`,
			)) {
				emitter.fire();
			}
		}),
	];
	return {
		event: emitter.event,
		dispose: () => {
			for (const subscription of subscriptions) {
				subscription.dispose();
			}
			emitter.dispose();
		},
	};
}

export function createExtensionMcpServerDefinitionProvider(
	context: vscode.ExtensionContext,
	onDidChangeMcpServerDefinitions?: vscode.Event<void>,
): vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition> {
	const version = context.extension.packageJSON.version;
	if (typeof version !== 'string') {
		throw new Error('Reforger Script Tools could not determine its bundled MCP Runtime version.');
	}
	return createMcpServerDefinitionProvider({
		extensionVersion: version,
		resolveLaunch: () => resolveMcpLaunchPolicy(context),
		onDidChangeMcpServerDefinitions,
	});
}

export function registerMcpServerDefinitionProvider(
	context: vscode.ExtensionContext,
): void {
	const projectDescriptorChanges = new vscode.EventEmitter<void>();
	let projectDescriptorWatchers = watchWorkspaceProjectDescriptors(projectDescriptorChanges);
	const refreshProjectDescriptorWatchers = vscode.workspace.onDidChangeWorkspaceFolders(() => {
		disposeAll(projectDescriptorWatchers);
		projectDescriptorWatchers = watchWorkspaceProjectDescriptors(projectDescriptorChanges);
	});
	const launchScopeChanges = createMcpLaunchScopeChangeEvent({
		onDidChangeWorkspaceFolders: vscode.workspace.onDidChangeWorkspaceFolders,
		onDidChangeConfiguration: vscode.workspace.onDidChangeConfiguration,
		onDidChangeProjectDescriptors: projectDescriptorChanges.event,
	});
	context.subscriptions.push(
		projectDescriptorChanges,
		refreshProjectDescriptorWatchers,
		launchScopeChanges,
		{ dispose: () => disposeAll(projectDescriptorWatchers) },
		vscode.lm.registerMcpServerDefinitionProvider(
			mcpServer.providerId,
			createExtensionMcpServerDefinitionProvider(context, launchScopeChanges.event),
		),
	);
}

function watchWorkspaceProjectDescriptors(
	emitter: vscode.EventEmitter<void>,
): vscode.Disposable[] {
	return (vscode.workspace.workspaceFolders ?? []).flatMap(folder => {
		const watcher = vscode.workspace.createFileSystemWatcher(
			new vscode.RelativePattern(folder, '*.gproj'),
		);
		return [
			watcher,
			watcher.onDidCreate(() => emitter.fire()),
			watcher.onDidChange(() => emitter.fire()),
			watcher.onDidDelete(() => emitter.fire()),
		];
	});
}

function disposeAll(disposables: vscode.Disposable[]): void {
	for (const disposable of disposables) {
		disposable.dispose();
	}
}
