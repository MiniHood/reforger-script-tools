export const languageClientIds = {
	id: 'reforgerScriptTools.languageClient',
	name: 'Reforger Script Tools Language Server',
	debugOutputName: 'Reforger Script Tools Hover Debug',
} as const;

export const languageClientCommands = {
	debugHoverAtCursor: 'reforger-sript-tools.debug.hoverAtCursor',
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
	hoverDebugFolder: 'hover-debug',
	hoverDebugLatestFile: 'latest.md',
} as const;

export const languageClientIndexCache = {
	rootFolder: 'index-cache',
	gameDataIndexFile: 'game-data-symbol-index.v2.json',
} as const;

export const languageClientLanguage = {
	id: 'enforce',
} as const;

export const languageClientRequests = {
	debugHover: 'reforger/debugHover',
} as const;

export const languageClientDocumentSelector = [
	{ scheme: 'file', language: languageClientLanguage.id },
] as const;
