import { EnforceToken, enforceKeywords, EnforceTokenKind } from './tokens';

const punctuationCharacters = new Set(['(', ')', '{', '}', '[', ']', ',', ';', ':', '.']);
const singleCharacterOperators = new Set(['+', '-', '*', '/', '%', '=', '!', '<', '>', '&', '|', '^', '~', '?']);
const multiCharacterOperators = [
	'>>=',
	'<<=',
	'++',
	'--',
	'==',
	'!=',
	'<=',
	'>=',
	'&&',
	'||',
	'+=',
	'-=',
	'*=',
	'/=',
	'%=',
	'&=',
	'|=',
	'^=',
	'<<',
	'>>',
	'::',
	'->',
];

export function tokenizeEnforce(text: string): EnforceToken[] {
	const tokens: EnforceToken[] = [];
	let offset = 0;
	let line = 0;
	let character = 0;
	let atLineStart = true;

	const push = (kind: EnforceTokenKind, start: number, startLine: number, startCharacter: number, end: number, unterminated = false): void => {
		const tokenText = text.slice(start, end);
		tokens.push({
			kind,
			text: tokenText,
			start,
			end,
			line: startLine,
			character: startCharacter,
			endLine: line,
			endCharacter: character,
			unterminated: unterminated || undefined,
		});
	};

	const advance = (count = 1): void => {
		for (let i = 0; i < count; i++) {
			const ch = text[offset++];
			if (ch === '\r') {
				if (text[offset] === '\n') {
					offset++;
				}
				line++;
				character = 0;
				atLineStart = true;
				i++;
			} else if (ch === '\n') {
				line++;
				character = 0;
				atLineStart = true;
			} else {
				character++;
				if (ch !== ' ' && ch !== '\t') {
					atLineStart = false;
				}
			}
		}
	};

	while (offset < text.length) {
		const start = offset;
		const startLine = line;
		const startCharacter = character;
		const ch = text[offset];
		const next = text[offset + 1];

		if (ch === '\r' || ch === '\n') {
			advance(ch === '\r' && next === '\n' ? 2 : 1);
			push('newline', start, startLine, startCharacter, offset);
			continue;
		}

		if (ch === ' ' || ch === '\t') {
			while (text[offset] === ' ' || text[offset] === '\t') {
				advance();
			}
			push('whitespace', start, startLine, startCharacter, offset);
			continue;
		}

		if (atLineStart && ch === '#') {
			while (offset < text.length && text[offset] !== '\r' && text[offset] !== '\n') {
				advance();
			}
			push('preprocessor', start, startLine, startCharacter, offset);
			continue;
		}

		if (ch === '/' && next === '/') {
			advance(2);
			while (offset < text.length && text[offset] !== '\r' && text[offset] !== '\n') {
				advance();
			}
			push('comment', start, startLine, startCharacter, offset);
			continue;
		}

		if (ch === '/' && next === '*') {
			advance(2);
			let terminated = false;
			while (offset < text.length) {
				if (text[offset] === '*' && text[offset + 1] === '/') {
					advance(2);
					terminated = true;
					break;
				}
				advance();
			}
			push('comment', start, startLine, startCharacter, offset, !terminated);
			continue;
		}

		if (ch === '"' || ch === "'") {
			const quote = ch;
			advance();
			let terminated = false;
			while (offset < text.length) {
				if (text[offset] === '\\') {
					advance(Math.min(2, text.length - offset));
					continue;
				}
				if (text[offset] === quote) {
					advance();
					terminated = true;
					break;
				}
				if (text[offset] === '\r' || text[offset] === '\n') {
					break;
				}
				advance();
			}
			push('string', start, startLine, startCharacter, offset, !terminated);
			continue;
		}

		if (isIdentifierStart(ch)) {
			advance();
			while (offset < text.length && isIdentifierPart(text[offset])) {
				advance();
			}
			const tokenText = text.slice(start, offset);
			push(enforceKeywords.has(tokenText) ? 'keyword' : 'identifier', start, startLine, startCharacter, offset);
			continue;
		}

		if (isDigit(ch)) {
			advance();
			while (offset < text.length && /[A-Za-z0-9_.]/.test(text[offset])) {
				advance();
			}
			push('number', start, startLine, startCharacter, offset);
			continue;
		}

		const operator = multiCharacterOperators.find(candidate => text.startsWith(candidate, offset));
		if (operator) {
			advance(operator.length);
			push('operator', start, startLine, startCharacter, offset);
			continue;
		}

		if (punctuationCharacters.has(ch)) {
			advance();
			push('punctuation', start, startLine, startCharacter, offset);
			continue;
		}

		if (singleCharacterOperators.has(ch)) {
			advance();
			push('operator', start, startLine, startCharacter, offset);
			continue;
		}

		advance();
		push('unknown', start, startLine, startCharacter, offset);
	}

	tokens.push({
		kind: 'eof',
		text: '',
		start: offset,
		end: offset,
		line,
		character,
		endLine: line,
		endCharacter: character,
	});

	return tokens;
}

function isIdentifierStart(value: string | undefined): boolean {
	return value !== undefined && /[A-Za-z_]/.test(value);
}

function isIdentifierPart(value: string | undefined): boolean {
	return value !== undefined && /[A-Za-z0-9_]/.test(value);
}

function isDigit(value: string | undefined): boolean {
	return value !== undefined && /[0-9]/.test(value);
}
