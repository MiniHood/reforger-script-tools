import * as vscode from 'vscode';

interface SemanticTokensLike {
	data: Uint32Array;
}

interface BoundarySnapshot {
	version: number;
	ranges: readonly vscode.Range[];
}

export function semanticTokenBoundaryGuardDecorationOptions(): vscode.DecorationRenderOptions {
	return {
		color: new vscode.ThemeColor('editor.foreground'),
		rangeBehavior: vscode.DecorationRangeBehavior.OpenOpen,
	};
}

export function semanticTokenBoundaryRanges(
	tokens: SemanticTokensLike | null | undefined,
): vscode.Range[] {
	if (!tokens) {
		return [];
	}

	const boundaries: vscode.Position[] = [];
	const appendBoundary = (position: vscode.Position) => {
		if (!boundaries.at(-1)?.isEqual(position)) {
			boundaries.push(position);
		}
	};
	let line = 0;
	let character = 0;
	for (let index = 0; index + 4 < tokens.data.length; index += 5) {
		const deltaLine = tokens.data[index];
		const deltaCharacter = tokens.data[index + 1];
		const length = tokens.data[index + 2];
		line += deltaLine;
		character = deltaLine === 0 ? character + deltaCharacter : deltaCharacter;
		if (length === 0) {
			continue;
		}
		appendBoundary(new vscode.Position(line, character));
		appendBoundary(new vscode.Position(line, character + length));
	}

	return boundaries.map(position => new vscode.Range(position, position));
}

export interface SemanticTokenBoundaryGuardBridge extends vscode.Disposable {
	update(
		document: vscode.TextDocument,
		version: number,
		tokens: SemanticTokensLike | null | undefined,
	): void;
}

export function registerSemanticTokenBoundaryGuardBridge(): SemanticTokenBoundaryGuardBridge {
	const decoration = vscode.window.createTextEditorDecorationType(
		semanticTokenBoundaryGuardDecorationOptions(),
	);
	const snapshots = new Map<string, BoundarySnapshot>();
	let disposed = false;

	const applySnapshot = (editor: vscode.TextEditor) => {
		const snapshot = snapshots.get(editor.document.uri.toString());
		editor.setDecorations(
			decoration,
			snapshot?.version === editor.document.version ? snapshot.ranges : [],
		);
	};
	const visibleEditorChanges = vscode.window.onDidChangeVisibleTextEditors(editors => {
		for (const editor of editors) {
			applySnapshot(editor);
		}
	});
	const documentCloses = vscode.workspace.onDidCloseTextDocument(document => {
		snapshots.delete(document.uri.toString());
	});

	return {
		update: (document, version, tokens) => {
			if (disposed || document.version !== version) {
				return;
			}
			const snapshot: BoundarySnapshot = {
				version,
				ranges: semanticTokenBoundaryRanges(tokens),
			};
			snapshots.set(document.uri.toString(), snapshot);
			for (const editor of vscode.window.visibleTextEditors) {
				if (editor.document.uri.toString() === document.uri.toString()) {
					applySnapshot(editor);
				}
			}
		},
		dispose: () => {
			disposed = true;
			snapshots.clear();
			for (const editor of vscode.window.visibleTextEditors) {
				editor.setDecorations(decoration, []);
			}
			visibleEditorChanges.dispose();
			documentCloses.dispose();
			decoration.dispose();
		},
	};
}
