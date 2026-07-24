import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { experimentalAutoFormattingEnabled } from '../extensionConfig/experimentalAutoFormatting';
import { languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { type VersionedEditResponse } from './versionedEditorEdit';
import { VersionedEditorTransaction } from './versionedEditorTransaction';
import { blockCommentPairPosition, typingAssistRequest } from './typingAssistBridge';

interface BlockCommentPairResponse extends VersionedEditResponse {}

export function registerBlockCommentPair(getClient: () => LanguageClient | undefined): vscode.Disposable {
	let pending: BlockCommentPairTransaction | undefined;
	const documentChanges = vscode.workspace.onDidChangeTextDocument(event => {
		if (!experimentalAutoFormattingEnabled()) {
			return;
		}
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
	if (!experimentalAutoFormattingEnabled()) {
		clear();
		return;
	}
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
