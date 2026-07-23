import * as vscode from 'vscode';

export const bracketColoringConfig = {
	section: 'reforgerScriptTools',
	setting: 'bracketColoring',
	defaultMode: 'semantic',
} as const;

export type BracketColoringMode = 'semantic' | 'punctuation' | 'vscode';

export function getBracketColoringMode(): BracketColoringMode {
	const value = vscode.workspace
		.getConfiguration(bracketColoringConfig.section)
		.get<unknown>(bracketColoringConfig.setting);
	return value === 'punctuation' || value === 'vscode'
		? value
		: bracketColoringConfig.defaultMode;
}
