export type EnforceTokenKind =
	| 'identifier'
	| 'keyword'
	| 'number'
	| 'string'
	| 'comment'
	| 'preprocessor'
	| 'operator'
	| 'punctuation'
	| 'whitespace'
	| 'newline'
	| 'unknown'
	| 'eof';

export interface EnforceToken {
	kind: EnforceTokenKind;
	text: string;
	start: number;
	end: number;
	line: number;
	character: number;
	endLine: number;
	endCharacter: number;
	unterminated?: boolean;
}

export const enforceKeywords = new Set([
	'autoptr',
	'break',
	'case',
	'class',
	'const',
	'continue',
	'default',
	'delete',
	'else',
	'enum',
	'event',
	'extends',
	'external',
	'false',
	'for',
	'foreach',
	'if',
	'inout',
	'modded',
	'native',
	'new',
	'notnull',
	'null',
	'out',
	'owned',
	'override',
	'private',
	'protected',
	'proto',
	'public',
	'ref',
	'return',
	'sealed',
	'static',
	'super',
	'switch',
	'this',
	'true',
	'typedef',
	'volatile',
	'void',
	'while',
]);
