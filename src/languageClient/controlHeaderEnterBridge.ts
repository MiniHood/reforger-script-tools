import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { experimentalAutoFormattingEnabled } from '../extensionConfig/experimentalAutoFormatting';
import { languageClientCommands, languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { inputRouteRequest } from './typingAssistBridge';
import { applyVersionedEditorEdits, isCurrentSingleCaret, type VersionedEditResponse } from './versionedEditorEdit';

const nativeTypeCommand = 'type';

interface InputRouteResult extends Omit<VersionedEditResponse, 'snippet'> {
	snippet?: never;
	owner?: 'controlHeader';
	reason?: string;
}

export function registerControlHeaderEnter(getClient: () => LanguageClient | undefined): vscode.Disposable {
	return vscode.commands.registerCommand(languageClientCommands.insertNewline, async () => {
		const editor = vscode.window.activeTextEditor;
		await executeInsertNewline(editor, getClient());
	});
}

export async function executeInsertNewline(
	editor: vscode.TextEditor | undefined,
	client: LanguageClient | undefined,
	nativeFallback: () => Promise<void> = typeNativeEnter,
): Promise<void> {
	const startedAt = Date.now();
	if (!editor || editor.document.languageId !== languageClientLanguage.id) {
		await nativeFallback();
		traceInputRoute('nativeFallback', 'ineligibleEditor', undefined, startedAt);
		return;
	}
	if (!experimentalAutoFormattingEnabled()) {
		await nativeFallback();
		traceInputRoute('nativeFallback', 'formattingDisabled', undefined, startedAt);
		return;
	}
	if (!mayBeControlHeader(editor)) {
		await nativeFallback();
		traceInputRoute('nativeFallback', 'notCandidate', undefined, startedAt);
		return;
	}
	if (!client) {
		await nativeFallback();
		traceInputRoute('nativeFallback', 'serverUnavailable', undefined, startedAt);
		return;
	}
	const position = editor.selection.active;
	const version = editor.document.version;
	try {
		const response = await client.sendRequest<InputRouteResult>(
			languageClientRequests.inputRoute,
			inputRouteRequest(editor.document, editor),
		);
		if (response.edits.length === 0) {
			await nativeFallback();
			traceInputRoute('nativeFallback', response.reason ?? 'declined', undefined, startedAt);
			return;
		}
		if (!isCurrentSingleCaret(editor.document, version, position)) {
			await nativeFallback();
			traceInputRoute('nativeFallback', 'staleDecision', response.owner, startedAt, false);
			return;
		}
		const applied = await applyVersionedEditorEdits(editor, response);
		if (!applied) {
			await nativeFallback();
			traceInputRoute('nativeFallback', 'editRejected', response.owner, startedAt);
			return;
		}
		if (response.triggerSuggest) {
			await vscode.commands.executeCommand('editor.action.triggerSuggest');
		}
		traceInputRoute('applied', undefined, response.owner, startedAt);
	} catch {
		await nativeFallback();
		traceInputRoute('nativeFallback', 'requestFailed', undefined, startedAt);
	}
}

function mayBeControlHeader(editor: vscode.TextEditor): boolean {
	const position = editor.selection.active;
	const line = editor.document.lineAt(position.line).text;
	const before = line.slice(0, position.character);
	const after = line.slice(position.character);
	return /\b(?:for|foreach|while|switch)\s*\([^{}]*$/.test(before)
		&& (after.includes(')') || /\)\s*$/.test(before));
}

async function typeNativeEnter(): Promise<void> {
	await vscode.commands.executeCommand(nativeTypeCommand, { text: '\n' });
}

function traceInputRoute(
	outcome: 'applied' | 'nativeFallback',
	reason: string | undefined,
	owner: InputRouteResult['owner'],
	startedAt: number,
	versionMatch = true,
): void {
	diagnostic('inputRoute', {
		operation: 'insertNewline',
		outcome,
		reason,
		owner,
		versionMatch,
		elapsedMs: Date.now() - startedAt,
	});
}
