export const gameDataRepository = {
	owner: 'BohemiaInteractive',
	name: 'Arma-Reforger-Script-Diff',
	branch: 'main',
} as const;

export const gameDataCommands = {
	checkForUpdates: 'reforger-sript-tools.gameData.checkForUpdates',
	openStorageFolder: 'reforger-sript-tools.gameData.openStorageFolder',
	selectManualFolder: 'reforger-sript-tools.gameData.selectManualFolder',
} as const;

export const gameDataConfig = {
	section: 'reforgerScriptTools.gameData',
	settings: {
		manualFolder: 'manualFolder',
	},
} as const;

export const gameDataStateKeys = {
	downloadAllowed: 'reforgerScriptTools.gameData.downloadAllowed',
	warnedLowScriptCountManualFolders: 'reforgerScriptTools.gameData.warnedLowScriptCountManualFolders',
} as const;

export const gameDataStorage = {
	rootFolder: 'game-data',
	scriptsFolder: 'scripts',
	metadataFile: 'metadata.json',
	stagingPrefix: 'staging-',
} as const;

export const gameDataThresholds = {
	lowScriptCount: 5000,
} as const;
