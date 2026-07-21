import * as vscode from 'vscode';

export interface LspPosition {
	line: number;
	character: number;
}

export interface LspRange {
	start: LspPosition;
	end: LspPosition;
}

export interface LspTextEdit {
	range: LspRange;
	newText: string;
}

export interface VersionedEditResponse {
	edits: readonly LspTextEdit[];
	selection?: LspPosition;
}

export function isCurrentSingleCaret(
	document: vscode.TextDocument,
	expectedVersion: number,
	expectedPosition: vscode.Position,
): boolean {
	const editor = vscode.window.activeTextEditor;
	return editor?.document.uri.toString() === document.uri.toString()
		&& document.version === expectedVersion
		&& editor.selections.length === 1
		&& editor.selection.isEmpty
		&& editor.selection.active.isEqual(expectedPosition);
}

export async function applyVersionedEditorEdits(
	editor: vscode.TextEditor,
	response: VersionedEditResponse,
): Promise<boolean> {
	const applied = await editor.edit(
		editBuilder => response.edits.forEach(edit => editBuilder.replace(rangeFromLsp(edit.range), edit.newText)),
		{ undoStopBefore: false, undoStopAfter: false },
	);
	if (applied && response.selection) {
		const position = new vscode.Position(response.selection.line, response.selection.character);
		editor.selection = new vscode.Selection(position, position);
	}
	return applied;
}

export function rangeFromLsp(range: LspRange): vscode.Range {
	return new vscode.Range(
		new vscode.Position(range.start.line, range.start.character),
		new vscode.Position(range.end.line, range.end.character),
	);
}
