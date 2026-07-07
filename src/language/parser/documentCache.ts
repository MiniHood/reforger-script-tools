import * as vscode from 'vscode';
import { tracePerformance } from '../../core/performanceTrace';
import { parseEnforceSource } from './declarationParser';
import type { EnforceParserPosition, EnforceParserRange, ParsedEnforceSource } from './ast';

interface CachedParsedDocument {
	version: number;
	text: string;
	parsed: ParsedEnforceSource;
}

const parsedDocuments = new Map<string, CachedParsedDocument>();

export function getParsedDocument(document: vscode.TextDocument): ParsedEnforceSource {
	const key = document.uri.toString();
	const text = document.getText();
	const existing = parsedDocuments.get(key);
	if (existing && existing.version === document.version && existing.text === text) {
		return existing.parsed;
	}

	const parsed = tracePerformance(
		'parser.getParsedDocument.miss',
		`${document.uri.fsPath.split(/[\\/]/).pop() ?? document.uri.toString()} | chars=${text.length} | version=${document.version}`,
		() => parseEnforceSource(text, document.uri)
	);
	parsedDocuments.set(key, { version: document.version, text, parsed });
	return parsed;
}

export function toParserPosition(position: vscode.Position): EnforceParserPosition {
	return { line: position.line, character: position.character };
}

export function toVscodeRange(range: EnforceParserRange): vscode.Range {
	return new vscode.Range(
		range.start.line,
		range.start.character,
		range.end.line,
		range.end.character
	);
}
