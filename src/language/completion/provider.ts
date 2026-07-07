import * as vscode from 'vscode';
import { EnforceContainerMemberSymbol, EnforceSymbol, EnforceSymbolIndex } from '../index/symbolIndex';
import { buildLanguageModel, grammarKeywords, typeKeywords } from '../model/languageModel';
import { getParsedDocument, toParserPosition } from '../parser/documentCache';
import { getEnclosingFunction, isIgnoredPosition } from '../parser/query';

const maxCompletionItems = 100;

export interface BasicCompletionCandidate {
	label: string;
	detail: string;
	kind: vscode.CompletionItemKind;
	insertText?: string | vscode.SnippetString;
	command?: vscode.Command;
	filterText?: string;
	range?: CompletionRange;
	preselect?: boolean;
	sortText: string;
	searchText?: string;
	valueType?: string;
	ranking?: Partial<CompletionRankingDebug>;
}

export interface CompletionRankingDebug {
	tier: number;
	tierName: string;
	source: string;
	reason: string;
	matchScore?: number;
	valueType?: string;
	expectedType?: string;
	typeContext?: string;
}

interface CompletionItemData extends Partial<CompletionRankingDebug> {
	finalRank?: number;
	sortText?: string;
	filterText?: string;
	completionCategory?: string;
	completionText?: string;
}

interface CheapCompletionContext {
	prefix: string;
	directive: boolean;
	operator: boolean;
	conditionOperandOperator: boolean;
	conditionAssertionValue: boolean;
	attribute: boolean;
	attributeIncludesBracket: boolean;
	attributeBare: boolean;
	enumPlaceholderReceiver?: string;
	enumPlaceholderFilterText?: string;
	ignored: boolean;
	range: CompletionRange;
	linePrefix: string;
}

export class BasicCompletionProvider implements vscode.CompletionItemProvider {
	private readonly probeCandidates = getProbeCandidates();
	private readonly grammarCandidates = getGrammarCandidates();
	private readonly directiveCandidates = getDirectiveCandidates();
	private readonly operatorCandidates = getOperatorCandidates();
	private readonly conditionOperatorCandidates = getConditionOperatorCandidates();
	private readonly conditionAssertionValueCandidates = getConditionAssertionValueCandidates();

	constructor(private readonly symbolIndex?: EnforceSymbolIndex) {}

	provideCompletionItems(
		document: vscode.TextDocument,
		position: vscode.Position,
		token: vscode.CancellationToken,
		completionContext?: vscode.CompletionContext
	): vscode.CompletionList | undefined {
		const trace = (result: vscode.CompletionList | undefined, reason: string, prefix = '', linePrefix = ''): vscode.CompletionList | undefined => {
			return result;
		};
		if (token.isCancellationRequested) {
			return trace(undefined, 'cancelled');
		}

		const context = getCheapCompletionContext(document, position);
		if (context.ignored) {
			return trace(new vscode.CompletionList([], false), 'ignored comment/string', context.prefix, context.linePrefix);
		}

		if (context.directive) {
			return trace(incompleteCompletionList(toCompletionItems(withRanking(filterCandidates(this.directiveCandidates, context.prefix), 1, 'hard context', 'directive', 'line is a compiler directive'), context.range, context.prefix)), 'directive', context.prefix, context.linePrefix);
		}

		if (context.conditionOperandOperator) {
			const insertRange = new vscode.Range(position, position);
			return trace(incompleteCompletionList(toCompletionItems(withRanking(this.conditionOperatorCandidates, 1, 'hard context', 'operator', 'condition operand is complete'), insertRange, '')), 'condition operand operator', context.prefix, context.linePrefix);
		}

		if (context.operator) {
			return trace(incompleteCompletionList(toCompletionItems(withRanking(filterCandidates(this.operatorCandidates, context.prefix), 1, 'hard context', 'operator', 'cursor is on an operator prefix'), context.range, context.prefix)), 'operator', context.prefix, context.linePrefix);
		}

		const strictGrammarCandidates = filterStrictPrefixCandidates(this.grammarCandidates, context.prefix);
		if (!this.symbolIndex && strictGrammarCandidates.length > 0) {
			return trace(incompleteCompletionList(toCompletionItems(withRanking(strictGrammarCandidates, 5, 'strong syntax/context matches', 'grammar', 'static grammar without symbol index'), context.range, context.prefix)), 'strict grammar without index', context.prefix, context.linePrefix);
		}
		if (isDeclarationLeadingGrammarPrefix(document, position, context.linePrefix, context.prefix) && strictGrammarCandidates.length > 0) {
			return trace(incompleteCompletionList(toCompletionItems(withRanking(strictGrammarCandidates, 1, 'hard context', 'grammar', 'declaration-leading grammar prefix'), context.range, context.prefix)), 'declaration-leading grammar', context.prefix, context.linePrefix);
		}

		if (context.attribute && this.symbolIndex) {
			const attributeCandidates = this.getAttributeCandidates(context.prefix, context.attributeIncludesBracket, context.attributeBare);
			if (attributeCandidates.length > 0) {
				return trace(incompleteCompletionList(toCompletionItems(withRanking(attributeCandidates, 1, 'hard context', 'attribute', 'decorator context'), context.range, context.prefix)), 'attribute decorator context', context.prefix, context.linePrefix);
			}
		}

		if (context.enumPlaceholderReceiver && this.symbolIndex) {
			const enumPlaceholderCandidates = this.getQualifiedEnumPlaceholderCandidates(context.enumPlaceholderReceiver, context.enumPlaceholderFilterText);
			if (enumPlaceholderCandidates.length > 0) {
				return trace(incompleteCompletionList(toCompletionItems(withRanking(enumPlaceholderCandidates, 1, 'hard context', 'enum placeholder', 'selected qualified enum argument placeholder'), context.range, context.prefix)), 'qualified enum placeholder', context.prefix, context.linePrefix);
			}
		}

		const overrideSignatureContext = isAfterOverrideKeyword(context.linePrefix);
		const typeContext = getCheapTypeCompletionContext(document.lineAt(position.line).text.slice(0, position.character));
		const typeCandidates = this.getTypeCandidates(typeContext, context.prefix, document, position, overrideSignatureContext, isCheapValueExpressionPrefix(context.linePrefix));
		const refreshingIndex = isIndexRefreshing(this.symbolIndex);
		const model = !refreshingIndex && this.symbolIndex ? buildLanguageModel(document, this.symbolIndex) : undefined;
		const modelContext = model?.contextAt(position);
		const allowEmptyValueScope = context.prefix.length === 0 && isValueCompletionContext(modelContext?.kind);
		const insertCallableReference = model?.expectsCallableArgument(position) ?? false;
		const callableForwardingArgument = model?.callableForwardingArgument(position);
		const overrideSignatureCandidates = this.getOverrideSignatureCandidates(model, position, context.prefix, context.linePrefix);
		const scopeCandidates = this.getScopeCandidates(model, position, context.prefix, allowEmptyValueScope);
		const classMemberCandidates = this.getCurrentClassMemberCandidates(model, position, context.prefix, allowEmptyValueScope, modelContext?.kind === 'argument' ? modelContext.call?.name : undefined, insertCallableReference, callableForwardingArgument?.passthroughParameterCount);
		const globalFunctionCandidates = this.getGlobalFunctionCandidates(context.prefix, modelContext?.kind);
		const hasValueLayerCandidates = scopeCandidates.length > 0 || classMemberCandidates.length > 0 || globalFunctionCandidates.length > 0;
		if (!overrideSignatureContext && typeContext.strong && !hasValueLayerCandidates && typeCandidates.length > 0) {
			const rankedTypes = withRanking(typeCandidates, 1, 'hard context', 'indexed type', `strong type context: ${typeContext.kind}`, { typeContext: typeContext.kind });
			const rankedGrammar = withRanking(filterCandidates(this.grammarCandidates, context.prefix), 1, 'hard context', 'grammar', `grammar match in strong type context: ${typeContext.kind}`, { typeContext: typeContext.kind });
			const hardContextCandidates = rankEquivalentCandidates(mergeHardContextCandidates(rankedTypes, rankedGrammar, context.prefix), context.prefix);
			return trace(incompleteCompletionList(toCompletionItems(hardContextCandidates, context.range, context.prefix)), `strong type context ${typeContext.kind}`, context.prefix, context.linePrefix);
		}

		if (refreshingIndex) {
			return trace(incompleteCompletionList([]), 'index refreshing', context.prefix, context.linePrefix);
		}

		if (context.conditionAssertionValue) {
			const insertRange = new vscode.Range(position, position);
			const conditionValueCandidates = this.getConditionAssertionValueCandidates(model, position);
			return trace(incompleteCompletionList(toCompletionItems(withRanking(conditionValueCandidates, 1, 'hard context', 'condition value', 'condition assertion operator is complete'), insertRange, '')), 'condition assertion value', context.prefix, context.linePrefix);
		}
		const expected = model?.expectedType(position);
		const expectedType = normalizeTypeName(expected?.valueType);
		const expectedEnumArgumentCandidates = this.getExpectedEnumArgumentCandidates(expectedType, expected?.context, context.linePrefix, position, context.prefix);
		const memberAccessCandidates = this.getMemberAccessCandidates(model, position, context.prefix, modelContext, context.linePrefix, insertCallableReference, callableForwardingArgument?.passthroughParameterCount);
		const explicitEnumMemberAccess = isMemberAccessContext(modelContext) && this.isKnownEnumReceiver(modelContext.receiver);
		const completeExpectedEnumMember = explicitEnumMemberAccess
			&& expectedEnumArgumentCandidates.length > 0
			&& context.prefix.length > 0
			&& this.isKnownEnumValue(modelContext.receiver, context.prefix);
		if (memberAccessCandidates && (expectedEnumArgumentCandidates.length === 0 || (explicitEnumMemberAccess && !completeExpectedEnumMember))) {
			return trace(incompleteCompletionList(toCompletionItems(withRanking(memberAccessCandidates, 1, 'hard context', 'model member access', 'receiver member access'), context.range, context.prefix)), 'member access', context.prefix, context.linePrefix);
		}
		const grammarCandidates = filterCandidates(this.grammarCandidates, context.prefix);
		const rankedExpectedEnumArgumentCandidates = withRanking(expectedEnumArgumentCandidates, 2, 'expected-type matches', 'expected enum', `argument expects enum ${expectedType}`, { expectedType });
		const typedSearchCandidates = expectedType && context.prefix
			? [...scopeCandidates, ...classMemberCandidates, ...globalFunctionCandidates, ...typeCandidates]
				.filter(candidate => candidateSearchMatchScore(candidate, context.prefix) !== undefined)
			: [];
		const expectedTypeCandidates = expectedType
			? withRanking(
				dedupeCandidates([
					...[...scopeCandidates, ...classMemberCandidates, ...globalFunctionCandidates, ...typeCandidates].filter(candidate => candidateMatchesExpectedType(candidate, expectedType, this.symbolIndex)),
					...typedSearchCandidates,
				]),
				2,
				'expected-type matches',
				'model expected type',
				`candidate value type or typed name matches expected type ${expectedType}`,
				{ expectedType }
			)
			: [];
		const rankedExpectedCandidates = rankExpectedTypeCandidates(rankedExpectedEnumArgumentCandidates, expectedTypeCandidates, context.prefix);
		const rankedTypeCandidates = withRanking(typeCandidates, overrideSignatureContext ? 5 : 6, overrideSignatureContext ? 'strong syntax/context matches' : 'broad indexed symbols', 'symbol index', overrideSignatureContext ? 'indexed type is valid after override keyword' : `indexed ${typeContext.classOnly ? 'class' : 'type'} match`, { typeContext: overrideSignatureContext ? 'override' : typeContext.kind });
		const rankedGlobalFunctionCandidates = withRanking(globalFunctionCandidates, 6, 'broad indexed symbols', 'symbol index', 'indexed global function match', { typeContext: modelContext?.kind });
		const rankedGrammarCandidates = withRanking(grammarCandidates, 5, 'strong syntax/context matches', 'grammar', 'static grammar match');
		const candidates = dedupeCandidates([
			...withRanking(overrideSignatureCandidates, 1, 'hard context', 'model override signatures', 'inherited function signature available after override keyword'),
			...rankedExpectedCandidates,
			...withRanking(scopeCandidates, 3, 'scope / locals / params', 'parser scope', 'visible parser local or parameter'),
			...withRanking(classMemberCandidates, 4, 'current class members', 'model members(this)', 'current or inherited class member'),
			...(overrideSignatureContext ? rankedTypeCandidates : rankedGrammarCandidates),
			...rankedGlobalFunctionCandidates,
			...(overrideSignatureContext ? rankedGrammarCandidates : rankedTypeCandidates),
		]);
		if (candidates.length === 0 && context.prefix) {
			if (shouldRequeryAfterShortPrefix(context.prefix, typeContext, modelContext?.kind)) {
				return trace(incompleteCompletionList([]), 'waiting for more prefix signal', context.prefix, context.linePrefix);
			}
			return trace(new vscode.CompletionList([], false), 'no candidates for typed prefix', context.prefix, context.linePrefix);
		}
		const visibleCandidates = candidates.length > 0 || context.prefix
			? candidates
			: withRanking(this.probeCandidates, 7, 'fallback grammar/probe', 'probe', 'no useful prefix or candidates');
		return trace(incompleteCompletionList(toCompletionItems(visibleCandidates, context.range, context.prefix)), candidates.length > 0 ? 'layered candidates' : 'probe fallback', context.prefix, context.linePrefix);
	}

	resolveCompletionItem(item: vscode.CompletionItem): vscode.CompletionItem {
		const data = (item as vscode.CompletionItem & { data?: CompletionItemData }).data;
		if (data?.completionCategory && data.completionText) {
			item.documentation = completionDocumentation({
				category: data.completionCategory,
				completionText: data.completionText,
			});
		}
		return item;
	}

	private getScopeCandidates(model: ReturnType<typeof buildLanguageModel> | undefined, position: vscode.Position, prefix: string, allowEmpty = false): BasicCompletionCandidate[] {
		if (!model || (!allowEmpty && prefix.length < 2)) {
			return [];
		}
		return filterCandidates(model.visibleLocals(position).map((local, index) => ({
			label: local.name ?? '',
			detail: local.kind === 'parameter' ? `Parameter: ${formatScopeDetail(local.valueType, local.name)}` : `Local: ${formatScopeDetail(local.valueType, local.name)}`,
			kind: local.kind === 'parameter' ? vscode.CompletionItemKind.Variable : vscode.CompletionItemKind.Variable,
			insertText: local.name,
			sortText: `05_${index.toString().padStart(4, '0')}_${local.name ?? ''}`,
			valueType: local.valueType,
		})).filter(candidate => candidate.label), prefix);
	}

	private getCurrentClassMemberCandidates(model: ReturnType<typeof buildLanguageModel> | undefined, position: vscode.Position, prefix: string, allowEmpty = false, suppressedCallableName?: string, insertCallableReference = false, forwardedArgumentSlots = 0): BasicCompletionCandidate[] {
		if (!model || (!allowEmpty && prefix.length < 2)) {
			return [];
		}
		return filterCandidates(model.members('this', position)
			.filter(member => !suppressedCallableName || member.type !== 'memberFunction' || member.name !== suppressedCallableName)
			.map((member, index) => memberToCandidate(member, index, { insertCallableReference, forwardedArgumentSlots })), prefix);
	}

	private getMemberAccessCandidates(model: ReturnType<typeof buildLanguageModel> | undefined, position: vscode.Position, prefix: string, context: { kind: string; receiver?: string } | undefined, linePrefix: string, insertCallableReference = false, forwardedArgumentSlots = 0): BasicCompletionCandidate[] | undefined {
		if (!model || !context?.receiver || (context.kind !== 'member' && context.kind !== 'staticMember')) {
			return undefined;
		}
		const enumCandidates = this.getEnumMemberAccessCandidates(context.receiver, prefix, linePrefix, position);
		if (enumCandidates.length > 0) {
			return enumCandidates;
		}
		return filterCandidates(model.members(context.receiver, position, { includeStaticInstanceMembers: true }).map((member, index) => memberToCandidate(member, index, { insertCallableReference, forwardedArgumentSlots })), prefix);
	}

	private getEnumMemberAccessCandidates(receiver: string, prefix: string, linePrefix: string, position: vscode.Position): BasicCompletionCandidate[] {
		const symbolIndex = this.symbolIndex;
		if (!symbolIndex || !this.isKnownEnumReceiver(receiver)) {
			return [];
		}
		const range = enumMemberAccessCompletionRange(receiver, linePrefix, position, prefix);
		const expressionFilterText = trailingEnumExpressionFilterText(linePrefix) || prefix;
		return filterCandidates(symbolIndex.getEnumValueSymbols(receiver).map((symbol, index) => ({
			...enumValueToCandidate(symbol, receiver, index),
			label: `${receiver}.${symbol.name}`,
			insertText: `${receiver}.${symbol.name}`,
			filterText: expressionFilterText || symbol.name,
			searchText: symbol.name,
			range,
		})), prefix);
	}

	private getQualifiedEnumPlaceholderCandidates(receiver: string, filterText: string | undefined): BasicCompletionCandidate[] {
		if (!this.symbolIndex || !this.symbolIndex.getEnumSymbols().some(symbol => symbol.name === receiver)) {
			return [];
		}
		return this.symbolIndex.getEnumValueSymbols(receiver).map((symbol, index) => ({
			...enumValueToCandidate(symbol, receiver, index),
			label: `${receiver}.${symbol.name}`,
			insertText: `${receiver}.${symbol.name}`,
			filterText: filterText || symbol.name,
		}));
	}

	private getExpectedEnumArgumentCandidates(expectedType: string | undefined, contextKind: string | undefined, linePrefix: string, position: vscode.Position, prefix: string): BasicCompletionCandidate[] {
		if (!expectedType || contextKind !== 'argument' || !this.symbolIndex?.getEnumSymbols().some(symbol => symbol.name === expectedType)) {
			return [];
		}
		const values = this.symbolIndex.getEnumValueSymbols(expectedType);
		if (values.length === 0) {
			return [];
		}
		const replacementRange = enumArgumentCompletionRange(linePrefix, position, prefix);
		const expressionFilterText = enumArgumentExpressionFilterText(linePrefix) || prefix;
		return values.map((symbol, index) => ({
			...enumValueToCandidate(symbol, expectedType, index),
			label: `${expectedType}.${symbol.name}`,
			insertText: `${expectedType}.${symbol.name}`,
			filterText: expressionFilterText || symbol.name,
			searchText: symbol.name,
			range: replacementRange,
		}));
	}

	private isKnownEnumReceiver(receiver: string | undefined): receiver is string {
		return !!receiver && !!this.symbolIndex?.getEnumSymbols().some(symbol => symbol.name === receiver);
	}

	private isKnownEnumValue(receiver: string, valueName: string): boolean {
		return !!this.symbolIndex?.getEnumValueSymbols(receiver).some(symbol => symbol.name === valueName);
	}

	private getOverrideSignatureCandidates(model: ReturnType<typeof buildLanguageModel> | undefined, position: vscode.Position, prefix: string, linePrefix: string): BasicCompletionCandidate[] {
		const currentClass = model?.currentClass(position)?.name;
		if (!model || !currentClass || !isAfterOverrideKeyword(linePrefix)) {
			return [];
		}
		const inheritedMembers = model.members('this', position)
			.filter(member =>
				member.containerName !== currentClass
				&& member.type === 'memberFunction'
				&& member.signature
				&& member.declarationKind !== 'constructor'
				&& member.declarationKind !== 'destructor'
				&& !member.modifiers?.includes('static')
			);
		return filterCandidates(dedupeCandidates(inheritedMembers.map((member, index) => overrideSignatureCandidate(member, index))), prefix);
	}

	private getTypeCandidates(typeContext: CheapTypeCompletionContext, prefix: string, document: vscode.TextDocument, position: vscode.Position, allowEmptyOverrideTypes = false, allowShortValuePrefix = false): BasicCompletionCandidate[] {
		const allowEmptyClassBaseList = prefix.length === 0 && typeContext.kind === 'classBase';
		const allowEmptyOverrideTypeList = allowEmptyOverrideTypes && prefix.length === 0;
		const allowShortIndexedValueList = allowShortValuePrefix && prefix.length >= 1 && typeContext.kind === 'broadClass';
		const typeOptions = { constructorCall: typeContext.kind === 'newExpression', symbolIndex: this.symbolIndex };
		if (!this.symbolIndex || (!allowEmptyClassBaseList && !allowEmptyOverrideTypeList && !allowShortIndexedValueList && prefix.length < 2) || (typeContext.kind === 'none' && !allowEmptyOverrideTypeList)) {
			return [];
		}
		if (allowEmptyOverrideTypeList) {
			return this.symbolIndex.getTypeSymbols()
				.filter(symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
				.slice(0, 30)
				.map((symbol, index) => typeSymbolToCandidate(symbol, index, prefix, typeOptions));
		}
		if (typeContext.classOnly) {
			if (allowEmptyClassBaseList) {
				return this.symbolIndex.getClassSymbols()
					.filter(symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
					.slice(0, maxCompletionItems)
					.map((symbol, index) => typeSymbolToCandidate(symbol, index, prefix, typeOptions));
			}
			if (typeof this.symbolIndex.findClassesByPrefix === 'function') {
				return this.symbolIndex.findClassesByPrefix(prefix, maxCompletionItems, symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
					.map((symbol, index) => typeSymbolToCandidate(symbol, index, prefix, typeOptions));
			}
			return filterCandidates(this.symbolIndex.getClassSymbols()
				.filter(symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
				.map((symbol, index) => typeSymbolToCandidate(symbol, index, undefined, typeOptions)), prefix).slice(0, maxCompletionItems);
		}
		if (typeof this.symbolIndex.findTypesByPrefix === 'function') {
			return this.symbolIndex.findTypesByPrefix(prefix, maxCompletionItems, symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
				.map((symbol, index) => typeSymbolToCandidate(symbol, index, prefix, typeOptions));
		}
		return filterCandidates(this.symbolIndex.getTypeSymbols()
			.filter(symbol => isUsableTypeCandidate(symbol, prefix, typeContext, document, position))
			.map((symbol, index) => typeSymbolToCandidate(symbol, index, undefined, typeOptions)), prefix).slice(0, maxCompletionItems);
	}

	private getConditionAssertionValueCandidates(model: ReturnType<typeof buildLanguageModel> | undefined, position: vscode.Position): BasicCompletionCandidate[] {
		const boolScopeCandidates = this.getScopeCandidates(model, position, '', true).filter(isBoolValueCandidate);
		const boolMemberCandidates = this.getCurrentClassMemberCandidates(model, position, '', true).filter(isBoolValueCandidate);
		return dedupeCandidates([
			...this.conditionAssertionValueCandidates,
			...boolScopeCandidates,
			...boolMemberCandidates,
		]);
	}

	private getGlobalFunctionCandidates(prefix: string, modelContextKind: string | undefined): BasicCompletionCandidate[] {
		if (!this.symbolIndex || prefix.length < 2 || !isValueCompletionContext(modelContextKind)) {
			return [];
		}
		const symbols = typeof this.symbolIndex.findFunctionsByPrefix === 'function'
			? this.symbolIndex.findFunctionsByPrefix(prefix, maxCompletionItems)
			: this.symbolIndex.getFunctionSymbols();
		return filterCandidates(symbols.map((symbol, index) => globalFunctionToCandidate(symbol, index)), prefix);
	}

	private getAttributeCandidates(prefix: string, includeOpeningBracket: boolean, bareContext: boolean): BasicCompletionCandidate[] {
		if (!this.symbolIndex) {
			return [];
		}
		const symbolIndex = this.symbolIndex;
		const observedNames = typeof symbolIndex.findDecoratorsByPrefix === 'function'
			? [...symbolIndex.findDecoratorsByPrefix(prefix, maxCompletionItems)]
			: [...symbolIndex.getDecoratorNames()].filter(name => candidateMatchScore(name, prefix) !== undefined).slice(0, maxCompletionItems);
		const classFamilyNames = typeof symbolIndex.findClassesByPrefix === 'function'
			? symbolIndex.findClassesByPrefix(prefix, maxCompletionItems).map(symbol => symbol.name)
			: [];
		const names = bareContext && !isBareDecoratorFamilyPrefix(prefix)
			? bareDecoratorDirectNames(prefix, observedNames, symbolIndex)
			: dedupeNames([...observedNames, ...classFamilyNames]).slice(0, maxCompletionItems);
		return names.map((name, index) => decoratorToCandidate(name, index, includeOpeningBracket, symbolIndex));
	}
}

function isValueCompletionContext(kind: string | undefined): boolean {
	return kind === 'assignment' || kind === 'return' || kind === 'argument' || kind === 'case' || kind === 'value';
}

function shouldRequeryAfterShortPrefix(prefix: string, typeContext: CheapTypeCompletionContext, modelContextKind: string | undefined): boolean {
	return prefix.length > 0
		&& prefix.length < 2
		&& (isValueCompletionContext(modelContextKind) || typeContext.kind !== 'none');
}

function isIndexRefreshing(symbolIndex: EnforceSymbolIndex | undefined): boolean {
	return typeof symbolIndex?.isRefreshing === 'function' && symbolIndex.isRefreshing();
}

export function getProbeCandidates(): BasicCompletionCandidate[] {
	return [{
		label: 'array',
		detail: 'Completion probe item',
		kind: vscode.CompletionItemKind.Class,
		sortText: '9000_array',
	}];
}

export function getGrammarCandidates(): BasicCompletionCandidate[] {
	return [
		...keywordCandidates(grammarKeywords),
		...typeCandidates(typeKeywords),
	];
}

export function getDirectiveCandidates(): BasicCompletionCandidate[] {
	return ['#define', '#ifdef', '#ifndef', '#else', '#endif', '#include'].map((label, index) => ({
		label,
		detail: 'Compiler directive',
		kind: vscode.CompletionItemKind.Keyword,
		insertText: label,
		sortText: `00_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

export function getOperatorCandidates(): BasicCompletionCandidate[] {
	return [
		'==', '!=', '<=', '>=', '&&', '||',
		'++', '--',
		'+=', '-=', '*=', '/=', '%=',
		'<<', '>>', '<<=', '>>=',
		'&=', '|=', '^=',
	].map((label, index) => ({
		label,
		detail: 'Enforce operator',
		kind: vscode.CompletionItemKind.Operator,
		insertText: label,
		sortText: `30_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

type CompletionRange = vscode.Range | { inserting: vscode.Range; replacing: vscode.Range };

function toCompletionItems(candidates: BasicCompletionCandidate[], range?: CompletionRange, filterText?: string): vscode.CompletionItem[] {
	return candidates.slice(0, maxCompletionItems).map((candidate, index) => toCompletionItem(candidate, index, range, filterText));
}

function toCompletionItem(candidate: BasicCompletionCandidate, rank: number, range?: CompletionRange, filterText?: string): vscode.CompletionItem {
	const metadata = completionMetadata(candidate);
	const item = new vscode.CompletionItem({ label: candidate.label, description: metadata.category }, candidate.kind);
	item.documentation = completionDocumentation(metadata);
	item.filterText = candidate.filterText ?? (shouldPreserveProviderMatchedCandidate(candidate, filterText) ? filterText || candidate.label : candidate.label);
	const insertText = candidate.insertText ?? candidate.label;
	item.insertText = typeof insertText === 'string' && insertText.includes('$')
		? new vscode.SnippetString(insertText)
		: insertText;
	item.command = candidate.command;
	item.preselect = candidate.preselect;
	item.sortText = `${rank.toString().padStart(5, '0')}_${candidate.sortText}`;
	item.range = candidate.range ?? range;
	(item as vscode.CompletionItem & { data?: CompletionItemData }).data = {
		...(candidate.ranking ?? {}),
		finalRank: rank + 1,
		sortText: item.sortText,
		filterText: item.filterText,
		valueType: candidate.ranking?.valueType ?? candidate.valueType,
		completionCategory: metadata.category,
		completionText: metadata.completionText,
	};
	return item;
}

function shouldPreserveProviderMatchedCandidate(candidate: BasicCompletionCandidate, filterText?: string): boolean {
	return !!filterText
		&& typeof candidate.ranking?.matchScore === 'number'
		&& candidate.label !== filterText;
}

function completionMetadata(candidate: BasicCompletionCandidate): { category: string; completionText: string } {
	const match = /^([^:]+):\s*(.*)$/.exec(candidate.detail);
	const category = match ? normalizeCompletionCategory(match[1]) : completionKindName(candidate.kind);
	return {
		category,
		completionText: formatCompletionPreviewText(category, completionInsertDisplayText(candidate).trim()),
	};
}

function completionInsertDisplayText(candidate: BasicCompletionCandidate): string {
	const insertText = candidate.insertText ?? candidate.label;
	const value = insertText instanceof vscode.SnippetString ? insertText.value : insertText;
	return value
		.replace(/\$\{\d+:([^}]*)\}/g, '$1')
		.replace(/\$\d+/g, '');
}

function formatCompletionPreviewText(category: string, completionText: string): string {
	const normalizedCategory = normalizeCompletionCategory(category);
	if (normalizedCategory === 'class' && !/^\s*class\b/.test(completionText)) {
		return `class ${completionText}`;
	}
	if (normalizedCategory === 'enum' && !/^\s*enum\b/.test(completionText)) {
		return `enum ${completionText}`;
	}
	if (normalizedCategory === 'function' && !/\(/.test(completionText)) {
		return `function ${completionText}`;
	}
	return completionText;
}

function completionDocumentation(metadata: { category: string; completionText: string }): vscode.MarkdownString {
	const documentation = new vscode.MarkdownString(undefined, true);
	const lines = [
		'```enforce-hover-header',
		metadata.category,
		'```',
		'',
		'```enforce-hover',
		metadata.completionText,
		'```',
	];
	documentation.appendMarkdown(lines.join('\n'));
	return documentation;
}

function isMemberAccessContext(context: { kind: string; receiver?: string } | undefined): context is { kind: 'member' | 'staticMember'; receiver: string } {
	return !!context?.receiver && (context.kind === 'member' || context.kind === 'staticMember');
}

function normalizeCompletionCategory(category: string): string {
	const normalized = category.trim().toLowerCase();
	return normalized === 'local' ? 'variable' : normalized;
}

function completionKindName(kind: vscode.CompletionItemKind): string {
	switch (kind) {
		case vscode.CompletionItemKind.Class:
			return 'class';
		case vscode.CompletionItemKind.Enum:
			return 'enum';
		case vscode.CompletionItemKind.Function:
			return 'function';
		case vscode.CompletionItemKind.Property:
			return 'property';
		case vscode.CompletionItemKind.Variable:
			return 'variable';
		case vscode.CompletionItemKind.Keyword:
			return 'keyword';
		case vscode.CompletionItemKind.Operator:
			return 'operator';
		default:
			return 'completion';
	}
}

function incompleteCompletionList(items: vscode.CompletionItem[]): vscode.CompletionList {
	return new vscode.CompletionList(items, true);
}

function formatScopeDetail(valueType: string | undefined, name: string | undefined): string {
	return `${valueType ?? 'var'} ${name ?? ''}`.trim();
}

function memberToCandidate(member: EnforceContainerMemberSymbol, index: number, options: { insertCallableReference?: boolean; forwardedArgumentSlots?: number } = {}): BasicCompletionCandidate {
	const valueType = member.type === 'property'
		? declarationValueType(member.signature, member.name)
		: functionReturnType(member.signature, member.name);
	const isFunction = member.type === 'memberFunction';
	const insertCallableReference = isFunction && options.insertCallableReference;
	const callableReferenceInsert = insertCallableReference
		? callableReferenceInsertText(member.name, member.signature, options.forwardedArgumentSlots ?? 0)
		: undefined;
	return {
		label: member.name,
		detail: isFunction ? `Function: ${member.signature ?? member.name}` : `Property: ${member.signature ?? member.name}`,
		kind: isFunction ? vscode.CompletionItemKind.Function : vscode.CompletionItemKind.Property,
		insertText: isFunction
			? insertCallableReference ? callableReferenceInsert?.insertText ?? member.name : functionCallSnippet(member.name, member.signature)
			: member.name,
		command: isFunction && ((insertCallableReference && callableReferenceInsert?.triggersSuggest) || (!insertCallableReference && signatureHasSnippetArguments(member.signature))) ? { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' } : undefined,
		sortText: `06_${index.toString().padStart(4, '0')}_${member.name}`,
		valueType,
	};
}

function globalFunctionToCandidate(symbol: EnforceSymbol, index: number): BasicCompletionCandidate {
	return {
		label: symbol.name,
		detail: `Function: ${symbol.signature ?? symbol.name}`,
		kind: vscode.CompletionItemKind.Function,
		insertText: functionCallSnippet(symbol.name, symbol.signature),
		command: signatureHasSnippetArguments(symbol.signature) ? { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' } : undefined,
		sortText: `48_${index.toString().padStart(4, '0')}_${symbol.name}`,
		valueType: functionReturnType(symbol.signature, symbol.name),
	};
}

function functionCallSnippet(name: string, signature: string | undefined): vscode.SnippetString {
	const argumentText = functionArgumentSnippetText(signature);
	return new vscode.SnippetString(`${name}(${argumentText})$0`);
}

function callableReferenceInsertText(name: string, signature: string | undefined, forwardedArgumentSlots: number): { insertText: string | vscode.SnippetString; triggersSuggest: boolean } {
	if (forwardedArgumentSlots <= 0) {
		return { insertText: name, triggersSuggest: false };
	}
	const forwardedArguments = forwardedCallableArgumentSnippetText(signature, forwardedArgumentSlots);
	if (!forwardedArguments) {
		return { insertText: name, triggersSuggest: false };
	}
	return {
		insertText: new vscode.SnippetString(`${name}, ${forwardedArguments}$0`),
		triggersSuggest: true,
	};
}

function decoratorSnippet(name: string, signature: string | undefined, includeOpeningBracket: boolean, symbolIndex?: EnforceSymbolIndex): vscode.SnippetString {
	const argumentText = decoratorArgumentSnippetText(signature, symbolIndex);
	const open = includeOpeningBracket ? '[' : '';
	return new vscode.SnippetString(`${open}${name}(${argumentText})]$0`);
}

function signatureHasSnippetArguments(signature: string | undefined): boolean {
	const parameterText = signatureParameterText(signature);
	if (parameterText === undefined) {
		return true;
	}
	const trimmed = parameterText.trim();
	return trimmed.length > 0
		&& trimmed !== 'void'
		&& functionSnippetParameters(splitTopLevelParameters(trimmed)).length > 0;
}

function functionArgumentSnippetText(signature: string | undefined): string {
	const parameterText = signatureParameterText(signature);
	if (parameterText === undefined) {
		return '$1';
	}
	const trimmed = parameterText.trim();
	if (!trimmed || trimmed === 'void') {
		return '';
	}
	const names = functionSnippetParameters(splitTopLevelParameters(trimmed))
		.map(({ parameter, originalIndex }) => parameterPlaceholderFromDeclaration(parameter, originalIndex));
	if (names.length === 0) {
		return '';
	}
	return names.map((name, index) => `\${${index + 1}:${escapeSnippetPlaceholder(name)}}`).join(', ');
}

function forwardedCallableArgumentSnippetText(signature: string | undefined, forwardedArgumentSlots: number): string {
	const parameterText = signatureParameterText(signature);
	if (parameterText === undefined) {
		return '';
	}
	const trimmed = parameterText.trim();
	if (!trimmed || trimmed === 'void') {
		return '';
	}
	const parameters = functionSnippetParameters(splitTopLevelParameters(trimmed));
	if (parameters.length === 0 || parameters.length > forwardedArgumentSlots) {
		return '';
	}
	const names = parameters.map(({ parameter, originalIndex }) => parameterPlaceholderFromDeclaration(parameter, originalIndex));
	return names.map((name, index) => `\${${index + 1}:${escapeSnippetPlaceholder(name)}}`).join(', ');
}

function functionSnippetParameters(parameters: string[]): { parameter: string; originalIndex: number }[] {
	return parameters
		.map((parameter, originalIndex) => ({ parameter, originalIndex }))
		.filter(({ parameter }) => !hasTopLevelDefaultValue(parameter));
}

function decoratorArgumentSnippetText(signature: string | undefined, symbolIndex?: EnforceSymbolIndex): string {
	const parameterText = signatureParameterText(signature);
	if (parameterText === undefined) {
		return '$1';
	}
	const trimmed = parameterText.trim();
	if (!trimmed || trimmed === 'void') {
		return '';
	}
	const parameters = decoratorSnippetParameters(splitTopLevelParameters(trimmed));
	if (parameters.length === 0) {
		return '$1';
	}
	return parameters.map((parameter, index) => decoratorParameterSnippet(parameter, index, symbolIndex)).join(', ');
}

function decoratorSnippetParameters(parameters: string[]): string[] {
	const required = parameters.filter(parameter => !hasTopLevelDefaultValue(parameter));
	if (required.length > 0) {
		return required;
	}
	return parameters.slice(0, 1);
}

function decoratorParameterSnippet(parameter: string, index: number, symbolIndex?: EnforceSymbolIndex): string {
	const name = parameterPlaceholderFromDeclaration(parameter, index);
	const value = decoratorParameterValueSnippet(parameter, name, index + 1, symbolIndex);
	return value;
}

function decoratorParameterValueSnippet(parameter: string, name: string, tabstop: number, symbolIndex?: EnforceSymbolIndex): string {
	const type = normalizeDecoratorParameterType(parameter);
	if (type === 'string' || type === 'ResourceName') {
		return `"$${tabstop}"`;
	}
	const enumDefault = decoratorEnumDefaultSnippet(type, symbolIndex);
	if (enumDefault) {
		return `\${${tabstop}:${escapeSnippetPlaceholder(enumDefault)}}`;
	}
	if (type === 'bool') {
		return `\${${tabstop}:false}`;
	}
	if (type === 'int') {
		return `\${${tabstop}:0}`;
	}
	if (type === 'float') {
		return `\${${tabstop}:0}`;
	}
	return `\${${tabstop}:${escapeSnippetPlaceholder(name)}}`;
}

function decoratorEnumDefaultSnippet(type: string, symbolIndex: EnforceSymbolIndex | undefined): string | undefined {
	if (!type || !symbolIndex?.getEnumSymbols().some(symbol => symbol.name === type)) {
		return undefined;
	}
	const value = symbolIndex.getEnumValueSymbols(type)[0]?.name;
	return value ? `${type}.${value}` : undefined;
}

function signatureParameterText(signature: string | undefined): string | undefined {
	const value = signature ?? '';
	const openIndex = value.indexOf('(');
	if (openIndex < 0) {
		return undefined;
	}
	let depth = 0;
	let stringQuote: string | undefined;
	let escaped = false;
	let lineComment = false;
	let blockComment = false;
	for (let index = openIndex; index < value.length; index++) {
		const char = value[index];
		const next = value[index + 1];
		if (lineComment) {
			if (char === '\n' || char === '\r') {
				lineComment = false;
			}
			continue;
		}
		if (blockComment) {
			if (char === '*' && next === '/') {
				blockComment = false;
				index++;
			}
			continue;
		}
		if (stringQuote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '/' && next === '/') {
			lineComment = true;
			index++;
			continue;
		}
		if (char === '/' && next === '*') {
			blockComment = true;
			index++;
			continue;
		}
		if (char === '"' || char === '\'') {
			stringQuote = char;
			continue;
		}
		if (char === '(') {
			depth++;
			continue;
		}
		if (char === ')') {
			depth--;
			if (depth === 0) {
				return value.slice(openIndex + 1, index);
			}
		}
	}
	return value.slice(openIndex + 1);
}

function splitTopLevelParameters(value: string): string[] {
	const parts: string[] = [];
	let start = 0;
	let angleDepth = 0;
	let parenDepth = 0;
	let bracketDepth = 0;
	let braceDepth = 0;
	let stringQuote: string | undefined;
	let escaped = false;
	let lineComment = false;
	let blockComment = false;
	for (let index = 0; index < value.length; index++) {
		const char = value[index];
		const next = value[index + 1];
		if (lineComment) {
			if (char === '\n' || char === '\r') {
				lineComment = false;
			}
			continue;
		}
		if (blockComment) {
			if (char === '*' && next === '/') {
				blockComment = false;
				index++;
			}
			continue;
		}
		if (stringQuote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '/' && next === '/') {
			lineComment = true;
			index++;
			continue;
		}
		if (char === '/' && next === '*') {
			blockComment = true;
			index++;
			continue;
		}
		if (char === '"' || char === '\'') {
			stringQuote = char;
			continue;
		}
		if (char === '<') {
			angleDepth++;
		} else if (char === '>') {
			angleDepth = Math.max(0, angleDepth - 1);
		} else if (char === '(') {
			parenDepth++;
		} else if (char === ')') {
			parenDepth = Math.max(0, parenDepth - 1);
		} else if (char === '[') {
			bracketDepth++;
		} else if (char === ']') {
			bracketDepth = Math.max(0, bracketDepth - 1);
		} else if (char === '{') {
			braceDepth++;
		} else if (char === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
		} else if (char === ',' && angleDepth === 0 && parenDepth === 0 && bracketDepth === 0 && braceDepth === 0) {
			parts.push(value.slice(start, index).trim());
			start = index + 1;
		}
	}
	parts.push(value.slice(start).trim());
	return parts.filter(Boolean);
}

function parameterNameFromDeclaration(parameter: string): string | undefined {
	const beforeDefault = topLevelBeforeDefault(parameter).trim();
	const withoutComments = beforeDefault.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/.*$/g, ' ').trim();
	const match = /(?:^|[\s*&>\]])([A-Za-z_][A-Za-z0-9_]*)$/.exec(withoutComments);
	return match?.[1];
}

function normalizeDecoratorParameterType(parameter: string): string {
	const beforeDefault = topLevelBeforeDefault(parameter).trim();
	const withoutComments = beforeDefault.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/.*$/g, ' ').trim();
	const name = parameterNameFromDeclaration(parameter);
	if (!name) {
		return '';
	}
	return withoutComments
		.slice(0, Math.max(0, withoutComments.lastIndexOf(name)))
		.replace(/\b(?:const|ref|autoptr|notnull|out|inout)\b/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

function topLevelBeforeDefault(value: string): string {
	const parts = splitTopLevelAssignment(value);
	return parts[0] ?? value;
}

function hasTopLevelDefaultValue(value: string): boolean {
	return splitTopLevelAssignment(value).length > 1;
}

function splitTopLevelAssignment(value: string): string[] {
	const parts: string[] = [];
	let start = 0;
	let angleDepth = 0;
	let parenDepth = 0;
	let bracketDepth = 0;
	let braceDepth = 0;
	let stringQuote: string | undefined;
	let escaped = false;
	let lineComment = false;
	let blockComment = false;
	for (let index = 0; index < value.length; index++) {
		const char = value[index];
		const next = value[index + 1];
		if (lineComment) {
			if (char === '\n' || char === '\r') {
				lineComment = false;
			}
			continue;
		}
		if (blockComment) {
			if (char === '*' && next === '/') {
				blockComment = false;
				index++;
			}
			continue;
		}
		if (stringQuote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '/' && next === '/') {
			lineComment = true;
			index++;
			continue;
		}
		if (char === '/' && next === '*') {
			blockComment = true;
			index++;
			continue;
		}
		if (char === '"' || char === '\'') {
			stringQuote = char;
			continue;
		}
		if (char === '<') {
			angleDepth++;
		} else if (char === '>') {
			angleDepth = Math.max(0, angleDepth - 1);
		} else if (char === '(') {
			parenDepth++;
		} else if (char === ')') {
			parenDepth = Math.max(0, parenDepth - 1);
		} else if (char === '[') {
			bracketDepth++;
		} else if (char === ']') {
			bracketDepth = Math.max(0, bracketDepth - 1);
		} else if (char === '{') {
			braceDepth++;
		} else if (char === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
		} else if (char === '=' && next !== '=' && angleDepth === 0 && parenDepth === 0 && bracketDepth === 0 && braceDepth === 0) {
			parts.push(value.slice(start, index).trim());
			start = index + 1;
		}
	}
	parts.push(value.slice(start).trim());
	return parts.filter(Boolean);
}

function parameterPlaceholderFromDeclaration(parameter: string, index: number): string {
	return parameterNameFromDeclaration(parameter) ?? `arg${index + 1}`;
}

function escapeSnippetPlaceholder(value: string): string {
	return value.replace(/[$}\\]/g, '\\$&');
}

function overrideSignatureCandidate(member: EnforceContainerMemberSymbol, index: number): BasicCompletionCandidate {
	const signature = overrideCompletionSignature(member.signature ?? member.name);
	return {
		label: signature,
		detail: `Function: ${signature}`,
		kind: vscode.CompletionItemKind.Function,
		insertText: new vscode.SnippetString(`${signature}\n{\n\t$0\n}`),
		sortText: `01_${index.toString().padStart(4, '0')}_${member.name}`,
		searchText: `${member.name} ${signature}`,
		valueType: functionReturnType(member.signature, member.name),
	};
}

function overrideCompletionSignature(signature: string): string {
	return signature
		.replace(/[;{]\s*$/, '')
		.replace(/\b(?:event|proto|native|external|override)\b/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

function isAfterOverrideKeyword(linePrefix: string): boolean {
	return /\boverride\b(?:\s+(?:[A-Za-z_][A-Za-z0-9_]*)?)?$/.test(linePrefix);
}

interface TypeCandidateOptions {
	constructorCall?: boolean;
	symbolIndex?: EnforceSymbolIndex;
}

function typeSymbolToCandidate(symbol: EnforceSymbol, index: number, prefix?: string, options: TypeCandidateOptions = {}): BasicCompletionCandidate {
	const matchScore = prefix ? candidateMatchScore(symbol.name, prefix) ?? 100 : undefined;
	const constructorSignature = options.constructorCall && symbol.type === 'class'
		? getConstructorSignature(symbol.name, options.symbolIndex, symbol)
		: undefined;
	const insertText = options.constructorCall && symbol.type === 'class'
		? constructorCallSnippet(symbol.name, constructorSignature)
		: symbol.name;
	return {
		label: symbol.name,
		detail: symbol.type === 'enum' ? `Enum: ${symbol.signature ?? symbol.name}` : `Class: ${symbol.signature ?? symbol.name}`,
		kind: symbol.type === 'enum' ? vscode.CompletionItemKind.Enum : vscode.CompletionItemKind.Class,
		insertText,
		command: options.constructorCall && symbol.type === 'class' && constructorSignature !== undefined && signatureHasSnippetArguments(constructorSignature) ? { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' } : undefined,
		sortText: `07_${index.toString().padStart(5, '0')}_${symbol.name}`,
		valueType: symbol.name,
		ranking: matchScore === undefined ? undefined : { matchScore },
	};
}

function constructorCallSnippet(name: string, signature: string | undefined): vscode.SnippetString {
	return signature === undefined
		? new vscode.SnippetString(`${name}()$0`)
		: functionCallSnippet(name, signature);
}

function enumValueToCandidate(symbol: EnforceSymbol, enumName: string, index: number): BasicCompletionCandidate {
	return {
		label: symbol.name,
		detail: `EnumValue: ${symbol.signature ?? symbol.name}`,
		kind: vscode.CompletionItemKind.EnumMember,
		insertText: symbol.name,
		sortText: `03_${index.toString().padStart(4, '0')}_${symbol.name}`,
		valueType: enumName,
	};
}

function bareDecoratorDirectNames(prefix: string, observedNames: readonly string[], symbolIndex: EnforceSymbolIndex): string[] {
	if (prefix.length < 2) {
		return [];
	}
	const observedDirect = observedNames.filter(name => directPrefixMatchScore(name, prefix) !== undefined);
	const classDirect = typeof symbolIndex.findClassesByPrefix === 'function'
		? symbolIndex.findClassesByPrefix(prefix, maxCompletionItems)
			.filter(symbol => isAttributeLikeClass(symbol, symbolIndex))
			.map(symbol => symbol.name)
		: [];
	const exactKnownDecorators = observedDirect.filter(name => name === prefix);
	const exactAttributeLikeClasses = classDirect.filter(name => name === prefix);
	const exactClassSymbols = typeof symbolIndex.getClassSymbolsByName === 'function'
		? symbolIndex.getClassSymbolsByName(prefix)
			.filter(symbol => symbol.name === prefix)
			.map(symbol => symbol.name)
		: [];
	const exactConstructorSymbols = symbolIndex.getContainerMemberSymbolsForContainersAndName([prefix], prefix)
		.filter(symbol => symbol.declarationKind === 'constructor' || symbol.name === prefix)
		.map(symbol => symbol.containerName)
		.filter((name): name is string => name !== undefined);
	return dedupeNames([...observedDirect, ...classDirect, ...exactKnownDecorators, ...exactAttributeLikeClasses, ...exactClassSymbols, ...exactConstructorSymbols]).slice(0, maxCompletionItems);
}

function isAttributeLikeClass(symbol: EnforceSymbol, symbolIndex: EnforceSymbolIndex): boolean {
	if (symbol.name === 'Attribute' || symbol.name.endsWith('Attribute')) {
		return true;
	}
	const ancestors = typeof symbolIndex.getClassAncestorNames === 'function'
		? symbolIndex.getClassAncestorNames(symbol.name, false)
		: [];
	return ancestors.some(name => name === 'Attribute' || name.endsWith('Attribute'));
}

function decoratorToCandidate(name: string, index: number, includeOpeningBracket: boolean, symbolIndex: EnforceSymbolIndex): BasicCompletionCandidate {
	const signature = getDecoratorConstructorSignature(name, symbolIndex);
	return {
		label: name,
		detail: `Attribute: ${signature ?? name}`,
		kind: vscode.CompletionItemKind.Class,
		insertText: decoratorSnippet(name, signature, includeOpeningBracket, symbolIndex),
		command: { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' },
		sortText: `04_${index.toString().padStart(4, '0')}_${name}`,
		ranking: { matchScore: 100 },
	};
}

function getDecoratorConstructorSignature(name: string, symbolIndex: EnforceSymbolIndex): string | undefined {
	return getConstructorSignature(name, symbolIndex);
}

function getConstructorSignature(name: string, symbolIndex: EnforceSymbolIndex | undefined, classSymbol?: EnforceSymbol): string | undefined {
	const constructor = symbolIndex?.getContainerMemberSymbolsForContainersAndName([name], name)
		.find(symbol => symbol.declarationKind === 'constructor' || symbol.name === name);
	if (constructor?.signature) {
		return constructor.signature;
	}
	const indexedClassSymbol = classSymbol ?? (typeof symbolIndex?.getClassSymbol === 'function' ? symbolIndex.getClassSymbol(name) : undefined);
	return indexedClassSymbol?.functions?.find(signature => constructorSignatureName(signature) === name);
}

function constructorSignatureName(signature: string): string | undefined {
	const beforeArguments = signature.split('(')[0]?.trim();
	return beforeArguments?.split(/\s+/).filter(Boolean).pop();
}

function isUsableTypeCandidate(symbol: EnforceSymbol, prefix: string, typeContext: CheapTypeCompletionContext, document: vscode.TextDocument, position: vscode.Position): boolean {
	return !isCurrentLineSymbol(symbol, document, position)
		&& !(symbol.name === prefix && shouldSuppressExactTypeCandidate(typeContext));
}

function isCurrentLineSymbol(symbol: EnforceSymbol, document: vscode.TextDocument, position: vscode.Position): boolean {
	return symbol.uri.toString() === document.uri.toString()
		&& symbol.selectionRange.start.line === position.line;
}

function shouldSuppressExactTypeCandidate(typeContext: CheapTypeCompletionContext): boolean {
	return typeContext.suppressExact === true;
}

function candidateMatchesExpectedType(candidate: BasicCompletionCandidate, expectedType: string, symbolIndex: EnforceSymbolIndex | undefined): boolean {
	const candidateType = normalizeTypeName(candidate.valueType);
	if (!candidateType) {
		return false;
	}
	if (candidateType === expectedType) {
		return true;
	}
	const ancestors = typeof symbolIndex?.getClassAncestorNames === 'function'
		? symbolIndex.getClassAncestorNames(candidateType, true).map(normalizeTypeName)
		: [];
	return ancestors.includes(expectedType);
}

function functionReturnType(signature: string | undefined, name: string): string | undefined {
	const beforeArguments = signature?.split('(')[0]?.trim();
	if (!beforeArguments) {
		return undefined;
	}
	const parts = beforeArguments.split(/\s+/).filter(Boolean);
	const nameIndex = parts.lastIndexOf(name);
	const typeParts = nameIndex >= 0 ? parts.slice(0, nameIndex) : parts.slice(0, -1);
	return normalizeDeclaredType(typeParts.join(' '));
}

function declarationValueType(signature: string | undefined, name: string): string | undefined {
	const beforeInitializer = (signature ?? '').split('=')[0]?.replace(/;$/, '').trim();
	if (!beforeInitializer) {
		return undefined;
	}
	const parts = beforeInitializer.split(/\s+/).filter(Boolean);
	const nameIndex = parts.lastIndexOf(name);
	const typeParts = nameIndex >= 0 ? parts.slice(0, nameIndex) : parts.slice(0, -1);
	return normalizeDeclaredType(typeParts.join(' '));
}

function normalizeDeclaredType(value: string | undefined): string | undefined {
	const normalized = normalizeTypeName(value);
	return normalized || undefined;
}

function normalizeTypeName(value: string | undefined): string {
	return (value ?? '')
		.replace(/\b(?:private|protected|static|proto|native|external|override|event|owned|ref|autoptr|notnull|const|out|inout)\b/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
}

interface CheapTypeCompletionContext {
	kind: 'none' | 'strong' | 'ambiguous' | 'className' | 'classBase' | 'newExpression' | 'broadClass';
	strong: boolean;
	classOnly?: boolean;
	suppressExact?: boolean;
}

function getCheapTypeCompletionContext(linePrefix: string): CheapTypeCompletionContext {
	if (/\bclass\s+[A-Za-z_]\w*\s*(?:extends\s+|:\s*)[A-Za-z_]\w*$/.test(linePrefix)) {
		return { kind: 'classBase', strong: true, classOnly: true, suppressExact: true };
	}
	if (/\bclass\s+[A-Za-z_]\w*\s*(?:extends\s+|:\s*)$/.test(linePrefix)) {
		return { kind: 'classBase', strong: true, classOnly: true, suppressExact: true };
	}
	if (/\bnew\s+[A-Za-z_]\w*$/.test(linePrefix)) {
		return { kind: 'newExpression', strong: true, classOnly: true };
	}
	if (/\bclass\s+[A-Za-z_]\w*(?:\s+(?:extends\s+|:\s*)?|\s*:\s*)[A-Za-z_]\w*$/.test(linePrefix)) {
		return { kind: 'strong', strong: true, suppressExact: true };
	}
	if (/\bclass\s+[A-Za-z_]\w*$/.test(linePrefix)) {
		return { kind: 'className', strong: true, classOnly: true, suppressExact: true };
	}

	const match = /(?:^\s*|[<,(]\s*)(?:ref\s+|autoptr\s+|notnull\s+|out\s+|inout\s+|const\s+)*([A-Za-z_]\w*)$/.exec(linePrefix);
	if (!match) {
		return isBroadClassPrefix(linePrefix)
			? { kind: 'broadClass', strong: false, classOnly: true }
			: { kind: 'none', strong: false };
	}

	const prefix = match[1];
	const suppressExact = isGenericTypeArgumentPrefix(linePrefix, prefix);
	if (prefix.includes('_') || /^[A-Z]/.test(prefix)) {
		return { kind: 'strong', strong: true, suppressExact };
	}
	return prefix.length >= 3
		? { kind: 'ambiguous', strong: false, suppressExact: true }
		: isBroadClassPrefix(linePrefix)
			? { kind: 'broadClass', strong: false, classOnly: true }
			: { kind: 'none', strong: false };
}

function isGenericTypeArgumentPrefix(linePrefix: string, prefix: string): boolean {
	const beforePrefix = linePrefix.slice(0, Math.max(0, linePrefix.length - prefix.length));
	return /<\s*(?:ref\s+|autoptr\s+|notnull\s+|out\s+|inout\s+|const\s+)*$/.test(beforePrefix);
}

function isBroadClassPrefix(linePrefix: string): boolean {
	return /[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix) && !/(?:\.|::)\s*[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix);
}

function isCheapValueExpressionPrefix(linePrefix: string): boolean {
	return /(?:^|[^=!<>])=(?!=)\s*[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix)
		|| /\breturn\s+[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix)
		|| /\([^()]*[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix);
}

function dedupeCandidates(candidates: BasicCompletionCandidate[]): BasicCompletionCandidate[] {
	const seen = new Set<string>();
	return candidates.filter(candidate => {
		const key = candidate.label;
		if (seen.has(key)) {
			return false;
		}
		seen.add(key);
		return true;
	});
}

function dedupeNames(names: readonly string[]): string[] {
	const seen = new Set<string>();
	return names.filter(name => {
		if (seen.has(name)) {
			return false;
		}
		seen.add(name);
		return true;
	});
}

function withRanking(
	candidates: BasicCompletionCandidate[],
	tier: number,
	tierName: string,
	source: string,
	reason: string,
	extra: Partial<CompletionRankingDebug> = {}
): BasicCompletionCandidate[] {
	return candidates.map(candidate => ({
		...candidate,
		ranking: {
			...candidate.ranking,
			...extra,
			tier,
			tierName,
			source,
			reason,
			valueType: extra.valueType ?? candidate.ranking?.valueType ?? candidate.valueType,
		},
	}));
}

export function getConditionOperatorCandidates(): BasicCompletionCandidate[] {
	return ['==', '!=', '&&', '||', '<', '>', '<=', '>='].map((label, index) => ({
		label,
		detail: 'Enforce operator',
		kind: vscode.CompletionItemKind.Operator,
		insertText: `${label} `,
		command: { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' },
		sortText: `28_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

export function getConditionAssertionValueCandidates(): BasicCompletionCandidate[] {
	return ['true', 'false', 'null'].map((label, index) => ({
		label,
		detail: 'Enforce keyword',
		kind: vscode.CompletionItemKind.Keyword,
		insertText: label,
		sortText: `29_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

function keywordCandidates(labels: string[]): BasicCompletionCandidate[] {
	return labels.map((label, index) => ({
		label,
		detail: 'Enforce keyword',
		kind: vscode.CompletionItemKind.Keyword,
		insertText: keywordInsertText(label),
		command: keywordCompletionCommand(label),
		preselect: label === 'override',
		sortText: `10_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

function typeCandidates(labels: string[]): BasicCompletionCandidate[] {
	return labels.map((label, index) => ({
		label,
		detail: 'Enforce type',
		kind: vscode.CompletionItemKind.Class,
		insertText: containerTypeSnippet(label),
		sortText: `20_${index.toString().padStart(2, '0')}_${label}`,
	}));
}

function containerTypeSnippet(label: string): string | vscode.SnippetString | undefined {
	if (label === 'array' || label === 'set') {
		return new vscode.SnippetString(`${label}<\${1:T}>$0`);
	}
	if (label === 'map') {
		return new vscode.SnippetString('map<${1:TKey}, ${2:TValue}>$0');
	}
	return undefined;
}

function keywordInsertText(label: string): string {
	if (['if', 'while', 'for', 'foreach', 'switch'].includes(label)) {
		return `${label} ($1)`;
	}
	return ['class', 'modded', 'sealed', 'enum', 'override', 'private', 'protected', 'static', 'if', 'else', 'foreach', 'for', 'while', 'switch', 'case', 'new'].includes(label)
		? `${label} `
		: label;
}

function keywordCompletionCommand(label: string): vscode.Command | undefined {
	return label === 'override'
		? { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' }
		: undefined;
}

function getCheapCompletionContext(document: vscode.TextDocument, position: vscode.Position): CheapCompletionContext {
	const linePrefix = document.lineAt(position.line).text.slice(0, position.character);
	const placeholderRange = getActiveArgumentPlaceholderRange(document, position);
	const placeholderText = placeholderRange ? document.getText(placeholderRange).trim() : '';
	const qualifiedPlaceholderMatch = /^([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)$/.exec(placeholderText);
	const prefix = placeholderRange ? '' : getPrefix(linePrefix);
	const operator = isOperatorPrefix(prefix);
	const parsed = getParsedDocument(document);
	const attributeContext = placeholderRange ? undefined : getDecoratorCompletionContext(parsed, linePrefix, position, prefix);
	return {
		prefix,
		linePrefix,
		directive: /^\s*#/.test(linePrefix),
		operator,
		conditionOperandOperator: !operator && isConditionOperandOperatorContext(linePrefix),
		conditionAssertionValue: isConditionAssertionValueContext(linePrefix),
		attribute: attributeContext !== undefined,
		attributeIncludesBracket: attributeContext?.includeOpeningBracket ?? false,
		attributeBare: attributeContext?.bare ?? false,
		enumPlaceholderReceiver: qualifiedPlaceholderMatch?.[1],
		enumPlaceholderFilterText: qualifiedPlaceholderMatch?.[2],
		ignored: isIgnoredPosition(parsed, toParserPosition(position)),
		range: attributeContext?.range ?? (placeholderRange
			? { inserting: new vscode.Range(position, position), replacing: placeholderRange }
			: new vscode.Range(position.line, position.character - prefix.length, position.line, position.character)),
	};
}

function getDecoratorCompletionContext(parsed: ReturnType<typeof getParsedDocument>, linePrefix: string, position: vscode.Position, prefix: string): { includeOpeningBracket: boolean; bare: boolean; range: CompletionRange } | undefined {
	const openIndex = linePrefix.lastIndexOf('[');
	const bareDeclarationPrefix = /^\s*[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix);
	if (openIndex < 0 && bareDeclarationPrefix && !isInsideFunctionBody(parsed, position)) {
		return {
			includeOpeningBracket: true,
			bare: true,
			range: new vscode.Range(position.line, position.character - prefix.length, position.line, position.character),
		};
	}
	if (openIndex < 0 || linePrefix.slice(openIndex + 1).includes(']')) {
		return undefined;
	}
	const beforeOpen = linePrefix.slice(0, openIndex);
	if (beforeOpen.trim().length > 0) {
		return undefined;
	}
	const content = linePrefix.slice(openIndex + 1);
	if (!isDecoratorNameSlot(content)) {
		return undefined;
	}
	const prefixStart = position.character - prefix.length;
	const includeOpeningBracket = content.trimStart() === prefix;
	return {
		includeOpeningBracket,
		bare: false,
		range: includeOpeningBracket
			? new vscode.Range(position.line, openIndex, position.line, position.character)
			: new vscode.Range(position.line, prefixStart, position.line, position.character),
	};
}

function isInsideFunctionBody(parsed: ReturnType<typeof getParsedDocument>, position: vscode.Position): boolean {
	const bodyRange = getEnclosingFunction(parsed, toParserPosition(position))?.bodyRange;
	if (!bodyRange) {
		return false;
	}
	const parserPosition = toParserPosition(position);
	return compareParserPositions(bodyRange.start, parserPosition) <= 0 && compareParserPositions(parserPosition, bodyRange.end) <= 0;
}

function compareParserPositions(left: { line: number; character: number }, right: { line: number; character: number }): number {
	return left.line === right.line ? left.character - right.character : left.line - right.line;
}

function isBareDecoratorFamilyPrefix(prefix: string): boolean {
	const normalized = prefix.toLowerCase();
	return normalized.length >= 3 && ('attribute'.startsWith(normalized) || 'attributes'.startsWith(normalized) || 'decorator'.startsWith(normalized) || 'decorators'.startsWith(normalized));
}

function isDecoratorNameSlot(content: string): boolean {
	let parenDepth = 0;
	let braceDepth = 0;
	let stringQuote: string | undefined;
	let escaped = false;
	for (let index = 0; index < content.length; index++) {
		const char = content[index];
		if (stringQuote) {
			if (escaped) {
				escaped = false;
			} else if (char === '\\') {
				escaped = true;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '"' || char === '\'') {
			stringQuote = char;
		} else if (char === '(') {
			parenDepth++;
		} else if (char === ')') {
			parenDepth = Math.max(0, parenDepth - 1);
		} else if (char === '{') {
			braceDepth++;
		} else if (char === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
		}
	}
	if (parenDepth !== 0 || braceDepth !== 0) {
		return false;
	}
	return /(?:^|,)\s*[A-Za-z_][A-Za-z0-9_]*$/.test(content) || /^\s*$/.test(content);
}

function getActiveArgumentPlaceholderRange(document: vscode.TextDocument, position: vscode.Position): vscode.Range | undefined {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.uri.toString() !== document.uri.toString() || editor.selection.isEmpty) {
		return undefined;
	}
	const selection = editor.selection;
	if (selection.start.line !== selection.end.line || position.line !== selection.start.line) {
		return undefined;
	}
	if (!selection.contains(position) && !selection.end.isEqual(position) && !selection.start.isEqual(position)) {
		return undefined;
	}
	const selectedText = document.getText(selection);
	if (!/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?$/.test(selectedText)) {
		return undefined;
	}
	const lineText = document.lineAt(selection.start.line).text;
	const before = lineText.slice(0, selection.start.character);
	const after = lineText.slice(selection.end.character);
	if (!/[,(]\s*$/.test(before) || !/^\s*[,)]/.test(after)) {
		return undefined;
	}
	return new vscode.Range(selection.start, selection.end);
}

function formatTraceCompletionLabels(list: vscode.CompletionList): string {
	return list.items.slice(0, 10).map(item => typeof item.label === 'string' ? item.label : item.label.label).join(', ');
}

function getPrefix(linePrefix: string): string {
	const directiveMatch = /^\s*#\s*([A-Za-z_]*)$/.exec(linePrefix);
	if (directiveMatch) {
		return directiveMatch[1] ? `#${directiveMatch[1]}` : '#';
	}
	const operatorMatch = /[=!<>|&+\-*\/%^~]+$/.exec(linePrefix);
	if (operatorMatch) {
		return operatorMatch[0];
	}
	return /[A-Za-z_][A-Za-z0-9_]*$/.exec(linePrefix)?.[0] ?? '';
}

function enumArgumentCompletionRange(linePrefix: string, position: vscode.Position, prefix: string): CompletionRange {
	const argumentStart = currentCallArgumentStart(linePrefix);
	const currentArgument = argumentStart === undefined ? '' : linePrefix.slice(argumentStart);
	const tokenMatch = /([A-Za-z_][A-Za-z0-9_]*(?:\.(?:[A-Za-z_][A-Za-z0-9_]*)?)?)$/.exec(currentArgument);
	const replaceStart = argumentStart !== undefined && tokenMatch && currentArgument.slice(0, tokenMatch.index).trim().length === 0
		? argumentStart + tokenMatch.index
		: position.character;
	return new vscode.Range(position.line, replaceStart, position.line, position.character);
}

function enumMemberAccessCompletionRange(receiver: string, linePrefix: string, position: vscode.Position, prefix: string): CompletionRange {
	const escapedReceiver = escapeRegExp(receiver);
	const match = new RegExp(`\\b${escapedReceiver}\\s*\\.\\s*(?:[A-Za-z_][A-Za-z0-9_]*)?$`).exec(linePrefix);
	if (!match) {
		return new vscode.Range(position.line, position.character - prefix.length, position.line, position.character);
	}
	return new vscode.Range(position.line, match.index, position.line, position.character);
}

function enumArgumentExpressionFilterText(linePrefix: string): string {
	const argumentStart = currentCallArgumentStart(linePrefix);
	const searchText = argumentStart === undefined ? linePrefix : linePrefix.slice(argumentStart);
	const tokenMatch = /([A-Za-z_][A-Za-z0-9_]*(?:\.(?:[A-Za-z_][A-Za-z0-9_]*)?)?)$/.exec(searchText);
	if (!tokenMatch || searchText.slice(0, tokenMatch.index).trim().length > 0) {
		return '';
	}
	return tokenMatch[1];
}

function trailingEnumExpressionFilterText(linePrefix: string): string {
	return /([A-Za-z_][A-Za-z0-9_]*(?:\.(?:[A-Za-z_][A-Za-z0-9_]*)?)?)$/.exec(linePrefix)?.[1] ?? '';
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function currentCallArgumentStart(linePrefix: string): number | undefined {
	const openIndex = innermostOpenParenIndex(linePrefix);
	if (openIndex < 0) {
		return undefined;
	}
	let start = openIndex + 1;
	let parenDepth = 0;
	let angleDepth = 0;
	let bracketDepth = 0;
	let braceDepth = 0;
	let stringQuote: string | undefined;
	for (let index = openIndex + 1; index < linePrefix.length; index++) {
		const char = linePrefix[index];
		if (stringQuote) {
			if (char === '\\') {
				index++;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			stringQuote = char;
			continue;
		}
		if (char === '(') {
			parenDepth++;
		} else if (char === ')') {
			parenDepth = Math.max(0, parenDepth - 1);
		} else if (char === '<') {
			angleDepth++;
		} else if (char === '>') {
			angleDepth = Math.max(0, angleDepth - 1);
		} else if (char === '[') {
			bracketDepth++;
		} else if (char === ']') {
			bracketDepth = Math.max(0, bracketDepth - 1);
		} else if (char === '{') {
			braceDepth++;
		} else if (char === '}') {
			braceDepth = Math.max(0, braceDepth - 1);
		} else if (char === ',' && parenDepth === 0 && angleDepth === 0 && bracketDepth === 0 && braceDepth === 0) {
			start = index + 1;
		}
	}
	return start;
}

function innermostOpenParenIndex(linePrefix: string): number {
	const stack: number[] = [];
	let stringQuote: string | undefined;
	for (let index = 0; index < linePrefix.length; index++) {
		const char = linePrefix[index];
		if (stringQuote) {
			if (char === '\\') {
				index++;
			} else if (char === stringQuote) {
				stringQuote = undefined;
			}
			continue;
		}
		if (char === '"' || char === "'") {
			stringQuote = char;
		} else if (char === '(') {
			stack.push(index);
		} else if (char === ')') {
			stack.pop();
		}
	}
	return stack[stack.length - 1] ?? -1;
}

function isOperatorPrefix(prefix: string): boolean {
	return /^[=!<>|&+\-*\/%^~]+$/.test(prefix);
}

function isConditionAssertionValueContext(linePrefix: string): boolean {
	if (!/\b(?:if|while|for)\s*\(/.test(linePrefix)) {
		return false;
	}
	const conditionText = textAfterLastOpenConditionParen(linePrefix);
	if (conditionText === undefined) {
		return false;
	}
	if (/\bfor\s*\([^;]*;[^;]*;/.test(linePrefix)) {
		return false;
	}
	return /(?:==|!=|<=|>=|<|>)\s*$/.test(conditionText);
}

function isConditionOperandOperatorContext(linePrefix: string): boolean {
	if (!/[ \t]$/.test(linePrefix) || !/\b(?:if|while|for)\s*\(/.test(linePrefix)) {
		return false;
	}
	const conditionText = textAfterLastOpenConditionParen(linePrefix);
	if (conditionText === undefined) {
		return false;
	}
	if (/\bfor\s*\([^;]*;[^;]*;/.test(linePrefix)) {
		return false;
	}
	const trimmed = conditionText.trimEnd();
	if (!trimmed || /(?:==|!=|<=|>=|<|>|&&|\|\||[=!<>|&+\-*\/%^~]|\b(?:if|while|for|foreach|switch|return|new)\b|\(|,)$/.test(trimmed)) {
		return false;
	}
	return /(?:[A-Za-z_][A-Za-z0-9_]*(?:\s*(?:\.|::)\s*[A-Za-z_][A-Za-z0-9_]*)*(?:\s*\([^()]*\))?|\)|\]|\btrue\b|\bfalse\b|\bnull\b|\d+(?:\.\d+)?)$/.test(trimmed);
}

function withConditionOperandCompletionAssist(candidates: BasicCompletionCandidate[], enabled: boolean): BasicCompletionCandidate[] {
	if (!enabled) {
		return candidates;
	}
	return candidates.map(candidate => {
		if (candidate.kind !== vscode.CompletionItemKind.Variable && candidate.kind !== vscode.CompletionItemKind.Property) {
			return candidate;
		}
		const insertText = candidate.insertText instanceof vscode.SnippetString
			? candidate.insertText
			: `${candidate.insertText ?? candidate.label} `;
		return {
			...candidate,
			insertText,
			command: { title: 'Trigger Suggest', command: 'editor.action.triggerSuggest' },
		};
	});
}

function isBoolValueCandidate(candidate: BasicCompletionCandidate): boolean {
	return normalizeTypeName(candidate.valueType) === 'bool';
}

function textAfterLastOpenConditionParen(linePrefix: string): string | undefined {
	let depth = 0;
	let lastConditionOpen = -1;
	for (let index = 0; index < linePrefix.length; index++) {
		const char = linePrefix[index];
		if (char === '(') {
			depth++;
			const beforeOpen = linePrefix.slice(0, index).trimEnd();
			if (/\b(?:if|while|for)$/.test(beforeOpen)) {
				lastConditionOpen = index;
			}
			continue;
		}
		if (char === ')') {
			depth = Math.max(0, depth - 1);
			if (depth === 0) {
				lastConditionOpen = -1;
			}
		}
	}
	return lastConditionOpen >= 0 ? linePrefix.slice(lastConditionOpen + 1) : undefined;
}

function filterStrictPrefixCandidates(items: BasicCompletionCandidate[], prefix: string): BasicCompletionCandidate[] {
	if (!prefix) {
		return [];
	}
	const normalized = prefix.toLowerCase();
	return items
		.filter(item => item.label.toLowerCase().startsWith(normalized))
		.sort((left, right) => left.label.length - right.label.length || left.sortText.localeCompare(right.sortText))
		.map(item => ({ ...item, ranking: { ...item.ranking, matchScore: directPrefixMatchScore(item.label, prefix) ?? 100 } }));
}

function isDeclarationLeadingGrammarPrefix(document: vscode.TextDocument, position: vscode.Position, linePrefix: string, prefix: string): boolean {
	if (prefix.length < 2 || !/^\s*[A-Za-z_][A-Za-z0-9_]*$/.test(linePrefix)) {
		return false;
	}
	const normalized = prefix.toLowerCase();
	if ('override'.startsWith(normalized)) {
		return true;
	}
	if (getEnclosingFunction(getParsedDocument(document), toParserPosition(position))) {
		return false;
	}
	return declarationLeadingGrammarKeywords.some(keyword => keyword.startsWith(normalized));
}

const declarationLeadingGrammarKeywords = [
	'modded', 'sealed', 'override', 'private', 'protected', 'public',
	'static', 'const', 'ref', 'autoptr', 'notnull', 'event', 'proto', 'external',
	'native', 'owned', 'volatile',
];

function filterCandidates(items: BasicCompletionCandidate[], prefix: string): BasicCompletionCandidate[] {
	if (!prefix) {
		return items;
	}
	return scoreCompletionCandidates(items, prefix)
		.filter((match): match is { item: BasicCompletionCandidate; score: number; index: number } => match.score !== undefined)
		.sort((left, right) => compareScoredCandidates(left, right))
		.map(match => ({ ...match.item, ranking: { ...match.item.ranking, matchScore: match.score } }));
}

function rankEquivalentCandidates(items: BasicCompletionCandidate[], prefix: string): BasicCompletionCandidate[] {
	if (!prefix) {
		return items;
	}
	return scoreDirectPrefixCompletionCandidates(items, prefix)
		.sort((left, right) => {
			if (left.score === undefined && right.score === undefined) {
				return left.index - right.index;
			}
			if (left.score === undefined || right.score === undefined) {
				return left.index - right.index;
			}
			return compareScoredCandidates(
				left as { item: BasicCompletionCandidate; score: number; index: number },
				right as { item: BasicCompletionCandidate; score: number; index: number }
			);
		})
		.map(match => match.score === undefined
			? match.item
			: { ...match.item, ranking: { ...match.item.ranking, matchScore: match.score } });
}

function rankExpectedTypeCandidates(enumCandidates: BasicCompletionCandidate[], valueCandidates: BasicCompletionCandidate[], prefix: string): BasicCompletionCandidate[] {
	if (!prefix || enumCandidates.length === 0 || valueCandidates.length === 0) {
		return [...enumCandidates, ...valueCandidates];
	}
	const combined = [...enumCandidates, ...valueCandidates];
	const scored = combined.map((item, index) => ({
		item,
		index,
		score: candidateSearchMatchScore(item, prefix),
		isExpectedEnum: item.ranking?.source === 'expected enum',
	}));
	const hasNonEnumTypedMatch = scored.some(match => !match.isExpectedEnum && match.score !== undefined);
	if (!hasNonEnumTypedMatch) {
		return combined;
	}
	return scored
		.sort((left, right) => {
			const leftScore = left.score ?? (left.isExpectedEnum ? 200 : 300);
			const rightScore = right.score ?? (right.isExpectedEnum ? 200 : 300);
			return leftScore - rightScore || left.item.sortText.localeCompare(right.item.sortText) || left.index - right.index;
		})
		.map(match => match.score === undefined
			? match.item
			: { ...match.item, ranking: { ...match.item.ranking, matchScore: match.score } });
}

function mergeHardContextCandidates(typeCandidates: BasicCompletionCandidate[], grammarCandidates: BasicCompletionCandidate[], prefix: string): BasicCompletionCandidate[] {
	if (grammarCandidates.length === 0) {
		return dedupeCandidates(typeCandidates);
	}

	let insertIndex = 0;
	while (insertIndex < typeCandidates.length && directPrefixMatchScore(typeCandidates[insertIndex].label, prefix) !== undefined) {
		insertIndex++;
	}
	return dedupeCandidates([
		...typeCandidates.slice(0, insertIndex),
		...grammarCandidates,
		...typeCandidates.slice(insertIndex),
	]);
}

function scoreCompletionCandidates(items: BasicCompletionCandidate[], prefix: string): Array<{ item: BasicCompletionCandidate; score: number | undefined; index: number }> {
	return items.map((item, index) => ({ item, score: candidateSearchMatchScore(item, prefix), index }));
}

function scoreDirectPrefixCompletionCandidates(items: BasicCompletionCandidate[], prefix: string): Array<{ item: BasicCompletionCandidate; score: number | undefined; index: number }> {
	return items.map((item, index) => ({ item, score: directPrefixMatchScore(item.label, prefix), index }));
}

function compareScoredCandidates(
	left: { item: BasicCompletionCandidate; score: number; index: number },
	right: { item: BasicCompletionCandidate; score: number; index: number }
): number {
	return left.score - right.score || left.item.sortText.localeCompare(right.item.sortText) || left.index - right.index;
}

function directPrefixMatchScore(label: string, prefix: string): number | undefined {
	const lengthPenalty = (label.length - prefix.length) * 10;
	if (label.startsWith(prefix)) {
		return lengthPenalty;
	}
	const normalizedLabel = label.toLowerCase();
	const normalizedPrefix = prefix.toLowerCase();
	if (normalizedLabel.startsWith(normalizedPrefix)) {
		return lengthPenalty + 1;
	}
	return undefined;
}

function candidateMatchScore(label: string, prefix: string): number | undefined {
	const normalizedLabel = label.toLowerCase();
	const normalizedPrefix = prefix.toLowerCase();
	const directScore = directPrefixMatchScore(label, prefix);
	if (directScore !== undefined) {
		return directScore;
	}
	if (normalizedLabel.length === 0) {
		return undefined;
	}
	if (normalizedPrefix.length >= 4 && editDistanceAtMostOneOrAdjacentTransposition(normalizedPrefix, normalizedLabel)) {
		return 20 + Math.abs(normalizedLabel.length - normalizedPrefix.length);
	}
	const labelPrefix = normalizedLabel.slice(0, Math.min(normalizedPrefix.length, normalizedLabel.length));
	if (normalizedPrefix.length >= 4 && editDistanceAtMostOneOrAdjacentTransposition(normalizedPrefix, labelPrefix)) {
		return 40 + Math.abs(normalizedLabel.length - normalizedPrefix.length);
	}
	const subsequenceScore = orderedSubsequenceScore(normalizedPrefix, normalizedLabel);
	return subsequenceScore === undefined
		? undefined
		: 60 + subsequenceScore;
}

function candidateSearchMatchScore(item: BasicCompletionCandidate, prefix: string): number | undefined {
	const labelScore = candidateMatchScore(item.label, prefix);
	const searchText = item.searchText;
	if (!searchText) {
		return labelScore;
	}
	const keyScores = searchKeys(searchText)
		.map(key => candidateMatchScore(key, prefix))
		.filter((score): score is number => score !== undefined)
		.map(score => score + 5);
	if (labelScore !== undefined) {
		keyScores.push(labelScore);
	}
	return keyScores.length > 0 ? Math.min(...keyScores) : undefined;
}

function searchKeys(value: string): string[] {
	const keys = new Set<string>();
	for (const part of value.split(/[^A-Za-z0-9_]+/).filter(Boolean)) {
		keys.add(part);
		keys.add(part.toLowerCase());
	}
	keys.add(value);
	return [...keys];
}

function editDistanceAtMostOneOrAdjacentTransposition(left: string, right: string): boolean {
	if (editDistanceAtMostOne(left, right)) {
		return true;
	}
	if (left.length !== right.length) {
		return false;
	}
	let firstMismatch = -1;
	for (let index = 0; index < left.length; index++) {
		if (left[index] !== right[index]) {
			if (firstMismatch >= 0) {
				return index === firstMismatch + 1
					&& left[firstMismatch] === right[index]
					&& left[index] === right[firstMismatch]
					&& left.slice(index + 1) === right.slice(index + 1);
			}
			firstMismatch = index;
		}
	}
	return false;
}

function editDistanceAtMostOne(left: string, right: string): boolean {
	if (left === right) {
		return true;
	}
	if (Math.abs(left.length - right.length) > 1) {
		return false;
	}

	let edits = 0;
	let leftIndex = 0;
	let rightIndex = 0;
	while (leftIndex < left.length && rightIndex < right.length) {
		if (left[leftIndex] === right[rightIndex]) {
			leftIndex++;
			rightIndex++;
			continue;
		}

		edits++;
		if (edits > 1) {
			return false;
		}

		if (left.length > right.length) {
			leftIndex++;
		} else if (right.length > left.length) {
			rightIndex++;
		} else {
			leftIndex++;
			rightIndex++;
		}
	}

	if (leftIndex < left.length || rightIndex < right.length) {
		edits++;
	}
	return edits <= 1;
}

function orderedSubsequenceScore(prefix: string, label: string): number | undefined {
	if (prefix.length < 3 || label.length - prefix.length > 2) {
		return undefined;
	}

	let searchFrom = 0;
	let previousIndex = -1;
	let gapPenalty = 0;
	let firstMatchIndex = -1;
	for (const char of prefix) {
		const matchIndex = label.indexOf(char, searchFrom);
		if (matchIndex < 0) {
			return undefined;
		}
		if (firstMatchIndex < 0) {
			firstMatchIndex = matchIndex;
			if (firstMatchIndex > 1) {
				return undefined;
			}
		}
		if (previousIndex >= 0) {
			gapPenalty += Math.max(0, matchIndex - previousIndex - 1);
		}
		previousIndex = matchIndex;
		searchFrom = matchIndex + 1;
	}

	return firstMatchIndex + gapPenalty + Math.abs(label.length - prefix.length);
}


