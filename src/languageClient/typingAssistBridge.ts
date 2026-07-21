import * as vscode from 'vscode';

export interface TypingAssistRequest {
	textDocument: { uri: string };
	position: { line: number; character: number };
	version: number;
	options: { tabSize: vscode.TextEditorOptions['tabSize']; insertSpaces: vscode.TextEditorOptions['insertSpaces'] };
	ch?: '\n' | '\t';
}

/** Builds editor transport data only; Rust decides whether an assist applies. */
export function typingAssistRequest(
	document: vscode.TextDocument,
	position: vscode.Position,
	editor: vscode.TextEditor,
	trigger?: '\n' | '\t',
): TypingAssistRequest {
	return {
		textDocument: { uri: document.uri.toString() },
		position: { line: position.line, character: position.character },
		version: document.version,
		options: { tabSize: editor.options.tabSize, insertSpaces: editor.options.insertSpaces },
		...(trigger ? { ch: trigger } : {}),
	};
}
