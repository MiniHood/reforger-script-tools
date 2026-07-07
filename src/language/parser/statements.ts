import type { EnforceSymbol } from '../index/symbolIndex';
import type { EnforceParserDiagnostic, EnforceParserRange, EnforceParserScopeFact, EnforceSyntaxNode, EnforceSyntaxNodeKind } from './ast';
import { buildExpressionNodes } from './expressions';
import { findTopLevelCharacter, offsetFromPosition, SourceTextInfo, splitTopLevel, tokenRangeToParserRange } from './source';
import type { EnforceToken } from './tokens';
import { findMatchingTokenIndex, isTrivia, nextSignificantToken, previousSignificantToken, tokenIndexAfter, tokensText } from './tokenCursor';
import { getDeclarationValueTypeFromTokens, parseValueDeclarationText } from './types';

const ignoredValueNames = new Set([
	'if', 'for', 'foreach', 'while', 'switch', 'return', 'new', 'delete',
	'break', 'case', 'continue', 'default', 'else', 'null', 'super', 'this', 'true', 'false'
]);

const declarationModifiers = new Set([
	'autoptr', 'const', 'event', 'external', 'inout', 'modded', 'native', 'notnull', 'out', 'owned',
	'override', 'private', 'protected', 'proto', 'public', 'ref', 'sealed', 'static', 'volatile'
]);

export interface BodyParseResult {
	nodes: EnforceSyntaxNode[];
	scopes: EnforceParserScopeFact[];
	diagnostics: EnforceParserDiagnostic[];
}

export function parseFunctionBodies(symbols: EnforceSymbol[], tokens: EnforceToken[], source: SourceTextInfo): BodyParseResult {
	const result: BodyParseResult = { nodes: [], scopes: [], diagnostics: [] };
	for (const symbol of symbols) {
		if ((symbol.type !== 'memberFunction' && symbol.type !== 'function') || !symbol.bodyRange) {
			continue;
		}

		result.scopes.push(...collectParameterScopes(symbol, tokens));
		const body = parseFunctionBody(symbol, tokens, source);
		result.nodes.push(...body.nodes);
		result.scopes.push(...body.scopes);
		result.diagnostics.push(...body.diagnostics);
	}
	return result;
}

export function collectParserScopes(symbols: EnforceSymbol[], tokens: EnforceToken[], source: SourceTextInfo): EnforceParserScopeFact[] {
	return parseFunctionBodies(symbols, tokens, source).scopes;
}

export function buildBodySyntaxNodes(symbols: EnforceSymbol[], tokens: EnforceToken[], source: SourceTextInfo): EnforceSyntaxNode[] {
	return parseFunctionBodies(symbols, tokens, source).nodes;
}

function parseFunctionBody(symbol: EnforceSymbol, tokens: EnforceToken[], source: SourceTextInfo): BodyParseResult {
	const result: BodyParseResult = { nodes: [], scopes: [], diagnostics: [] };
	const missingSemicolonDiagnostics = new Set<string>();
	if (!symbol.bodyRange) {
		return result;
	}

	const bodyStart = offsetFromPosition(source, symbol.bodyRange.start.line, symbol.bodyRange.start.character);
	const bodyEnd = offsetFromPosition(source, symbol.bodyRange.end.line, symbol.bodyRange.end.character);
	let depth = 0;
	for (let index = 0; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.end <= bodyStart || token.start >= bodyEnd || token.kind === 'string' || token.kind === 'comment') {
			continue;
		}

		if (token.text === '{') {
			depth++;
			const closeBrace = findMatchingBraceToken(tokens, index) ?? token;
			result.scopes.push({
				kind: 'block',
				range: {
					start: { line: token.line, character: token.character },
					end: { line: closeBrace.endLine, character: closeBrace.endCharacter },
				},
				containerName: symbol.containerName,
				functionName: symbol.name,
				depth,
			});
			continue;
		}
		if (token.text === '}') {
			depth = Math.max(0, depth - 1);
			continue;
		}

		const controlNode = bodyControlNodeAt(tokens, index, symbol, depth);
		if (controlNode) {
			result.nodes.push(controlNode);
			if (isUnbracedControlKeyword(token.text)) {
				const bodyRange = findUnbracedControlBodyRange(tokens, index);
				if (bodyRange) {
					result.scopes.push({
						kind: 'block',
						range: bodyRange,
						containerName: symbol.containerName,
						functionName: symbol.name,
						depth: depth + 1,
					});
				}
			}
		}

		if (token.text === 'foreach') {
			result.scopes.push(...collectForeachScopesAt(tokens, index, source, symbol, depth));
		}

		if (token.text === 'for') {
			const forFacts = parseForHeaderAt(tokens, index, symbol, depth);
			result.nodes.push(...forFacts.nodes);
			result.scopes.push(...forFacts.scopes);
			if (forFacts.headerEndIndex !== undefined) {
				index = forFacts.headerEndIndex;
			}
		}

		if (token.text === 'switch') {
			result.scopes.push({
				kind: 'switch',
				range: createControlRange(tokens, index),
				containerName: symbol.containerName,
				functionName: symbol.name,
				depth,
			});
		}

		if (token.text === 'case' || token.text === 'default') {
			result.scopes.push({
				kind: 'case',
				range: createCaseRange(tokens, index),
				containerName: symbol.containerName,
				functionName: symbol.name,
				depth,
			});
		}

		if (token.text === 'break') {
			const statementEnd = findStatementEndIndex(tokens, index);
			result.nodes.push(createStatementNode('breakStatement', tokens, index, statementEnd, symbol, depth));
			pushMissingSemicolonDiagnostic(result, missingSemicolonDiagnostics, tokens, index, statementEnd);
		}

		if (token.text === 'continue') {
			const statementEnd = findStatementEndIndex(tokens, index);
			result.nodes.push(createStatementNode('continueStatement', tokens, index, statementEnd, symbol, depth));
			pushMissingSemicolonDiagnostic(result, missingSemicolonDiagnostics, tokens, index, statementEnd);
		}

		if (token.text === 'return') {
			const statementEnd = findStatementEndIndex(tokens, index);
			result.nodes.push(createStatementNode('returnStatement', tokens, index, statementEnd, symbol, depth));
			pushMissingSemicolonDiagnostic(result, missingSemicolonDiagnostics, tokens, index, statementEnd);
		}

		if (token.text === ';') {
			result.nodes.push(createStatementNode('emptyStatement', tokens, index, index, symbol, depth));
		}

		const statementEnd = findStatementEndIndex(tokens, index);
		const assignmentIndex = findTopLevelAssignmentTokenIndex(tokens, index, statementEnd);
		if (assignmentIndex >= 0 && isStatementStart(tokens, index)) {
			result.nodes.push(createStatementNode('assignmentExpression', tokens, index, statementEnd, symbol, depth));
			pushMissingSemicolonDiagnostic(result, missingSemicolonDiagnostics, tokens, index, statementEnd);
		}

		if (isStatementStart(tokens, index)) {
			const declarationFacts = parseLocalDeclarationAt(tokens, index, statementEnd, symbol, depth);
			if (declarationFacts) {
				result.scopes.push(...declarationFacts.scopes);
				result.nodes.push(...declarationFacts.nodes);
				pushMissingSemicolonDiagnostic(result, missingSemicolonDiagnostics, tokens, index, statementEnd);
				index = statementEnd;
				continue;
			}
			result.nodes.push(...buildExpressionNodes(tokens, index, statementEnd, source, symbol, depth));
		}
	}

	return result;
}

function collectParameterScopes(symbol: EnforceSymbol, tokens: EnforceToken[]): EnforceParserScopeFact[] {
	const nameIndex = tokens.findIndex(token =>
		token.text === symbol.name
		&& token.line === symbol.selectionRange.start.line
		&& token.character === symbol.selectionRange.start.character
	);
	if (nameIndex < 0) {
		return [];
	}

	const openIndex = findNextTokenIndex(tokens, nameIndex + 1, '(');
	if (openIndex < 0) {
		return [];
	}
	const closeIndex = findMatchingTokenIndex(tokens, openIndex, '(', ')');
	if (closeIndex < 0) {
		return [];
	}

	return splitParameterSegments(tokens, openIndex + 1, closeIndex)
		.map(segment => parameterScopeFromSegment(tokens, segment.start, segment.end, symbol))
		.filter((scope): scope is EnforceParserScopeFact => scope !== undefined);
}

function parameterScopeFromSegment(tokens: EnforceToken[], startIndex: number, endIndex: number, symbol: EnforceSymbol): EnforceParserScopeFact | undefined {
	const firstToken = nextSignificantToken(tokens, startIndex);
	const lastToken = previousSignificantToken(tokens, endIndex - 1);
	if (!firstToken || !lastToken || firstToken.start > lastToken.start) {
		return undefined;
	}

	const parameter = parseValueDeclarationText(tokensText(tokens, firstToken, lastToken));
	if (!parameter) {
		return undefined;
	}
	const nameToken = findNameTokenInSegment(tokens, startIndex, endIndex, parameter.name);
	if (!nameToken) {
		return undefined;
	}

	return {
			kind: 'parameter',
			name: parameter.name,
			valueType: parameter.valueType,
			containerName: symbol.containerName,
			functionName: symbol.name,
		range: {
			start: { line: firstToken.line, character: firstToken.character },
			end: { line: lastToken.endLine, character: lastToken.endCharacter },
		},
		selectionRange: tokenRangeToParserRange(nameToken),
			depth: 0,
	};
}

function splitParameterSegments(tokens: EnforceToken[], startIndex: number, endIndex: number): { start: number; end: number }[] {
	const segments: { start: number; end: number }[] = [];
	let segmentStart = startIndex;
	let depth = 0;
	for (let index = startIndex; index < endIndex; index++) {
		const token = tokens[index];
		if (isTrivia(token)) {
			continue;
		}
		if (token.text === '(' || token.text === '[' || token.text === '<') {
			depth++;
		} else if (token.text === ')' || token.text === ']' || token.text === '>') {
			depth = Math.max(0, depth - 1);
		} else if (token.text === ',' && depth === 0) {
			segments.push({ start: segmentStart, end: index });
			segmentStart = index + 1;
		}
	}
	if (segmentStart < endIndex) {
		segments.push({ start: segmentStart, end: endIndex });
	}
	return segments;
}

function findNameTokenInSegment(tokens: EnforceToken[], startIndex: number, endIndex: number, name: string): EnforceToken | undefined {
	for (let index = endIndex - 1; index >= startIndex; index--) {
		const token = tokens[index];
		if (token.kind === 'identifier' && token.text === name) {
			return token;
		}
	}
	return undefined;
}

function parseLocalDeclarationAt(tokens: EnforceToken[], index: number, statementEnd: number, symbol: EnforceSymbol, depth: number): { scopes: EnforceParserScopeFact[]; nodes: EnforceSyntaxNode[] } | undefined {
	const token = tokens[index];
	if (!isDeclarationBoundaryBefore(tokens, index) || !canStartDeclaration(token)) {
		return undefined;
	}

	const firstSegmentEnd = findForInitializerSegmentEnd(tokens, index, statementEnd);
	const firstSegmentDelimiter = tokens[firstSegmentEnd + 1]?.text === ',' ? firstSegmentEnd + 1 : firstSegmentEnd;
	const nameToken = getPropertyNameToken(tokens, index, firstSegmentDelimiter);
	if (!nameToken) {
		return undefined;
	}
	const valueType = getDeclarationValueTypeFromTokens(tokens, index, nameToken);
	if (!valueType) {
		return undefined;
	}

	const endToken = tokens[statementEnd] ?? token;
	const scopes: EnforceParserScopeFact[] = [];
	const nodes: EnforceSyntaxNode[] = [];
	let segmentStart = index;
	while (segmentStart <= statementEnd) {
		const segmentEnd = findForInitializerSegmentEnd(tokens, segmentStart, statementEnd);
		const nameSegmentEnd = tokens[segmentEnd]?.text === ';' ? segmentEnd - 1 : segmentEnd;
		const segmentNameToken = segmentStart === index
			? nameToken
			: getPropertyNameTokenForInitializerContinuation(tokens, segmentStart, nameSegmentEnd);
		if (segmentNameToken) {
			scopes.push({
				kind: 'local',
				name: segmentNameToken.text,
				valueType,
				range: tokenRangeToParserRange(token),
				selectionRange: tokenRangeToParserRange(segmentNameToken),
				containerName: symbol.containerName,
				functionName: symbol.name,
				depth,
			});
			nodes.push({
				kind: 'declarationStatement',
				name: segmentNameToken.text,
				valueType,
				containerName: symbol.containerName,
				expression: tokensText(tokens, token, endToken),
				range: {
					start: { line: token.line, character: token.character },
					end: { line: endToken.endLine, character: endToken.endCharacter },
				},
				selectionRange: tokenRangeToParserRange(segmentNameToken),
				complete: endToken.text === ';',
				confidence: 'medium',
				declarationKind: depth.toString(),
			});
		}
		segmentStart = segmentEnd + 2;
	}
	if (scopes.length === 0) {
		return undefined;
	}
	return {
		scopes,
		nodes,
	};
}

function collectForeachScopesAt(tokens: EnforceToken[], foreachIndex: number, source: SourceTextInfo, symbol: EnforceSymbol, depth: number): EnforceParserScopeFact[] {
	const openIndex = findNextTokenIndex(tokens, foreachIndex + 1, '(');
	if (openIndex < 0) {
		return [];
	}
	const closeIndex = findMatchingTokenIndex(tokens, openIndex, '(', ')');
	if (closeIndex < 0) {
		return [];
	}
	const content = source.text.slice(tokens[openIndex].end, tokens[closeIndex].start);
	const colonIndex = findTopLevelCharacter(content, ':');
	if (colonIndex < 0) {
		return [];
	}

	const bodyRange = findControlBodyRange(tokens, foreachIndex) ?? tokenRangeToParserRange(tokens[foreachIndex]);
	const declarationsText = content.slice(0, colonIndex);
	return splitTopLevel(declarationsText)
		.map(part => parseValueDeclarationText(part))
		.filter((declaration): declaration is { name: string; valueType: string } => declaration !== undefined)
		.map(declaration => ({
			kind: 'foreach',
			name: declaration.name,
			valueType: declaration.valueType,
			range: bodyRange,
			selectionRange: tokenRangeToParserRange(tokens[foreachIndex]),
			containerName: symbol.containerName,
			functionName: symbol.name,
			depth,
		}));
}

function parseForHeaderAt(tokens: EnforceToken[], forIndex: number, symbol: EnforceSymbol, depth: number): { nodes: EnforceSyntaxNode[]; scopes: EnforceParserScopeFact[]; headerEndIndex?: number } {
	const openIndex = findNextTokenIndex(tokens, forIndex + 1, '(');
	if (openIndex < 0) {
		return { nodes: [], scopes: [] };
	}
	const closeIndex = findMatchingTokenIndex(tokens, openIndex, '(', ')');
	if (closeIndex < 0) {
		return { nodes: [], scopes: [] };
	}

	const sections = splitForHeaderSections(tokens, openIndex + 1, closeIndex);
	const nodes: EnforceSyntaxNode[] = [];
	const scopes: EnforceParserScopeFact[] = [];
	if (sections[0]) {
		nodes.push(createStatementNode('forInitializer', tokens, sections[0].start, sections[0].end, symbol, depth));
		scopes.push(...collectForInitializerScopes(tokens, sections[0].start, sections[0].end, symbol, depth));
	}
	if (sections[2]) {
		nodes.push(createStatementNode('forUpdate', tokens, sections[2].start, sections[2].end, symbol, depth));
	}
	return { nodes, scopes, headerEndIndex: closeIndex };
}

function splitForHeaderSections(tokens: EnforceToken[], startIndex: number, closeIndex: number): Array<{ start: number; end: number } | undefined> {
	const sections: Array<{ start: number; end: number } | undefined> = [];
	let sectionStart = startIndex;
	let parens = 0;
	let brackets = 0;
	for (let index = startIndex; index < closeIndex; index++) {
		const token = tokens[index];
		if (token.text === '(') {
			parens++;
		} else if (token.text === ')') {
			parens = Math.max(0, parens - 1);
		} else if (token.text === '[') {
			brackets++;
		} else if (token.text === ']') {
			brackets = Math.max(0, brackets - 1);
		} else if (token.text === ';' && parens === 0 && brackets === 0) {
			sections.push(trimTokenSection(tokens, sectionStart, index - 1));
			sectionStart = index + 1;
		}
	}
	sections.push(trimTokenSection(tokens, sectionStart, closeIndex - 1));
	return sections;
}

function trimTokenSection(tokens: EnforceToken[], startIndex: number, endIndex: number): { start: number; end: number } | undefined {
	let start = startIndex;
	let end = endIndex;
	while (start <= end && isTrivia(tokens[start])) {
		start++;
	}
	while (end >= start && isTrivia(tokens[end])) {
		end--;
	}
	return start <= end ? { start, end } : undefined;
}

function collectForInitializerScopes(tokens: EnforceToken[], startIndex: number, endIndex: number, symbol: EnforceSymbol, depth: number): EnforceParserScopeFact[] {
	const firstName = getPropertyNameToken(tokens, startIndex, findForInitializerSegmentEnd(tokens, startIndex, endIndex));
	if (!firstName) {
		return [];
	}
	const valueType = getDeclarationValueTypeFromTokens(tokens, startIndex, firstName);
	if (!valueType) {
		return [];
	}

	const scopes: EnforceParserScopeFact[] = [];
	let segmentStart = startIndex;
	while (segmentStart <= endIndex) {
		const segmentEnd = findForInitializerSegmentEnd(tokens, segmentStart, endIndex);
		const nameToken = segmentStart === startIndex
			? firstName
			: getPropertyNameTokenForInitializerContinuation(tokens, segmentStart, segmentEnd);
		if (nameToken) {
			scopes.push({
				kind: 'local',
				name: nameToken.text,
				valueType,
				range: tokenRangeToParserRange(tokens[segmentStart]),
				selectionRange: tokenRangeToParserRange(nameToken),
				containerName: symbol.containerName,
				functionName: symbol.name,
				depth,
			});
		}
		segmentStart = segmentEnd + 2;
	}
	return scopes;
}

function findForInitializerSegmentEnd(tokens: EnforceToken[], startIndex: number, endIndex: number): number {
	let parens = 0;
	let brackets = 0;
	let angles = 0;
	for (let index = startIndex; index <= endIndex; index++) {
		const token = tokens[index];
		if (token.text === '(') {
			parens++;
		} else if (token.text === ')') {
			parens = Math.max(0, parens - 1);
		} else if (token.text === '[') {
			brackets++;
		} else if (token.text === ']') {
			brackets = Math.max(0, brackets - 1);
		} else if (token.text === '<') {
			angles++;
		} else if (token.text === '>>') {
			angles = Math.max(0, angles - 2);
		} else if (token.text === '>') {
			angles = Math.max(0, angles - 1);
		} else if (token.text === ',' && parens === 0 && brackets === 0 && angles === 0) {
			return Math.max(startIndex, index - 1);
		}
	}
	return endIndex;
}

function getPropertyNameTokenForInitializerContinuation(tokens: EnforceToken[], startIndex: number, endIndex: number): EnforceToken | undefined {
	const assignmentIndex = findTopLevelTokenIndex(tokens, startIndex, endIndex + 1, '=');
	const stopIndex = assignmentIndex >= 0 ? assignmentIndex : endIndex + 1;
	const nameToken = previousSignificantToken(tokens, stopIndex - 1);
	return nameToken && isIdentifierLike(nameToken) && /^[A-Za-z_]\w*$/.test(nameToken.text) ? nameToken : undefined;
}

function bodyControlNodeAt(tokens: EnforceToken[], index: number, symbol: EnforceSymbol, depth: number): EnforceSyntaxNode | undefined {
	const token = tokens[index];
	const map: Record<string, EnforceSyntaxNodeKind> = {
		if: 'if',
		else: 'else',
		while: 'while',
		for: 'for',
		foreach: 'foreach',
		switch: 'switch',
		case: 'case',
		default: 'case',
	};
	const kind = map[token.text];
	if (!kind) {
		return undefined;
	}
	const range = kind === 'case'
		? createCaseRange(tokens, index)
		: createControlRange(tokens, index);
	return {
		kind,
		name: token.text,
		containerName: symbol.containerName,
		expression: createControlExpression(tokens, index),
		signature: createControlExpression(tokens, index),
		range,
		selectionRange: tokenRangeToParserRange(token),
		complete: true,
		confidence: 'high',
		declarationKind: depth.toString(),
	};
}

function createStatementNode(
	kind: EnforceSyntaxNodeKind,
	tokens: EnforceToken[],
	startIndex: number,
	endIndex: number,
	symbol: EnforceSymbol,
	depth: number
): EnforceSyntaxNode {
	const start = tokens[startIndex];
	const end = tokens[Math.max(startIndex, endIndex)] ?? start;
	return {
		kind,
		name: start.text,
		containerName: symbol.containerName,
		expression: tokensText(tokens, start, end),
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

function pushMissingSemicolonDiagnostic(
	result: BodyParseResult,
	seen: Set<string>,
	tokens: EnforceToken[],
	startIndex: number,
	endIndex: number
): void {
	const start = tokens[startIndex];
	const end = tokens[Math.max(startIndex, endIndex)] ?? start;
	if (!start || !end || end.text === ';' || ['.', '::', '='].includes(end.text)) {
		return;
	}
	if (!statementRecoveredAtBoundary(tokens, endIndex)) {
		return;
	}

	const key = `${end.endLine}:${end.endCharacter}`;
	if (seen.has(key)) {
		return;
	}
	seen.add(key);
	result.diagnostics.push({
		message: "Missing ';' at end of statement.",
		range: {
			start: { line: end.endLine, character: end.endCharacter },
			end: { line: end.endLine, character: end.endCharacter + 1 },
		},
		severity: 'warning',
	});
}

function statementRecoveredAtBoundary(tokens: EnforceToken[], endIndex: number): boolean {
	for (let index = endIndex + 1; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'whitespace' || token.kind === 'comment') {
			continue;
		}
		return token.kind === 'newline' || token.kind === 'eof' || token.text === '}';
	}
	return true;
}

function createControlRange(tokens: EnforceToken[], index: number): EnforceParserRange {
	return findControlBodyRange(tokens, index) ?? tokenRangeToParserRange(tokens[index]);
}

function createCaseRange(tokens: EnforceToken[], index: number): EnforceParserRange {
	const endIndex = findCaseEndIndex(tokens, index);
	const start = tokens[index];
	const end = tokens[endIndex] ?? start;
	return {
		start: { line: start.line, character: start.character },
		end: { line: end.endLine, character: end.endCharacter },
	};
}

function createControlExpression(tokens: EnforceToken[], index: number): string {
	const token = tokens[index];
	if (token.text === 'case' || token.text === 'default') {
		const endIndex = findStatementEndIndex(tokens, index);
		return tokensText(tokens, token, tokens[endIndex] ?? token);
	}
	if (token.text === 'else') {
		return token.text;
	}
	const openIndex = findNextTokenIndex(tokens, index + 1, '(');
	const closeIndex = openIndex >= 0 ? findMatchingTokenIndex(tokens, openIndex, '(', ')') : -1;
	if (openIndex < 0 || closeIndex < 0) {
		return token.text;
	}
	return tokensText(tokens, token, tokens[closeIndex]);
}

function findControlBodyRange(tokens: EnforceToken[], index: number): EnforceParserRange | undefined {
	const token = tokens[index];
	if (token.text === 'else') {
		return findBodyRangeAfter(tokens, index);
	}
	const openIndex = findNextTokenIndex(tokens, index + 1, '(');
	if (openIndex < 0) {
		return undefined;
	}
	const closeIndex = findMatchingTokenIndex(tokens, openIndex, '(', ')');
	if (closeIndex < 0) {
		return undefined;
	}
	return findBodyRangeAfter(tokens, closeIndex);
}

function findUnbracedControlBodyRange(tokens: EnforceToken[], index: number): EnforceParserRange | undefined {
	const range = findControlBodyRange(tokens, index);
	if (!range) {
		return undefined;
	}
	const bodyStart = nextSignificantToken(tokens, rangeStartTokenIndex(tokens, range));
	if (!bodyStart || bodyStart.text === '{') {
		return undefined;
	}
	return range;
}

function findBodyRangeAfter(tokens: EnforceToken[], index: number): EnforceParserRange | undefined {
	const bodyStartIndex = nextSignificantTokenIndex(tokens, index + 1);
	if (bodyStartIndex < 0) {
		return undefined;
	}
	const start = tokens[bodyStartIndex];
	if (start.text === '{') {
		const closeIndex = findMatchingTokenIndex(tokens, bodyStartIndex, '{', '}');
		const end = tokens[closeIndex >= 0 ? closeIndex : bodyStartIndex];
		return {
			start: { line: start.line, character: start.character },
			end: { line: end.endLine, character: end.endCharacter },
		};
	}
	const endIndex = findStatementEndIndex(tokens, bodyStartIndex);
	const end = tokens[endIndex] ?? start;
	return {
		start: { line: start.line, character: start.character },
		end: { line: end.endLine, character: end.endCharacter },
	};
}

function findCaseEndIndex(tokens: EnforceToken[], index: number): number {
	let depth = 0;
	for (let cursor = index + 1; cursor < tokens.length; cursor++) {
		const token = tokens[cursor];
		if (token.kind === 'string' || token.kind === 'comment') {
			continue;
		}
		if (token.text === '{') {
			depth++;
		} else if (token.text === '}') {
			if (depth === 0) {
				return Math.max(index, cursor - 1);
			}
			depth--;
		} else if ((token.text === 'case' || token.text === 'default') && depth === 0) {
			return Math.max(index, cursor - 1);
		}
	}
	return index;
}

function findStatementEndIndex(tokens: EnforceToken[], startIndex: number): number {
	let parens = 0;
	let brackets = 0;
	let braces = 0;
	for (let index = startIndex; index < tokens.length; index++) {
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
		} else if (token.text === '{') {
			braces++;
		} else if (token.text === '}') {
			if (braces === 0) {
				return Math.max(startIndex, index - 1);
			}
			braces = Math.max(0, braces - 1);
		} else if ((token.text === ';' || token.kind === 'newline' || token.kind === 'eof') && parens === 0 && brackets === 0 && braces === 0) {
			return Math.max(startIndex, token.text === ';' ? index : index - 1);
		}
	}
	return startIndex;
}

function findTopLevelTokenIndex(tokens: EnforceToken[], startIndex: number, stopIndex: number, text: string): number {
	let parens = 0;
	let brackets = 0;
	let angles = 0;
	for (let index = startIndex; index < stopIndex; index++) {
		const token = tokens[index];
		if (token.text === '(') {
			if (text === '(' && parens === 0 && brackets === 0 && angles === 0) {
				return index;
			}
			parens++;
		} else if (token.text === ')') {
			parens = Math.max(0, parens - 1);
		} else if (token.text === '[') {
			brackets++;
		} else if (token.text === ']') {
			brackets = Math.max(0, brackets - 1);
		} else if (token.text === '<') {
			angles++;
		} else if (token.text === '>>') {
			angles = Math.max(0, angles - 2);
		} else if (token.text === '>') {
			angles = Math.max(0, angles - 1);
		} else if (token.text === text && parens === 0 && brackets === 0 && angles === 0) {
			return index;
		}
	}
	return -1;
}

function findTopLevelAssignmentTokenIndex(tokens: EnforceToken[], startIndex: number, stopIndex: number): number {
	const assignmentOperators = new Set(['=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=']);
	let parens = 0;
	let brackets = 0;
	let angles = 0;
	for (let index = startIndex; index < stopIndex; index++) {
		const token = tokens[index];
		if (token.text === '(') {
			parens++;
		} else if (token.text === ')') {
			parens = Math.max(0, parens - 1);
		} else if (token.text === '[') {
			brackets++;
		} else if (token.text === ']') {
			brackets = Math.max(0, brackets - 1);
		} else if (token.text === '<') {
			angles++;
		} else if (token.text === '>>') {
			angles = Math.max(0, angles - 2);
		} else if (token.text === '>') {
			angles = Math.max(0, angles - 1);
		} else if (assignmentOperators.has(token.text) && parens === 0 && brackets === 0 && angles === 0) {
			return index;
		}
	}
	return -1;
}

function getPropertyNameToken(tokens: EnforceToken[], startIndex: number, delimiterIndex: number): EnforceToken | undefined {
	const assignmentIndex = findTopLevelTokenIndex(tokens, startIndex, delimiterIndex, '=');
	const endIndex = assignmentIndex >= 0 ? assignmentIndex : delimiterIndex;
	if (findTopLevelTokenIndex(tokens, startIndex, endIndex, '(') >= 0) {
		return undefined;
	}
	const nameToken = getDeclarationNameBeforeEnd(tokens, startIndex, endIndex);
	if (!nameToken || !isIdentifierLike(nameToken) || ignoredValueNames.has(nameToken.text)) {
		return undefined;
	}
	const declarationParts = significantTokensBetween(tokens, startIndex, endIndex).filter(token => !isDeclarationModifier(token));
	if (declarationParts.length < 2) {
		return undefined;
	}
	if (previousSignificantToken(tokens, tokenIndexAfter(tokens, nameToken) - 2)?.text === '.') {
		return undefined;
	}
	return /^[A-Za-z_]\w*$/.test(nameToken.text) ? nameToken : undefined;
}

function getDeclarationNameBeforeEnd(tokens: EnforceToken[], startIndex: number, endIndex: number): EnforceToken | undefined {
	const previous = previousSignificantToken(tokens, endIndex - 1);
	if (!previous) {
		return undefined;
	}
	if (previous.text !== ']') {
		return previous;
	}

	const openBracketIndex = findMatchingOpenBracketIndex(tokens, tokenIndexAfter(tokens, previous) - 1, startIndex);
	if (openBracketIndex < 0) {
		return previous;
	}
	return previousSignificantToken(tokens, openBracketIndex - 1);
}

function findMatchingOpenBracketIndex(tokens: EnforceToken[], closeBracketIndex: number, stopIndex: number): number {
	let depth = 0;
	for (let index = closeBracketIndex; index >= stopIndex; index--) {
		const token = tokens[index];
		if (token.text === ']') {
			depth++;
		} else if (token.text === '[') {
			depth--;
			if (depth === 0) {
				return index;
			}
		}
	}
	return -1;
}

function findDeclarationTerminator(tokens: EnforceToken[], startIndex: number): { delimiterIndex: number; delimiterToken: EnforceToken } | undefined {
	let parens = 0;
	let brackets = 0;
	let angles = 0;
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'eof') {
			return undefined;
		}
		if (token.kind === 'comment' || token.kind === 'string') {
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
		} else if (token.text === '<') {
			angles++;
		} else if (token.text === '>>') {
			angles = Math.max(0, angles - 2);
		} else if (token.text === '>') {
			angles = Math.max(0, angles - 1);
		} else if ((token.text === ';' || token.text === '{') && parens === 0 && brackets === 0 && angles === 0) {
			return { delimiterIndex: index, delimiterToken: token };
		}
	}
	return undefined;
}

function isDeclarationBoundaryBefore(tokens: EnforceToken[], startIndex: number): boolean {
	const previous = previousSignificantToken(tokens, startIndex - 1);
	if (!previous) {
		return true;
	}
	const startToken = tokens[startIndex];
	if (previous.endLine < startToken.line) {
		if (startsCompleteDeclarationLine(tokens, startIndex)) {
			return true;
		}
		if (['(', ',', '.'].includes(previous.text)) {
			return false;
		}
		if (isDeclarationModifier(startToken)) {
			return true;
		}
	}
	return previous.kind === 'preprocessor' || [';', '{', '}', ']'].includes(previous.text);
}

function startsCompleteDeclarationLine(tokens: EnforceToken[], startIndex: number): boolean {
	const lineEndIndex = findLineEndIndex(tokens, startIndex);
	if (lineContainsTokenText(tokens, startIndex, lineEndIndex, ';') || lineContainsTokenText(tokens, startIndex, lineEndIndex, '{')) {
		return true;
	}
	if (!lineContainsTokenText(tokens, startIndex, lineEndIndex, '(') || !lineContainsTokenText(tokens, startIndex, lineEndIndex, ')')) {
		return false;
	}
	const next = nextSignificantToken(tokens, lineEndIndex + 1);
	return next?.text === '{' || next?.text === ';';
}

function lineContainsTokenText(tokens: EnforceToken[], startIndex: number, lineEndIndex: number, text: string): boolean {
	for (let index = startIndex; index < lineEndIndex; index++) {
		if (tokens[index].text === text) {
			return true;
		}
	}
	return false;
}

function findLineEndIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		if (tokens[index].kind === 'newline' || tokens[index].kind === 'eof') {
			return index;
		}
	}
	return tokens.length - 1;
}

function significantTokensBetween(tokens: EnforceToken[], startIndex: number, stopIndex: number): EnforceToken[] {
	const result: EnforceToken[] = [];
	for (let index = startIndex; index < stopIndex; index++) {
		const token = tokens[index];
		if (!isTrivia(token)) {
			result.push(token);
		}
	}
	return result;
}

function canStartDeclaration(token: EnforceToken): boolean {
	return isIdentifierLike(token) && !ignoredValueNames.has(token.text);
}

function isDeclarationModifier(token: EnforceToken): boolean {
	return declarationModifiers.has(token.text);
}

function isIdentifierLike(token: EnforceToken): boolean {
	return token.kind === 'identifier' || token.kind === 'keyword';
}

function findMatchingBraceToken(tokens: EnforceToken[], openBraceIndex: number): EnforceToken | undefined {
	const closeIndex = findMatchingTokenIndex(tokens, openBraceIndex, '{', '}');
	return closeIndex >= 0 ? tokens[closeIndex] : undefined;
}

function findNextTokenIndex(tokens: EnforceToken[], startIndex: number, text: string): number {
	for (let index = startIndex; index < tokens.length; index++) {
		if (tokens[index].text === text) {
			return index;
		}
		if (!isTrivia(tokens[index])) {
			return -1;
		}
	}
	return -1;
}

function nextSignificantTokenIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		if (!isTrivia(tokens[index])) {
			return index;
		}
	}
	return -1;
}

function rangeStartTokenIndex(tokens: EnforceToken[], range: EnforceParserRange): number {
	return tokens.findIndex(token => token.line === range.start.line && token.character === range.start.character);
}

function isUnbracedControlKeyword(text: string): boolean {
	return text === 'if' || text === 'else' || text === 'while' || text === 'for' || text === 'foreach';
}

function isStatementStart(tokens: EnforceToken[], index: number): boolean {
	const previous = previousSignificantToken(tokens, index - 1);
	return !previous || [';', '{', '}'].includes(previous.text) || previous.endLine < tokens[index].line;
}

function vscodeRangeToParserRange(range: { start: { line: number; character: number }; end: { line: number; character: number } }): EnforceParserRange {
	return {
		start: { line: range.start.line, character: range.start.character },
		end: { line: range.end.line, character: range.end.character },
	};
}
