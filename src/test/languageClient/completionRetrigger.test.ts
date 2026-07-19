import * as assert from 'node:assert';
import * as vscode from 'vscode';
import {
	canTriggerCompletionWhenActive,
	shouldRetriggerCompletionAfterDeletion,
	shouldRetriggerCompletionAfterInsertion,
} from '../../languageClient/languageClient';

function editorForPrefix(
	prefix: string,
	languageId = 'enforce',
	selectionIsEmpty = true,
): vscode.TextEditor {
	const active = new vscode.Position(0, prefix.length);
	const anchor = selectionIsEmpty ? active : new vscode.Position(0, 0);
	return {
		document: {
			uri: vscode.Uri.parse(`untitled:completion-retrigger-${languageId}`),
			languageId,
			lineAt: () => ({ text: prefix }),
		},
		selection: new vscode.Selection(anchor, active),
	} as unknown as vscode.TextEditor;
}

suite('completion retrigger guards', () => {
	test('forwards valid identifier prefixes without inspecting Enforce syntax', () => {
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('value')), true);
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('// value')), true);
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('"value')), true);
		assert.strictEqual(shouldRetriggerCompletionAfterDeletion(editorForPrefix('// value')), true);
	});

	test('keeps editor-state and prefix suppression at the client boundary', () => {
		const validEditor = editorForPrefix('value');
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('v')), false);
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('value-')), false);
		assert.strictEqual(shouldRetriggerCompletionAfterInsertion(editorForPrefix('value', 'enforce', false)), false);
		assert.strictEqual(
			canTriggerCompletionWhenActive(validEditor, validEditor.document.uri.toString(), shouldRetriggerCompletionAfterInsertion),
			true,
		);
		assert.strictEqual(
			canTriggerCompletionWhenActive(undefined, validEditor.document.uri.toString(), shouldRetriggerCompletionAfterInsertion),
			false,
		);
		assert.strictEqual(
			canTriggerCompletionWhenActive(editorForPrefix('value', 'plaintext'), validEditor.document.uri.toString(), shouldRetriggerCompletionAfterInsertion),
			false,
		);
		assert.strictEqual(
			canTriggerCompletionWhenActive(validEditor, 'untitled:another-document', shouldRetriggerCompletionAfterInsertion),
			false,
		);
	});
});
