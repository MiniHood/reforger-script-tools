export const languageClientIds = {
	id: 'reforgerScriptTools.languageClient',
	name: 'Reforger Script Tools Language Server',
	debugOutputName: 'Reforger Script Tools Hover Debug',
	completionDebugOutputName: 'Reforger Script Tools Completion Debug',
} as const;

export const languageClientCrashHandling = {
	maxRestartCount: 4,
	restartWindowMs: 3 * 60 * 1000,
	finalCrashMessage: 'Reforger Script Tools Language Server Crashed',
} as const;

export const languageClientCompletion = {
	// Cleanup only: it never delays or triggers completion. Keep a multi-field
	// snippet transaction alive long enough for a person to choose each value.
	snippetSuggestTransactionTimeoutMs: 30_000,
} as const;

export const experimentalAutoFormattingConfig = {
	section: 'reforgerScriptTools',
	setting: 'experimentalAutoFormatting',
} as const;

export const languageClientCommands = {
	debugHoverAtCursor: 'reforger-sript-tools.debug.hoverAtCursor',
	debugCompletionAtCursor: 'reforger-sript-tools.debug.completionAtCursor',
	triggerSuggestAtSnippetPlaceholder: 'reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholder',
	triggerSuggestAfterCustomCollectionInitializer: 'reforger-sript-tools.completion.triggerSuggestAfterCustomCollectionInitializer',
	advanceSnippetPlaceholderAfterAccept: 'reforger-sript-tools.completion.advanceSnippetPlaceholderAfterAccept',
	normalizeIfSpaceCommit: 'reforger-sript-tools.completion.normalizeIfSpaceCommit',
	applyDirectiveSeparator: 'reforger-sript-tools.completion.applyDirectiveSeparator',
	insertNewline: 'reforger-sript-tools.input.insertNewline',
	indent: 'reforger-sript-tools.input.indent',
	insertSpace: 'reforger-sript-tools.input.insertSpace',
	openSymbolLocation: 'reforger-sript-tools.openSymbolLocation',
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
	startupTimingLogFile: 'language-client-startup.log',
	hoverDebugFolder: 'hover-debug',
	hoverDebugLatestFile: 'latest.md',
	completionDebugFolder: 'completion-debug',
	completionDebugLatestFile: 'latest.md',
} as const;

export const languageClientIndexCache = {
	rootFolder: 'index-cache',
	gameDataIndexFile: 'game-data-symbol-index.v9.bin',
} as const;

export const languageClientLanguage = {
	id: 'enforce',
} as const;

export const languageClientRequests = {
	debugHover: 'reforger/debugHover',
	debugCompletion: 'reforger/debugCompletion',
	inputRoute: 'reforger/inputRoute',
	blockCommentPair: 'reforger/blockCommentPair',
} as const;

export const languageClientNotifications = {
	workspaceFileChanged: 'reforger/workspaceFileChanged',
	workspaceFileDeleted: 'reforger/workspaceFileDeleted',
} as const;

export const languageClientDocumentSelector = [
	{ scheme: 'file', language: languageClientLanguage.id },
] as const;
