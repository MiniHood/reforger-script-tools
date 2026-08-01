import * as vscode from 'vscode';
import {
	type BracketColoringMode,
} from '../extensionConfig/bracketColoring';
import {
	languageClientLanguage,
} from '../extensionConfig/languageClient';

export async function applyBracketColoringEditorMode(
	mode: BracketColoringMode,
): Promise<void> {
	const native = mode === 'vscode';
	const configuration = vscode.workspace.getConfiguration('editor', {
		languageId: languageClientLanguage.id,
	});
	if (configuration.inspect<boolean>('bracketPairColorization.enabled')?.globalLanguageValue !== native) {
		await configuration.update(
			'bracketPairColorization.enabled',
			native,
			vscode.ConfigurationTarget.Global,
			true,
		);
	}
	const matchBrackets = native ? 'always' : 'never';
	if (configuration.inspect<string>('matchBrackets')?.globalLanguageValue !== matchBrackets) {
		await configuration.update(
			'matchBrackets',
			matchBrackets,
			vscode.ConfigurationTarget.Global,
			true,
		);
	}
}

export function bracketColoringServerArguments(
	mode: BracketColoringMode,
): string[] {
	return ['--bracket-coloring', mode];
}

export function usesCustomScopeDelimiterPresentation(
	mode: BracketColoringMode,
): boolean {
	return mode !== 'vscode';
}
