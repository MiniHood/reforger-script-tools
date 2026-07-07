import type { EnforceSymbol } from '../index/symbolIndex';
import type { EnforceSyntaxNode, EnforceSyntaxNodeKind } from './ast';
import { SourceTextInfo, tokenRangeToParserRange } from './source';
import type { EnforceToken } from './tokens';
import { findMatchingTokenIndex, nextSignificantToken, previousSignificantToken, tokenIndexAfter, tokensText } from './tokenCursor';

const ignoredCallNames = new Set([
	'if', 'for', 'foreach', 'while', 'switch', 'return', 'new', 'delete'
]);

const binaryOperators = new Set([
	'||', '&&', '|', '^', '&', '==', '!=', '<', '<=', '>', '>=', '<<', '>>',
	'+', '-', '*', '/', '%'
]);

const assignmentOperators = new Set(['=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=']);
const unaryOperators = new Set(['!', '~', '++', '--', '+', '-']);
const ignoredIdentifierNames = new Set([...ignoredCallNames, 'case', 'default', 'break', 'continue']);

export function buildExpressionNodes(
	tokens: EnforceToken[],
	startIndex: number,
	endIndex: number,
	source: SourceTextInfo,
	symbol: EnforceSymbol,
	depth: number
): EnforceSyntaxNode[] {
	const nodes: EnforceSyntaxNode[] = [];
	for (let index = startIndex; index <= endIndex && index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'comment') {
			continue;
		}

		if (token.kind === 'string' || token.kind === 'number' || token.text === 'true' || token.text === 'false' || token.text === 'null') {
			nodes.push(createExpressionNode('literal', tokens, index, index, symbol, depth));
		}

		if (isIdentifierLike(token) && !ignoredIdentifierNames.has(token.text)) {
			nodes.push(createExpressionNode('identifier', tokens, index, index, symbol, depth));
		}

		if (isUnaryOperatorAt(tokens, index, startIndex)) {
			nodes.push(createExpressionNode('unaryExpression', tokens, index, findExpressionEndIndex(tokens, index, endIndex), symbol, depth, token.text));
		}

		if (binaryOperators.has(token.text)) {
			nodes.push(createExpressionNode('binaryExpression', tokens, startIndex, endIndex, symbol, depth, token.text));
		}

		if (assignmentOperators.has(token.text)) {
			nodes.push(createExpressionNode('assignmentExpression', tokens, startIndex, endIndex, symbol, depth, token.text));
		}

		if (token.text === 'new') {
			nodes.push(createExpressionNode('newExpression', tokens, index, findExpressionEndIndex(tokens, index, endIndex), symbol, depth));
		}

		if (token.text === '.' || token.text === '::') {
			const member = nextSignificantToken(tokens, index + 1);
			const receiver = extractReceiverBeforeToken(source.text, token.start);
			if (member && isIdentifierLike(member) && receiver) {
				nodes.push({
					kind: 'memberAccess',
					name: member.text,
					memberName: member.text,
					receiver,
					accessOperator: token.text === '::' ? '::' : '.',
					expression: `${receiver}${token.text}${member.text}`,
					containerName: symbol.containerName,
					signature: `${receiver}${token.text}${member.text}`,
					range: receiverRange(source.text, token, receiver, member),
					selectionRange: tokenRangeToParserRange(member),
					complete: true,
					confidence: 'medium',
				});
			} else if (receiver) {
				nodes.push({
					kind: 'incompleteExpression',
					receiver,
					accessOperator: token.text === '::' ? '::' : '.',
					expression: `${receiver}${token.text}`,
					containerName: symbol.containerName,
					range: {
						start: { line: token.line, character: Math.max(0, token.character - receiver.length) },
						end: { line: token.endLine, character: token.endCharacter },
					},
					complete: false,
					missingToken: 'identifier',
					incomplete: true,
					confidence: 'medium',
				});
			}
		}

		if (token.text === '[' && previousSignificantToken(tokens, index - 1)) {
			nodes.push(createExpressionNode('indexAccess', tokens, tokenIndexAfter(tokens, previousSignificantToken(tokens, index - 1) ?? token) - 1, findExpressionEndIndex(tokens, index, endIndex), symbol, depth));
		}

		if (token.text === '(') {
			const callable = previousSignificantToken(tokens, index - 1);
			const closeIndex = findMatchingTokenIndex(tokens, index, '(', ')');
			if (callable && isIdentifierLike(callable) && !ignoredCallNames.has(callable.text)) {
				const accessToken = previousSignificantToken(tokens, tokenIndexAfter(tokens, callable) - 2);
				const kind: EnforceSyntaxNodeKind = accessToken?.text === '.' && callable.text === 'Cast'
					? 'castExpression'
					: 'callExpression';
				nodes.push(createExpressionNode(kind, tokens, tokenIndexAfter(tokens, callable) - 1, closeIndex >= 0 ? closeIndex : findExpressionEndIndex(tokens, index, endIndex), symbol, depth));
				nodes.push(createExpressionNode('argumentList', tokens, index, closeIndex >= 0 ? closeIndex : findExpressionEndIndex(tokens, index, endIndex), symbol, depth));
			} else {
				nodes.push(createExpressionNode('parenthesizedExpression', tokens, index, closeIndex >= 0 ? closeIndex : findExpressionEndIndex(tokens, index, endIndex), symbol, depth));
			}
		}
	}
	return dedupeExpressionNodes(nodes);
}

export function createExpressionNode(
	kind: EnforceSyntaxNodeKind,
	tokens: EnforceToken[],
	startIndex: number,
	endIndex: number,
	symbol: EnforceSymbol,
	depth: number,
	operator?: string
): EnforceSyntaxNode {
	const start = tokens[startIndex];
	const end = tokens[Math.max(startIndex, endIndex)] ?? start;
	return {
		kind,
		name: start.text,
		containerName: symbol.containerName,
		expression: tokensText(tokens, start, end),
		operator,
		range: {
			start: { line: start.line, character: start.character },
			end: { line: end.endLine, character: end.endCharacter },
		},
		selectionRange: tokenRangeToParserRange(start),
		complete: !['.', '::', '='].includes(end.text),
		incomplete: ['.', '::', '='].includes(end.text),
		missingToken: ['.', '::'].includes(end.text) ? 'identifier' : undefined,
		confidence: 'medium',
		declarationKind: depth.toString(),
	};
}

function findExpressionEndIndex(tokens: EnforceToken[], startIndex: number, stopIndex: number): number {
	let parens = 0;
	let brackets = 0;
	for (let index = startIndex; index <= stopIndex && index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'string' || token.kind === 'comment') {
			continue;
		}
		if (token.text === '(') {
			parens++;
		} else if (token.text === ')') {
			parens = Math.max(0, parens - 1);
		} else if (token.text === '[') {
			brackets++;
		} else if (token.text === ']') {
			brackets = Math.max(0, brackets - 1);
		} else if ((token.text === ';' || token.kind === 'newline' || token.kind === 'eof') && parens === 0 && brackets === 0) {
			return Math.max(startIndex, token.text === ';' ? index : index - 1);
		}
	}
	return Math.max(startIndex, stopIndex);
}

function extractReceiverBeforeToken(text: string, tokenStart: number): string | undefined {
	let index = tokenStart - 1;
	while (index >= 0 && /\s/.test(text[index])) {
		index--;
	}
	const end = index + 1;
	let parens = 0;
	let brackets = 0;
	while (index >= 0) {
		const char = text[index];
		if (char === ')') {
			parens++;
		} else if (char === '(') {
			if (parens === 0) {
				break;
			}
			parens--;
		} else if (char === ']') {
			brackets++;
		} else if (char === '[') {
			if (brackets === 0) {
				break;
			}
			brackets--;
		} else if (parens === 0 && brackets === 0 && !/[A-Za-z0-9_\]\)\.]/.test(char)) {
			break;
		}
		index--;
	}
	const receiver = text.slice(index + 1, end).trim();
	return receiver || undefined;
}

function receiverRange(text: string, accessToken: EnforceToken, receiver: string, member: EnforceToken) {
	const receiverStartOffset = Math.max(0, accessToken.start - receiver.length);
	const lineStart = text.lastIndexOf('\n', receiverStartOffset) + 1;
	return {
		start: { line: accessToken.line, character: receiverStartOffset - lineStart },
		end: { line: member.endLine, character: member.endCharacter },
	};
}

function isIdentifierLike(token: EnforceToken): boolean {
	return token.kind === 'identifier' || token.kind === 'keyword';
}

function isUnaryOperatorAt(tokens: EnforceToken[], index: number, statementStartIndex: number): boolean {
	const token = tokens[index];
	if (!unaryOperators.has(token.text)) {
		return false;
	}
	const previous = previousSignificantToken(tokens, index - 1);
	return !previous || index === statementStartIndex || ['(', '[', '{', ',', ';', '=', '==', '!=', '<', '<=', '>', '>=', '&&', '||', 'return', 'case'].includes(previous.text);
}

function dedupeExpressionNodes(nodes: EnforceSyntaxNode[]): EnforceSyntaxNode[] {
	const seen = new Set<string>();
	const result: EnforceSyntaxNode[] = [];
	for (const node of nodes) {
		const key = `${node.kind}:${node.range.start.line}:${node.range.start.character}:${node.range.end.line}:${node.range.end.character}:${node.expression ?? ''}:${node.operator ?? ''}`;
		if (!seen.has(key)) {
			seen.add(key);
			result.push(node);
		}
	}
	return result;
}
