import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { experimentalAutoFormattingEnabled } from '../extensionConfig/experimentalAutoFormatting';
import { languageClientCommands, languageClientLanguage, languageClientRequests } from '../extensionConfig/languageClient';
import { diagnostic, inputRouteTraceEnabled } from '../diagnostics/diagnostics';
import { inputRouteRequest } from './typingAssistBridge';
import { applyVersionedEditorEdits, isCurrentSingleCaret, type VersionedEditResponse } from './versionedEditorEdit';

const nativeTypeCommand = 'type';

interface InputRouteResult extends VersionedEditResponse {
	owner?: 'controlHeader' | 'ifHeader' | 'semicolon' | 'unbracedIfBody';
	reason?: string;
}

export function registerControlHeaderEnter(getClient: () => LanguageClient | undefined): vscode.Disposable[] {
	return [vscode.commands.registerCommand(languageClientCommands.insertNewline, async (args?: { dismissSuggest?: boolean }) => {
		const editor = vscode.window.activeTextEditor;
		await executeInsertNewline(
			editor,
			getClient(),
			undefined,
			undefined,
			args?.dismissSuggest ? () => vscode.commands.executeCommand('hideSuggestWidget') : undefined,
		);
	}), vscode.commands.registerCommand(languageClientCommands.indent, async () => {
		await executeIndent(vscode.window.activeTextEditor, getClient());
	})];
}

export async function executeInsertNewline(
	editor: vscode.TextEditor | undefined,
	client: LanguageClient | undefined,
	nativeFallback: () => Promise<void> = typeNativeEnter,
	triggerSuggest: () => Thenable<unknown> = () => vscode.commands.executeCommand('editor.action.triggerSuggest'),
	dismissSuggest?: () => Thenable<unknown>,
): Promise<void> {
	await executeInputRoute(editor, client, 'insertNewline', nativeFallback, triggerSuggest, dismissSuggest);
}

export async function executeIndent(
	editor: vscode.TextEditor | undefined,
	client: LanguageClient | undefined,
): Promise<void> {
	await executeInputRoute(editor, client, 'indent', typeNativeIndent);
}

async function executeInputRoute(
	editor: vscode.TextEditor | undefined,
	client: LanguageClient | undefined,
	operation: 'insertNewline' | 'indent',
	nativeFallback: () => Promise<void>,
	triggerSuggest: () => Thenable<unknown> = () => vscode.commands.executeCommand('editor.action.triggerSuggest'),
	dismissSuggest?: () => Thenable<unknown>,
): Promise<void> {
	const startedAt = Date.now();
	if (dismissSuggest) {
		await dismissSuggest();
	}
	if (!editor || editor.document.languageId !== languageClientLanguage.id) {
		await nativeFallback();
		traceInputRoute(operation, 'nativeFallback', 'ineligibleEditor', undefined, startedAt);
		return;
	}
	if (!experimentalAutoFormattingEnabled()) {
		await nativeFallback();
		traceInputRoute(operation, 'nativeFallback', 'formattingDisabled', undefined, startedAt);
		return;
	}
	if (!client) {
		await nativeFallback();
		traceInputRoute(operation, 'nativeFallback', 'serverUnavailable', undefined, startedAt);
		return;
	}
	const position = editor.selection.active;
	const version = editor.document.version;
	try {
		const response = await client.sendRequest<InputRouteResult>(
			languageClientRequests.inputRoute,
			inputRouteRequest(editor.document, editor, inputRouteTraceEnabled(), operation),
		);
		if (response.edits.length === 0) {
			await nativeFallback();
			traceInputRoute(operation, 'nativeFallback', response.reason ?? 'declined', undefined, startedAt);
			return;
		}
		if (!isCurrentSingleCaret(editor.document, version, position)) {
			await nativeFallback();
			traceInputRoute(operation, 'nativeFallback', 'staleDecision', response.owner, startedAt, false);
			return;
		}
		const applied = await applyVersionedEditorEdits(editor, response);
		if (!applied) {
			await nativeFallback();
			traceInputRoute(operation, 'nativeFallback', 'editRejected', response.owner, startedAt);
			return;
		}
		if (response.triggerSuggest) {
			await triggerSuggest();
		}
		traceInputRoute(operation, 'applied', undefined, response.owner, startedAt);
	} catch {
		await nativeFallback();
		traceInputRoute(operation, 'nativeFallback', 'requestFailed', undefined, startedAt);
	}
}

async function typeNativeEnter(): Promise<void> {
	await vscode.commands.executeCommand(nativeTypeCommand, { text: '\n' });
}

async function typeNativeIndent(): Promise<void> {
	await vscode.commands.executeCommand('editor.action.indentLines');
}

function traceInputRoute(
	operation: 'insertNewline' | 'indent',
	outcome: 'applied' | 'nativeFallback',
	reason: string | undefined,
	owner: InputRouteResult['owner'],
	startedAt: number,
	versionMatch = true,
): void {
	if (!inputRouteTraceEnabled()) {
		return;
	}
	diagnostic('inputRoute', {
		operation,
		outcome,
		reason,
		owner,
		versionMatch,
		elapsedMs: Date.now() - startedAt,
	});
}
