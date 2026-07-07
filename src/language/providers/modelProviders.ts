import * as vscode from 'vscode';
import { tracePerformance } from '../../core/performanceTrace';
import { EnforceSymbolIndex } from '../index/symbolIndex';
import { buildCodeIntelligenceModel, CodeReferenceKind } from '../model/codeIntelligence';
import {
	buildLanguageModel,
	typeKeywords,
} from '../model/languageModel';
import type { EnforceParserRange, EnforceSyntaxNode, ParsedEnforceSource } from '../parser/ast';
import { getParsedDocument, toVscodeRange } from '../parser/documentCache';
import type { EnforceToken } from '../parser/tokens';

export const modelSemanticTokenTypes = ['class', 'enum', 'function', 'variable', 'type', 'keyword', 'comment', 'string', 'punctuation', 'preprocessor'];
export const modelSemanticTokenModifiers = ['declaration', 'static', 'readonly', 'defaultLibrary', 'modded', 'override', 'proto', 'native', 'external'];
export const modelSemanticTokenLegend = new vscode.SemanticTokensLegend(modelSemanticTokenTypes, modelSemanticTokenModifiers);

const declarationModifiers = new Set([
	'autoptr', 'const', 'event', 'external', 'inout', 'modded', 'native', 'notnull', 'out', 'owned',
	'override', 'private', 'protected', 'proto', 'public', 'ref', 'sealed', 'static', 'volatile',
]);
const scalarKeywordTypes = new Set(['void', 'bool', 'int', 'float']);
const valueTypeKeywords = new Set(['string']);
const containerClassTypeKeywords = new Set(['array', 'set', 'map']);

export class ModelHoverProvider implements vscode.HoverProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	async provideHover(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Hover | undefined> {
		await prepareNavigationIndex(this.symbolIndex, document);
		const code = buildCodeIntelligenceModel(document, this.symbolIndex);
		const identity = code.resolveIdentityAt(position);
		if (!identity || identity.confidence !== 'high') {
			return undefined;
		}
		return new vscode.Hover(code.formatHover(identity), toVscodeRange(identity.range));
	}
}

export class ModelDefinitionProvider implements vscode.DefinitionProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	async provideDefinition(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Definition | undefined> {
		await prepareNavigationIndex(this.symbolIndex, document);
		const code = buildCodeIntelligenceModel(document, this.symbolIndex);
		const identity = code.resolveIdentityAt(position);
		if (!identity) {
			return undefined;
		}
		const locations = code.resolveDefinition(identity);
		if (locations.length === 0) {
			return undefined;
		}
		return locations.length === 1 ? locations[0] : locations;
	}
}

export class ModelReferenceProvider implements vscode.ReferenceProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	async provideReferences(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location[]> {
		await prepareNavigationIndex(this.symbolIndex, document);
		const code = buildCodeIntelligenceModel(document, this.symbolIndex);
		const identity = code.resolveIdentityAt(position);
		return identity ? code.resolveReferences(identity).map(reference => reference.location) : [];
	}
}

export class ModelRenameProvider implements vscode.RenameProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	async provideRenameEdits(document: vscode.TextDocument, position: vscode.Position, newName: string): Promise<vscode.WorkspaceEdit | undefined> {
		await prepareNavigationIndex(this.symbolIndex, document);
		const code = buildCodeIntelligenceModel(document, this.symbolIndex);
		const identity = code.resolveIdentityAt(position);
		if (!identity || identity.confidence !== 'high' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(newName)) {
			return undefined;
		}
		const references = code.resolveReferences(identity);
		if (references.length === 0) {
			return undefined;
		}
		const edit = new vscode.WorkspaceEdit();
		for (const reference of references) {
			edit.replace(reference.location.uri, reference.location.range, newName);
		}
		return edit;
	}
}

export class ModelDocumentHighlightProvider implements vscode.DocumentHighlightProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	async provideDocumentHighlights(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.DocumentHighlight[]> {
		await prepareNavigationIndex(this.symbolIndex, document);
		const code = buildCodeIntelligenceModel(document, this.symbolIndex);
		const identity = code.resolveIdentityAt(position);
		return identity
			? code.resolveReferences(identity).map(reference => new vscode.DocumentHighlight(reference.location.range, toHighlightKind(reference.kind)))
			: [];
	}
}

async function prepareNavigationIndex(symbolIndex: EnforceSymbolIndex, document: vscode.TextDocument): Promise<void> {
	await symbolIndex.ensureGameDataIndex();
	await symbolIndex.ensureDocumentIndexCurrent(document);
}

export class ModelDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	provideDocumentSymbols(document: vscode.TextDocument): vscode.DocumentSymbol[] {
		return this.symbolIndex.getDocumentSymbols(document.uri).map(symbol => new vscode.DocumentSymbol(
			symbol.name,
			symbol.signature ?? symbol.detail ?? '',
			toSymbolKind(symbol.type),
			symbol.range,
			symbol.selectionRange
		));
	}
}

export class ModelWorkspaceSymbolProvider implements vscode.WorkspaceSymbolProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	provideWorkspaceSymbols(query: string): vscode.SymbolInformation[] {
		const normalized = query.toLowerCase();
		return this.symbolIndex.getAllSymbols()
			.filter(symbol => symbol.name.toLowerCase().includes(normalized))
			.slice(0, 200)
			.map(symbol => new vscode.SymbolInformation(symbol.name, toSymbolKind(symbol.type), symbol.containerName ?? '', new vscode.Location(symbol.uri, symbol.selectionRange)));
	}
}

export class ModelSemanticTokensProvider implements vscode.DocumentSemanticTokensProvider {
	constructor(private readonly symbolIndex: EnforceSymbolIndex) {}

	provideDocumentSemanticTokens(document: vscode.TextDocument): vscode.SemanticTokens {
		return tracePerformance(
			'model.semanticTokens',
			`${shortFileName(document)} | lines=${document.lineCount}`,
			() => {
				const builder = new vscode.SemanticTokensBuilder(modelSemanticTokenLegend);
				const tokens: SemanticToken[] = [];
				const parsed = getParsedDocument(document);
				const typeTokenKeys = collectTypeTokenKeys(parsed);
				const genericParameterTokenKeys = collectGenericParameterTokenKeys(parsed);
				const declarationTokenTypes = collectDeclarationTokenTypes(parsed);
				const modelReferenceTokenTypes = collectModelReferenceTokenTypes(document, parsed, this.symbolIndex);
				const decoratorClassNames = collectDecoratorClassNames(parsed, this.symbolIndex);
				const indexedTypeTokenTypes = collectIndexedTypeTokenTypes(document, parsed, this.symbolIndex, typeTokenKeys);
				for (let index = 0; index < parsed.tokens.length; index++) {
					tokens.push(...semanticTokensForParserToken(parsed.tokens, index, typeTokenKeys, genericParameterTokenKeys, declarationTokenTypes, modelReferenceTokenTypes, decoratorClassNames, indexedTypeTokenTypes));
				}
				for (const symbol of this.symbolIndex.getDocumentSymbols(document.uri)) {
					if (symbol.declarationKind === 'constructor' || symbol.declarationKind === 'destructor') {
						continue;
					}
					tokens.push({
						line: symbol.selectionRange.start.line,
						character: symbol.selectionRange.start.character,
						length: Math.max(1, symbol.selectionRange.end.character - symbol.selectionRange.start.character),
						type: semanticTypeForSymbol(symbol.type),
						modifiers: symbol.modifiers ?? [],
					});
				}
				for (const token of dedupeSemanticTokens(tokens)) {
					const typeIndex = modelSemanticTokenTypes.indexOf(token.type);
					if (typeIndex >= 0) {
						builder.push(token.line, token.character, token.length, typeIndex, encodeModifiers(token.modifiers));
					}
				}
				return builder.build();
			}
		);
	}
}

export function getModelDiagnostics(document: vscode.TextDocument, symbolIndex: EnforceSymbolIndex): vscode.Diagnostic[] {
	const model = buildLanguageModel(document, symbolIndex);
	return model.parsed.diagnostics.map(diagnostic => new vscode.Diagnostic(
		toVscodeRange(diagnostic.range),
		diagnostic.message,
		diagnostic.severity === 'error' ? vscode.DiagnosticSeverity.Error : diagnostic.severity === 'warning' ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Information
	));
}

export function formatModelInspection(document: vscode.TextDocument, position: vscode.Position, symbolIndex: EnforceSymbolIndex): string {
	const startedAt = performance.now();
	const model = buildLanguageModel(document, symbolIndex);
	const context = model.contextAt(position);
	const code = buildCodeIntelligenceModel(document, symbolIndex);
	const symbol = code.resolveIdentityAt(position);
	const classNode = model.currentClass(position);
	const functionNode = model.currentFunction(position);
	const locals = model.visibleLocals(position);
	const expected = model.expectedType(position);
	const elapsed = performance.now() - startedAt;
	return [
		`file=${document.uri.fsPath}`,
		`position=${position.line + 1}:${position.character + 1}`,
		`context=${context.kind} prefix=${context.prefix} receiver=${context.receiver ?? ''}`,
		`class=${classNode?.name ?? ''}`,
		`function=${functionNode?.signature ?? functionNode?.name ?? ''}`,
		`symbol=${symbol ? `${symbol.kind}:${symbol.name}` : ''}`,
		`expected=${expected.context}:${expected.valueType ?? ''}`,
		`locals=${locals.map(local => `${local.valueType ?? 'var'} ${local.name}`).join(', ')}`,
		`diagnostics=${model.parsed.diagnostics.length}`,
		`elapsedMs=${elapsed.toFixed(2)}`,
	].join('\n');
}

function toSymbolKind(type: string): vscode.SymbolKind {
	switch (type) {
		case 'class': return vscode.SymbolKind.Class;
		case 'enum': return vscode.SymbolKind.Enum;
		case 'enumValue': return vscode.SymbolKind.EnumMember;
		case 'function':
		case 'memberFunction': return vscode.SymbolKind.Function;
		case 'property': return vscode.SymbolKind.Property;
		case 'macro': return vscode.SymbolKind.Constant;
		default: return vscode.SymbolKind.Variable;
	}
}

function semanticTypeForSymbol(type: string): string {
	switch (type) {
		case 'class': return 'class';
		case 'enum': return 'enum';
		case 'function':
		case 'memberFunction': return 'function';
		case 'property': return 'variable';
		case 'enumValue': return 'variable';
		default: return typeKeywords.includes(type) ? 'type' : 'variable';
	}
}

function toHighlightKind(kind: CodeReferenceKind): vscode.DocumentHighlightKind {
	switch (kind) {
		case 'write':
			return vscode.DocumentHighlightKind.Write;
		case 'declaration':
		case 'read':
		case 'typeUsage':
			return vscode.DocumentHighlightKind.Read;
		default:
			return vscode.DocumentHighlightKind.Text;
	}
}

function semanticTokensForParserToken(tokens: readonly EnforceToken[], index: number, typeTokenKeys: ReadonlySet<string>, genericParameterTokenKeys: ReadonlySet<string>, declarationTokenTypes: ReadonlyMap<string, string>, modelReferenceTokenTypes: ReadonlyMap<string, string>, decoratorClassNames: ReadonlySet<string>, indexedTypeTokenTypes: ReadonlyMap<string, string>): SemanticToken[] {
	const token = tokens[index];
	if (token.kind === 'preprocessor') {
		return semanticTokensForPreprocessor(token);
	}
	const type = semanticTypeForParserToken(tokens, index, typeTokenKeys, genericParameterTokenKeys, declarationTokenTypes, modelReferenceTokenTypes, decoratorClassNames, indexedTypeTokenTypes);
	if (!type) {
		return [];
	}
	return splitTokenLines(token, type);
}

function semanticTypeForParserToken(tokens: readonly EnforceToken[], index: number, typeTokenKeys: ReadonlySet<string>, genericParameterTokenKeys: ReadonlySet<string>, declarationTokenTypes: ReadonlyMap<string, string>, modelReferenceTokenTypes: ReadonlyMap<string, string>, decoratorClassNames: ReadonlySet<string>, indexedTypeTokenTypes: ReadonlyMap<string, string>): string | undefined {
	const token = tokens[index];
	const declarationType = declarationTokenTypes.get(tokenKey(token));
	const modelReferenceType = modelReferenceTokenTypes.get(tokenKey(token));
	const indexedType = indexedTypeTokenTypes.get(tokenKey(token));
	if (declarationType) {
		return declarationType;
	}
	switch (token.kind) {
		case 'comment': return 'comment';
		case 'string': return 'string';
		case 'preprocessor': return undefined;
		case 'keyword': return 'keyword';
		case 'operator':
		case 'punctuation': return decoratorCallBracketType(tokens, index, decoratorClassNames) ?? (isBracketToken(token) ? undefined : 'punctuation');
		case 'number': return 'variable';
		case 'identifier':
			if (valueTypeKeywords.has(token.text)) {
				return 'type';
			}
			if (containerClassTypeKeywords.has(token.text)) {
				return 'class';
			}
			if (genericParameterTokenKeys.has(tokenKey(token)) || (typeTokenKeys.has(tokenKey(token)) && isGenericTypeParameterName(token.text))) {
				return 'keyword';
			}
			if (indexedType && !scalarKeywordTypes.has(token.text)) {
				return indexedType;
			}
			if (modelReferenceType) {
				return modelReferenceType;
			}
			if (typeKeywords.includes(token.text)) {
				return 'keyword';
			}
			if (isAttributeName(tokens, index)) {
				return decoratorClassNames.has(token.text) ? 'class' : 'variable';
			}
			if (typeTokenKeys.has(tokenKey(token))) {
				return isLikelyTypeIdentifier(token.text) || indexedType ? 'type' : 'variable';
			}
			if (nextSignificantToken(tokens, index + 1)?.text === '(') {
				return 'function';
			}
			return 'variable';
		default: return undefined;
	}
}

function semanticTokensForPreprocessor(token: EnforceToken): SemanticToken[] {
	const tokens: SemanticToken[] = [];
	const commentStart = findLineCommentStart(token.text);
	const directiveText = commentStart >= 0 ? token.text.slice(0, commentStart) : token.text;
	const commentText = commentStart >= 0 ? token.text.slice(commentStart) : undefined;
	const directiveMatch = /^\s*#\s*[A-Za-z_]\w*/.exec(directiveText);
	if (directiveMatch) {
		const start = directiveMatch[0].search(/#/);
		tokens.push({
			line: token.line,
			character: token.character + start,
			length: directiveMatch[0].length - start,
			type: 'preprocessor',
			modifiers: [],
		});
	}
	for (const match of directiveText.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) {
		if (directiveMatch && match.index !== undefined && match.index < directiveMatch[0].length) {
			continue;
		}
		tokens.push({
			line: token.line,
			character: token.character + (match.index ?? 0),
			length: match[0].length,
			type: 'variable',
			modifiers: [],
		});
	}
	if (commentStart >= 0 && commentText) {
		tokens.push({
			line: token.line,
			character: token.character + commentStart,
			length: commentText.length,
			type: 'comment',
			modifiers: [],
		});
	}
	return tokens;
}

function findLineCommentStart(text: string): number {
	let quote: '"' | "'" | undefined;
	let escaped = false;
	for (let index = 0; index < text.length - 1; index++) {
		const char = text[index];
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
			continue;
		}
		if (char === '/' && text[index + 1] === '/') {
			return index;
		}
	}
	return -1;
}

function collectDecoratorClassNames(parsed: ParsedEnforceSource, symbolIndex: EnforceSymbolIndex): Set<string> {
	const result = new Set<string>();
	const attributeNames = new Set<string>();
	for (let index = 0; index < parsed.tokens.length; index++) {
		const token = parsed.tokens[index];
		if (token.kind === 'identifier' && isAttributeName(parsed.tokens, index)) {
			attributeNames.add(token.text);
		}
	}
	for (const name of attributeNames) {
		if (isDecoratorClass(name, symbolIndex)) {
			result.add(name);
		}
	}
	return result;
}

function collectDeclarationTokenTypes(parsed: ParsedEnforceSource): Map<string, string> {
	const result = new Map<string, string>();
	for (const node of parsed.nodes) {
		if (!node.selectionRange) {
			continue;
		}
		if (node.kind === 'constructor' || node.kind === 'destructor') {
			for (const token of tokensInParserRange(parsed.tokens, node.selectionRange)) {
				if (token.kind === 'identifier' && token.text === constructorClassName(node)) {
					result.set(tokenKey(token), 'class');
				}
			}
			continue;
		}
		const type = semanticTypeForDeclarationNode(node);
		if (!type) {
			continue;
		}
		for (const token of tokensInParserRange(parsed.tokens, node.selectionRange)) {
			result.set(tokenKey(token), type);
		}
	}
	return result;
}

function semanticTypeForDeclarationNode(node: EnforceSyntaxNode): string | undefined {
	switch (node.kind) {
		case 'class': return 'class';
		case 'enum': return 'enum';
		case 'function':
		case 'memberFunction': return 'function';
		case 'property':
		case 'enumMember':
		case 'macro':
		case 'parameter':
		case 'local':
		case 'foreach': return 'variable';
		default: return undefined;
	}
}

function constructorClassName(node: EnforceSyntaxNode): string | undefined {
	return node.containerName ?? node.name?.replace(/^~/, '');
}

function collectModelReferenceTokenTypes(document: vscode.TextDocument, parsed: ParsedEnforceSource, symbolIndex: EnforceSymbolIndex): Map<string, string> {
	const result = new Map<string, string>();
	let code: ReturnType<typeof buildCodeIntelligenceModel> | undefined;
	for (let index = 0; index < parsed.tokens.length; index++) {
		const token = parsed.tokens[index];
		if (token.kind !== 'identifier') {
			continue;
		}
		const previous = previousSignificantToken(parsed.tokens, index - 1);
		const next = nextSignificantToken(parsed.tokens, index + 1);
		if (!isBareTypeReferenceContext(previous, next)) {
			continue;
		}
		code ??= buildCodeIntelligenceModel(document, symbolIndex);
		const identity = code.resolveIdentityAt(new vscode.Position(token.line, token.character));
		if (identity?.symbol?.type === 'memberFunction' || identity?.symbol?.type === 'function') {
			result.set(tokenKey(token), 'function');
		}
	}
	return result;
}

function isDecoratorClass(name: string, symbolIndex: EnforceSymbolIndex): boolean {
	return Boolean(symbolIndex.getClassSymbol(name));
}

function collectIndexedTypeTokenTypes(document: vscode.TextDocument, parsed: ParsedEnforceSource, symbolIndex: EnforceSymbolIndex, typeTokenKeys: ReadonlySet<string>): Map<string, string> {
	const result = new Map<string, string>();
	const enumNames = new Set(symbolIndex.getEnumSymbols().map(symbol => symbol.name));
	let code: ReturnType<typeof buildCodeIntelligenceModel> | undefined;
	for (let index = 0; index < parsed.tokens.length; index++) {
		const token = parsed.tokens[index];
		if (token.kind !== 'identifier') {
			continue;
		}
		const previous = previousSignificantToken(parsed.tokens, index - 1);
		const next = nextSignificantToken(parsed.tokens, index + 1);
		const isTypeReceiver = next?.text === '.' || next?.text === '::';
		const isConstructedType = previous?.text === 'new';
		const isDeclaredType = typeTokenKeys.has(tokenKey(token));
		const isBareTypeReference = isBareTypeReferenceContext(previous, next);
		if (!isTypeReceiver && !isConstructedType && !isDeclaredType && !isBareTypeReference) {
			continue;
		}
		if (symbolIndex.getClassSymbol(token.text)) {
			if (isBareTypeReference) {
				code ??= buildCodeIntelligenceModel(document, symbolIndex);
				if (code.resolveIdentityAt(new vscode.Position(token.line, token.character))?.symbol?.type !== 'class') {
					continue;
				}
			}
			result.set(tokenKey(token), 'class');
		} else if (enumNames.has(token.text)) {
			if (isBareTypeReference) {
				code ??= buildCodeIntelligenceModel(document, symbolIndex);
				if (code.resolveIdentityAt(new vscode.Position(token.line, token.character))?.symbol?.type !== 'enum') {
					continue;
				}
			}
			result.set(tokenKey(token), 'enum');
		}
	}
	return result;
}

function isBareTypeReferenceContext(previous: EnforceToken | undefined, next: EnforceToken | undefined): boolean {
	return previous !== undefined
		&& next !== undefined
		&& ['(', ','].includes(previous.text)
		&& [')', ',', ';'].includes(next.text);
}

function isAttributeName(tokens: readonly EnforceToken[], index: number): boolean {
	const previousIndex = previousSignificantTokenIndex(tokens, index - 1);
	const previous = previousIndex >= 0 ? tokens[previousIndex] : undefined;
	const next = nextSignificantToken(tokens, index + 1);
	if (previous?.text !== '[' || (next?.text !== '(' && next?.text !== ']')) {
		return false;
	}
	const beforeBracket = previousSignificantToken(tokens, previousIndex - 1);
	return beforeBracket === undefined
		|| beforeBracket.line !== previous.line
		|| ['{', ';'].includes(beforeBracket.text);
}

function decoratorCallBracketType(tokens: readonly EnforceToken[], index: number, decoratorClassNames: ReadonlySet<string>): string | undefined {
	const token = tokens[index];
	if (token.text === '(') {
		const previousIndex = previousSignificantTokenIndex(tokens, index - 1);
		const previous = previousIndex >= 0 ? tokens[previousIndex] : undefined;
		return previous && isAttributeName(tokens, previousIndex) && (decoratorClassNames.has(previous.text) || isLikelyTypeIdentifier(previous.text)) ? 'class' : undefined;
	}
	if (token.text !== ')') {
		return undefined;
	}
	const openIndex = matchingOpenParenIndex(tokens, index);
	if (openIndex < 0) {
		return undefined;
	}
	const previousIndex = previousSignificantTokenIndex(tokens, openIndex - 1);
	const previous = previousIndex >= 0 ? tokens[previousIndex] : undefined;
	return previous && isAttributeName(tokens, previousIndex) && (decoratorClassNames.has(previous.text) || isLikelyTypeIdentifier(previous.text)) ? 'class' : undefined;
}

function matchingOpenParenIndex(tokens: readonly EnforceToken[], closeIndex: number): number {
	let depth = 0;
	for (let index = closeIndex; index >= 0; index--) {
		const token = tokens[index];
		if (token.text === ')') {
			depth++;
		} else if (token.text === '(') {
			depth--;
			if (depth === 0) {
				return index;
			}
		}
	}
	return -1;
}

function isBracketToken(token: EnforceToken): boolean {
	return ['(', ')', '{', '}', '[', ']'].includes(token.text);
}

function isIdentifierLike(token: EnforceToken): boolean {
	return token.kind === 'identifier' || token.kind === 'keyword';
}

function isLikelyTypeIdentifier(value: string): boolean {
	return /^[A-Z]/.test(value) || valueTypeKeywords.has(value) || containerClassTypeKeywords.has(value) || typeKeywords.includes(value);
}

function isGenericTypeParameterName(value: string): boolean {
	return /^T(?:[A-Z][A-Za-z0-9_]*|\d*)$/.test(value);
}

function isShortGenericTypeParameterName(value: string): boolean {
	return /^T\d*$/.test(value);
}

function isGenericParameterBodyTypeUse(tokens: readonly EnforceToken[], token: EnforceToken): boolean {
	const index = tokens.indexOf(token);
	if (index < 0) {
		return false;
	}
	const previous = previousSignificantToken(tokens, index - 1);
	const next = nextSignificantToken(tokens, index + 1);
	if (!next || previous?.text === '.' || previous?.text === '::') {
		return false;
	}
	return isIdentifierLike(next)
		|| ['<', '>', ',', ')', '[', '&'].includes(next.text)
		|| previous?.text === '<'
		|| previous?.text === ','
		|| previous?.text === '(';
}

function previousSignificantToken(tokens: readonly EnforceToken[], startIndex: number): EnforceToken | undefined {
	const index = previousSignificantTokenIndex(tokens, startIndex);
	return index >= 0 ? tokens[index] : undefined;
}

function previousSignificantTokenIndex(tokens: readonly EnforceToken[], startIndex: number): number {
	for (let index = startIndex; index >= 0; index--) {
		const token = tokens[index];
		if (!isSemanticTrivia(token)) {
			return index;
		}
	}
	return -1;
}

function nextSignificantToken(tokens: readonly EnforceToken[], startIndex: number): EnforceToken | undefined {
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (!isSemanticTrivia(token)) {
			return token;
		}
	}
	return undefined;
}

function isSemanticTrivia(token: EnforceToken): boolean {
	return token.kind === 'whitespace' || token.kind === 'newline' || token.kind === 'comment';
}

function collectTypeTokenKeys(parsed: ParsedEnforceSource): Set<string> {
	const result = new Set<string>();
	const context: TypeTokenContext = { tokensByLine: tokensByLine(parsed.tokens) };
	for (const node of parsed.nodes) {
		for (const token of typeTokensForNode(context, node)) {
			result.add(tokenKey(token));
		}
	}
	return result;
}

function collectGenericParameterTokenKeys(parsed: ParsedEnforceSource): Set<string> {
	const result = new Set<string>();
	const context: TypeTokenContext = { tokensByLine: tokensByLine(parsed.tokens) };
	const genericParametersByClass = new Map<string, Set<string>>();
	for (const node of parsed.nodes) {
		if (node.kind !== 'class' || node.declarationKind === 'typedef' || !node.name) {
			continue;
		}
		const parameterTokens = classGenericParameterNameTokens(context, node);
		if (parameterTokens.length === 0) {
			continue;
		}
		genericParametersByClass.set(node.name, new Set(parameterTokens.map(token => token.text)));
		for (const token of parameterTokens) {
			result.add(tokenKey(token));
		}
	}
	for (const node of parsed.nodes) {
		const parameters = node.containerName ? genericParametersByClass.get(node.containerName) : undefined;
		if (!parameters) {
			continue;
		}
		for (const token of typeTokensForNode(context, node)) {
			if (parameters.has(token.text)) {
				result.add(tokenKey(token));
			}
		}
	}
	for (const node of parsed.nodes) {
		if (node.kind !== 'class' || !node.bodyRange || !node.name) {
			continue;
		}
		const parameters = genericParametersByClass.get(node.name);
		if (!parameters) {
			continue;
		}
		for (const token of tokensInRange(context, node.bodyRange)) {
			if (isIdentifierLike(token) && parameters.has(token.text) && isGenericParameterBodyTypeUse(parsed.tokens, token)) {
				result.add(tokenKey(token));
			}
		}
	}
	return result;
}

function typeTokensForNode(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	if (!node.selectionRange) {
		return [];
	}
	if (node.complete === false || node.incomplete || node.recovered || node.confidence === 'low') {
		return [];
	}
	switch (node.kind) {
		case 'property':
		case 'local':
		case 'parameter':
		case 'foreach':
			return declarationTypeTokens(context, node.range, node.selectionRange);
		case 'declarationStatement':
			return declarationStatementTypeTokens(context, node.range, node.selectionRange);
		case 'function':
		case 'memberFunction':
			return [
				...declarationTypeTokens(context, node.range, node.selectionRange),
				...functionParameterTypeTokens(context, node),
			];
		case 'class':
			return node.declarationKind === 'typedef'
				? declarationTypeTokens(context, node.range, node.selectionRange)
				: [
					...classGenericParameterTypeTokens(context, node),
					...classBaseTypeTokens(context, node),
				];
		case 'newExpression':
			return newExpressionTypeTokens(context, node);
		default:
			return [];
	}
}

function declarationTypeTokens(context: TypeTokenContext, range: EnforceParserRange, nameRange: EnforceParserRange): EnforceToken[] {
	return tokensInRange(context, range)
		.filter(token => compareTokenStart(token, nameRange.start) < 0)
		.filter(token => isIdentifierLike(token) && !declarationModifiers.has(token.text));
}

function declarationStatementTypeTokens(context: TypeTokenContext, range: EnforceParserRange, nameRange: EnforceParserRange): EnforceToken[] {
	const tokens = tokensInRange(context, range);
	const firstCommaOrAssignment = findTopLevelTokenBefore(tokens, token => token.text === ',' || token.text === '=', range.end);
	const firstDeclaratorName = previousIdentifierBefore(tokens, firstCommaOrAssignment, nameRange);
	const effectiveRange = firstCommaOrAssignment
		? { start: range.start, end: { line: firstCommaOrAssignment.line, character: firstCommaOrAssignment.character } }
		: range;
	return declarationTypeTokens(context, effectiveRange, firstDeclaratorName ?? nameRange);
}

function functionParameterTypeTokens(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	const tokens = tokensInRange(context, node.range);
	const nameIndex = tokens.findIndex(token => node.selectionRange && rangeStartsAtToken(node.selectionRange, token));
	if (nameIndex < 0) {
		return [];
	}
	const openIndex = tokens.findIndex((token, index) => index > nameIndex && token.text === '(');
	if (openIndex < 0) {
		return [];
	}
	const closeIndex = findMatchingTokenIndex(tokens, openIndex, '(', ')');
	if (closeIndex < 0) {
		return [];
	}
	const result: EnforceToken[] = [];
	let parameterStart = openIndex + 1;
	let angleDepth = 0;
	let bracketDepth = 0;
	let parenDepth = 0;
	for (let index = openIndex + 1; index <= closeIndex; index++) {
		const token = tokens[index];
		if (token.text === '<') {
			angleDepth++;
		} else if (token.text === '>') {
			angleDepth = Math.max(0, angleDepth - 1);
		} else if (token.text === '[') {
			bracketDepth++;
		} else if (token.text === ']') {
			bracketDepth = Math.max(0, bracketDepth - 1);
		} else if (token.text === '(') {
			parenDepth++;
		} else if (token.text === ')') {
			if (index !== closeIndex) {
				parenDepth = Math.max(0, parenDepth - 1);
			}
		}
		if ((token.text === ',' || index === closeIndex) && angleDepth === 0 && bracketDepth === 0 && parenDepth === 0) {
			result.push(...parameterTypeTokens(tokens.slice(parameterStart, index)));
			parameterStart = index + 1;
		}
	}
	return result;
}

function parameterTypeTokens(tokens: readonly EnforceToken[]): EnforceToken[] {
	const assignmentIndex = tokens.findIndex(token => token.text === '=');
	const declarationTokens = assignmentIndex >= 0 ? tokens.slice(0, assignmentIndex) : tokens;
	const nameIndex = lastIdentifierIndex(declarationTokens);
	if (nameIndex < 0) {
		return [];
	}
	return declarationTokens
		.slice(0, nameIndex)
		.filter(token => isIdentifierLike(token) && !declarationModifiers.has(token.text));
}

function lastIdentifierIndex(tokens: readonly EnforceToken[]): number {
	for (let index = tokens.length - 1; index >= 0; index--) {
		const token = tokens[index];
		if (isIdentifierLike(token) && !declarationModifiers.has(token.text)) {
			return index;
		}
	}
	return -1;
}

function findMatchingTokenIndex(tokens: readonly EnforceToken[], openIndex: number, open: string, close: string): number {
	let depth = 0;
	for (let index = openIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.text === open) {
			depth++;
		} else if (token.text === close) {
			depth--;
			if (depth === 0) {
				return index;
			}
		}
	}
	return -1;
}

function findTopLevelTokenBefore(tokens: readonly EnforceToken[], predicate: (token: EnforceToken) => boolean, stop: { line: number; character: number }): EnforceToken | undefined {
	let angleDepth = 0;
	let bracketDepth = 0;
	let parenDepth = 0;
	let braceDepth = 0;
	for (const token of tokens) {
		if (compareTokenStart(token, stop) >= 0) {
			break;
		}
		if (token.text === '<') {
			angleDepth++;
		} else if (token.text === '>') {
			angleDepth = Math.max(0, angleDepth - 1);
		} else if (token.text === '[') {
			bracketDepth++;
		} else if (token.text === ']') {
			bracketDepth = Math.max(0, bracketDepth - 1);
		} else if (token.text === '(') {
			parenDepth++;
		} else if (token.text === ')') {
			parenDepth = Math.max(0, parenDepth - 1);
		} else if (token.text === '{') {
			braceDepth++;
		} else if (token.text === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
		}
		if (angleDepth === 0 && bracketDepth === 0 && parenDepth === 0 && braceDepth === 0 && predicate(token)) {
			return token;
		}
	}
	return undefined;
}

function previousIdentifierBefore(tokens: readonly EnforceToken[], before: EnforceToken | undefined, fallback: EnforceParserRange): EnforceParserRange | undefined {
	const stop = before ? { line: before.line, character: before.character } : fallback.start;
	for (let index = tokens.length - 1; index >= 0; index--) {
		const token = tokens[index];
		if (compareTokenStart(token, stop) >= 0) {
			continue;
		}
		if (isIdentifierLike(token) && !declarationModifiers.has(token.text)) {
			return {
				start: { line: token.line, character: token.character },
				end: { line: token.endLine, character: token.endCharacter },
			};
		}
	}
	return undefined;
}

function classBaseTypeTokens(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	const inHeader = tokensInRange(context, node.range)
		.filter(token => !node.bodyRange || compareTokenStart(token, node.bodyRange.start) < 0);
	const nameIndex = inHeader.findIndex(token => node.selectionRange && rangeStartsAtToken(node.selectionRange, token));
	if (nameIndex < 0) {
		return [];
	}
	const markerIndex = inHeader.findIndex((token, index) => index > nameIndex && (token.text === ':' || token.text === 'extends'));
	if (markerIndex < 0) {
		return [];
	}
	const result: EnforceToken[] = [];
	for (const token of inHeader.slice(markerIndex + 1)) {
		if (token.text === '{' || token.text === ';' || token.kind === 'newline') {
			break;
		}
		if (isIdentifierLike(token)) {
			result.push(token);
		}
	}
	return result;
}

function classGenericParameterTypeTokens(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	const inHeader = tokensInRange(context, node.range)
		.filter(token => !node.bodyRange || compareTokenStart(token, node.bodyRange.start) < 0);
	const nameIndex = inHeader.findIndex(token => node.selectionRange && rangeStartsAtToken(node.selectionRange, token));
	if (nameIndex < 0) {
		return [];
	}
	const openIndex = inHeader.findIndex((token, index) => index > nameIndex && token.text === '<');
	if (openIndex < 0) {
		return [];
	}
	const closeIndex = findMatchingTokenIndex(inHeader, openIndex, '<', '>');
	if (closeIndex < 0) {
		return [];
	}
	return inHeader
		.slice(openIndex + 1, closeIndex)
		.filter(token => isIdentifierLike(token) && !declarationModifiers.has(token.text));
}

function classGenericParameterNameTokens(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	const inHeader = tokensInRange(context, node.range)
		.filter(token => !node.bodyRange || compareTokenStart(token, node.bodyRange.start) < 0);
	const nameIndex = inHeader.findIndex(token => node.selectionRange && rangeStartsAtToken(node.selectionRange, token));
	if (nameIndex < 0) {
		return [];
	}
	const openIndex = inHeader.findIndex((token, index) => index > nameIndex && token.text === '<');
	if (openIndex < 0) {
		return [];
	}
	const closeIndex = findMatchingTokenIndex(inHeader, openIndex, '<', '>');
	if (closeIndex < 0) {
		return [];
	}
	const result: EnforceToken[] = [];
	let segmentStart = openIndex + 1;
	let angleDepth = 0;
	for (let index = openIndex + 1; index <= closeIndex; index++) {
		const token = inHeader[index];
		if (token.text === '<') {
			angleDepth++;
		} else if (token.text === '>') {
			if (index !== closeIndex) {
				angleDepth = Math.max(0, angleDepth - 1);
			}
		}
		if ((token.text === ',' || index === closeIndex) && angleDepth === 0) {
			const identifiers = inHeader
				.slice(segmentStart, index)
				.filter(candidate => isIdentifierLike(candidate) && !declarationModifiers.has(candidate.text));
			const parameterToken = identifiers.length > 1
				? identifiers[identifiers.length - 1]
				: identifiers.find(candidate => isGenericTypeParameterName(candidate.text));
			if (parameterToken) {
				result.push(parameterToken);
			}
			segmentStart = index + 1;
		}
	}
	return result;
}

function newExpressionTypeTokens(context: TypeTokenContext, node: EnforceSyntaxNode): EnforceToken[] {
	const inExpression = tokensInRange(context, node.range);
	const newIndex = inExpression.findIndex(token => token.text === 'new');
	if (newIndex < 0) {
		return [];
	}
	const typeTokens: EnforceToken[] = [];
	for (const token of inExpression.slice(newIndex + 1)) {
		if (token.text === '(' || token.text === ';' || token.text === '=') {
			break;
		}
		if (isIdentifierLike(token)) {
			typeTokens.push(token);
		}
	}
	return typeTokens;
}

function tokensByLine(tokens: readonly EnforceToken[]): Map<number, EnforceToken[]> {
	const result = new Map<number, EnforceToken[]>();
	for (const token of tokens) {
		const lineTokens = result.get(token.line) ?? [];
		lineTokens.push(token);
		result.set(token.line, lineTokens);
	}
	return result;
}

function tokensInRange(context: TypeTokenContext, range: EnforceParserRange): EnforceToken[] {
	const result: EnforceToken[] = [];
	for (let line = range.start.line; line <= range.end.line; line++) {
		for (const token of context.tokensByLine.get(line) ?? []) {
			if (isTokenInRange(token, range)) {
				result.push(token);
			}
		}
	}
	return result;
}

function isTokenInRange(token: EnforceToken, range: EnforceParserRange): boolean {
	return comparePositions({ line: token.line, character: token.character }, range.start) >= 0
		&& comparePositions({ line: token.endLine, character: token.endCharacter }, range.end) <= 0;
}

function rangeStartsAtToken(range: EnforceParserRange, token: EnforceToken): boolean {
	return range.start.line === token.line && range.start.character === token.character;
}

function tokensInParserRange(tokens: readonly EnforceToken[], range: EnforceParserRange): EnforceToken[] {
	return tokens.filter(token => !isSemanticTrivia(token) && isTokenInRange(token, range));
}

function compareTokenStart(token: EnforceToken, position: { line: number; character: number }): number {
	return comparePositions({ line: token.line, character: token.character }, position);
}

function comparePositions(left: { line: number; character: number }, right: { line: number; character: number }): number {
	return left.line !== right.line ? left.line - right.line : left.character - right.character;
}

function tokenKey(token: EnforceToken): string {
	return `${token.line}:${token.character}:${token.endLine}:${token.endCharacter}`;
}

function splitTokenLines(token: EnforceToken, type: string): SemanticToken[] {
	if (token.line === token.endLine) {
		return [{
			line: token.line,
			character: token.character,
			length: Math.max(1, token.endCharacter - token.character),
			type,
			modifiers: [],
		}];
	}

	const lines = token.text.split(/\r\n|\r|\n/);
	return lines
		.map((lineText, index): SemanticToken | undefined => {
			const line = token.line + index;
			const character = index === 0 ? token.character : 0;
			const length = index === lines.length - 1
				? token.endCharacter - character
				: lineText.length;
			return length > 0 ? { line, character, length, type, modifiers: [] } : undefined;
		})
		.filter((value): value is SemanticToken => value !== undefined);
}

function dedupeSemanticTokens(tokens: SemanticToken[]): SemanticToken[] {
	const byRange = new Map<string, SemanticToken>();
	for (const token of tokens) {
		byRange.set(`${token.line}:${token.character}:${token.length}`, token);
	}
	return [...byRange.values()].sort((a, b) => a.line - b.line || a.character - b.character || a.length - b.length);
}

function encodeModifiers(modifiers: readonly string[]): number {
	let result = 0;
	for (const modifier of modifiers) {
		const index = modelSemanticTokenModifiers.indexOf(modifier);
		if (index >= 0) {
			result |= 1 << index;
		}
	}
	return result;
}

function shortFileName(document: vscode.TextDocument): string {
	return document.uri.fsPath.split(/[\\/]/).pop() ?? document.uri.toString();
}

interface SemanticToken {
	line: number;
	character: number;
	length: number;
	type: string;
	modifiers: readonly string[];
}

interface TypeTokenContext {
	tokensByLine: Map<number, EnforceToken[]>;
}

