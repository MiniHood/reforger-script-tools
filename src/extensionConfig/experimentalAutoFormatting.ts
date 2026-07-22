import * as vscode from 'vscode';
import { experimentalAutoFormattingConfig } from './languageClient';

/** Reads the editor-owned preference at the moment an automatic edit applies. */
export function experimentalAutoFormattingEnabled(): boolean {
	return vscode.workspace.getConfiguration(experimentalAutoFormattingConfig.section)
		.get<boolean>(experimentalAutoFormattingConfig.setting, true);
}
