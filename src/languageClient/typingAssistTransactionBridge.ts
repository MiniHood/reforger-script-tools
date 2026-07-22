import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { type VersionedEditResponse } from './versionedEditorEdit';
import { isCurrentSingleVersionedEditorCaret, VersionedEditorTransaction } from './versionedEditorTransaction';
import { blockCommentPairPosition, enterAfterPosition, tabAfterPosition, typingAssistRequest } from './typingAssistBridge';

interface BlockCommentPairResponse extends VersionedEditResponse {}

interface EnterTypingAssistResponse extends VersionedEditResponse {}

export function registerBlockCommentPair(getClient: () => LanguageClient | undefined): vscode.Disposable {
	let pending: BlockCommentPairTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.commentPair', { outcome: 'superseded', version: pending.version });
			pending.reject();
			pending = undefined;
		}
		if (event.document.languageId !== languageClientLanguage.id) {
			return;
		}
		const position = blockCommentPairPosition(event.contentChanges);
		if (!position) {
			return;
		}
		const transaction = new VersionedEditorTransaction<BlockCommentPairResponse>(
			event.document, event.document.version, position, event.contentChanges[0].range.start,
		);
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestBlockCommentPair(transaction, getClient, () => pending === transaction, () => {
					transaction.reject();
					if (pending === transaction) {
						pending = undefined;
					}
				});
			}
		});
	});
	const selectionChanges = vscode.window.onDidChangeTextEditorSelection(event => {
		const transaction = pending;
		if (!transaction || event.textEditor.document.uri.toString() !== transaction.document.uri.toString()) {
			return;
		}
		if (transaction.observeSelection() === 'ready') {
			void applyPendingBlockCommentPair(transaction, () => pending === transaction, () => {
				transaction.reject();
				if (pending === transaction) {
					pending = undefined;
				}
			});
			return;
		}
		if (transaction.observeSelection() === 'moved') {
			diagnostic('formatting.commentPair', { outcome: 'caretMoved', version: transaction.version });
			transaction.reject();
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

type BlockCommentPairTransaction = VersionedEditorTransaction<BlockCommentPairResponse>;

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
			typingAssistRequest(transaction.document, transaction.expectedPosition, editor),
		);
		if (!isCurrent() || !transaction.accept(response)) {
			diagnostic('formatting.commentPair', {
				outcome: response.edits.length === 0 ? 'noEdits' : 'staleResponse',
				version: transaction.version,
			});
			clear();
			return;
		}
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
	if (!isCurrent()) {
		diagnostic('formatting.commentPair', { outcome: 'staleResponse', version: transaction.version });
		clear();
		return;
	}
	const outcome = await transaction.apply();
	if (outcome === 'pending') {
		return;
	}
	diagnostic('formatting.commentPair', {
		outcome: outcome === 'stale' ? 'staleResponse' : outcome,
		version: transaction.version,
		edits: transaction.response?.edits.length,
	});
	clear();
}

export function registerEnterTypingAssist(getClient: () => LanguageClient | undefined): vscode.Disposable {
	let pending: EnterTypingAssistTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (pending && pending.document.uri.toString() === event.document.uri.toString()
			&& event.document.version > pending.version) {
			diagnostic('formatting.enter', { outcome: 'superseded', version: pending.version });
			pending.reject();
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
		const transaction = Object.assign(
			new VersionedEditorTransaction<EnterTypingAssistResponse>(
				event.document, event.document.version, position, change.range.start,
			),
			{ trigger: enterPosition ? '\n' as const : '\t' as const },
		);
		pending = transaction;
		queueMicrotask(() => {
			if (pending === transaction) {
				void requestEnterTypingAssist(transaction, getClient, () => pending === transaction, () => {
					transaction.reject();
					if (pending === transaction) {
						pending = undefined;
					}
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
			transaction.reject();
			pending = undefined;
			return;
		}
		if (transaction.observeSelection() === 'ready') {
			void applyPendingEnterTypingAssist(transaction, () => pending === transaction, () => {
				transaction.reject();
				if (pending === transaction) {
					pending = undefined;
				}
			});
			return;
		}
		if (transaction.observeSelection() === 'moved') {
			diagnostic('formatting.enter', { outcome: 'caretMoved', version: transaction.version });
			transaction.reject();
			pending = undefined;
		}
	});
	return vscode.Disposable.from(documentChanges, selectionChanges);
}

type EnterTypingAssistTransaction = VersionedEditorTransaction<EnterTypingAssistResponse> & {
	trigger: '\n' | '\t';
};

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
		line: transaction.expectedPosition.line,
		character: transaction.expectedPosition.character,
	});
	try {
		const response = await activeClient.sendRequest<EnterTypingAssistResponse>(
			languageClientRequests.enterTypingAssist,
			typingAssistRequest(transaction.document, transaction.expectedPosition, editor, transaction.trigger),
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
		if (!transaction.accept(response)) {
			diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'caretMoved' });
			clear();
			return;
		}
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
	if (!isCurrent()) {
		diagnostic('formatting.enter', { outcome: 'staleResponse', version: transaction.version, reason: 'caretMoved' });
		clear();
		return;
	}
	const outcome = await transaction.apply();
	if (outcome === 'pending') {
		return;
	}
	diagnostic('formatting.enter', {
		outcome: outcome === 'stale' ? 'staleResponse' : outcome,
		version: transaction.version,
		edits: transaction.response?.edits.length,
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
	return isCurrentSingleVersionedEditorCaret(
		documentVersion, expectedVersion, selectionCount, selectionIsEmpty, selectionActive, expectedPosition,
	);
}
