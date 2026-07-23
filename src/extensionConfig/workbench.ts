export const workbenchConfig = {
	section: 'reforgerScriptTools.workbench',
	settings: {
		enabled: 'enabled',
		host: 'host',
		port: 'port',
		validationDelaySeconds: 'compilerValidationDelaySeconds',
		validationProfile: 'compilerValidationProfile',
	},
} as const;

export const workbenchDefaults = {
	enabled: true,
	host: '127.0.0.1',
	port: 5775,
	validationDelaySeconds: 3,
	validationProfile: 'WORKBENCH',
} as const;

export const workbenchCommands = {
	validateScripts: 'reforger-sript-tools.workbench.validateScripts',
} as const;

export const workbenchDiagnostics = {
	collectionName: 'Workbench',
	source: 'Workbench Compiler',
} as const;
