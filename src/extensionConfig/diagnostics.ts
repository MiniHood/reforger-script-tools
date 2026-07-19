export const diagnosticsConfig = {
	section: 'reforgerScriptTools.diagnostics',
	settings: {
		enabled: 'enabled',
	},
} as const;

export const diagnosticsLogs = {
	rootFolder: 'logs',
	extensionFile: 'extension-diagnostics.jsonl',
	serverFile: 'language-server-diagnostics.jsonl',
	maxBytes: 1024 * 1024,
} as const;
