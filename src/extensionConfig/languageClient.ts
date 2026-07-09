export const languageClientIds = {
	id: 'reforgerScriptTools.languageClient',
	name: 'Reforger Script Tools Language Server',
} as const;

export const languageClientServer = {
	binaryName: process.platform === 'win32' ? 'reforger_language_server.exe' : 'reforger_language_server',
	distFolder: 'server',
	devBinaryRelativePath: process.platform === 'win32'
		? ['server', 'target', 'debug', 'reforger_language_server.exe']
		: ['server', 'target', 'debug', 'reforger_language_server'],
} as const;

export const languageClientLogs = {
	rootFolder: 'logs',
	serverLogFile: 'language-server.log',
} as const;

export const languageClientDocumentSelector = [
	{ scheme: 'file', pattern: '**/{Scripts,scripts}/**/*.c' },
] as const;
