export const gameDataCommands = {
	refreshSources: 'reforger-sript-tools.gameData.refreshSources',
	openStorageFolder: 'reforger-sript-tools.gameData.openStorageFolder',
	selectBaseGameAddonsFolder: 'reforger-sript-tools.gameData.selectBaseGameAddonsFolder',
	selectWorkbenchAddonsFolder: 'reforger-sript-tools.gameData.selectWorkbenchAddonsFolder',
	selectUserAddonsFolder: 'reforger-sript-tools.gameData.selectUserAddonsFolder',
} as const;

export const gameDataConfig = {
	section: 'reforgerScriptTools.gameData',
	settings: {
		baseGameAddonsFolder: 'baseGameAddonsFolder',
		workbenchAddonsFolder: 'workbenchAddonsFolder',
		userAddonsFolder: 'userAddonsFolder',
	},
} as const;

export const gameDataStorage = {
	rootFolder: 'addon-sources',
	inventoryPrefix: 'inventory-v1-',
} as const;
