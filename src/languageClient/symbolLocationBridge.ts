import * as vscode from 'vscode';

interface OpenSymbolLocationArgs {
	uri: string;
	startByte: number;
	endByte: number;
}

/** Applies the Rust-authored byte range in VS Code without interpreting source. */
export async function openSymbolLocation(args: unknown): Promise<void> {
	if (!isOpenSymbolLocationArgs(args)) {
		vscode.window.showWarningMessage('Invalid Reforger symbol location.');
		return;
	}

	const uri = vscode.Uri.parse(args.uri, true);
	const document = await vscode.workspace.openTextDocument(uri);
	const editor = await vscode.window.showTextDocument(document);
	const range = rangeFromByteOffsets(document.getText(), args.startByte, args.endByte);
	editor.selection = new vscode.Selection(range.start, range.end);
	editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
}

function isOpenSymbolLocationArgs(value: unknown): value is OpenSymbolLocationArgs {
	if (!value || typeof value !== 'object') {
		return false;
	}
	const candidate = value as Partial<OpenSymbolLocationArgs>;
	return typeof candidate.uri === 'string'
		&& Number.isInteger(candidate.startByte) && Number.isInteger(candidate.endByte)
		&& candidate.startByte !== undefined && candidate.endByte !== undefined
		&& candidate.startByte >= 0 && candidate.endByte >= candidate.startByte;
}

function rangeFromByteOffsets(text: string, startByte: number, endByte: number): vscode.Range {
	return new vscode.Range(positionFromByteOffset(text, startByte), positionFromByteOffset(text, endByte));
}

export function positionFromByteOffset(text: string, byteOffset: number): vscode.Position {
	let line = 0;
	let character = 0;
	let consumedBytes = 0;
	for (const char of text) {
		const charBytes = Buffer.byteLength(char, 'utf8');
		if (consumedBytes + charBytes > byteOffset) {
			break;
		}
		consumedBytes += charBytes;
		if (char === '\n') { line += 1; character = 0; } else { character += char.length; }
	}
	return new vscode.Position(line, character);
}
