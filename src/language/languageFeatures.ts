import * as vscode from 'vscode';
import { ExtensionLogger } from '../core/logger';
import { BasicCompletionProvider } from './completion/provider';
import { registerCompletionDebugTools } from './debug/completionDebugTools';
import { EnforceSymbolIndex } from './index/symbolIndex';
import { handleTypedFormatting } from './formatting/typedFormatting';
import { registerBracketDecorations } from './providers/bracketDecorations';
import {
	getModelDiagnostics,
	ModelDefinitionProvider,
	ModelDocumentHighlightProvider,
	ModelDocumentSymbolProvider,
	ModelHoverProvider,
	ModelReferenceProvider,
	ModelRenameProvider,
	ModelSemanticTokensProvider,
	ModelWorkspaceSymbolProvider,
	modelSemanticTokenLegend,
} from './providers/modelProviders';

const selector: vscode.DocumentSelector = { language: 'enforce', scheme: 'file' };
const completionTriggers = [
	...('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_'.split('')),
	'.',
	':',
	'[',
	'#',
	'=',
	'!',
	'<',
	'>',
	'&',
	'|',
];

export function registerLanguageFeatures(
	context: vscode.ExtensionContext,
	logger: ExtensionLogger,
	output?: vscode.OutputChannel
): { diagnostics: vscode.DiagnosticCollection; symbolIndex: EnforceSymbolIndex } {
	const diagnostics = vscode.languages.createDiagnosticCollection('reforgerScriptSyntax');
	const symbolIndex = new EnforceSymbolIndex(context, logger, output);
	symbolIndex.register(context);
	registerCompletionDebugTools(context, logger, symbolIndex);
	registerBracketDecorations(context);
	context.subscriptions.push(diagnostics);

	context.subscriptions.push(
		vscode.languages.registerCompletionItemProvider(selector, new BasicCompletionProvider(symbolIndex), ...completionTriggers),
		vscode.languages.registerHoverProvider(selector, new ModelHoverProvider(symbolIndex)),
		vscode.languages.registerDefinitionProvider(selector, new ModelDefinitionProvider(symbolIndex)),
		vscode.languages.registerReferenceProvider(selector, new ModelReferenceProvider(symbolIndex)),
		vscode.languages.registerRenameProvider(selector, new ModelRenameProvider(symbolIndex)),
		vscode.languages.registerDocumentHighlightProvider(selector, new ModelDocumentHighlightProvider(symbolIndex)),
		vscode.languages.registerDocumentSemanticTokensProvider(selector, new ModelSemanticTokensProvider(symbolIndex), modelSemanticTokenLegend),
		vscode.languages.registerDocumentSymbolProvider(selector, new ModelDocumentSymbolProvider(symbolIndex)),
		vscode.languages.registerWorkspaceSymbolProvider(new ModelWorkspaceSymbolProvider(symbolIndex)),
		vscode.workspace.onDidOpenTextDocument(document => updateDiagnostics(document, diagnostics, symbolIndex)),
		vscode.workspace.onDidSaveTextDocument(document => updateDiagnostics(document, diagnostics, symbolIndex)),
		vscode.workspace.onDidCloseTextDocument(document => diagnostics.delete(document.uri)),
		vscode.workspace.onDidChangeTextDocument(event => handleTypedFormatting(event, symbolIndex)),
		vscode.workspace.onDidChangeTextDocument(event => triggerSuggestAfterUsefulDeletion(event))
	);

	for (const document of vscode.workspace.textDocuments) {
		updateDiagnostics(document, diagnostics, symbolIndex);
	}

	logger.info('Parser-first language model features registered.');
	return { diagnostics, symbolIndex };
}

function triggerSuggestAfterUsefulDeletion(event: vscode.TextDocumentChangeEvent): void {
	const editor = vscode.window.activeTextEditor;
	if (
		event.document.languageId !== 'enforce'
		|| event.document.uri.scheme !== 'file'
		|| editor?.document.uri.toString() !== event.document.uri.toString()
		|| event.contentChanges.length !== 1
	) {
		return;
	}

	const change = event.contentChanges[0];
	if (change.text.length > 0 || change.rangeLength <= 0 || !editor.selection.isEmpty) {
		return;
	}

	const linePrefix = event.document.lineAt(editor.selection.active.line).text.slice(0, editor.selection.active.character);
	if (!isUsefulCompletionPrefix(linePrefix)) {
		return;
	}

	void vscode.commands.executeCommand('editor.action.triggerSuggest');
}

function isUsefulCompletionPrefix(linePrefix: string): boolean {
	if (isLineComment(linePrefix) || isOpenString(linePrefix)) {
		return false;
	}
	return /(?:[A-Za-z_][A-Za-z0-9_]{1,}|#\s*[A-Za-z_]*|[=!<>|&])$/.test(linePrefix);
}

function isLineComment(linePrefix: string): boolean {
	let inString: string | undefined;
	for (let index = 0; index < linePrefix.length - 1; index++) {
		const char = linePrefix[index];
		if (inString) {
			if (char === '\\') {
				index++;
			} else if (char === inString) {
				inString = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			inString = char;
			continue;
		}
		if (char === '/' && linePrefix[index + 1] === '/') {
			return true;
		}
	}
	return false;
}

function isOpenString(linePrefix: string): boolean {
	let inString: string | undefined;
	for (let index = 0; index < linePrefix.length; index++) {
		const char = linePrefix[index];
		if (inString) {
			if (char === '\\') {
				index++;
			} else if (char === inString) {
				inString = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			inString = char;
		}
	}
	return inString !== undefined;
}

function updateDiagnostics(
	document: vscode.TextDocument,
	diagnostics: vscode.DiagnosticCollection,
	symbolIndex: EnforceSymbolIndex
): void {
	if (document.languageId !== 'enforce' || document.uri.scheme !== 'file') {
		return;
	}
	diagnostics.set(document.uri, getModelDiagnostics(document, symbolIndex));
}
