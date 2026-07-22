import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { experimentalAutoFormattingEnabled } from '../extensionConfig/experimentalAutoFormatting';
import { languageClientCommands, languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { typingAssistRequest } from './typingAssistBridge';
import { applyVersionedEditorEdits, isCurrentSingleCaret, type VersionedEditResponse } from './versionedEditorEdit';

const nativeTypeCommand = 'type';

export function registerControlHeaderEnter(getClient: () => LanguageClient | undefined): vscode.Disposable {
	return vscode.commands.registerCommand(languageClientCommands.controlHeaderEnter, async () => {
		const editor = vscode.window.activeTextEditor;
		if (!editor || editor.document.languageId !== languageClientLanguage.id || !experimentalAutoFormattingEnabled()
			|| !mayBeControlHeader(editor)) {
			await typeNativeEnter();
			return;
		}
		const client = getClient();
		const position = editor.selection.active;
		const version = editor.document.version;
		if (!client || !editor.selection.isEmpty || editor.selections.length !== 1) {
			await typeNativeEnter();
			return;
		}
		try {
			const response = await client.sendRequest<VersionedEditResponse>(
				languageClientRequests.controlHeaderEnter,
				typingAssistRequest(editor.document, position, editor, '\n'),
			);
			if (response.edits.length > 0 && isCurrentSingleCaret(editor.document, version, position)) {
				const applied = await applyVersionedEditorEdits(editor, response);
				diagnostic('formatting.controlHeaderEnter', { outcome: applied ? 'applied' : 'editRejected', version });
				return;
			}
			if (editor.document.version === version && editor.selection.active.isEqual(position)) {
				await typeNativeEnter();
			}
		} catch {
			if (editor.document.version === version && editor.selection.active.isEqual(position)) {
				await typeNativeEnter();
			}
		}
	});
}

function mayBeControlHeader(editor: vscode.TextEditor): boolean {
	const position = editor.selection.active;
	const line = editor.document.lineAt(position.line).text;
	const before = line.slice(0, position.character);
	const after = line.slice(position.character);
	return /\b(?:for|foreach|while|switch)\s*\([^{};]*$/.test(before)
		&& (after.includes(')') || /\)\s*$/.test(before));
}

async function typeNativeEnter(): Promise<void> {
	await vscode.commands.executeCommand(nativeTypeCommand, { text: '\n' });
}
