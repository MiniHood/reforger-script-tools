import { normalizeSourceText } from './source';
import type { EnforceToken } from './tokens';
import { previousSignificantToken, tokenIndexAfter, tokensText } from './tokenCursor';

export interface ParsedValueDeclaration {
	name: string;
	valueType: string;
}

const nonTypeModifiers = new Set([
	'private',
	'protected',
	'public',
	'static',
	'const',
	'event',
	'override',
	'proto',
	'external',
	'native',
	'owned',
	'volatile',
	'ref',
	'autoptr',
	'out',
	'inout',
	'notnull',
]);

export function parseValueDeclarationText(value: string): ParsedValueDeclaration | undefined {
	const withoutDefault = stripTopLevelInitializer(value).trim();
	const match = /^(?<type>.+?)\s+(?<name>[A-Za-z_]\w*)\s*(?<staticArray>\[[^\]]*\])?$/.exec(withoutDefault);
	if (!match?.groups?.type || !match.groups.name) {
		return undefined;
	}
	const valueType = normalizeValueType(match.groups.type, match.groups.staticArray);
	return valueType ? { name: match.groups.name, valueType } : undefined;
}

export function getDeclarationValueTypeFromTokens(tokens: EnforceToken[], startIndex: number, nameToken: EnforceToken): string | undefined {
	const value = tokensText(tokens, tokens[startIndex], previousSignificantToken(tokens, tokenIndexAfter(tokens, nameToken) - 2) ?? tokens[startIndex]);
	return normalizeValueType(value, staticArraySuffixAfterName(tokens, nameToken));
}

export function normalizeValueType(value: string, staticArraySuffix = ''): string | undefined {
	let normalized = value.trim();
	let changed = true;
	while (changed) {
		changed = false;
		for (const modifier of nonTypeModifiers) {
			const next = normalized.replace(new RegExp(`^\\s*${escapeRegExp(modifier)}\\b\\s*`), '');
			if (next !== normalized) {
				normalized = next;
				changed = true;
			}
		}
	}
	normalized = normalizeSourceText(normalized);
	if (!hasBalancedTypeDelimiters(normalized) || /[<,]$/.test(normalized)) {
		return undefined;
	}
	if (normalized && staticArraySuffix) {
		normalized += staticArraySuffix.replace(/\s+/g, '');
	}
	return normalized || undefined;
}

function hasBalancedTypeDelimiters(value: string): boolean {
	let angleDepth = 0;
	for (const char of value) {
		if (char === '<') {
			angleDepth++;
		} else if (char === '>') {
			angleDepth--;
			if (angleDepth < 0) {
				return false;
			}
		}
	}
	return angleDepth === 0;
}

export function stripTopLevelInitializer(value: string): string {
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
		} else if (char === '=' && depth === 0) {
			return value.slice(0, index);
		}
	}
	return value;
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function staticArraySuffixAfterName(tokens: EnforceToken[], nameToken: EnforceToken): string {
	const nameIndex = tokenIndexAfter(tokens, nameToken) - 1;
	const openBracket = tokens[nameIndex + 1];
	if (openBracket?.text !== '[') {
		return '';
	}
	let suffix = '';
	for (let index = nameIndex + 1; index < tokens.length; index++) {
		const token = tokens[index];
		suffix += token.text;
		if (token.text === ']') {
			break;
		}
		if (token.kind === 'newline' || token.kind === 'eof' || token.text === ';' || token.text === '=') {
			return '';
		}
	}
	return suffix.endsWith(']') ? suffix : '';
}
