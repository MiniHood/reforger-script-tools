import * as vscode from 'vscode';
import type { EnforceSymbol } from '../index/symbolIndex';
import type { EnforceParserRange, EnforceSyntaxNode, EnforceSyntaxNodeKind } from './ast';

export function addClassMemberSummaries(symbols: EnforceSymbol[]): void {
	for (const classSymbol of symbols.filter(symbol => symbol.type === 'class')) {
		const members = symbols.filter(symbol => symbol.containerName === classSymbol.name);
		classSymbol.functions = members
			.filter(symbol => symbol.type === 'memberFunction')
			.map(symbol => symbol.signature ?? symbol.name);
		classSymbol.properties = members
			.filter(symbol => symbol.type === 'property')
			.map(symbol => symbol.signature ?? symbol.name);
	}
}

export function symbolToSyntaxNode(symbol: EnforceSymbol): EnforceSyntaxNode {
	const symbolRange = vscodeRangeToParserRange(symbol.range);
	const bodyRange = symbol.bodyRange ? vscodeRangeToParserRange(symbol.bodyRange) : undefined;
	return {
		kind: symbolTypeToNodeKind(symbol),
		name: symbol.name,
		containerName: symbol.containerName,
		signature: symbol.signature,
		declarationKind: symbol.declarationKind,
		modifiers: symbol.modifiers,
		range: bodyRange ? { start: symbolRange.start, end: bodyRange.end } : symbolRange,
		bodyRange,
		selectionRange: vscodeRangeToParserRange(symbol.selectionRange),
		complete: true,
		confidence: 'high',
	};
}

function symbolTypeToNodeKind(symbol: EnforceSymbol): EnforceSyntaxNodeKind {
	if (symbol.type === 'enumValue') {
		return 'enumMember';
	}
	if (symbol.declarationKind === 'constructor') {
		return 'constructor';
	}
	if (symbol.declarationKind === 'destructor') {
		return 'destructor';
	}
	return symbol.type;
}

export function vscodeRangeToParserRange(range: vscode.Range): EnforceParserRange {
	return {
		start: { line: range.start.line, character: range.start.character },
		end: { line: range.end.line, character: range.end.character },
	};
}
