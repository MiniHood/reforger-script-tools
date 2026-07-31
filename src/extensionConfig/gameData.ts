export const gameDataCommands = {
	refreshSources: 'reforger-sript-tools.gameData.refreshSources',
	openStorageFolder: 'reforger-sript-tools.gameData.openStorageFolder',
	openIndexReport: 'reforger-sript-tools.gameData.openIndexReport',
} as const;

export const gameDataStorage = {
	rootFolder: 'addon-sources',
	inventoryFile: 'workbench-graph-v1.json',
	indexReportFile: 'addon-index-report.md',
} as const;
