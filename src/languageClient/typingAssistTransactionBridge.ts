import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { applyVersionedEditorEdits, isCurrentSingleCaret, type VersionedEditResponse } from './versionedEditorEdit';
import { blockCommentPairPosition, enterAfterPosition, tabAfterPosition, typingAssistRequest } from './typingAssistBridge';

interface BlockCommentPairResponse extends VersionedEditResponse {}

interface EnterTypingAssistResponse extends VersionedEditResponse {}

export function registerBlockCommentPair(getClient: () => LanguageClient | undefined): vscode.Disposable {
	let pending: BlockCommentPairTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.commentPair', { outcome: 'superseded', version: pending.version });
			pending = undefined;
		}
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}
		const position = blockCommentPairPosition(event.contentChanges);
		if (!position) {
			return;
		}
		const transaction: BlockCommentPairTransaction = {
			document: event.document,
			version: event.document.version,
			prePairPosition: event.contentChanges[0].range.start,
			position,
			caretReady: hasSingleEmptyCaretAt(event.document, position, event.document.version),
		};
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestBlockCommentPair(transaction, getClient, () => pending === transaction, () => {
					pending = undefined;
				});
			}
		});
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		const transaction = pending;
		if (!transaction || event.textEditor.document.uri.toString() !== transaction.document.uri.toString()) {
			return;
		}
		if (hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
			transaction.caretReady = true;
			void applyPendingBlockCommentPair(transaction, () => pending === transaction, () => {
				pending = undefined;
			});
			return;
		}
		if (transaction.document.version !== transaction.version
			|| !hasSingleEmptyCaretAt(transaction.document, transaction.prePairPosition, transaction.version)) {
			diagnostic('formatting.commentPair', { outcome: 'caretMoved', version: transaction.version });
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

interface BlockCommentPairTransaction {
	document: vscode.TextDocument;
	version: number;
	prePairPosition: vscode.Position;
	position: vscode.Position;
	caretReady: boolean;
	response?: BlockCommentPairResponse;
}

async function requestBlockCommentPair(
	transaction: BlockCommentPairTransaction,
	getClient: () => LanguageClient | undefined,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	const activeClient = getClient();
	const editor = vscode.window.activeTextEditor;
	if (!activeClient || !editor || editor.document.uri.toString() !== transaction.document.uri.toString()
		|| transaction.document.version !== transaction.version) {
		diagnostic('formatting.commentPair', { outcome: 'rejectedEditorState', version: transaction.version });
		clear();
		return;
	}
	try {
		const response = await activeClient.sendRequest<BlockCommentPairResponse>(
			languageClientRequests.blockCommentPair,
			typingAssistRequest(transaction.document, transaction.position, editor),
		);
		if (!isCurrent() || transaction.document.version !== transaction.version || response.edits.length === 0) {
			diagnostic('formatting.commentPair', {
				outcome: response.edits.length === 0 ? 'noEdits' : 'staleResponse',
				version: transaction.version,
			});
			clear();
			return;
		}
		transaction.response = response;
		await applyPendingBlockCommentPair(transaction, isCurrent, clear);
	} catch {
		diagnostic('formatting.commentPair', { outcome: 'requestError', version: transaction.version });
		clear();
	}
}

async function applyPendingBlockCommentPair(
	transaction: BlockCommentPairTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	if (!transaction.response || !transaction.caretReady) {
		return;
	}
	if (!isCurrent() || transaction.document.version !== transaction.version
		|| !hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
		diagnostic('formatting.commentPair', { outcome: 'staleResponse', version: transaction.version });
		clear();
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		clear();
		return;
	}
	const applied = await applyVersionedEditorEdits(editor, transaction.response);
	diagnostic('formatting.commentPair', {
		outcome: applied ? 'applied' : 'editRejected',
		version: transaction.version,
		edits: transaction.response.edits.length,
	});
	clear();
}

export function registerEnterTypingAssist(getClient: () => LanguageClient | undefined): vscode.Disposable {
	let pending: EnterTypingAssistTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.enter', { outcome: 'superseded', version: pending.version });
			pending = undefined;
		}
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}
		const editor = vscode.window.activeTextEditor;
		const enterPosition = enterAfterPosition(event.contentChanges);
		const tabPosition = editor && editor.document.uri.toString() === event.document.uri.toString()
			? tabAfterPosition(event.contentChanges)
			: undefined;
		const position = enterPosition ?? tabPosition;
		if (!position) {
			return;
		}
		const change = event.contentChanges[0];
		const transaction: EnterTypingAssistTransaction = {
			document: event.document,
			version: event.document.version,
			preEnterPosition: change.range.start,
			position,
			trigger: enterPosition ? '\n' : '\t',
			caretReady: hasSingleEmptyCaretAt(event.document, position, event.document.version),
		};
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestEnterTypingAssist(transaction, getClient, () => pending === transaction, () => {
					pending = undefined;
				});
			}
		});
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		const transaction = pending;
		if (!transaction || event.textEditor.document.uri.toString() !== transaction.document.uri.toString()) {
			return;
		}
		if (transaction.document.version !== transaction.version) {
			diagnostic('formatting.enter', { outcome: 'superseded', version: transaction.version });
			pending = undefined;
			return;
		}
		if (hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
			transaction.caretReady = true;
			void applyPendingEnterTypingAssist(transaction, () => pending === transaction, () => {
				pending = undefined;
			});
			return;
		}
		if (!hasSingleEmptyCaretAt(transaction.document, transaction.preEnterPosition, transaction.version)) {
			diagnostic('formatting.enter', { outcome: 'caretMoved', version: transaction.version });
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

interface EnterTypingAssistTransaction {
	document: vscode.TextDocument;
	version: number;
	preEnterPosition: vscode.Position;
	position: vscode.Position;
	trigger: '\n' | '\t';
	caretReady: boolean;
	response?: EnterTypingAssistResponse;
}

async function requestEnterTypingAssist(
	transaction: EnterTypingAssistTransaction,
	getClient: () => LanguageClient | undefined,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	const activeClient = getClient();
	const editor = vscode.window.activeTextEditor;
	if (!activeClient || !editor || editor.document.uri.toString() !== transaction.document.uri.toString()
		|| transaction.document.version !== transaction.version) {
		diagnostic('formatting.enter', { outcome: 'rejectedEditorState', version: transaction.version });
		clear();
		return;
	}
	diagnostic('formatting.enter', {
		outcome: 'admitted',
		version: transaction.version,
		line: transaction.position.line,
		character: transaction.position.character,
	});
	try {
		const response = await activeClient.sendRequest<EnterTypingAssistResponse>(
			languageClientRequests.enterTypingAssist,
			typingAssistRequest(transaction.document, transaction.position, editor, transaction.trigger),
		);
		if (!isCurrent() || transaction.document.version !== transaction.version) {
			diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'documentChanged' });
			clear();
			return;
		}
		if (response.edits.length === 0) {
			diagnostic('formatting.enter', { outcome: 'noEdits', version: transaction.version });
			clear();
			return;
		}
		transaction.response = response;
		if (!transaction.caretReady) {
			diagnostic('formatting.enter', { outcome: 'awaitingCaret', version: transaction.version });
		}
		await applyPendingEnterTypingAssist(transaction, isCurrent, clear);
	} catch {
		// A typing assist must never surface transport failures while the user edits.
		diagnostic('formatting.enter', { outcome: 'requestError', version: transaction.version });
		clear();
	}
}

async function applyPendingEnterTypingAssist(
	transaction: EnterTypingAssistTransaction,
	isCurrent: () => boolean,
	clear: () => void,
): Promise<void> {
	if (!transaction.response || !transaction.caretReady) {
		return;
	}
	if (!isCurrent() || transaction.document.version !== transaction.version
		|| !hasSingleEmptyCaretAt(transaction.document, transaction.position, transaction.version)) {
		diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'caretMoved' });
		clear();
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor) {
		clear();
		return;
	}
	const applied = await applyVersionedEditorEdits(editor, transaction.response);
	diagnostic('formatting.enter', {
		outcome: applied ? 'applied' : 'editRejected',
		version: transaction.version,
		edits: transaction.response.edits.length,
	});
	clear();
}

export function isCurrentSingleTypingAssistCaret(
	documentVersion: number,
	expectedVersion: number,
	selectionCount: number,
	selectionIsEmpty: boolean,
	selectionActive: vscode.Position,
	expectedPosition: vscode.Position,
): boolean {
	return documentVersion === expectedVersion
		&& selectionCount === 1
		&& selectionIsEmpty
		&& selectionActive.isEqual(expectedPosition);
}

function hasSingleEmptyCaretAt(
	document: vscode.TextDocument,
	position: vscode.Position,
	expectedVersion: number,
): boolean {
	return isCurrentSingleCaret(document, expectedVersion, position);
}
