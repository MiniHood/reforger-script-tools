export const mcpCommands = {
	copyConfiguration: 'reforger-sript-tools.mcp.copyConfiguration',
} as const;

export const mcpServer = {
	name: 'reforger-script-tools',
	providerId: 'reforger-script-tools.mcp-runtime',
	label: 'Reforger Script Tools',
	runtimeUnavailableMessage: 'Reforger Script Tools could not find its bundled MCP Runtime. Reinstall or update the extension.',
	initializationDeadlineSeconds: 120,
	toolTimeoutSeconds: 130,
} as const;
