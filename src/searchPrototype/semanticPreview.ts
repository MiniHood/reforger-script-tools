import * as vscode from 'vscode';
import {
	hoverSemanticPaletteForDocument,
	semanticTokenTypes,
	type HoverSemanticForegrounds,
} from '../languageClient/hoverSemanticPalette';
import { stripSourceComments } from './mcpSearchClient';

export interface SemanticPreviewToken {
	start: number;
	length: number;
	role: string;
}

export interface SemanticPreview {
	text: string;
	tokens: SemanticPreviewToken[];
	foregrounds: HoverSemanticForegrounds;
	enabled: boolean;
}

/** Decodes the LSP delta-encoded semantic token stream for one document line. */
export function semanticTokenSpansForLine(
	data: Iterable<number>,
	targetLine: number,
): SemanticPreviewToken[] {
	if (!Number.isInteger(targetLine) || targetLine < 0) {
		return [];
	}
	const values = Array.from(data);
	const tokens: SemanticPreviewToken[] = [];
	let line = 0;
	let character = 0;
	for (let index = 0; index + 4 < values.length; index += 5) {
		const deltaLine = values[index] ?? 0;
		line += deltaLine;
		character = deltaLine === 0 ? character + (values[index + 1] ?? 0) : (values[index + 1] ?? 0);
		const length = values[index + 2] ?? 0;
		const role = semanticTokenTypes[values[index + 3] ?? -1];
		if (line === targetLine && length > 0 && role) {
			tokens.push({ start: character, length, role });
		}
	}
	return tokens;
}

/** Builds a trimmed, palette-aware preview from the same semantic tokens used by the editor. */
export function semanticPreviewForLine(
	document: vscode.TextDocument,
	semanticTokens: vscode.SemanticTokens,
	targetLine: number,
): SemanticPreview | undefined {
	if (targetLine < 0 || targetLine >= document.lineCount) {
		return undefined;
	}
	const sourceText = stripSourceComments(document.lineAt(targetLine).text);
	const leadingWhitespace = sourceText.length - sourceText.trimStart().length;
	const text = sourceText.slice(leadingWhitespace).trimEnd();
	const tokens = semanticTokenSpansForLine(semanticTokens.data, targetLine)
		.map(token => ({
			...token,
			start: token.start - leadingWhitespace,
		}))
		.map(token => ({
			...token,
			length: Math.min(token.length, text.length - token.start),
		}))
		.filter(token => token.start >= 0 && token.length > 0 && token.start < text.length);
	if (tokens.length === 0) {
		return undefined;
	}
	const palette = hoverSemanticPaletteForDocument(document);
	return {
		text,
		tokens,
		foregrounds: palette.foregrounds,
		enabled: palette.enabled,
	};
}
