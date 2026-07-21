import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { diagnostic } from '../diagnostics/diagnostics';
import { languageClientDocumentSelector } from '../extensionConfig/languageClient';
import { rangeFromLsp, type LspRange } from './versionedEditorEdit';

interface LspHoverResponse {
	contents: LspMarkupContent | string | Array<LspMarkupContent | string>;
	range?: LspRange;
}

interface LspMarkupContent {
	kind?: string;
	value?: string;
}

/** Registers the VS Code rendering bridge for Rust-authored hover results. */
export function registerHtmlHoverBridge(
	client: LanguageClient,
	outputChannel: vscode.LogOutputChannel,
): vscode.Disposable {
	return vscode.languages.registerHoverProvider(languageClientDocumentSelector, {
		provideHover: async (document, position, token) => {
			const startedAt = Date.now();
			try {
				const hover = await client.sendRequest<LspHoverResponse | null>(
					'textDocument/hover',
					{
						textDocument: { uri: document.uri.toString() },
						position: { line: position.line, character: position.character },
					},
					token,
				);
				diagnostic('lsp.hover', { outcome: hover ? 'hit' : 'empty', elapsedMs: Date.now() - startedAt });
				return hover ? hoverFromLspResponse(hover) : null;
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				outputChannel.debug(`HTML hover request failed for ${document.uri.toString()}: ${message}`);
				diagnostic('lsp.hover', { outcome: 'error', elapsedMs: Date.now() - startedAt });
				return null;
			}
		},
	});
}

function hoverFromLspResponse(hover: LspHoverResponse): vscode.Hover | null {
	const contents = Array.isArray(hover.contents) ? hover.contents : [hover.contents];
	const markdown = contents.map(content => htmlMarkdownContent(content));
	if (markdown.length === 0) {
		return null;
	}
	return new vscode.Hover(markdown, hover.range ? rangeFromLsp(hover.range) : undefined);
}

function htmlMarkdownContent(content: LspMarkupContent | string): vscode.MarkdownString {
	const markdown = new vscode.MarkdownString();
	markdown.isTrusted = true;
	markdown.supportHtml = true;
	if (typeof content === 'string') {
		markdown.appendMarkdown(content);
	} else if (content.kind === 'plaintext') {
		markdown.appendText(content.value ?? '');
	} else {
		markdown.appendMarkdown(content.value ?? '');
	}
	return markdown;
}
