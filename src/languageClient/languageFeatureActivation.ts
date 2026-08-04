import * as vscode from 'vscode';
import { languageClientLanguage } from '../extensionConfig/languageClient';

export interface LanguageFeatureActivationSource {
	readonly textDocuments: readonly vscode.TextDocument[];
	readonly onDidOpenTextDocument: vscode.Event<vscode.TextDocument>;
}

export function registerLanguageFeatureActivation(
	source: LanguageFeatureActivationSource,
	start: () => void,
): vscode.Disposable {
	let started = false;
	let subscription: vscode.Disposable | undefined;
	const startOnce = (): void => {
		if (started) {
			return;
		}
		started = true;
		subscription?.dispose();
		start();
	};
	if (source.textDocuments.some(document => document.languageId === languageClientLanguage.id)) {
		startOnce();
		return new vscode.Disposable(() => undefined);
	}
	subscription = source.onDidOpenTextDocument(document => {
		if (document.languageId === languageClientLanguage.id) {
			startOnce();
		}
	});
	return subscription;
}
