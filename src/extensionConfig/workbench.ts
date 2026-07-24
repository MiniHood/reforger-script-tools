export const workbenchConfig = {
	section: 'reforgerScriptTools.workbench',
	settings: {
		enabled: 'enabled',
		host: 'host',
		port: 'port',
	},
} as const;

export const workbenchDefaults = {
	enabled: true,
	host: '127.0.0.1',
	port: 5775,
} as const;

export const workbenchCommands = {
	validateScripts: 'reforger-sript-tools.workbench.validateScripts',
	openCompilerDiagnostic: 'reforger-sript-tools.workbench.openCompilerDiagnostic',
} as const;

export const workbenchTestCommands = {
	observeCompiler: 'reforger-sript-tools.test.observeWorkbenchCompiler',
	disposeCompiler: 'reforger-sript-tools.test.disposeWorkbenchCompiler',
	restartCompiler: 'reforger-sript-tools.test.restartWorkbenchCompiler',
	armStartupValidation: 'reforger-sript-tools.test.armWorkbenchStartupValidation',
} as const;

export const workbenchDiagnostics = {
	collectionName: 'Workbench',
	source: 'Workbench Compiler',
	outputChannelName: 'Reforger Workbench Compiler',
	outputLanguageId: 'reforger-workbench-compiler-output',
} as const;
