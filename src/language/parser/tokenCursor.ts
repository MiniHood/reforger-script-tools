import type { EnforceToken } from './tokens';

export class TokenCursor {
	constructor(
		private readonly tokens: EnforceToken[],
		private index = 0
	) {}

	current(): EnforceToken | undefined {
		return this.tokens[this.index];
	}

	peek(offset = 0): EnforceToken | undefined {
		return this.tokens[this.index + offset];
	}

	position(): number {
		return this.index;
	}

	seek(index: number): void {
		this.index = Math.max(0, Math.min(this.tokens.length, index));
	}

	advance(count = 1): EnforceToken | undefined {
		const token = this.current();
		this.seek(this.index + count);
		return token;
	}

	nextSignificant(startIndex = this.index): EnforceToken | undefined {
		const index = nextSignificantTokenIndex(this.tokens, startIndex);
		return index >= 0 ? this.tokens[index] : undefined;
	}

	previousSignificant(startIndex = this.index): EnforceToken | undefined {
		return previousSignificantToken(this.tokens, startIndex);
	}

	findMatchingClose(openIndex: number, openText: string, closeText: string): number {
		return findMatchingTokenIndex(this.tokens, openIndex, openText, closeText);
	}
}

export function isTrivia(token: EnforceToken): boolean {
	return token.kind === 'whitespace' || token.kind === 'newline' || token.kind === 'comment';
}

export function tokensText(tokens: EnforceToken[], startToken: EnforceToken, endToken: EnforceToken): string {
	const firstIndex = tokenIndexAfter(tokens, startToken) - 1;
	const lastIndex = tokenIndexAfter(tokens, endToken) - 1;
	return tokens.slice(firstIndex, lastIndex + 1).map(token => token.text).join('');
}

export function tokenIndexAfter(tokens: EnforceToken[], target: EnforceToken): number {
	const index = tokens.indexOf(target);
	return index < 0 ? 0 : index + 1;
}

export function nextSignificantToken(tokens: EnforceToken[], startIndex: number): EnforceToken | undefined {
	const index = nextSignificantTokenIndex(tokens, startIndex);
	return index >= 0 ? tokens[index] : undefined;
}

export function nextSignificantTokenIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		if (!isTrivia(tokens[index])) {
			return index;
		}
	}
	return -1;
}

export function previousSignificantToken(tokens: EnforceToken[], startIndex: number): EnforceToken | undefined {
	for (let index = startIndex; index >= 0; index--) {
		if (!isTrivia(tokens[index])) {
			return tokens[index];
		}
	}
	return undefined;
}

export function findMatchingTokenIndex(tokens: EnforceToken[], openIndex: number, openText: string, closeText: string): number {
	let depth = 0;
	for (let index = openIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'string' || token.kind === 'comment') {
			continue;
		}
		if (token.text === openText) {
			depth++;
		} else if (token.text === closeText) {
			depth--;
			if (depth === 0) {
				return index;
			}
		}
	}
	return -1;
}
