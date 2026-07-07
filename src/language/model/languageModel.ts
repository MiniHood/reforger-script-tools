import * as vscode from 'vscode';
import { getParsedDocument, toParserPosition } from '../parser/documentCache';
import {
	getCallAt,
	getEnclosingClass,
	getEnclosingFunction,
	getExpectedType as getParserExpectedType,
	getMemberAccessAt,
	getSwitchContext,
	getTypeUsageAt,
	getVisibleLocals,
	type EnforceExpectedType,
} from '../parser/query';
import type { EnforceParserRange, EnforceParserScopeFact, EnforceSyntaxNode, ParsedEnforceSource } from '../parser/ast';
import { EnforceContainerMemberSymbol, EnforceSymbol, EnforceSymbolIndex } from '../index/symbolIndex';
import type { EnforceToken } from '../parser/tokens';

export type LanguageContextKind =
	| 'argument'
	| 'assignment'
	| 'attribute'
	| 'case'
	| 'classInheritance'
	| 'declaration'
	| 'directive'
	| 'grammar'
	| 'member'
	| 'none'
	| 'override'
	| 'return'
	| 'staticMember'
	| 'symbol'
	| 'type'
	| 'value';

export interface LanguagePosition {
	line: number;
	character: number;
}

export interface LanguageRange {
	start: LanguagePosition;
	end: LanguagePosition;
}

export interface LanguageContext {
	kind: LanguageContextKind;
	prefix: string;
	range: LanguageRange;
	receiver?: string;
	call?: LanguageCallContext;
	expectedType?: LanguageResolvedType;
}

export interface LanguageCallContext {
	name: string;
	receiver?: string;
	argumentIndex: number;
}

export interface LanguageResolvedType {
	name: string;
	genericArgs: LanguageResolvedType[];
	arrayShape?: string;
	accessKind: 'instance' | 'static' | 'type' | 'unknown';
}

export interface LanguageMemberLookupOptions {
	includeStaticInstanceMembers?: boolean;
}

export interface LanguageModel {
	document: vscode.TextDocument;
	parsed: ParsedEnforceSource;
	currentClass(position: vscode.Position): EnforceSyntaxNode | undefined;
	currentFunction(position: vscode.Position): EnforceSyntaxNode | undefined;
	contextAt(position: vscode.Position): LanguageContext;
	expectedType(position: vscode.Position): EnforceExpectedType;
	expectsCallableArgument(position: vscode.Position): boolean;
	callableForwardingArgument(position: vscode.Position): { passthroughParameterCount: number } | undefined;
	visibleLocals(position: vscode.Position): EnforceParserScopeFact[];
	members(receiver: string, position: vscode.Position, options?: LanguageMemberLookupOptions): readonly EnforceContainerMemberSymbol[];
	classAncestorNames(className: string, includeSelf?: boolean): readonly string[];
	resolveTypeOfExpression(expression: string, position: vscode.Position): LanguageResolvedType | undefined;
}

export const grammarKeywords = [
	'class', 'enum', 'modded', 'sealed', 'override', 'private', 'protected', 'public',
	'static', 'const', 'ref', 'autoptr', 'out', 'inout', 'notnull', 'event', 'proto',
	'external', 'native', 'owned', 'volatile', 'if', 'else', 'foreach', 'for', 'while', 'switch',
	'case', 'default', 'return', 'continue', 'break', 'new', 'null', 'true', 'false',
	'this', 'super',
];

export const typeKeywords = ['void', 'bool', 'int', 'float', 'string', 'vector', 'typename', 'array', 'set', 'map'];
export const directiveKeywords = ['#define', '#ifdef', '#ifndef', '#else', '#endif', '#include'];
const typeReceiverMemberContainers = ['Class'];

export function buildLanguageModel(document: vscode.TextDocument, symbolIndex: EnforceSymbolIndex): LanguageModel {
	return new ParserBackedLanguageModel(document, getParsedDocument(document), symbolIndex);
}

class ParserBackedLanguageModel implements LanguageModel {
	constructor(
		readonly document: vscode.TextDocument,
		readonly parsed: ParsedEnforceSource,
		private readonly symbolIndex: EnforceSymbolIndex
	) {}

	currentClass(position: vscode.Position): EnforceSyntaxNode | undefined {
		return getEnclosingClass(this.parsed, toParserPosition(position));
	}

	currentFunction(position: vscode.Position): EnforceSyntaxNode | undefined {
		return getEnclosingFunction(this.parsed, toParserPosition(position));
	}

	expectedType(position: vscode.Position): EnforceExpectedType {
		const parserExpected = getParserExpectedType(this.parsed, toParserPosition(position));
		const enriched = this.getExpectedTypeFromText(position);
		return enriched.valueType || enriched.context !== 'unknown' ? enriched : parserExpected;
	}

	expectsCallableArgument(position: vscode.Position): boolean {
		const expected = this.expectedType(position);
		return expected.context === 'argument'
			&& !!expected.valueType
			&& this.isCallableLikeType(parseType(expected.valueType, 'unknown'));
	}

	callableForwardingArgument(position: vscode.Position): { passthroughParameterCount: number } | undefined {
		const linePrefix = stripLineComment(this.document.lineAt(position.line).text.slice(0, position.character));
		const call = getOpenCallArgument(linePrefix);
		if (!call) {
			return undefined;
		}
		const resolved = this.resolveCallableWithReceiverType(call.name, call.receiver, position);
		const parameters = resolved?.callable.signature ? getParameterDeclarations(resolved.callable.signature) : [];
		const activeParameter = parameters[call.argumentIndex];
		if (!activeParameter || !this.isCallableLikeType(parseType(getDeclarationType(activeParameter) ?? '', 'unknown'))) {
			return undefined;
		}
		const passthroughParameters = parameters.slice(call.argumentIndex + 1).filter(isOptionalVoidPassthroughParameter);
		return passthroughParameters.length > 0 ? { passthroughParameterCount: passthroughParameters.length } : undefined;
	}

	visibleLocals(position: vscode.Position): EnforceParserScopeFact[] {
		return collapseVisibleLocals(getVisibleLocals(this.parsed, toParserPosition(position)));
	}

	contextAt(position: vscode.Position): LanguageContext {
		const emptyRange = toLanguageRange(new vscode.Range(position, position));
		if (this.isIgnored(position)) {
			return { kind: 'none', prefix: '', range: emptyRange };
		}

		const lineText = this.document.lineAt(position.line).text;
		const linePrefix = lineText.slice(0, position.character);
		const prefix = getIdentifierPrefix(linePrefix);
		const range = toLanguageRange(new vscode.Range(position.line, position.character - prefix.length, position.line, position.character));
		const parserPosition = toParserPosition(position);
		const expected = this.expectedType(position);
		const expectedType = expected.valueType ? parseType(expected.valueType, 'unknown') : undefined;

		const directive = /^\s*#\s*([A-Za-z_]*)$/.exec(linePrefix);
		if (directive) {
			return {
				kind: 'directive',
				prefix: directive[1] ? `#${directive[1]}` : '#',
				range: toLanguageRange(new vscode.Range(position.line, position.character - directive[1].length - 1, position.line, position.character)),
			};
		}

		const member = getMemberAccessAt(this.parsed, parserPosition);
		if (member?.receiver && rangeContains(member.range, parserPosition)) {
			return {
				kind: member.accessOperator === '::' || this.isClassLikeReceiver(member.receiver, position) ? 'staticMember' : 'member',
				prefix,
				receiver: member.receiver,
				range,
				expectedType,
			};
		}
		const textMember = getTextMemberAccessContext(linePrefix);
		if (textMember) {
			return {
				kind: textMember.accessOperator === '::' || this.isClassLikeReceiver(textMember.receiver, position) ? 'staticMember' : 'member',
				prefix,
				receiver: textMember.receiver,
				range,
				expectedType,
			};
		}

		if (/^\s*\[[^\]]*[A-Za-z_]\w*$/.test(linePrefix)) {
			return { kind: 'attribute', prefix, range };
		}

		if (/^\s*#\s*(?:ifdef|ifndef)\s+[A-Za-z_]\w*$/.test(linePrefix)) {
			return { kind: 'directive', prefix, range };
		}

		if (/\boverride\b(?:\s+[A-Za-z_]\w*)?\s*$/.test(linePrefix)) {
			return { kind: 'override', prefix, range };
		}

		if (/\bclass\s+[A-Za-z_]\w*(?:\s+(?:extends\s+|:\s*)?|\s*:\s*)([A-Za-z_]\w*)?$/.test(linePrefix)) {
			return { kind: 'classInheritance', prefix, range };
		}

		if (getSwitchContext(this.parsed, parserPosition) && /^\s*case\b/.test(linePrefix.trimStart())) {
			return { kind: 'case', prefix, range, expectedType };
		}

		const activeCall = getOpenCallArgument(linePrefix);
		if (activeCall || getCallAt(this.parsed, parserPosition) || expected.context === 'argument') {
			return { kind: 'argument', prefix, range, expectedType, call: activeCall };
		}

		if (expected.context === 'return') {
			return { kind: 'return', prefix, range, expectedType };
		}
		if (expected.context === 'assignment') {
			return { kind: 'assignment', prefix, range, expectedType };
		}

		if (this.currentFunction(position) && isBareValueExpressionPrefix(linePrefix)) {
			return { kind: 'value', prefix, range, expectedType };
		}

		const typeUsage = getTypeUsageAt(this.parsed, parserPosition);
		if (typeUsage && ['property', 'local', 'parameter', 'foreach', 'function', 'memberFunction', 'newExpression', 'castExpression'].includes(typeUsage.kind)) {
			return { kind: 'declaration', prefix, range, expectedType };
		}

		if (isGrammarPrefix(prefix)) {
			return { kind: 'grammar', prefix, range };
		}

		if (/\bnew\s+[A-Za-z_]\w*$/.test(linePrefix) || isTypePosition(linePrefix)) {
			return { kind: 'type', prefix, range, expectedType };
		}

		if (prefix.length >= 2) {
			return { kind: 'symbol', prefix, range, expectedType };
		}

		return { kind: 'none', prefix, range, expectedType };
	}

	members(receiver: string, position: vscode.Position, options: LanguageMemberLookupOptions = {}): readonly EnforceContainerMemberSymbol[] {
		const receiverType = this.resolveTypeOfExpression(receiver, position);
		if (!receiverType) {
			return [];
		}

		const containers = this.memberContainersForType(receiverType);
		const members = this.filterAccessibleMembers(this.getContainerMemberSymbolsForContainers(containers), position);
		if (options.includeStaticInstanceMembers) {
			return members;
		}
		if (receiverType.accessKind === 'static' || receiverType.accessKind === 'type') {
			return members.filter(member => member.modifiers?.includes('static') || member.name === 'Cast');
		}
		return members.filter(member => !member.modifiers?.includes('static'));
	}

	resolveTypeOfExpression(expression: string, position: vscode.Position): LanguageResolvedType | undefined {
		const normalized = expression.trim();
		if (!normalized) {
			return undefined;
		}
		const parenthesized = unwrapParenthesized(normalized);
		if (parenthesized !== normalized) {
			return this.resolveTypeOfExpression(parenthesized, position);
		}
		if (normalized === 'this') {
			const className = this.currentClass(position)?.name;
			return className ? parseType(className, 'instance') : undefined;
		}
		if (normalized === 'super') {
			const className = this.currentClass(position)?.name;
			const base = className ? this.baseClassName(className) : undefined;
			return base ? parseType(base, 'instance') : undefined;
		}
		if (/^new\s+/.test(normalized)) {
			const typeName = /^new\s+([A-Za-z_]\w*(?:\s*<.*>)?)/.exec(normalized)?.[1];
			return typeName ? parseType(typeName, 'instance') : undefined;
		}

		const cast = /^([A-Za-z_]\w*)\s*\.\s*Cast\s*\(/.exec(normalized);
		if (cast && this.symbolIndex.getClassSymbol(cast[1])) {
			return parseType(cast[1], 'instance');
		}

		const local = this.visibleLocals(position).find(value => value.name === normalized);
		if (local?.valueType) {
			return parseType(local.valueType, 'instance');
		}
		const currentMember = this.findCurrentClassMember(normalized, position);
		if (currentMember?.signature) {
			return parseType(currentMember.type === 'memberFunction' ? getReturnType(currentMember.signature) ?? '' : getDeclarationType(currentMember.signature) ?? '', 'instance');
		}
		if (this.symbolIndex.getClassSymbol(normalized) || this.symbolIndex.getEnumSymbols().some(symbol => symbol.name === normalized)) {
			return parseType(normalized, 'type');
		}

		const chain = splitMemberChain(normalized);
		if (chain.length > 1) {
			return this.resolveMemberChainType(chain, position);
		}

		const callName = /^([A-Za-z_]\w*)\s*\(/.exec(normalized)?.[1];
		if (callName) {
			const callable = [
				...this.findCurrentClassMembers(callName, position),
				...this.symbolIndex.getFunctionSymbols().filter(symbol => symbol.name === callName),
			].find(symbol => symbol.signature);
			const returnType = callable?.signature ? getReturnType(callable.signature) : undefined;
			return returnType ? parseType(returnType, 'instance') : undefined;
		}

		return undefined;
	}

	private getExpectedTypeFromText(position: vscode.Position): EnforceExpectedType {
		const linePrefix = stripLineComment(this.document.lineAt(position.line).text.slice(0, position.character));
		if (/\breturn\b/.test(linePrefix)) {
			return { context: 'return', valueType: getReturnType(this.currentFunction(position)?.signature ?? '') };
		}
		const assignment = getAssignmentLeft(linePrefix);
		if (assignment) {
			return { context: 'assignment', valueType: this.resolveAssignmentTargetType(assignment, position)?.name };
		}
		const call = getOpenCallArgument(linePrefix);
		if (call) {
			const resolved = this.resolveCallableWithReceiverType(call.name, call.receiver, position);
			const parameterType = resolved?.callable.signature ? getParameterTypes(resolved.callable.signature)[call.argumentIndex] : undefined;
			const substituted = parameterType ? this.substituteGenericType(parameterType, resolved?.receiverType, resolved?.callable.containerName) : undefined;
			return { context: 'argument', valueType: substituted ?? parameterType };
		}
		if (/^\s*case\b/.test(linePrefix.trimStart())) {
			const switchExpression = getSwitchContext(this.parsed, toParserPosition(position))?.expression;
			return { context: 'case', valueType: switchExpression ? this.resolveTypeOfExpression(switchExpression, position)?.name : undefined };
		}
		return { context: 'unknown' };
	}

	private resolveAssignmentTargetType(target: string, position: vscode.Position): LanguageResolvedType | undefined {
		const declarationType = getDeclarationTargetType(target);
		if (declarationType) {
			return parseType(declarationType, 'instance');
		}
		const memberChain = splitMemberChain(target);
		if (memberChain.length > 1) {
			const receiver = memberChain.slice(0, -1).join('.');
			const memberName = cleanCallSegment(memberChain[memberChain.length - 1]);
			const member = this.members(receiver, position).find(symbol => symbol.name === memberName && symbol.type === 'property');
			return member?.signature ? parseType(getDeclarationType(member.signature) ?? '', 'instance') : undefined;
		}
		const local = this.visibleLocals(position).find(value => value.name === target.trim());
		if (local?.valueType) {
			return parseType(local.valueType, 'instance');
		}
		const property = this.findCurrentClassMember(target.trim(), position);
		return property?.signature ? parseType(getDeclarationType(property.signature) ?? '', 'instance') : undefined;
	}

	private resolveCallableWithReceiverType(name: string, receiver: string | undefined, position: vscode.Position): { callable: EnforceSymbol; receiverType?: LanguageResolvedType } | undefined {
		if (receiver) {
			const receiverType = this.resolveTypeOfExpression(receiver, position);
			const callable = receiverType
				? this.filterAccessibleMembers(this.getContainerMemberSymbolsForContainers(this.memberContainersForType(receiverType)), position)
					.find(symbol => symbol.name === name && symbol.type === 'memberFunction')
				: undefined;
			return callable ? { callable, receiverType } : undefined;
		}
		const callable = [
			...this.findCurrentClassMembers(name, position),
			...this.symbolIndex.getFunctionSymbols().filter(symbol => symbol.name === name),
			...this.symbolIndex.getContainerMemberSymbolsForContainersAndName([name], name).filter(symbol => symbol.declarationKind === 'constructor'),
		].find(symbol => symbol.type === 'memberFunction' || symbol.type === 'function');
		return callable ? { callable } : undefined;
	}

	private resolveMemberChainType(chain: string[], position: vscode.Position): LanguageResolvedType | undefined {
		let current = this.resolveTypeOfExpression(chain[0], position);
		for (let index = 1; index < chain.length && current; index++) {
			const rawSegment = chain[index];
			const memberName = cleanCallSegment(rawSegment);
			const containers = this.memberContainersForType(current);
			const member = this.getContainerMemberSymbolsForContainersAndName(containers, memberName)[0];
			if (!member?.signature) {
				return undefined;
			}
			const typeName = member.type === 'memberFunction' || rawSegment.includes('(')
				? getReturnType(member.signature)
				: getDeclarationType(member.signature);
			const substituted = typeName ? this.substituteGenericType(typeName, current, member.containerName) : undefined;
			current = substituted ? parseType(substituted, 'instance') : typeName ? parseType(typeName, 'instance') : undefined;
		}
		return current;
	}

	private substituteGenericType(typeName: string, receiverType: LanguageResolvedType | undefined, memberContainerName: string | undefined): string | undefined {
		const parsed = parseType(typeName, 'unknown');
		if (!parsed || !receiverType || !memberContainerName) {
			return typeName;
		}
		const constructedContainer = this.constructedContainerType(receiverType, memberContainerName);
		if (!constructedContainer?.genericArgs.length) {
			return typeName;
		}
		const parameters = this.genericParameterNames(memberContainerName);
		if (parameters.length === 0) {
			return typeName;
		}
		const genericMap = new Map<string, LanguageResolvedType>();
		parameters.forEach((parameter, index) => {
			const argument = constructedContainer.genericArgs[index];
			if (argument) {
				genericMap.set(parameter, argument);
			}
		});
		const substituted = substituteResolvedType(parsed, genericMap);
		return substituted ? typeToString(substituted) : typeName;
	}

	private constructedContainerType(receiverType: LanguageResolvedType, containerName: string): LanguageResolvedType | undefined {
		if (receiverType.name === containerName) {
			return receiverType;
		}
		for (const ancestorName of this.classAncestorNames(receiverType.name, true)) {
			const parsed = parseType(ancestorName, receiverType.accessKind);
			if (parsed?.name === containerName) {
				return parsed;
			}
		}
		return undefined;
	}

	private genericParameterNames(className: string): string[] {
		const symbol = this.parsed.symbols.find(candidate => candidate.type === 'class' && candidate.name === className)
			?? this.symbolIndex.getClassSymbol(className);
		const signature = symbol?.signature ?? '';
		const escapedClassName = escapeRegExp(className);
		const generic = new RegExp(`\\b${escapedClassName}\\s*<([^>]*)>`).exec(signature)?.[1];
		if (!generic) {
			return [];
		}
		return splitTopLevel(generic, ',')
			.map(parameter => /([A-Za-z_]\w*)\s*$/.exec(parameter.trim())?.[1])
			.filter((parameter): parameter is string => !!parameter);
	}

	private isCallableLikeType(type: LanguageResolvedType | undefined): boolean {
		if (!type) {
			return false;
		}
		if (type.name === 'func') {
			return true;
		}
		return this.findTypeAliasSymbols(type.name).some(symbol => /^typedef\s+func\b/.test(symbol.signature ?? ''));
	}

	private findTypeAliasSymbols(name: string): EnforceSymbol[] {
		const parsed = this.parsed.symbols.filter(symbol => symbol.name === name && symbol.declarationKind === 'typedef');
		const indexed = this.symbolIndex.find(name).filter(symbol => symbol.name === name && symbol.declarationKind === 'typedef');
		return dedupeSymbols([...parsed, ...indexed]);
	}

	private findCurrentClassMember(name: string | undefined, position: vscode.Position): EnforceContainerMemberSymbol | undefined {
		return this.findCurrentClassMembers(name, position)[0];
	}

	private findCurrentClassMembers(name: string | undefined, position: vscode.Position): EnforceContainerMemberSymbol[] {
		const className = this.currentClass(position)?.name;
		if (!className) {
			return [];
		}
		const containers = this.classAncestorNames(className, true);
		const members = name
			? this.getContainerMemberSymbolsForContainersAndName(containers, name)
			: this.getContainerMemberSymbolsForContainers(containers);
		return [...members];
	}

	private memberContainersForType(type: LanguageResolvedType): readonly string[] {
		const containers = [...this.classAncestorNames(type.name, true)];
		if (type.accessKind === 'static' || type.accessKind === 'type') {
			for (const container of typeReceiverMemberContainers) {
				if (!containers.includes(container) && this.hasClassSymbol(container)) {
					containers.push(container);
				}
			}
		}
		return containers;
	}

	classAncestorNames(className: string, includeSelf = false): readonly string[] {
		const ancestors: string[] = [];
		const seen = new Set<string>();
		let current: string | undefined = includeSelf ? className : this.baseClassName(className);
		while (current && !seen.has(current)) {
			seen.add(current);
			ancestors.push(current);
			const containerName = genericContainerName(current);
			if (containerName && containerName !== current && !seen.has(containerName)) {
				seen.add(containerName);
				ancestors.push(containerName);
			}
			current = this.baseClassName(containerName ?? current);
		}
		return ancestors;
	}

	private baseClassName(className: string): string | undefined {
		const parsedBase = this.parsed.symbols.find(symbol =>
			symbol.type === 'class'
			&& symbol.name === className
			&& symbol.baseClassName
		)?.baseClassName;
		return parsedBase ?? this.symbolIndex.getBaseClassName(className);
	}

	private hasClassSymbol(className: string): boolean {
		return this.parsed.symbols.some(symbol => symbol.type === 'class' && symbol.name === className)
			|| Boolean(this.symbolIndex.getClassSymbol(className));
	}

	private getContainerMemberSymbolsForContainers(containers: readonly string[]): EnforceContainerMemberSymbol[] {
		const parsedMembers = this.parsed.symbols.filter((symbol): symbol is EnforceContainerMemberSymbol =>
			(symbol.type === 'memberFunction' || symbol.type === 'property')
			&& symbol.containerName !== undefined
			&& containers.includes(symbol.containerName)
		);
		return dedupeSymbols([
			...parsedMembers,
			...this.symbolIndex.getContainerMemberSymbolsForContainers(containers),
		]) as EnforceContainerMemberSymbol[];
	}

	private getContainerMemberSymbolsForContainersAndName(containers: readonly string[], name: string): EnforceContainerMemberSymbol[] {
		const parsedMembers = this.parsed.symbols.filter((symbol): symbol is EnforceContainerMemberSymbol =>
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

	private filterAccessibleMembers(members: readonly EnforceContainerMemberSymbol[], position: vscode.Position): EnforceContainerMemberSymbol[] {
		const currentClass = this.currentClass(position)?.name;
		const currentAncestors = currentClass ? this.classAncestorNames(currentClass, false) : [];
		return members.filter(member => {
			if (!member.modifiers?.includes('private') && !member.modifiers?.includes('protected')) {
				return true;
			}
			if (!currentClass || !member.containerName) {
				return false;
			}
			if (member.containerName === currentClass) {
				return true;
			}
			return member.modifiers?.includes('protected') && currentAncestors.includes(member.containerName);
		});
	}

	private isClassLikeReceiver(receiver: string, position: vscode.Position): boolean {
		return this.resolveTypeOfExpression(receiver, position)?.accessKind === 'type';
	}

	private isIgnored(position: vscode.Position): boolean {
		const parserPosition = toParserPosition(position);
		return this.parsed.tokens.some(token =>
			(token.kind === 'comment' || token.kind === 'string')
			&& rangeContains(tokenRange(token), parserPosition)
		);
	}

}

function getIdentifierPrefix(value: string): string {
	return /[A-Za-z_][A-Za-z0-9_]*$/.exec(value)?.[0] ?? '';
}

function getTextMemberAccessContext(linePrefix: string): { receiver: string; accessOperator: '.' | '::' } | undefined {
	const match = /([A-Za-z_][A-Za-z0-9_]*(?:(?:\s*\([^()]*\)|\s*(?:\.|::)\s*[A-Za-z_][A-Za-z0-9_]*)*)?)\s*(\.|::)\s*[A-Za-z_][A-Za-z0-9_]*$/.exec(linePrefix)
		?? /([A-Za-z_][A-Za-z0-9_]*(?:(?:\s*\([^()]*\)|\s*(?:\.|::)\s*[A-Za-z_][A-Za-z0-9_]*)*)?)\s*(\.|::)\s*$/.exec(linePrefix);
	if (!match) {
		return undefined;
	}
	return {
		receiver: match[1].trim(),
		accessOperator: match[2] as '.' | '::',
	};
}

function isGrammarPrefix(prefix: string): boolean {
	const normalized = prefix.toLowerCase();
	return normalized.length >= 2 && [...grammarKeywords, ...typeKeywords].some(keyword => keyword.startsWith(normalized));
}

function isTypePosition(linePrefix: string): boolean {
	return /(?:^\s*|[<,(]\s*)(?:ref\s+|autoptr\s+|notnull\s+|out\s+|inout\s+|const\s+)*[A-Za-z_]\w*$/.test(linePrefix)
		&& !/[=+\-*/%&|!?:]\s*[A-Za-z_]\w*$/.test(linePrefix);
}

function isBareValueExpressionPrefix(linePrefix: string): boolean {
	return /^\s*[A-Za-z_]\w*$/.test(linePrefix);
}

function parseType(typeName: string, accessKind: LanguageResolvedType['accessKind']): LanguageResolvedType | undefined {
	const normalized = normalizeTypeName(typeName);
	if (!normalized) {
		return undefined;
	}
	const arrayShape = /\[[^\]]*\]\s*$/.exec(normalized)?.[0];
	const withoutArray = arrayShape ? normalized.slice(0, -arrayShape.length) : normalized;
	const genericStart = withoutArray.indexOf('<');
	if (genericStart >= 0 && withoutArray.endsWith('>')) {
		const name = withoutArray.slice(0, genericStart);
		const genericText = withoutArray.slice(genericStart + 1, -1);
		return {
			name,
			genericArgs: splitTopLevel(genericText, ',').map(part => parseType(part, 'instance')).filter((part): part is LanguageResolvedType => part !== undefined),
			arrayShape,
			accessKind,
		};
	}
	return { name: withoutArray, genericArgs: [], arrayShape, accessKind };
}

function normalizeTypeName(typeName: string): string {
	return typeName
		.replace(/\b(?:autoptr|const|event|external|inout|modded|native|notnull|out|owned|override|private|protected|proto|public|ref|sealed|static|volatile)\b/g, '')
		.replace(/\s+/g, ' ')
		.trim()
		.replace(/\s*<\s*/g, '<')
		.replace(/\s*>\s*/g, '>')
		.replace(/\s*,\s*/g, ', ');
}

function genericContainerName(typeName: string): string | undefined {
	const parsed = parseType(typeName, 'instance');
	return parsed?.genericArgs.length ? parsed.name : undefined;
}

function substituteResolvedType(type: LanguageResolvedType, genericMap: ReadonlyMap<string, LanguageResolvedType>): LanguageResolvedType | undefined {
	const mapped = genericMap.get(type.name);
	if (mapped && type.genericArgs.length === 0 && !type.arrayShape) {
		return mapped;
	}
	return {
		...type,
		genericArgs: type.genericArgs.map(argument => substituteResolvedType(argument, genericMap) ?? argument),
	};
}

function typeToString(type: LanguageResolvedType): string {
	const genericText = type.genericArgs.length
		? `<${type.genericArgs.map(typeToString).join(', ')}>`
		: '';
	return `${type.name}${genericText}${type.arrayShape ?? ''}`;
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function unwrapParenthesized(value: string): string {
	let current = value.trim();
	while (current.startsWith('(') && current.endsWith(')')) {
		const inner = current.slice(1, -1).trim();
		if (!inner || splitTopLevel(inner, ',').length > 1) {
			break;
		}
		current = inner;
	}
	return current;
}

function getReturnType(signature: string): string | undefined {
	if (!signature.includes('(')) {
		return undefined;
	}
	const beforeParen = signature.slice(0, signature.indexOf('(')).trim();
	const parts = beforeParen.split(/\s+/).filter(Boolean);
	return parts.length > 1 ? normalizeTypeName(parts[parts.length - 2]) : undefined;
}

function getDeclarationType(signature: string): string | undefined {
	const beforeInitializer = signature.split(/[=;]/)[0].trim();
	const parts = beforeInitializer.split(/\s+/).filter(Boolean);
	if (parts.length < 2) {
		return undefined;
	}
	return normalizeTypeName(parts.slice(0, -1).join(' '));
}

function getParameterTypes(signature: string): string[] {
	return getParameterDeclarations(signature)
		.map(parameter => getDeclarationType(parameter.trim()) ?? '')
		.filter(Boolean);
}

function getParameterDeclarations(signature: string): string[] {
	const start = signature.indexOf('(');
	const end = signature.lastIndexOf(')');
	if (start < 0 || end < start) {
		return [];
	}
	return splitTopLevel(signature.slice(start + 1, end), ',')
		.map(parameter => parameter.trim())
		.filter(Boolean);
}

function isOptionalVoidPassthroughParameter(parameter: string): boolean {
	const declarationType = getDeclarationType(parameter);
	return declarationType === 'void' && /=/.test(parameter);
}

function collapseVisibleLocals(locals: EnforceParserScopeFact[]): EnforceParserScopeFact[] {
	const byName = new Map<string, EnforceParserScopeFact>();
	for (const local of locals) {
		if (local.name) {
			byName.set(local.name, local);
		}
	}
	return [...byName.values()];
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

function rangeContains(range: EnforceParserRange, position: LanguagePosition): boolean {
	if (position.line < range.start.line || position.line > range.end.line) {
		return false;
	}
	if (position.line === range.start.line && position.character < range.start.character) {
		return false;
	}
	return !(position.line === range.end.line && position.character > range.end.character);
}

function toLanguageRange(range: vscode.Range): LanguageRange {
	return {
		start: { line: range.start.line, character: range.start.character },
		end: { line: range.end.line, character: range.end.character },
	};
}

function tokenRange(token: EnforceToken): EnforceParserRange {
	return {
		start: { line: token.line, character: token.character },
		end: { line: token.endLine, character: token.endCharacter },
	};
}

function stripLineComment(value: string): string {
	let inString: string | undefined;
	for (let index = 0; index < value.length - 1; index++) {
		const char = value[index];
		if (inString) {
			if (char === '\\') {
				index++;
				continue;
			}
			if (char === inString) {
				inString = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			inString = char;
			continue;
		}
		if (char === '/' && value[index + 1] === '/') {
			return value.slice(0, index);
		}
	}
	return value;
}

function getAssignmentLeft(linePrefix: string): string | undefined {
	const equalsIndex = findTopLevelAssignment(linePrefix);
	if (equalsIndex < 0) {
		return undefined;
	}
	return linePrefix.slice(0, equalsIndex).trim();
}

function getDeclarationTargetType(target: string): string | undefined {
	const match = /^(?:(?:private|protected|public|static|const|ref|autoptr|notnull|out|inout)\s+)*(.+?)\s+[A-Za-z_]\w*$/.exec(target.trim());
	return match?.[1] ? normalizeTypeName(match[1]) : undefined;
}

function getOpenCallArgument(linePrefix: string): { name: string; receiver?: string; argumentIndex: number } | undefined {
	const openIndex = linePrefix.lastIndexOf('(');
	if (openIndex < 0) {
		return undefined;
	}
	const before = linePrefix.slice(0, openIndex).trim();
	const nameMatch = /(?:(.+)\.)?([A-Za-z_]\w*)$/.exec(before);
	if (!nameMatch) {
		return undefined;
	}
	return {
		receiver: nameMatch[1]?.trim(),
		name: nameMatch[2],
		argumentIndex: linePrefix.slice(openIndex + 1).trim() ? splitTopLevel(linePrefix.slice(openIndex + 1), ',').length - 1 : 0,
	};
}

function findTopLevelAssignment(value: string): number {
	let depth = 0;
	for (let index = value.length - 1; index >= 0; index--) {
		const char = value[index];
		if (char === ')' || char === ']' || char === '>') {
			depth++;
		} else if (char === '(' || char === '[' || char === '<') {
			depth = Math.max(0, depth - 1);
		} else if (char === '=' && depth === 0 && value[index - 1] !== '=' && value[index + 1] !== '=') {
			return index;
		}
	}
	return -1;
}

function splitMemberChain(value: string): string[] {
	return splitTopLevel(value, '.').map(part => part.trim()).filter(Boolean);
}

function cleanCallSegment(value: string): string {
	return /^([A-Za-z_]\w*)/.exec(value.trim())?.[1] ?? value.trim();
}

function splitTopLevel(value: string, separator: string): string[] {
	const parts: string[] = [];
	let depth = 0;
	let start = 0;
	let inString: string | undefined;
	for (let index = 0; index < value.length; index++) {
		const char = value[index];
		if (inString) {
			if (char === '\\') {
				index++;
			} else if (char === inString) {
				inString = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			inString = char;
			continue;
		}
		if (char === '(' || char === '[' || char === '<') {
			depth++;
		} else if (char === ')' || char === ']' || char === '>') {
			depth = Math.max(0, depth - 1);
		} else if (char === separator && depth === 0) {
			parts.push(value.slice(start, index).trim());
			start = index + 1;
		}
	}
	parts.push(value.slice(start).trim());
	return parts.filter(Boolean);
}

