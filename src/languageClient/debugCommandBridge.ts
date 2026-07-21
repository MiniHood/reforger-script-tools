import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { diagnostic } from '../diagnostics/diagnostics';
import { languageClientCommands, languageClientLanguage, languageClientLogs, languageClientRequests } from '../extensionConfig/languageClient';

export function registerDebugCommandBridge(
	context: vscode.ExtensionContext,
	client: () => LanguageClient | undefined,
	debugOutput: vscode.OutputChannel,
	completionOutput: vscode.OutputChannel,
	completionLifecycleTrace: (documentUri: string) => string,
): vscode.Disposable[] {
	return [
		vscode.commands.registerCommand(languageClientCommands.debugHoverAtCursor, () => debugHoverAtCursor(context, client, debugOutput)),
		vscode.commands.registerCommand(languageClientCommands.debugCompletionAtCursor, () => debugCompletionAtCursor(context, client, completionOutput, completionLifecycleTrace)),
	];
}

async function debugHoverAtCursor(context: vscode.ExtensionContext, client: () => LanguageClient | undefined, output: vscode.OutputChannel): Promise<void> {
	const startedAt = Date.now();
	diagnostic('command.debugHover.start');
	const editor = activeEnforceEditor('Open an Enforce script file before running hover debug.', 'Hover debug is only available for Enforce language files.');
	if (!editor) {
		return;
	}
	const activeClient = client();
	if (!activeClient) {
		vscode.window.showWarningMessage('Reforger language server is not running.');
		return;
	}
	try {
		const position = editor.selection.active;
		const report = await activeClient.sendRequest<string>(languageClientRequests.debugHover, requestParams(editor, position));
		const reportPath = await writeReport(context, editor, position, report, 'hover');
		output.clear(); output.appendLine(`Hover debug report written to: ${reportPath}`); output.appendLine(''); output.appendLine(report); output.show(true);
		diagnostic('command.debugHover.complete', { elapsedMs: Date.now() - startedAt });
	} catch (error) {
		showDebugError(output, 'Hover debug', error);
		diagnostic('command.debugHover.error', { elapsedMs: Date.now() - startedAt });
	}
}

async function debugCompletionAtCursor(context: vscode.ExtensionContext, client: () => LanguageClient | undefined, output: vscode.OutputChannel, lifecycleTrace: (documentUri: string) => string): Promise<void> {
	const startedAt = Date.now();
	diagnostic('command.debugCompletion.start');
	const editor = activeEnforceEditor('Open an Enforce script file before running completion debug.', 'Completion debug is only available for Enforce language files.');
	if (!editor) {
		return;
	}
	const activeClient = client();
	if (!activeClient) {
		vscode.window.showWarningMessage('Reforger language server is not running.');
		return;
	}
	try {
		const position = editor.selection.active;
		const report = await activeClient.sendRequest<string>(languageClientRequests.debugCompletion, requestParams(editor, position));
		const reportPath = await writeReport(context, editor, position, `${lifecycleTrace(editor.document.uri.toString())}\n\n---\n\n${report}`, 'completion');
		output.clear(); output.appendLine(`Completion debug report written to: ${reportPath}`); output.appendLine(''); output.appendLine(report); output.show(true);
		diagnostic('command.debugCompletion.complete', { elapsedMs: Date.now() - startedAt });
	} catch (error) {
		showDebugError(output, 'Completion debug', error);
		diagnostic('command.debugCompletion.error', { elapsedMs: Date.now() - startedAt });
	}
}

function activeEnforceEditor(noEditor: string, wrongLanguage: string): vscode.TextEditor | undefined {
	const editor = vscode.window.activeTextEditor;
	if (!editor) { vscode.window.showWarningMessage(noEditor); return undefined; }
	if (editor.document.languageId !== languageClientLanguage.id) { vscode.window.showWarningMessage(wrongLanguage); return undefined; }
	return editor;
}

function requestParams(editor: vscode.TextEditor, position: vscode.Position): object {
	return { textDocument: { uri: editor.document.uri.toString() }, position: { line: position.line, character: position.character } };
}

function showDebugError(output: vscode.OutputChannel, name: string, error: unknown): void {
	const message = error instanceof Error ? error.message : String(error);
	output.appendLine(`${name} debug request failed: ${message}`); output.show(true);
	vscode.window.showWarningMessage(`${name} debug request failed: ${message}`);
}

async function writeReport(context: vscode.ExtensionContext, editor: vscode.TextEditor, position: vscode.Position, report: string, kind: 'hover' | 'completion'): Promise<string> {
	const folder = kind === 'hover' ? languageClientLogs.hoverDebugFolder : languageClientLogs.completionDebugFolder;
	const file = kind === 'hover' ? languageClientLogs.hoverDebugLatestFile : languageClientLogs.completionDebugLatestFile;
	const command = kind === 'hover' ? languageClientCommands.debugHoverAtCursor : languageClientCommands.debugCompletionAtCursor;
	const folderPath = path.join(context.globalStorageUri.fsPath, languageClientLogs.rootFolder, folder);
	await fs.mkdir(folderPath, { recursive: true });
	const prefix = [
		`# Reforger ${kind === 'hover' ? 'Hover' : 'Completion'} Debug Log`,
		'',
		`- Generated: ${new Date().toISOString()}`,
		`- Document URI: ${editor.document.uri.toString()}`,
		`- Document path: ${editor.document.uri.fsPath}`,
		`- Language ID: ${editor.document.languageId}`,
		`- Cursor: line ${position.line} character ${position.character} (UTF-16, zero-based)`,
		`- Source: VS Code command ${command}`,
		'',
		`This file is overwritten by each ${kind}-debug command run and is intentionally separate from the normal language-server runtime log.`,
		'',
		'---',
		'',
	].join('\n');
	const reportPath = path.join(folderPath, file);
	await fs.writeFile(reportPath, `${prefix}${report}\n`, 'utf8');
	return reportPath;
}
