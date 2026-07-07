import * as vscode from 'vscode';
import { EnforceContainerMemberSymbol, EnforceDecorator, EnforceSymbol, EnforceSymbolIndex, EnforceSymbolType } from '../index/symbolIndex';
import type { EnforceParserPosition, EnforceParserRange, EnforceParserScopeFact, EnforceSyntaxNode } from '../parser/ast';
import { toVscodeRange } from '../parser/documentCache';
import { getMemberAccessAt, getSwitchContext, getTypeUsageAt } from '../parser/query';
import type { EnforceToken } from '../parser/tokens';
import { buildLanguageModel, LanguageModel, LanguagePosition, LanguageRange } from './languageModel';

export type ResolvedIdentityKind =
	| 'attribute'
	| 'class'
	| 'enum'
	| 'enumValue'
	| 'function'
	| 'local'
	| 'macro'
	| 'parameter'
	| 'property'
	| 'unknown';

export type ResolvedIdentityConfidence = 'high' | 'low';

export type CodeReferenceKind =
	| 'call'
	| 'declaration'
	| 'memberAccess'
	| 'read'
	| 'typeUsage'
	| 'write';

export interface ResolvedIdentity {
	id: string;
	kind: ResolvedIdentityKind;
	name: string;
	range: LanguageRange;
	declaration?: EnforceSymbol | EnforceSyntaxNode | EnforceParserScopeFact;
	containerName?: string;
	signature?: string;
	detail?: string;
	documentation?: string;
	origin?: string;
	confidence: ResolvedIdentityConfidence;
	targetLocations: vscode.Location[];
	symbol?: EnforceSymbol;
	node?: EnforceSyntaxNode;
	local?: EnforceParserScopeFact;
}

export interface CodeReference {
	location: vscode.Location;
	kind: CodeReferenceKind;
}

export interface CodeIntelligenceModel {
	readonly language: LanguageModel;
	resolveIdentityAt(position: vscode.Position): ResolvedIdentity | undefined;
	resolveDefinition(identity: ResolvedIdentity): vscode.Location[];
	resolveReferences(identity: ResolvedIdentity, scope?: 'document'): CodeReference[];
	formatHover(identity: ResolvedIdentity): vscode.MarkdownString[];
}

export function buildCodeIntelligenceModel(document: vscode.TextDocument, symbolIndex: EnforceSymbolIndex): CodeIntelligenceModel {
	return new ParserBackedCodeIntelligenceModel(buildLanguageModel(document, symbolIndex), symbolIndex);
}

class ParserBackedCodeIntelligenceModel implements CodeIntelligenceModel {
	constructor(
		readonly language: LanguageModel,
		private readonly symbolIndex: EnforceSymbolIndex
	) {}

	resolveIdentityAt(position: vscode.Position): ResolvedIdentity | undefined {
		if (this.isIgnored(position)) {
			return undefined;
		}

		const parserPosition = toParserPosition(position);
		const identifierToken = this.findIdentifierTokenAt(parserPosition);
		const wordRange = identifierToken ? toVscodeRange(tokenRange(identifierToken)) : this.language.document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
		if (!wordRange) {
			return undefined;
		}

		const name = identifierToken?.text ?? this.language.document.getText(wordRange);
		const declaration = this.findExactDeclaration(name, parserPosition);
		if (declaration) {
			return this.identityForDeclarationNode(declaration, position) ?? nodeIdentity(declaration, this.language.document.uri);
		}

		const typeUsageIdentity = identifierToken ? this.findTypeUsageTokenIdentityAt(name, identifierToken, position) : undefined;
		if (typeUsageIdentity) {
			return typeUsageIdentity;
		}

		const qualifiedIdentity = identifierToken ? this.findQualifiedTokenIdentityAt(name, identifierToken, position) : undefined;
		if (qualifiedIdentity) {
			return qualifiedIdentity;
		}

		const memberIdentity = this.findMemberIdentityAt(name, position);
		if (memberIdentity) {
			return memberIdentity;
		}

		const local = this.language.visibleLocals(position).find(candidate => candidate.name === name);
		if (local) {
			return localIdentity(local, this.language.document);
		}

		const currentClassMember = this.findCurrentClassMember(name, position);
		if (currentClassMember) {
			return symbolIdentity(currentClassMember);
		}

		const enumMember = this.findEnumMemberIdentity(name, position);
		if (enumMember) {
			return enumMember;
		}

		const macro = this.findMacroIdentity(name, position);
		if (macro) {
			return macro;
		}

		const matches = this.findSymbolsByName(name).filter(symbol => ['class', 'enum', 'function'].includes(symbol.type));
		if (matches.length === 1) {
			return symbolIdentity(matches[0]);
		}
		if (matches.length > 1) {
			const typeIdentity = this.findTypeContextIdentity(matches, position, identifierToken);
			if (typeIdentity) {
				return typeIdentity;
			}
			return ambiguousSymbolIdentity(name, matches);
		}
		return undefined;
	}

	resolveDefinition(identity: ResolvedIdentity): vscode.Location[] {
		return identity.confidence === 'high' || identity.targetLocations.length > 1 ? identity.targetLocations : [];
	}

	resolveReferences(identity: ResolvedIdentity): CodeReference[] {
		if (identity.confidence !== 'high') {
			return [];
		}

		const references: CodeReference[] = [];
		for (const location of identity.targetLocations) {
			references.push({ location, kind: 'declaration' });
		}

		for (const token of this.language.parsed.tokens) {
			if (token.kind !== 'identifier' || token.text !== identity.name || tokenRangeEqualsIdentity(token, identity)) {
				continue;
			}
			const position = new vscode.Position(token.line, token.character);
			const candidate = this.resolveIdentityAt(position);
			if (candidate?.id === identity.id) {
				references.push({
					location: new vscode.Location(this.language.document.uri, toVscodeRange(tokenRange(token))),
					kind: referenceKindAt(this.language.document, position, candidate),
				});
			}
		}
		return dedupeReferences(references);
	}

	formatHover(identity: ResolvedIdentity): vscode.MarkdownString[] {
		const context = formatIdentityHeader(identity);
		const snippet = formatHoverSnippet(identity);
		return [
			formatHoverHeaderMarkdown(context),
			formatHoverSnippetMarkdown(identity, snippet),
			...(identity.documentation ? [new vscode.MarkdownString(formatHoverDocumentation(identity.documentation), true)] : []),
			...formatAttributeParamsHover(identity),
			...this.formatClassMembersHover(identity),
			...formatEnumMembersHover(identity),
		];
	}

	private findExactDeclaration(name: string, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
		return this.language.parsed.nodes.find(node =>
			node.name === name
			&& node.selectionRange
			&& rangeContains(node.selectionRange, position)
			&& isDeclarationNodeKind(node.kind)
		);
	}

	private identityForDeclarationNode(node: EnforceSyntaxNode, position: vscode.Position): ResolvedIdentity | undefined {
		if (node.kind === 'local' || node.kind === 'parameter' || node.kind === 'foreach') {
			const local = this.language.visibleLocals(position).find(candidate =>
				candidate.name === node.name
				&& candidate.selectionRange
				&& rangesEqual(candidate.selectionRange, node.selectionRange ?? node.range)
			);
			return local ? localIdentity(local, this.language.document) : undefined;
		}

		const expectedType = symbolTypeForNodeKind(node.kind);
		if (!expectedType || !node.name) {
			return undefined;
		}
		const selectionRange = node.selectionRange ?? node.range;
		const symbol = this.findSymbolsByName(node.name).find(candidate =>
			candidate.type === expectedType
			&& candidate.uri.toString() === this.language.document.uri.toString()
			&& rangesEqual(fromVscodeRange(candidate.selectionRange), selectionRange)
		);
		if (!symbol) {
			return undefined;
		}
		const overrideTarget = this.findOverrideTarget(symbol);
		return overrideTarget ? overrideIdentity(symbol, overrideTarget) : symbolIdentity(symbol);
	}

	private findMemberIdentityAt(name: string, position: vscode.Position): ResolvedIdentity | undefined {
		const member = getMemberAccessAt(this.language.parsed, toParserPosition(position));
		if (!member?.receiver) {
			return undefined;
		}
		const symbol = this.language.members(member.receiver, position, { includeStaticInstanceMembers: true }).find(candidate => candidate.name === name);
		if (symbol) {
			return symbolIdentity(symbol);
		}
		const enumValue = this.getEnumValueSymbols(member.receiver).find(candidate => candidate.name === name);
		return enumValue ? symbolIdentity(enumValue) : undefined;
	}

	private findQualifiedTokenIdentityAt(name: string, token: EnforceToken, position: vscode.Position): ResolvedIdentity | undefined {
		const tokenIndex = this.language.parsed.tokens.indexOf(token);
		if (tokenIndex < 0) {
			return undefined;
		}
		const accessToken = previousSignificantToken(this.language.parsed.tokens, tokenIndex - 1);
		if (accessToken?.text !== '.' && accessToken?.text !== '::') {
			return undefined;
		}
		const accessIndex = this.language.parsed.tokens.indexOf(accessToken);
		const receiver = previousSignificantToken(this.language.parsed.tokens, accessIndex - 1);
		if (!receiver || !isIdentifierLike(receiver)) {
			return undefined;
		}
		const enumValue = this.getEnumValueSymbols(receiver.text).find(candidate => candidate.name === name);
		if (enumValue) {
			return symbolIdentity(enumValue);
		}
		const member = this.language.members(receiver.text, position, { includeStaticInstanceMembers: true }).find(candidate => candidate.name === name);
		return member ? symbolIdentity(member) : undefined;
	}

	private findCurrentClassMember(name: string | undefined, position: vscode.Position): EnforceContainerMemberSymbol | undefined {
		const className = this.language.currentClass(position)?.name;
		if (!className || !name) {
			return undefined;
		}
		const containers = this.language.classAncestorNames(className, true);
		const members = this.getContainerMemberSymbolsForContainersAndName(containers, name);
		return members[0];
	}

	private findEnumMemberIdentity(name: string, position: vscode.Position): ResolvedIdentity | undefined {
		const member = getMemberAccessAt(this.language.parsed, toParserPosition(position));
		if (member?.receiver) {
			const symbol = this.getEnumValueSymbols(member.receiver).find(candidate => candidate.name === name);
			return symbol ? symbolIdentity(symbol) : undefined;
		}
		const switchExpression = getSwitchContext(this.language.parsed, toParserPosition(position))?.expression;
		const switchType = switchExpression ? this.language.resolveTypeOfExpression(switchExpression, position) : undefined;
		const symbol = switchType ? this.getEnumValueSymbols(switchType.name).find(candidate => candidate.name === name) : undefined;
		return symbol ? symbolIdentity(symbol) : undefined;
	}

	private findMacroIdentity(name: string, position: vscode.Position): ResolvedIdentity | undefined {
		const line = this.language.document.lineAt(position.line).text.trim();
		if (!/^#\s*(?:define|ifdef|ifndef)\b/.test(line)) {
			return undefined;
		}
		const symbol = this.getMacroSymbols().find(candidate => candidate.name === name);
		return symbol ? symbolIdentity(symbol) : undefined;
	}

	private findSymbolsByName(name: string): EnforceSymbol[] {
		return dedupeSymbols([
			...this.language.parsed.symbols.filter(symbol => symbol.name === name),
			...this.symbolIndex.find(name),
		]);
	}

	private findTypeContextIdentity(matches: readonly EnforceSymbol[], position: vscode.Position, token?: EnforceToken): ResolvedIdentity | undefined {
		const context = this.language.contextAt(position);
		if (!['classInheritance', 'declaration', 'type'].includes(context.kind) && !this.isGenericTypeArgumentToken(token)) {
			return undefined;
		}
		const typeMatch = chooseTypeSymbol(matches);
		return typeMatch ? symbolIdentity(typeMatch) : undefined;
	}

	private findTypeUsageTokenIdentityAt(name: string, token: EnforceToken, position: vscode.Position): ResolvedIdentity | undefined {
		if (!this.isParserBackedTypeToken(token, position) && !this.isGenericTypeArgumentToken(token)) {
			return undefined;
		}
		const typeMatch = chooseTypeSymbol(this.findSymbolsByName(name));
		return typeMatch ? symbolIdentity(typeMatch) : undefined;
	}

	private findOverrideTarget(symbol: EnforceSymbol): EnforceContainerMemberSymbol | undefined {
		if (symbol.type !== 'memberFunction' || !symbol.containerName || !symbol.modifiers?.includes('override')) {
			return undefined;
		}
		const ancestors = this.language.classAncestorNames(symbol.containerName, false);
		return this.getContainerMemberSymbolsForContainersAndName(ancestors, symbol.name)
			.find(candidate => candidate.type === 'memberFunction');
	}

	private isGenericTypeArgumentToken(token: EnforceToken | undefined): boolean {
		if (!token) {
			return false;
		}
		const tokenIndex = this.language.parsed.tokens.indexOf(token);
		if (tokenIndex < 0 || !this.hasClosingGenericBracketAfter(tokenIndex)) {
			return false;
		}
		let depth = 0;
		for (let index = tokenIndex - 1; index >= 0; index--) {
			const candidate = this.language.parsed.tokens[index];
			if (isTrivia(candidate)) {
				continue;
			}
			if (this.isGenericBoundary(candidate.text)) {
				return false;
			}
			if (candidate.text === '>') {
				depth++;
				continue;
			}
			if (candidate.text === '<') {
				if (depth === 0) {
					const receiver = previousSignificantToken(this.language.parsed.tokens, index - 1);
					return receiver !== undefined && isIdentifierLike(receiver);
				}
				depth--;
			}
		}
		return false;
	}

	private hasClosingGenericBracketAfter(tokenIndex: number): boolean {
		let depth = 0;
		for (let index = tokenIndex + 1; index < this.language.parsed.tokens.length; index++) {
			const candidate = this.language.parsed.tokens[index];
			if (isTrivia(candidate)) {
				continue;
			}
			if (candidate.text === '<') {
				depth++;
				continue;
			}
			if (candidate.text === '>') {
				if (depth === 0) {
					return true;
				}
				depth--;
				continue;
			}
			if (this.isGenericBoundary(candidate.text)) {
				return false;
			}
		}
		return false;
	}

	private isGenericBoundary(text: string): boolean {
		return [';', '=', '{', '}', '(', ')', '[', ']'].includes(text);
	}

	private isParserBackedTypeToken(token: EnforceToken, position: vscode.Position): boolean {
		const node = getTypeUsageAt(this.language.parsed, toParserPosition(position));
		if (!node || !isTypeUsageNodeKind(node.kind)) {
			return false;
		}
		if (node.selectionRange && comparePositions(tokenRange(token).start, node.selectionRange.start) >= 0) {
			return false;
		}
		return rangeContains(node.range, toParserPosition(position));
	}

	private getEnumValueSymbols(containerName: string): EnforceSymbol[] {
		return dedupeSymbols([
			...this.language.parsed.symbols.filter(symbol => symbol.type === 'enumValue' && symbol.containerName === containerName),
			...this.symbolIndex.getEnumValueSymbols(containerName),
		]);
	}

	private getMacroSymbols(): EnforceSymbol[] {
		return dedupeSymbols([
			...this.language.parsed.symbols.filter(symbol => symbol.type === 'macro'),
			...this.symbolIndex.getMacroSymbols(),
		]);
	}

	private getContainerMemberSymbolsForContainersAndName(containers: readonly string[], name: string): EnforceContainerMemberSymbol[] {
		const parsedMembers = this.language.parsed.symbols.filter((symbol): symbol is EnforceContainerMemberSymbol =>
			(symbol.type === 'memberFunction' || symbol.type === 'property')
			&& symbol.containerName !== undefined
			&& containers.includes(symbol.containerName)
			&& symbol.name === name
		);
		const indexedMembers = this.symbolIndex.getContainerMemberSymbolsForContainersAndName
			? this.symbolIndex.getContainerMemberSymbolsForContainersAndName(containers, name)
			: this.symbolIndex.getContainerMemberSymbolsForContainers(containers).filter(symbol => symbol.name === name);
		return dedupeSymbols([...parsedMembers, ...indexedMembers]) as EnforceContainerMemberSymbol[];
	}

	private getContainerMemberSymbolsForContainers(containers: readonly string[]): EnforceContainerMemberSymbol[] {
		const parsedMembers = this.language.parsed.symbols.filter((symbol): symbol is EnforceContainerMemberSymbol =>
			(symbol.type === 'memberFunction' || symbol.type === 'property')
			&& symbol.containerName !== undefined
			&& containers.includes(symbol.containerName)
		);
		return dedupeSymbols([
			...parsedMembers,
			...this.symbolIndex.getContainerMemberSymbolsForContainers(containers),
		]) as EnforceContainerMemberSymbol[];
	}

	private formatClassMembersHover(identity: ResolvedIdentity): vscode.MarkdownString[] {
		if (identity.kind !== 'class') {
			return [];
		}
		const members = this.getContainerMemberSymbolsForContainers([identity.name]);
		const constructors = members.filter(symbol => symbol.declarationKind === 'constructor');
		const functions = members.filter(symbol => symbol.type === 'memberFunction' && symbol.declarationKind !== 'constructor' && symbol.declarationKind !== 'destructor');
		const properties = members.filter(symbol => symbol.type === 'property');
		return [
			...formatMemberSectionHover('Constructors', constructors, 'enforce-hover-constructor', formatConstructorHoverLine, '\n\n'),
			...formatMemberSectionHover('Functions', functions),
			...formatMemberSectionHover('Properties', properties),
		];
	}

	private isIgnored(position: vscode.Position): boolean {
		const parserPosition = toParserPosition(position);
		return this.language.parsed.tokens.some(token =>
			(token.kind === 'comment' || token.kind === 'string')
			&& rangeContains(tokenRange(token), parserPosition)
		);
	}

	private findIdentifierTokenAt(position: EnforceParserPosition): EnforceToken | undefined {
		return this.language.parsed.tokens
			.filter(token => token.kind === 'identifier' && rangeContains(tokenRange(token), position))
			.sort((left, right) => rangeSize(tokenRange(left)) - rangeSize(tokenRange(right)))[0];
	}
}

function symbolIdentity(symbol: EnforceSymbol): ResolvedIdentity {
	const range = fromVscodeRange(symbol.selectionRange);
	const attribute = getAttributeDecorator(symbol);
	return {
		id: canonicalSymbolKey(symbol),
		kind: attribute && symbol.type === 'property' ? 'attribute' : symbolKind(symbol.type),
		name: symbol.name,
		range,
		declaration: symbol,
		containerName: symbol.containerName,
		signature: symbol.signature,
		detail: symbol.signature ?? symbol.detail ?? symbol.name,
		documentation: symbol.documentation ?? attributeDescription(attribute),
		origin: symbol.origin,
		confidence: 'high',
		targetLocations: [new vscode.Location(symbol.uri, symbol.selectionRange)],
		symbol,
	};
}

function overrideIdentity(symbol: EnforceSymbol, target: EnforceSymbol): ResolvedIdentity {
	const range = fromVscodeRange(symbol.selectionRange);
	return {
		id: canonicalSymbolKey(symbol),
		kind: symbolKind(target.type),
		name: target.name,
		range,
		declaration: symbol,
		containerName: target.containerName,
		signature: target.signature,
		detail: target.signature ?? target.detail ?? target.name,
		documentation: target.documentation,
		origin: target.origin,
		confidence: 'high',
		targetLocations: [new vscode.Location(target.uri, target.selectionRange)],
		symbol: target,
	};
}

function ambiguousSymbolIdentity(name: string, symbols: EnforceSymbol[]): ResolvedIdentity {
	const first = symbols[0];
	return {
		id: `ambiguous:${name}:${symbols.map(symbol => canonicalSymbolKey(symbol)).join('|')}`,
		kind: 'unknown',
		name,
		range: fromVscodeRange(first.selectionRange),
		detail: name,
		confidence: 'low',
		targetLocations: symbols.map(symbol => new vscode.Location(symbol.uri, symbol.selectionRange)),
	};
}

function nodeIdentity(node: EnforceSyntaxNode, uri: vscode.Uri): ResolvedIdentity {
	const range = node.selectionRange ?? node.range;
	return {
		id: `node:${uri.toString()}:${node.kind}:${node.containerName ?? ''}:${node.name ?? ''}:${rangeKey(range)}`,
		kind: nodeKind(node.kind),
		name: node.name ?? '',
		range,
		declaration: node,
		containerName: node.containerName,
		signature: node.signature,
		detail: node.signature ?? node.name,
		confidence: 'high',
		targetLocations: [new vscode.Location(uri, toVscodeRange(range))],
		node,
	};
}

function localIdentity(local: EnforceParserScopeFact, document: vscode.TextDocument): ResolvedIdentity {
	const range = local.selectionRange ?? local.range;
	return {
		id: `local:${document.uri.toString()}:${local.containerName ?? ''}:${local.functionName ?? ''}:${local.name ?? ''}:${rangeKey(range)}`,
		kind: local.kind === 'parameter' ? 'parameter' : 'local',
		name: local.name ?? '',
		range,
		declaration: local,
		containerName: local.containerName,
		detail: formatLocalDetail(local, document),
		confidence: 'high',
		targetLocations: [new vscode.Location(document.uri, toVscodeRange(range))],
		local,
	};
}

function formatIdentityHeader(identity: ResolvedIdentity): string {
	const kind = formatIdentityKind(identity.kind);
	if (identity.containerName) {
		return `${kind} in ${identity.containerName}`;
	}
	return kind;
}

function formatIdentityKind(kind: ResolvedIdentityKind): string {
	return kind === 'local' ? 'variable' : kind;
}

function formatHoverSnippet(identity: ResolvedIdentity): string {
	const snippet = identity.signature ?? identity.detail ?? identity.name;
	if (identity.kind === 'enumValue') {
		return formatEnumMemberHoverLine(snippet);
	}
	return formatHoverDeclarationText(snippet);
}

interface AttributeParameterSpec {
	name: string;
	type: string;
	defaultValue?: string;
}

interface AttributeParameterValue extends AttributeParameterSpec {
	value: string;
}

const attributeParameterSpecs: readonly AttributeParameterSpec[] = [
	{ name: 'defvalue', type: 'string', defaultValue: '""' },
	{ name: 'uiwidget', type: 'string', defaultValue: '"auto"' },
	{ name: 'desc', type: 'string', defaultValue: '""' },
	{ name: 'params', type: 'string', defaultValue: '""' },
	{ name: 'enums', type: 'ParamEnumArray', defaultValue: 'NULL' },
	{ name: 'category', type: 'string', defaultValue: '""' },
	{ name: 'precision', type: 'int', defaultValue: '3' },
	{ name: 'enumType', type: 'typename', defaultValue: 'void' },
	{ name: 'prefabbed', type: 'bool', defaultValue: 'false' },
];

const attributeConstructorSignature = 'void Attribute(string defvalue = "", string uiwidget = "auto"/*use UIWidgets*/, string desc = "", string params = "", ParamEnumArray enums = NULL, string category = "", int precision = 3, typename enumType = void, bool prefabbed = false);';

function getAttributeDecorator(symbol: EnforceSymbol | undefined): EnforceDecorator | undefined {
	return symbol?.decoratorDetails?.find(decorator => decorator.name === 'Attribute');
}

function attributeDescription(attribute: EnforceDecorator | undefined): string | undefined {
	if (!attribute?.arguments) {
		return undefined;
	}
	return parseAttributeArguments(attribute.arguments).find(value => value.name === 'desc')?.value.replace(/^"([\s\S]*)"$/, '$1');
}

function parseAttributeArguments(rawArguments: string): AttributeParameterValue[] {
	const args = splitTopLevelArguments(rawArguments);
	const values: AttributeParameterValue[] = [];
	let positionalHasWidget = false;
	for (let index = 0; index < args.length; index++) {
		const argument = args[index].trim();
		if (!argument) {
			continue;
		}
		const named = /^([A-Za-z_]\w*)\s*:\s*([\s\S]+)$/.exec(argument);
		const name = named?.[1] ?? attributePositionalName(index, argument, positionalHasWidget);
		const value = named?.[2]?.trim() ?? argument;
		if (index === 1 && !named && name === 'uiwidget') {
			positionalHasWidget = true;
		}
		values.push({
			name,
			type: attributeParameterType(name, value),
			value,
		});
	}
	return values;
}

function attributePositionalName(index: number, value: string, positionalHasWidget: boolean): string {
	if (index === 0) {
		return 'defvalue';
	}
	if (index === 1) {
		return isUiWidgetValue(value) ? 'uiwidget' : 'desc';
	}
	if (positionalHasWidget) {
		return ['desc', 'params', 'enums', 'category', 'precision', 'enumType', 'prefabbed'][index - 2] ?? `arg${index + 1}`;
	}
	return ['params', 'enums', 'category', 'precision', 'enumType', 'prefabbed'][index - 2] ?? `arg${index + 1}`;
}

function isUiWidgetValue(value: string): boolean {
	return /^\s*UIWidgets\./.test(value);
}

function attributeParameterType(name: string, value: string): string {
	const spec = attributeParameterSpecs.find(candidate => candidate.name === name);
	if (spec) {
		return spec.type;
	}
	if (/^-?\d+$/.test(value)) {
		return 'int';
	}
	if (/^-?\d+\.\d+$/.test(value)) {
		return 'float';
	}
	return /^[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?\(/.test(value) ? 'auto' : 'string';
}

function formatAttributeValue(value: AttributeParameterValue): string {
	return `${value.type} ${value.name} = ${value.value}`;
}

function formatAttributeSpec(spec: AttributeParameterSpec): string {
	return spec.defaultValue ? `${spec.type} ${spec.name} = ${spec.defaultValue}` : `${spec.type} ${spec.name}`;
}

function splitTopLevelArguments(text: string): string[] {
	const parts: string[] = [];
	let start = 0;
	let depth = 0;
	let quote: string | undefined;
	for (let index = 0; index < text.length; index++) {
		const char = text[index];
		const previous = index > 0 ? text[index - 1] : '';
		if (quote) {
			if (char === quote && previous !== '\\') {
				quote = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			quote = char;
			continue;
		}
		if (char === '(' || char === '[' || char === '<') {
			depth++;
			continue;
		}
		if (char === ')' || char === ']' || char === '>') {
			depth = Math.max(0, depth - 1);
			continue;
		}
		if (char === ',' && depth === 0) {
			parts.push(text.slice(start, index));
			start = index + 1;
		}
	}
	parts.push(text.slice(start));
	return parts;
}

function formatEnumMembersHover(identity: ResolvedIdentity): vscode.MarkdownString[] {
	if (identity.kind !== 'enum' || !identity.symbol?.enumMembers?.length) {
		return [];
	}
	const lines = identity.symbol.enumMembers.map(formatEnumMemberHoverLine);
	return formatTextSectionHover('Members', lines);
}

function formatAttributeParamsHover(identity: ResolvedIdentity): vscode.MarkdownString[] {
	if (identity.kind !== 'attribute') {
		return [];
	}
	const attribute = getAttributeDecorator(identity.symbol);
	if (!attribute?.arguments) {
		return [];
	}
	const values = parseAttributeArguments(attribute.arguments);
	const currentLines = values
		.filter(value => value.name !== 'desc')
		.map(value => formatAttributeValue(value));
	return [
		...formatCodeSectionHover('Params', currentLines),
		...formatCodeSectionHover('Constructor', [formatLongCallableSignature(formatHoverDeclarationText(attributeConstructorSignature))], 'enforce-hover-constructor'),
	];
}

function formatMemberSectionHover(
	title: string,
	members: readonly EnforceSymbol[],
	language = 'enforce-hover',
	formatMember: (member: EnforceSymbol) => string = member => formatHoverDeclarationText(member.signature ?? member.name),
	separator = '\n'
): vscode.MarkdownString[] {
	if (members.length === 0) {
		return [];
	}
	const markdown = new vscode.MarkdownString(undefined, true);
	markdown.appendMarkdown(`### ${title}\n`);
	markdown.appendCodeblock(members.map(formatMember).join(separator), language);
	return [markdown];
}

function formatConstructorHoverLine(member: EnforceSymbol): string {
	return formatLongCallableSignature(formatHoverDeclarationText(member.signature ?? member.name));
}

function formatLongCallableSignature(signature: string): string {
	const openParen = signature.indexOf('(');
	const closeParen = signature.lastIndexOf(')');
	if (openParen < 0 || closeParen < openParen) {
		return signature;
	}
	const params = splitTopLevelArguments(signature.slice(openParen + 1, closeParen)).map(param => param.trim()).filter(Boolean);
	if (params.length < 2) {
		return signature;
	}
	const prefix = signature.slice(0, openParen).trimEnd();
	const suffix = signature.slice(closeParen + 1).trimStart();
	return `${prefix}(\n\t${params.join(',\n\t')}\n)${suffix}`;
}

function formatHoverDeclarationText(text: string): string {
	return text.replace(/;\s*$/u, '');
}

function formatHoverDocumentation(documentation: string): string {
	return documentation.replace(/^(\s*)`?[@\\]([A-Za-z]+\b)`?/gmu, '$1`$2`');
}

function formatCodeSectionHover(title: string, lines: readonly string[], language = 'enforce-hover'): vscode.MarkdownString[] {
	if (lines.length === 0) {
		return [];
	}
	const markdown = new vscode.MarkdownString(undefined, true);
	markdown.appendMarkdown(`### ${title}\n`);
	markdown.appendCodeblock(lines.join('\n'), language);
	return [markdown];
}

function formatTextSectionHover(title: string, lines: readonly string[]): vscode.MarkdownString[] {
	if (lines.length === 0) {
		return [];
	}
	const markdown = new vscode.MarkdownString(undefined, true);
	markdown.appendMarkdown(`### ${title}\n`);
	markdown.appendCodeblock(lines.join('\n'));
	return [markdown];
}

function hoverCodeBlock(value: string): vscode.MarkdownString {
	const markdown = new vscode.MarkdownString(undefined, true);
	markdown.appendCodeblock(value, 'enforce-hover');
	return markdown;
}

function formatHoverHeaderMarkdown(header: string): vscode.MarkdownString {
	const markdown = new vscode.MarkdownString(undefined, true);
	markdown.appendCodeblock(header, 'enforce-hover-header');
	return markdown;
}

function formatHoverSnippetMarkdown(identity: ResolvedIdentity, snippet: string): vscode.MarkdownString {
	return identity.kind === 'enumValue' ? new vscode.MarkdownString(snippet, true) : hoverCodeBlock(snippet);
}

function formatEnumMemberHoverLine(member: string): string {
	const comment = member.match(/\s*(?:\/\/\/?<|\/\/!<|\/\/|\/~)\s*(.*)$/)?.[0].trim();
	const name = /^[A-Za-z_]\w*/.exec(member)?.[0] ?? member.trim();
	return comment ? `${name} ${comment}` : name;
}

function formatLocalDetail(local: EnforceParserScopeFact, document?: vscode.TextDocument): string {
	const initializedDeclaration = document ? getInitializedDeclarationText(local, document) : undefined;
	if (initializedDeclaration) {
		return initializedDeclaration;
	}
	return `${local.valueType ?? 'var'} ${local.name ?? ''}`.trim();
}

function getInitializedDeclarationText(local: EnforceParserScopeFact, document: vscode.TextDocument): string | undefined {
	if (!local.name || local.range.start.line !== local.range.end.line) {
		return undefined;
	}
	const line = document.lineAt(local.range.start.line).text;
	const nameIndex = line.indexOf(local.name, local.range.start.character);
	if (nameIndex < 0) {
		return undefined;
	}
	const afterName = line.slice(nameIndex + local.name.length);
	if (!/^\s*=/.test(afterName)) {
		return undefined;
	}
	const statementStart = line.slice(0, nameIndex).search(/\S/);
	const semicolonIndex = line.indexOf(';', nameIndex + local.name.length);
	const statementEnd = semicolonIndex >= 0 ? semicolonIndex + 1 : line.length;
	return line.slice(statementStart >= 0 ? statementStart : 0, statementEnd).trim();
}

function referenceKindAt(document: vscode.TextDocument, position: vscode.Position, identity: ResolvedIdentity): CodeReferenceKind {
	const line = document.lineAt(position.line).text;
	const after = line.slice(position.character + identity.name.length);
	if (/^\s*=/.test(after)) {
		return 'write';
	}
	if (identity.kind === 'class' || identity.kind === 'enum') {
		return 'typeUsage';
	}
	if (identity.kind === 'function') {
		return 'call';
	}
	if (identity.kind === 'property') {
		return 'memberAccess';
	}
	if (identity.kind === 'attribute') {
		return 'memberAccess';
	}
	return 'read';
}

function symbolKind(type: EnforceSymbolType): ResolvedIdentityKind {
	switch (type) {
		case 'class': return 'class';
		case 'enum': return 'enum';
		case 'enumValue': return 'enumValue';
		case 'function':
		case 'memberFunction': return 'function';
		case 'property': return 'property';
		case 'macro': return 'macro';
		default: return 'unknown';
	}
}

function nodeKind(kind: string): ResolvedIdentityKind {
	switch (kind) {
		case 'class': return 'class';
		case 'enum': return 'enum';
		case 'enumMember': return 'enumValue';
		case 'function':
		case 'memberFunction':
		case 'constructor':
		case 'destructor': return 'function';
		case 'property': return 'property';
		case 'macro': return 'macro';
		case 'parameter': return 'parameter';
		case 'local':
		case 'foreach': return 'local';
		default: return 'unknown';
	}
}

function isDeclarationNodeKind(kind: string): boolean {
	return ['class', 'enum', 'enumMember', 'function', 'memberFunction', 'constructor', 'destructor', 'property', 'macro', 'parameter', 'local', 'foreach'].includes(kind);
}

function isTypeUsageNodeKind(kind: string): boolean {
	return ['property', 'local', 'parameter', 'foreach', 'function', 'memberFunction', 'newExpression', 'castExpression'].includes(kind);
}

function chooseTypeSymbol(symbols: readonly EnforceSymbol[]): EnforceSymbol | undefined {
	const typeMatches = symbols.filter(symbol => symbol.type === 'class' || symbol.type === 'enum');
	if (typeMatches.length === 0) {
		return undefined;
	}
	const concreteClass = typeMatches.find(symbol => symbol.type === 'class' && symbol.declarationKind !== 'typedef');
	if (concreteClass) {
		return concreteClass;
	}
	const enumSymbol = typeMatches.find(symbol => symbol.type === 'enum');
	if (enumSymbol) {
		return enumSymbol;
	}
	return typeMatches[0];
}

function symbolTypeForNodeKind(kind: string): EnforceSymbol['type'] | undefined {
	switch (kind) {
		case 'class': return 'class';
		case 'enum': return 'enum';
		case 'enumMember': return 'enumValue';
		case 'memberFunction':
		case 'constructor':
		case 'destructor': return 'memberFunction';
		case 'function': return 'function';
		case 'property': return 'property';
		case 'macro': return 'macro';
		default: return undefined;
	}
}

function dedupeReferences(references: CodeReference[]): CodeReference[] {
	const seen = new Set<string>();
	return references.filter(reference => {
		const key = `${reference.location.uri.toString()}:${reference.location.range.start.line}:${reference.location.range.start.character}:${reference.kind}`;
		if (seen.has(key)) {
			return false;
		}
		seen.add(key);
		return true;
	});
}

function dedupeSymbols<T extends EnforceSymbol>(symbols: readonly T[]): T[] {
	const seen = new Set<string>();
	return symbols.filter(symbol => {
		const key = canonicalSymbolKey(symbol);
		if (seen.has(key)) {
			return false;
		}
		seen.add(key);
		return true;
	});
}

function canonicalSymbolKey(symbol: EnforceSymbol): string {
	return [
		canonicalUriKey(symbol.uri),
		symbol.type,
		symbol.containerName ?? '',
		symbol.name,
		symbol.selectionRange.start.line,
		symbol.selectionRange.start.character,
		symbol.selectionRange.end.line,
		symbol.selectionRange.end.character,
	].join('|');
}

function canonicalUriKey(uri: vscode.Uri): string {
	return uri.scheme === 'file' && uri.fsPath
		? uri.fsPath.replace(/\\/g, '/').toLowerCase()
		: uri.toString().toLowerCase();
}

function tokenRangeEqualsIdentity(token: EnforceToken, identity: ResolvedIdentity): boolean {
	const range = tokenRange(token);
	return comparePositions(range.start, identity.range.start) === 0
		&& comparePositions(range.end, identity.range.end) === 0;
}

function tokenRange(token: EnforceToken): EnforceParserRange {
	return {
		start: { line: token.line, character: token.character },
		end: { line: token.endLine, character: token.endCharacter },
	};
}

function previousSignificantToken(tokens: readonly EnforceToken[], startIndex: number): EnforceToken | undefined {
	for (let index = startIndex; index >= 0; index--) {
		const token = tokens[index];
		if (!isTrivia(token)) {
			return token;
		}
	}
	return undefined;
}

function isTrivia(token: EnforceToken): boolean {
	return token.kind === 'whitespace' || token.kind === 'newline' || token.kind === 'comment';
}

function isIdentifierLike(token: EnforceToken): boolean {
	return token.kind === 'identifier' || token.kind === 'keyword';
}

function toParserPosition(position: vscode.Position): EnforceParserPosition {
	return { line: position.line, character: position.character };
}

function fromVscodeRange(range: vscode.Range): LanguageRange {
	return {
		start: { line: range.start.line, character: range.start.character },
		end: { line: range.end.line, character: range.end.character },
	};
}

function rangesEqual(left: LanguageRange, right: LanguageRange): boolean {
	return comparePositions(left.start, right.start) === 0
		&& comparePositions(left.end, right.end) === 0;
}

function rangeContains(range: EnforceParserRange, position: LanguagePosition): boolean {
	if (position.line < range.start.line || position.line > range.end.line) {
		return false;
	}
	if (position.line === range.start.line && position.character < range.start.character) {
		return false;
	}
	return !(position.line === range.end.line && position.character > range.end.character);
}

function rangeSize(range: EnforceParserRange): number {
	return (range.end.line - range.start.line) * 100000 + (range.end.character - range.start.character);
}

function comparePositions(left: LanguagePosition, right: LanguagePosition): number {
	return left.line !== right.line ? left.line - right.line : left.character - right.character;
}

function rangeKey(range: LanguageRange): string {
	return `${range.start.line}:${range.start.character}-${range.end.line}:${range.end.character}`;
}
