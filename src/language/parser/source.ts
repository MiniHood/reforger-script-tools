import type { EnforceParserRange } from './ast';
import type { EnforceToken } from './tokens';

export interface SourceTextInfo {
	text: string;
	lines: string[];
	lineStartOffsets: number[];
}

export function createSourceTextInfo(text: string): SourceTextInfo {
	const lines = text.split(/\r?\n/);
	const lineStartOffsets: number[] = [];
	let offset = 0;
	for (const line of lines) {
		lineStartOffsets.push(offset);
		offset += line.length + 1;
	}
	return { text, lines, lineStartOffsets };
}

export function offsetFromPosition(source: SourceTextInfo, line: number, character: number): number {
	return (source.lineStartOffsets[line] ?? source.text.length) + character;
}

export function normalizeSourceText(value: string): string {
	return value
		.split(/\r?\n/)
		.map(line => line.trim())
		.filter(Boolean)
		.join(' ')
		.replace(/\s+/g, ' ')
		.replace(/\s+([;,)])\s*$/g, '$1')
		.trim();
}

export function splitTopLevel(value: string): string[] {
	const parts: string[] = [];
	let start = 0;
	let depth = 0;
	let quote: '"' | "'" | undefined;
	let escaped = false;
	for (let index = 0; index < value.length; index++) {
		const char = value[index];
		if (quote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === quote) {
				quote = undefined;
			}
			continue;
		}

		if (char === '"' || char === "'") {
			quote = char;
		} else if (char === '(' || char === '{' || char === '[' || char === '<') {
			depth++;
		} else if (char === ')' || char === '}' || char === ']' || char === '>') {
			depth = Math.max(0, depth - 1);
		} else if (char === ',' && depth === 0) {
			parts.push(value.slice(start, index));
			start = index + 1;
		}
	}
	parts.push(value.slice(start));
	return parts;
}

export function findTopLevelCharacter(value: string, target: string): number {
	let depth = 0;
	let quote: '"' | "'" | undefined;
	let escaped = false;
	for (let index = 0; index < value.length; index++) {
		const char = value[index];
		if (quote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === quote) {
				quote = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			quote = char;
		} else if (char === '(' || char === '[' || char === '<' || char === '{') {
			depth++;
		} else if (char === ')' || char === ']' || char === '>' || char === '}') {
			depth = Math.max(0, depth - 1);
		} else if (char === target && depth === 0) {
			return index;
		}
	}
	return -1;
}

export function tokenRangeToParserRange(token: EnforceToken): EnforceParserRange {
	return {
		start: { line: token.line, character: token.character },
		end: { line: token.endLine, character: token.endCharacter },
	};
}

export function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
