import * as vscode from 'vscode';
import {
	languageClientLanguage,
	languageClientNotifications,
	languageClientRequests,
} from '../extensionConfig/languageClient';
import {
	type LspRange,
	rangeFromLsp,
} from './versionedEditorEdit';

interface ActiveScopeDelimiterResponse {
	version: number;
	pending?: boolean;
	pairs: Array<{
		opener: LspRange;
		closer: LspRange;
	}>;
}

export interface ActiveScopeDelimiterRequestClient {
	sendRequest<Result>(method: string, params: unknown): Promise<Result>;
}

interface ActiveScopeDelimiterBridgeClient extends ActiveScopeDelimiterRequestClient {
	onNotification(
		method: string,
		handler: (params: { uri: string; version: number }) => void,
	): vscode.Disposable;
}

export function activeScopeDelimiterDecorationOptions(): vscode.DecorationRenderOptions {
	return {
		backgroundColor: new vscode.ThemeColor('editorBracketMatch.background'),
		borderColor: new vscode.ThemeColor('editorBracketMatch.border'),
		borderStyle: 'solid',
		borderWidth: '1px',
		rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
	};
}

interface ActiveScopeDelimiterProjection {
	pending: boolean;
	ranges: vscode.Range[];
}

async function activeScopeDelimiterProjectionForSnapshot(
	document: vscode.TextDocument,
	selections: readonly vscode.Selection[],
	client: ActiveScopeDelimiterRequestClient,
	isCurrent: () => boolean,
): Promise<ActiveScopeDelimiterProjection | undefined> {
	const version = document.version;
	const response = await client.sendRequest<ActiveScopeDelimiterResponse>(
		languageClientRequests.activeScopeDelimiters,
		{
			textDocument: { uri: document.uri.toString() },
			version,
			positions: selections.map(selection => ({
				line: selection.active.line,
				character: selection.active.character,
			})),
		},
	);
	if (!isCurrent() || response.version !== version) {
		return undefined;
	}
	return {
		pending: response.pending === true,
		ranges: response.pairs.flatMap(pair => [
			rangeFromLsp(pair.opener),
			rangeFromLsp(pair.closer),
		]),
	};
}

export async function activeScopeDelimiterRangesForSnapshot(
	document: vscode.TextDocument,
	selections: readonly vscode.Selection[],
	client: ActiveScopeDelimiterRequestClient,
	isCurrent: () => boolean,
): Promise<vscode.Range[] | undefined> {
	return (await activeScopeDelimiterProjectionForSnapshot(
		document,
		selections,
		client,
		isCurrent,
	))?.ranges;
}

export async function refreshActiveScopeDelimiterDecorationForSnapshot(
	document: vscode.TextDocument,
	selections: readonly vscode.Selection[],
	client: ActiveScopeDelimiterRequestClient,
	isCurrent: () => boolean,
	setRanges: (ranges: readonly vscode.Range[]) => void,
): Promise<boolean> {
	const projection = await activeScopeDelimiterProjectionForSnapshot(
		document,
		selections,
		client,
		isCurrent,
	);
	if (projection && !projection.pending && isCurrent()) {
		setRanges(projection.ranges);
	}
	return projection?.pending !== true;
}

export function registerActiveScopeDelimiterBridge(
	client: ActiveScopeDelimiterBridgeClient,
): vscode.Disposable {
	const decoration = vscode.window.createTextEditorDecorationType(
		activeScopeDelimiterDecorationOptions(),
	);
	let disposed = false;
	let generation = 0;
	let decoratedEditor: vscode.TextEditor | undefined;

	const clearDecoratedEditor = () => {
		decoratedEditor?.setDecorations(decoration, []);
		decoratedEditor = undefined;
	};
	const refresh = (editor: vscode.TextEditor | undefined = vscode.window.activeTextEditor) => {
		const requestGeneration = ++generation;
		if (!editor || editor.document.languageId !== languageClientLanguage.id) {
			clearDecoratedEditor();
			return;
		}
		if (decoratedEditor && decoratedEditor !== editor) {
			decoratedEditor.setDecorations(decoration, []);
		}
		decoratedEditor = editor;
		const version = editor.document.version;
		const selectionSnapshot = selectionKey(editor.selections);
		const isCurrent = () =>
			!disposed
			&& generation === requestGeneration
			&& vscode.window.activeTextEditor === editor
			&& editor.document.version === version
			&& selectionKey(editor.selections) === selectionSnapshot;
		void refreshActiveScopeDelimiterDecorationForSnapshot(
			editor.document,
			editor.selections,
			client,
			isCurrent,
			ranges => editor.setDecorations(decoration, ranges),
		).catch(() => {
			if (isCurrent()) {
				editor.setDecorations(decoration, []);
			}
		});
	};

	const foregroundReady = client.onNotification(
		languageClientNotifications.foregroundReady,
		params => {
			const editor = vscode.window.activeTextEditor;
			if (editor
				&& editor.document.uri.toString() === params.uri
				&& editor.document.version === params.version) {
				refresh(editor);
			}
		},
	);
	const activeEditorChanges = vscode.window.onDidChangeActiveTextEditor(editor => {
		refresh(editor);
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		if (event.textEditor === vscode.window.activeTextEditor) {
			refresh(event.textEditor);
		}
	});
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		const editor = vscode.window.activeTextEditor;
		if (editor?.document.uri.toString() === event.document.uri.toString()) {
			refresh(editor);
		}
	});
	queueMicrotask(() => refresh());

	return vscode.Disposable.from(
		activeEditorChanges,
		foregroundReady,
		selectionChanges,
		documentChanges,
		{
			dispose: () => {
				disposed = true;
				generation += 1;
				clearDecoratedEditor();
				decoration.dispose();
			},
		},
	);
}

function selectionKey(selections: readonly vscode.Selection[]): string {
	return selections
		.map(selection =>
			`${selection.anchor.line}:${selection.anchor.character}-${selection.active.line}:${selection.active.character}`)
		.join('|');
}
