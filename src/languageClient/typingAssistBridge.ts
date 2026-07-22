import * as vscode from 'vscode';

export interface TypingAssistRequest {
	textDocument: { uri: string };
	position: { line: number; character: number };
	version: number;
	options: { tabSize: vscode.TextEditorOptions['tabSize']; insertSpaces: vscode.TextEditorOptions['insertSpaces'] };
	ch?: '\n';
}

export interface InputRouteRequest {
	textDocument: { uri: string };
	version: number;
	operation: 'insertNewline';
	selections: Array<{ start: { line: number; character: number }; end: { line: number; character: number } }>;
	options: { tabSize: vscode.TextEditorOptions['tabSize']; insertSpaces: vscode.TextEditorOptions['insertSpaces'] };
}

export function inputRouteRequest(document: vscode.TextDocument, editor: vscode.TextEditor): InputRouteRequest {
	return {
		textDocument: { uri: document.uri.toString() },
		version: document.version,
		operation: 'insertNewline',
		selections: editor.selections.map(selection => ({
			start: { line: selection.start.line, character: selection.start.character },
			end: { line: selection.end.line, character: selection.end.character },
		})),
		options: { tabSize: editor.options.tabSize, insertSpaces: editor.options.insertSpaces },
	};
}

/** Builds editor transport data only; Rust decides whether an assist applies. */
export function typingAssistRequest(
	document: vscode.TextDocument,
	position: vscode.Position,
	editor: vscode.TextEditor,
	trigger?: '\n',
): TypingAssistRequest {
	return {
		textDocument: { uri: document.uri.toString() },
		position: { line: position.line, character: position.character },
		version: document.version,
		options: { tabSize: editor.options.tabSize, insertSpaces: editor.options.insertSpaces },
		...(trigger ? { ch: trigger } : {}),
	};
}

export function blockCommentPairPosition(changes: readonly vscode.TextDocumentContentChangeEvent[]): vscode.Position | undefined {
	if (changes.length !== 1 || changes[0].rangeLength !== 0 || changes[0].text !== '**/') {
		return undefined;
	}
	const change = changes[0];
	return new vscode.Position(change.range.start.line, change.range.start.character + 1);
}
