import * as vscode from 'vscode';
import type { EnforceDeclarationKind, EnforceDecorator, EnforceSymbol, EnforceSymbolType } from '../index/symbolIndex';
import { EnforceParserRange, EnforceParserScopeFact, ParsedEnforceSource } from './ast';
import { buildSyntaxNodes } from './astBuilder';
import { collectRecoveryDiagnostics } from './diagnostics';
import { createSourceTextInfo, escapeRegExp, findTopLevelCharacter, normalizeSourceText, offsetFromPosition, SourceTextInfo, splitTopLevel, tokenRangeToParserRange } from './source';
import { collectParserScopes as collectStructuredParserScopes, parseFunctionBodies } from './statements';
import { addClassMemberSummaries, symbolToSyntaxNode, vscodeRangeToParserRange } from './symbolAdapter';
import { tokenizeEnforce } from './tokenizer';
import type { EnforceToken } from './tokens';
import { findMatchingTokenIndex, isTrivia, nextSignificantToken, nextSignificantTokenIndex, previousSignificantToken, tokenIndexAfter, tokensText } from './tokenCursor';
import { getDeclarationValueTypeFromTokens, parseValueDeclarationText } from './types';

interface ClassContext {
	name: string;
	depth: number;
}

interface ParsedDeclaration {
	symbol: EnforceSymbol;
	hasBody: boolean;
	delimiterIndex: number;
}

const declarationModifiers = new Set([
	'autoptr', 'const', 'event', 'external', 'inout', 'modded', 'native', 'notnull', 'out', 'owned',
	'override', 'private', 'protected', 'proto', 'public', 'ref', 'sealed', 'static', 'volatile'
]);

const ignoredFunctionNames = new Set([
	'if', 'for', 'foreach', 'while', 'switch', 'return', 'new', 'delete'
]);

const ignoredPropertyNames = new Set([
	...ignoredFunctionNames,
	'break', 'case', 'continue', 'default', 'else', 'null', 'super', 'this', 'true', 'false'
]);

export function parseEnforceSource(text: string, uri: vscode.Uri): ParsedEnforceSource {
	const tokens = tokenizeEnforce(text);
	const symbols = parseParserSymbols(text, uri, tokens);
	addClassMemberSummaries(symbols);
	const source = createSourceTextInfo(text);
	const body = parseFunctionBodies(symbols, tokens, source);
	const diagnostics = collectRecoveryDiagnostics(tokens);

	return {
		sourceText: text,
		tokens,
		nodes: buildSyntaxNodes(symbols, body.scopes, body.nodes, tokens, source),
		symbols,
		scopes: body.scopes,
		diagnostics: [...diagnostics, ...body.diagnostics],
	};
}

export function parseEnforceDeclarations(text: string, uri: vscode.Uri): EnforceSymbol[] {
	const symbols = parseParserSymbols(text, uri, tokenizeEnforce(text));
	addClassMemberSummaries(symbols);
	return symbols;
}

export function parseParserSymbolsForTest(text: string, uri: vscode.Uri): EnforceSymbol[] {
	const symbols = parseParserSymbols(text, uri, tokenizeEnforce(text));
	addClassMemberSummaries(symbols);
	return symbols;
}

export function parseParserScopesForTest(text: string, uri: vscode.Uri): EnforceParserScopeFact[] {
	const tokens = tokenizeEnforce(text);
	const symbols = parseParserSymbols(text, uri, tokens);
	return collectStructuredParserScopes(symbols, tokens, createSourceTextInfo(text));
}

function parseParserSymbols(text: string, uri: vscode.Uri, tokens: EnforceToken[]): EnforceSymbol[] {
	const source = createSourceTextInfo(text);
	const symbols: EnforceSymbol[] = [];
	const classStack: ClassContext[] = [];
	const functionDepthStack: number[] = [];
	let braceDepth = 0;
	let pendingClassName: string | undefined;
	let pendingFunctionBody = false;
	let pendingDecorators: EnforceDecorator[] = [];
	let activeEnum: EnforceSymbol | undefined;
	let index = 0;

	while (index < tokens.length) {
		const token = tokens[index];

		if (token.kind === 'eof') {
			break;
		}

		if (isTrivia(token)) {
			index++;
			continue;
		}

		if (token.kind === 'preprocessor') {
			const macro = parseMacroToken(token, uri);
			if (macro) {
				symbols.push(macro);
			}
			index++;
			continue;
		}

		if (functionDepthStack.length === 0 && token.text === 'typedef') {
			const typedefSymbol = parseTypedefDeclarationAt(tokens, index, source, uri);
			if (typedefSymbol) {
				symbols.push(typedefSymbol.symbol);
				index = typedefSymbol.delimiterIndex + 1;
				continue;
			}
		}

		if (token.text === '{') {
			braceDepth++;
			if (pendingClassName) {
				classStack.push({ name: pendingClassName, depth: braceDepth });
				pendingClassName = undefined;
			}
			if (pendingFunctionBody) {
				functionDepthStack.push(braceDepth);
				pendingFunctionBody = false;
			}
			index++;
			continue;
		}

		if (token.text === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
			while (functionDepthStack.length > 0 && braceDepth < functionDepthStack[functionDepthStack.length - 1]) {
				functionDepthStack.pop();
			}
			while (classStack.length > 0 && braceDepth < classStack[classStack.length - 1].depth) {
				classStack.pop();
			}
			if (activeEnum && braceDepth <= symbolDepth(activeEnum)) {
				activeEnum = undefined;
			}
			index++;
			continue;
		}

		if (token.text === '[' && functionDepthStack.length === 0) {
			const decorator = parseDecoratorAt(tokens, index, text);
			if (decorator) {
				pendingDecorators.push(...decorator.decorators);
				index = decorator.endIndex + 1;
				continue;
			}
			pendingDecorators = [];
			index = findLineEndIndex(tokens, index) + 1;
			continue;
		}

		const classDeclaration = parseClassDeclarationAt(tokens, index, source, uri, pendingDecorators);
		if (classDeclaration) {
			symbols.push(classDeclaration.symbol);
			pendingClassName = classDeclaration.pendingClassName;
			index = classDeclaration.nextIndex;
			continue;
		}

		const enumDeclaration = parseEnumDeclarationAt(tokens, index, source, uri);
		if (enumDeclaration) {
			activeEnum = enumDeclaration;
			symbols.push(enumDeclaration);
			index = nextTokenAfterDeclarationHeader(tokens, index);
			continue;
		}

		if (activeEnum?.bodyRange && token.start >= offsetFromPosition(source, activeEnum.bodyRange.end.line, activeEnum.bodyRange.end.character)) {
			activeEnum = undefined;
		}
		if (activeEnum && functionDepthStack.length === 0) {
			const member = parseEnumMemberAt(tokens, index, source, uri, activeEnum);
			if (member) {
				activeEnum.enumMembers?.push(member.signature ?? member.name);
				symbols.push(member);
				index = nextEnumMemberIndex(tokens, index);
				continue;
			}
		}

		const containerName = classStack[classStack.length - 1]?.name;
		if (functionDepthStack.length === 0) {
			const declaration = parseFunctionOrPropertyAt(tokens, index, source, uri, containerName, pendingDecorators);
			if (declaration) {
				symbols.push(declaration.symbol);
				pendingFunctionBody = declaration.hasBody && (declaration.symbol.type === 'memberFunction' || declaration.symbol.type === 'function');
				index = declaration.delimiterIndex;
				continue;
			}
		}

		if (pendingDecorators.length > 0 && !isIgnorableDecoratorGap(token)) {
			pendingDecorators = [];
		}

		index++;
	}

	return symbols;
}

function parseMacroToken(token: EnforceToken, uri: vscode.Uri): EnforceSymbol | undefined {
	const match = /^\s*#\s*define\s+([A-Za-z_]\w*)\b/.exec(token.text);
	if (!match?.[1]) {
		return undefined;
	}

	const nameStart = token.character + match[0].lastIndexOf(match[1]);
	const range = new vscode.Range(token.line, 0, token.endLine, token.endCharacter);
	return {
		name: match[1],
		type: 'macro',
		uri,
		range,
		selectionRange: new vscode.Range(token.line, nameStart, token.line, nameStart + match[1].length),
		signature: token.text.trim(),
		detail: token.text.trim(),
		declarationKind: 'macro',
		declarationRange: range,
		modifiers: [],
		parserBacked: true,
	};
}

function parseTypedefDeclarationAt(tokens: EnforceToken[], startIndex: number, source: SourceTextInfo, uri: vscode.Uri): ParsedDeclaration | undefined {
	const token = tokens[startIndex];
	if (token.text !== 'typedef' || !isDeclarationBoundaryBefore(tokens, startIndex)) {
		return undefined;
	}
	const declarationEnd = findDeclarationTerminator(tokens, startIndex);
	if (!declarationEnd || declarationEnd.delimiterToken.text !== ';') {
		return undefined;
	}
	const significantTokens = tokens
		.slice(startIndex + 1, declarationEnd.delimiterIndex)
		.filter(candidate => !isTrivia(candidate));
	const nameToken = [...significantTokens].reverse().find(isIdentifierLike);
	if (!nameToken || significantTokens.filter(isIdentifierLike).length < 2) {
		return undefined;
	}
	const signature = normalizeSourceText(source.text.slice(token.start, declarationEnd.delimiterToken.end));
	const symbol = createSymbolFromTokenSpan('class', nameToken.text, token, declarationEnd.delimiterToken, nameToken, signature, uri, signature);
	symbol.declarationKind = 'typedef';
	return { symbol, hasBody: false, delimiterIndex: declarationEnd.delimiterIndex };
}

function parseClassDeclarationAt(
	tokens: EnforceToken[],
	startIndex: number,
	source: SourceTextInfo,
	uri: vscode.Uri,
	pendingDecorators: EnforceDecorator[]
): { symbol: EnforceSymbol; pendingClassName?: string; nextIndex: number } | undefined {
	const classIndex = findClassKeywordIndex(tokens, startIndex);
	if (classIndex < 0) {
		return undefined;
	}

	const classToken = tokens[classIndex];
	if (tokens.slice(startIndex, classIndex).some(token => !isTrivia(token) && !isDeclarationModifier(token))) {
		return undefined;
	}

	const nameToken = nextSignificantToken(tokens, classIndex + 1);
	if (!nameToken || !isIdentifierLike(nameToken)) {
		return undefined;
	}

	const headerEndIndex = findHeaderEndIndex(tokens, classIndex);
	const headerEndToken = tokens[headerEndIndex] ?? nameToken;
	const signatureEnd = headerEndToken.text === '{' || headerEndToken.kind === 'newline'
		? previousSignificantToken(tokens, headerEndIndex - 1) ?? nameToken
		: headerEndToken;
	const signature = normalizeSourceText(source.text.slice(tokens[startIndex].start, signatureEnd.end));
	const symbol = createSymbolFromTokenSpan('class', nameToken.text, tokens[startIndex], signatureEnd, nameToken, signature, uri, signature);
	const baseClassName = parseClassBaseName(tokens, nameToken, headerEndIndex);
	if (hasInheritanceMarker(tokens, nameToken, headerEndIndex) && !baseClassName) {
		return undefined;
	}
	symbol.baseClassName = baseClassName;
	symbol.documentation = extractLeadingDocumentation(source.lines, classToken.line);
	symbol.declarationKind = tokensBetween(tokens, startIndex, classIndex).some(token => token.text === 'modded') ? 'moddedClass' : 'class';
	symbol.modifiers = collectDeclarationModifiersFromText(source.text.slice(tokens[startIndex].start, classToken.start));
	const bodyOpenIndex = headerEndToken.text === '{'
		? headerEndIndex
		: nextSignificantTokenIndex(tokens, headerEndIndex + 1);
	if (bodyOpenIndex >= 0 && tokens[bodyOpenIndex]?.text === '{') {
		symbol.bodyRange = createBodyRange(tokens, bodyOpenIndex);
	}
	applyPendingDecorators(symbol, pendingDecorators);

	return {
		symbol,
		pendingClassName: nameToken.text,
		nextIndex: headerEndToken.text === '{' ? headerEndIndex : headerEndIndex + 1,
	};
}

function parseEnumDeclarationAt(tokens: EnforceToken[], startIndex: number, source: SourceTextInfo, uri: vscode.Uri): EnforceSymbol | undefined {
	const token = tokens[startIndex];
	if (token.text !== 'enum') {
		return undefined;
	}

	const nameToken = nextSignificantToken(tokens, startIndex + 1);
	if (!nameToken || !isIdentifierLike(nameToken)) {
		return undefined;
	}

	const headerEndIndex = findHeaderEndIndex(tokens, startIndex);
	const headerEndToken = tokens[headerEndIndex] ?? nameToken;
	const signatureEnd = headerEndToken.text === '{' || headerEndToken.kind === 'newline'
		? previousSignificantToken(tokens, headerEndIndex - 1) ?? nameToken
		: headerEndToken;
	const signature = normalizeSourceText(source.text.slice(token.start, signatureEnd.end));
	const symbol = createSymbolFromTokenSpan('enum', nameToken.text, token, signatureEnd, nameToken, signature, uri, signature);
	symbol.documentation = extractLeadingDocumentation(source.lines, token.line);
	symbol.enumMembers = [];
	symbol.declarationKind = 'enum';
	symbol.modifiers = [];
	if (headerEndToken.text === '{') {
		symbol.bodyRange = createBodyRange(tokens, headerEndIndex);
	}
	Object.defineProperty(symbol, '__parserDepth', { value: currentBraceDepthBefore(tokens, startIndex), enumerable: false });
	return symbol;
}

function parseEnumMemberAt(
	tokens: EnforceToken[],
	startIndex: number,
	source: SourceTextInfo,
	uri: vscode.Uri,
	activeEnum: EnforceSymbol
): EnforceSymbol | undefined {
	const token = tokens[startIndex];
	if (!isIdentifierLike(token)) {
		return undefined;
	}

	const lineEndIndex = findLineEndIndex(tokens, startIndex);
	const declarationEndIndex = findEnumMemberDeclarationEndIndex(tokens, startIndex, lineEndIndex);
	if (declarationEndIndex < startIndex) {
		return undefined;
	}

	const display = formatEnumMemberDisplay(source.lines[token.line]?.slice(token.character) ?? token.text);
	const endToken = tokens[declarationEndIndex] ?? token;
	const symbol = createSymbolFromTokenSpan('enumValue', token.text, token, endToken, token, display, uri, display);
	symbol.containerName = activeEnum.name;
	symbol.detail = `${activeEnum.name}.${token.text}`;
	symbol.declarationKind = 'enumMember';
	symbol.modifiers = [];
	return symbol;
}

function parseFunctionOrPropertyAt(
	tokens: EnforceToken[],
	startIndex: number,
	source: SourceTextInfo,
	uri: vscode.Uri,
	containerName: string | undefined,
	pendingDecorators: EnforceDecorator[]
): ParsedDeclaration | undefined {
	const startToken = tokens[startIndex];
	if (!canStartDeclaration(startToken)) {
		return undefined;
	}
	if (!isDeclarationBoundaryBefore(tokens, startIndex)) {
		return undefined;
	}

	const declarationEnd = findDeclarationTerminator(tokens, startIndex);
	if (!declarationEnd) {
		return undefined;
	}

	const { delimiterIndex, delimiterToken } = declarationEnd;
	const signatureEndToken = delimiterToken.text === '{'
		? previousSignificantToken(tokens, delimiterIndex - 1) ?? delimiterToken
		: delimiterToken;
	const signature = normalizeSourceText(source.text.slice(startToken.start, signatureEndToken.end));
	const functionNameToken = getFunctionNameToken(tokens, startIndex, delimiterIndex, containerName);
	if (functionNameToken) {
		const type: EnforceSymbolType = containerName ? 'memberFunction' : 'function';
		const symbol = createSymbolFromTokenSpan(type, functionNameToken.name, startToken, signatureEndToken, functionNameToken.token, signature, uri, signature, functionNameToken.selectionStartToken);
		symbol.containerName = containerName;
		symbol.signature = signature;
		symbol.detail = containerName ? `${containerName}.${functionNameToken.name}` : functionNameToken.name;
		symbol.documentation = extractLeadingDocumentation(source.lines, startToken.line);
		symbol.declarationKind = getFunctionDeclarationKind(type, functionNameToken.name, containerName);
		symbol.modifiers = collectDeclarationModifiersFromText(source.text.slice(startToken.start, functionNameToken.token.start));
		if (delimiterToken.text === '{') {
			symbol.bodyRange = createBodyRange(tokens, delimiterIndex);
		}
		applyPendingDecorators(symbol, pendingDecorators);
		return { symbol, hasBody: delimiterToken.text === '{', delimiterIndex };
	}

	const propertyNameToken = getPropertyNameToken(tokens, startIndex, delimiterIndex);
	if (propertyNameToken && containerName && delimiterToken.text === ';') {
		const symbol = createSymbolFromTokenSpan('property', propertyNameToken.text, startToken, signatureEndToken, propertyNameToken, signature, uri, signature);
		symbol.containerName = containerName;
		symbol.signature = signature;
		symbol.detail = `${containerName}.${propertyNameToken.text}`;
		symbol.documentation = extractLeadingDocumentation(source.lines, startToken.line);
		symbol.declarationKind = 'property';
		symbol.modifiers = collectDeclarationModifiersFromText(source.text.slice(startToken.start, propertyNameToken.start));
		applyPendingDecorators(symbol, pendingDecorators);
		return { symbol, hasBody: false, delimiterIndex };
	}

	return undefined;
}

function parseDecoratorAt(tokens: EnforceToken[], startIndex: number, text: string): { decorators: EnforceDecorator[]; endIndex: number } | undefined {
	let depth = 0;
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.text === '[') {
			depth++;
		} else if (token.text === ']') {
			depth--;
			if (depth === 0) {
				const rawContent = text.slice(tokens[startIndex].end, token.start);
				return {
					decorators: parseDecoratorsFromContent(rawContent),
					endIndex: index,
				};
			}
		}
	}

	return undefined;
}

function parseDecoratorsFromContent(content: string): EnforceDecorator[] {
	const decorators: EnforceDecorator[] = [];
	for (const item of splitTopLevel(content)) {
		const trimmed = normalizeSourceText(item);
		const match = /^([A-Za-z_]\w*)\s*(?:\((.*)\))?$/.exec(trimmed);
		if (match?.[1]) {
			decorators.push({
				name: match[1],
				arguments: match[2]?.trim(),
			});
		}
	}
	return decorators;
}

function findClassKeywordIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (isTrivia(token)) {
			continue;
		}
		if (token.text === 'class') {
			return index;
		}
		if (!isDeclarationModifier(token)) {
			return -1;
		}
	}
	return -1;
}

function parseClassBaseName(tokens: EnforceToken[], nameToken: EnforceToken, headerEndIndex: number): string | undefined {
	for (let index = tokenIndexAfter(tokens, nameToken); index < headerEndIndex; index++) {
		const token = tokens[index];
		if (token.text !== ':' && token.text !== 'extends') {
			continue;
		}

		const baseStart = nextSignificantToken(tokens, index + 1);
		if (!baseStart || !isIdentifierLike(baseStart)) {
			return undefined;
		}

		const baseEndIndex = findTypeExpressionEndIndex(tokens, tokenIndexAfter(tokens, baseStart) - 1, headerEndIndex);
		const baseEnd = tokens[baseEndIndex] ?? baseStart;
		return normalizeSourceText(tokens[0] ? tokensText(tokens, baseStart, baseEnd) : baseStart.text);
	}

	return undefined;
}

function hasInheritanceMarker(tokens: EnforceToken[], nameToken: EnforceToken, headerEndIndex: number): boolean {
	for (let index = tokenIndexAfter(tokens, nameToken); index < headerEndIndex; index++) {
		const token = tokens[index];
		if (token.text === ':' || token.text === 'extends') {
			return true;
		}
	}
	return false;
}

function getFunctionNameToken(tokens: EnforceToken[], startIndex: number, delimiterIndex: number, currentClassName?: string): { token: EnforceToken; selectionStartToken: EnforceToken; name: string } | undefined {
	const openParenIndex = findTopLevelTokenIndex(tokens, startIndex, delimiterIndex, '(');
	if (openParenIndex < 0) {
		return undefined;
	}

	const beforeParen = significantTokensBetween(tokens, startIndex, openParenIndex);
	if (beforeParen.some(token => isExpressionTokenBeforeFunctionName(token))) {
		return undefined;
	}

	const nameToken = previousSignificantToken(tokens, openParenIndex - 1);
	if (!nameToken || !isIdentifierLike(nameToken) || ignoredFunctionNames.has(nameToken.text)) {
		return undefined;
	}
	const tokenBeforeName = previousSignificantToken(tokens, tokenIndexAfter(tokens, nameToken) - 2);
	const name = tokenBeforeName?.text === '~' ? `~${nameToken.text}` : nameToken.text;
	if (!/^~?[A-Za-z_]\w*$/.test(name)) {
		return undefined;
	}

	const filteredParts = beforeParen.filter(token => !isDeclarationModifier(token));
	if (filteredParts.slice(0, -1).some(token => ignoredPropertyNames.has(token.text))) {
		return undefined;
	}

	if (filteredParts.length < 2 && name !== currentClassName && name !== `~${currentClassName}`) {
		return undefined;
	}

	return { token: nameToken, selectionStartToken: tokenBeforeName?.text === '~' ? tokenBeforeName : nameToken, name };
}

function getPropertyNameToken(tokens: EnforceToken[], startIndex: number, delimiterIndex: number): EnforceToken | undefined {
	const assignmentIndex = findTopLevelTokenIndex(tokens, startIndex, delimiterIndex, '=');
	const endIndex = assignmentIndex >= 0 ? assignmentIndex : delimiterIndex;
	if (findTopLevelTokenIndex(tokens, startIndex, endIndex, '(') >= 0) {
		return undefined;
	}
	const nameToken = getDeclarationNameBeforeEnd(tokens, startIndex, endIndex);
	if (!nameToken || !isIdentifierLike(nameToken) || ignoredPropertyNames.has(nameToken.text)) {
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
	let initializerBraces = 0;
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'eof') {
			return undefined;
		}
		if (token.kind === 'comment' || token.kind === 'string') {
			continue;
		}
		if (initializerBraces > 0) {
			if (token.text === '{') {
				initializerBraces++;
			} else if (token.text === '}') {
				initializerBraces = Math.max(0, initializerBraces - 1);
			}
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
		} else if (token.text === '{' && parens === 0 && brackets === 0 && angles === 0 && previousSignificantToken(tokens, index - 1)?.text === '=') {
			initializerBraces++;
		} else if ((token.text === ';' || token.text === '{') && parens === 0 && brackets === 0 && angles === 0) {
			return { delimiterIndex: index, delimiterToken: token };
		}
	}
	return undefined;
}

function findHeaderEndIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.text === '{' || token.text === ';' || token.kind === 'newline' || token.kind === 'eof') {
			return index;
		}
	}
	return startIndex;
}

function nextTokenAfterDeclarationHeader(tokens: EnforceToken[], startIndex: number): number {
	const endIndex = findHeaderEndIndex(tokens, startIndex);
	return tokens[endIndex]?.text === '{' ? endIndex : endIndex + 1;
}

function nextEnumMemberIndex(tokens: EnforceToken[], startIndex: number): number {
	const lineEnd = findLineEndIndex(tokens, startIndex);
	return lineEnd + 1;
}

function findLineEndIndex(tokens: EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index < tokens.length; index++) {
		if (tokens[index].kind === 'newline' || tokens[index].kind === 'eof') {
			return index;
		}
	}
	return tokens.length - 1;
}

function findEnumMemberDeclarationEndIndex(tokens: EnforceToken[], startIndex: number, lineEndIndex: number): number {
	let endIndex = startIndex;
	for (let index = startIndex + 1; index < lineEndIndex; index++) {
		const token = tokens[index];
		if (token.kind === 'comment') {
			break;
		}
		if (token.text === ',' || token.text === '=') {
			endIndex = token.text === ',' ? index : Math.max(startIndex, index - 1);
			break;
		}
		if (!isTrivia(token)) {
			endIndex = index;
		}
	}
	return endIndex;
}

function formatEnumMemberDisplay(lineText: string): string {
	return lineText.trim();
}

function findTypeExpressionEndIndex(tokens: EnforceToken[], startIndex: number, stopIndex: number): number {
	let angles = 0;
	let endIndex = startIndex;
	for (let index = startIndex; index < stopIndex; index++) {
		const token = tokens[index];
		if (isTrivia(token)) {
			continue;
		}
		if (token.text === '<') {
			angles++;
		} else if (token.text === '>>') {
			angles = Math.max(0, angles - 2);
		} else if (token.text === '>') {
			angles = Math.max(0, angles - 1);
		} else if ((token.text === '{' || token.text === ';' || token.text === ',' || token.text === '=') && angles === 0) {
			break;
		}
		endIndex = index;
	}
	return endIndex;
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

function tokensBetween(tokens: EnforceToken[], startIndex: number, stopIndex: number): EnforceToken[] {
	return tokens.slice(startIndex, Math.max(startIndex, stopIndex));
}

function collectDeclarationModifiersFromText(value: string): string[] {
	const modifiers = Array.from(value.matchAll(/\b[A-Za-z_]\w*\b/g))
		.map(match => match[0])
		.filter(word => declarationModifiers.has(word));
	return [...new Set(modifiers)];
}

function getFunctionDeclarationKind(type: EnforceSymbolType, name: string, containerName?: string): EnforceDeclarationKind {
	if (containerName && name === containerName) {
		return 'constructor';
	}
	if (containerName && name === `~${containerName}`) {
		return 'destructor';
	}
	return type === 'memberFunction' ? 'memberFunction' : 'function';
}

function createBodyRange(tokens: EnforceToken[], openBraceIndex: number): vscode.Range {
	const openBrace = tokens[openBraceIndex];
	const closeBrace = findMatchingBraceToken(tokens, openBraceIndex) ?? openBrace;
	return new vscode.Range(openBrace.line, openBrace.character, closeBrace.endLine, closeBrace.endCharacter);
}

function findMatchingBraceToken(tokens: EnforceToken[], openBraceIndex: number): EnforceToken | undefined {
	let depth = 0;
	for (let index = openBraceIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === 'string' || token.kind === 'comment') {
			continue;
		}
		if (token.text === '{') {
			depth++;
		} else if (token.text === '}') {
			depth--;
			if (depth === 0) {
				return token;
			}
		}
	}
	return undefined;
}

function applyPendingDecorators(symbol: EnforceSymbol, pendingDecorators: EnforceDecorator[]): void {
	if (pendingDecorators.length === 0) {
		return;
	}

	symbol.decoratorDetails = pendingDecorators.map(decorator => ({ ...decorator }));
	symbol.decorators = [...new Set(pendingDecorators.map(decorator => decorator.name))];
	pendingDecorators.length = 0;
}

function createSymbolFromTokenSpan(
	type: EnforceSymbolType,
	name: string,
	startToken: EnforceToken,
	endToken: EnforceToken,
	nameToken: EnforceToken,
	signature: string,
	uri: vscode.Uri,
	detail: string,
	selectionStartToken = nameToken
): EnforceSymbol {
	const range = new vscode.Range(startToken.line, 0, endToken.endLine, endToken.endCharacter);
	return {
		name,
		type,
		uri,
		range,
		selectionRange: new vscode.Range(selectionStartToken.line, selectionStartToken.character, nameToken.endLine, nameToken.endCharacter),
		signature,
		detail,
		declarationRange: range,
		parserBacked: true,
	};
}

function extractLeadingDocumentation(lines: string[], lineNumber: number): string | undefined {
	const docs: string[] = [];
	let inBlockDoc = false;
	for (let index = lineNumber - 1; index >= 0; index--) {
		const rawLine = lines[index];
		const line = rawLine.trim();

		if (inBlockDoc) {
			const parsed = parseBlockDocumentationLine(rawLine);
			if (parsed) {
				docs.unshift(parsed);
			}
			if (/^\/\*!?/.test(line)) {
				inBlockDoc = false;
				continue;
			}
			continue;
		}

		if (!line) {
			if (docs.length > 0) {
				break;
			}
			continue;
		}

		if (/\*\/$/.test(line)) {
			inBlockDoc = true;
			const parsed = parseBlockDocumentationLine(rawLine);
			if (parsed) {
				docs.unshift(parsed);
			}
			if (/^\/\*!?/.test(line)) {
				inBlockDoc = false;
			}
			continue;
		}

		const doc = parseDocumentationLine(line);
		if (doc === undefined) {
			break;
		}

		if (doc) {
			docs.unshift(doc);
		}
	}

	return docs.length > 0 ? formatDocumentation(docs) : undefined;
}

function parseBlockDocumentationLine(line: string): string {
	return line
		.replace(/^\s*\/\*!?/, '')
		.replace(/\*\/\s*$/, '')
		.replace(/^\s*\*\s?/, '')
		.replace(/\s+$/, '');
}

function parseDocumentationLine(line: string): string | undefined {
	if (/^\/\/[-=\s]+$/.test(line)) {
		return '';
	}

	const docMatch = /^(?:\/\/!|\/\/\/|\/\/)\s?(.*)$/.exec(line);
	if (docMatch) {
		return docMatch[1].trim();
	}

	return undefined;
}

function formatDocumentation(lines: string[]): string {
	const normalizedLines = lines
		.filter(line => !/^\\(?:addtogroup|ingroup|defgroup)\b/.test(line.trim()))
		.filter(line => !/^\\[{}]$/.test(line.trim()))
		.join('\n')
		.replace(/\s+(\\(?:brief|param|return|returns|see|note|warning|code|endcode)\b)/g, '\n$1')
		.split('\n');
	const sections: string[] = [];
	let codeLines: string[] = [];
	let inCodeBlock = false;
	for (const rawLine of normalizedLines) {
		const line = rawLine.trim();
		if (/^\\code\b/.test(line)) {
			inCodeBlock = true;
			codeLines = [];
			continue;
		}
		if (/^\\endcode\b/.test(line)) {
			sections.push(`\`\`\`enforce-hover\n${normalizeDocumentationCodeBlock(codeLines).join('\n')}\n\`\`\``);
			inCodeBlock = false;
			codeLines = [];
			continue;
		}
		if (inCodeBlock) {
			codeLines.push(rawLine);
			continue;
		}
		const formatted = line
			.replace(/^\\brief\s+/g, '')
			.replace(/^\\param\[(in|out|inout)\]\s+/g, '`@$1` ')
			.replace(/^\\param\s+/g, '`@param` ')
			.replace(/^\\return[s]?\s+/g, '`@return` ')
			.replace(/^\\(see|note|warning)\s+/g, '`@$1` ');
		if (formatted) {
			sections.push(formatted);
		}
	}
	if (inCodeBlock && codeLines.length > 0) {
		sections.push(`\`\`\`enforce-hover\n${normalizeDocumentationCodeBlock(codeLines).join('\n')}\n\`\`\``);
	}
	return sections.join('\n\n');
}

function normalizeDocumentationCodeBlock(lines: string[]): string[] {
	const trimmed = trimBlankEdgeLines(lines);
	const indentation = Math.min(
		...trimmed
			.filter(line => line.trim() !== '')
			.map(line => line.match(/^\s*/)?.[0].length ?? 0)
	);
	return Number.isFinite(indentation) && indentation > 0
		? trimmed.map(line => line.slice(indentation))
		: trimmed;
}

function trimBlankEdgeLines(lines: string[]): string[] {
	let start = 0;
	while (start < lines.length && lines[start].trim() === '') {
		start++;
	}
	let end = lines.length;
	while (end > 0 && lines[end - 1].trim() === '') {
		end--;
	}
	return lines.slice(start, end);
}

function canStartDeclaration(token: EnforceToken): boolean {
	return isIdentifierLike(token) || isDeclarationModifier(token) || token.text === '~';
}

function isIdentifierLike(token: EnforceToken): boolean {
	return token.kind === 'identifier' || token.kind === 'keyword';
}

function isDeclarationModifier(token: EnforceToken): boolean {
	return declarationModifiers.has(token.text);
}

function isExpressionTokenBeforeFunctionName(token: EnforceToken): boolean {
	return ['=', '+', '-', '*', '/', '%', '!', '?', '|', '&', '.', '[', ']'].includes(token.text);
}

function isIgnorableDecoratorGap(token: EnforceToken): boolean {
	return isTrivia(token) || token.kind === 'preprocessor';
}

function symbolDepth(symbol: EnforceSymbol): number {
	return (symbol as EnforceSymbol & { __parserDepth?: number }).__parserDepth ?? 0;
}

function currentBraceDepthBefore(tokens: EnforceToken[], stopIndex: number): number {
	let depth = 0;
	for (let index = 0; index < stopIndex; index++) {
		if (tokens[index].text === '{') {
			depth++;
		} else if (tokens[index].text === '}') {
			depth = Math.max(0, depth - 1);
		}
	}
	return depth;
}
