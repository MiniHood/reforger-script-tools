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

export function blockCommentPairPosition(changes: readonly vscode.TextDocumentContentChangeEvent[]): vscode.Position | undefined {
	if (changes.length !== 1 || changes[0].rangeLength !== 0 || changes[0].text !== '**/') {
		return undefined;
	}
	const change = changes[0];
	return new vscode.Position(change.range.start.line, change.range.start.character + 1);
}

export function enterAfterPosition(changes: readonly vscode.TextDocumentContentChangeEvent[]): vscode.Position | undefined {
	if (changes.length !== 1 || changes[0].rangeLength !== 0 || !/^\r?\n[\t ]*$/.test(changes[0].text)) {
		return undefined;
	}
	const change = changes[0];
	const newline = change.text.lastIndexOf('\n');
	return new vscode.Position(change.range.start.line + 1, change.text.length - newline - 1);
}

/**
 * Identifies one native Tab indentation edit. VS Code writes spaces rather
 * than a tab character when `editor.insertSpaces` is enabled, so the accepted
 * text must match the active indentation unit exactly.
 */
export function tabAfterPosition(
	changes: readonly vscode.TextDocumentContentChangeEvent[],
	tabSize: vscode.TextEditorOptions['tabSize'] = 4,
	insertSpaces = false,
): vscode.Position | undefined {
	const resolvedTabSize = typeof tabSize === 'number' ? Math.min(Math.max(tabSize, 1), 16) : 4;
	const indentationUnit = insertSpaces ? ' '.repeat(resolvedTabSize) : '\t';
	if (changes.length !== 1 || changes[0].rangeLength !== 0 || changes[0].text !== indentationUnit) {
		return undefined;
	}
	const change = changes[0];
	return new vscode.Position(change.range.start.line, change.range.start.character + indentationUnit.length);
}
