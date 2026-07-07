import type { EnforceSymbol } from '../index/symbolIndex';
import type { EnforceParserScopeFact, EnforceSyntaxNode } from './ast';
import { SourceTextInfo, tokenRangeToParserRange } from './source';
import { symbolToSyntaxNode } from './symbolAdapter';
import type { EnforceToken } from './tokens';

export function buildSyntaxNodes(
	symbols: EnforceSymbol[],
	scopes: EnforceParserScopeFact[],
	bodyNodes: EnforceSyntaxNode[],
	tokens: EnforceToken[],
	source: SourceTextInfo
): EnforceSyntaxNode[] {
	const eof = tokens.find(token => token.kind === 'eof');
	const endLine = source.lines.length > 0 ? source.lines.length - 1 : 0;
	const endCharacter = source.lines[endLine]?.length ?? 0;
	const root: EnforceSyntaxNode = {
		kind: 'sourceFile',
		range: {
			start: { line: 0, character: 0 },
			end: eof ? { line: eof.line, character: eof.character } : { line: endLine, character: endCharacter },
		},
		complete: tokens.every(token => !token.unterminated),
		confidence: 'high',
		children: [],
	};
	const nodes: EnforceSyntaxNode[] = [root];
	nodes.push(...symbols.map(symbolToSyntaxNode));
	nodes.push(...buildPreprocessorNodes(tokens));
	nodes.push(...scopes.map(scopeToSyntaxNode));
	nodes.push(...bodyNodes);
	assignSyntaxTree(nodes);
	return nodes;
}

function assignSyntaxTree(nodes: EnforceSyntaxNode[]): void {
	nodes.forEach((node, index) => {
		node.id = createNodeId(node, index);
		node.parentId = undefined;
		node.children = [];
	});

	const root = nodes[0];
	if (!root) {
		return;
	}

	for (const node of nodes.slice(1)) {
		const parent = findSmallestContainingParent(nodes, node) ?? root;
		node.parentId = parent.id;
		parent.children?.push(node);
	}

	for (const node of nodes) {
		node.children?.sort((left, right) => compareRanges(left.range, right.range));
	}
}

function findSmallestContainingParent(nodes: EnforceSyntaxNode[], node: EnforceSyntaxNode): EnforceSyntaxNode | undefined {
	return nodes
		.filter(candidate => candidate !== node && rangeStrictlyContains(candidate.range, node.range))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

function createNodeId(node: EnforceSyntaxNode, index: number): string {
	const start = `${node.range.start.line}:${node.range.start.character}`;
	const end = `${node.range.end.line}:${node.range.end.character}`;
	const name = node.name ?? node.memberName ?? node.operator ?? '';
	return `${node.kind}:${start}:${end}:${name}:${index}`;
}

function rangeStrictlyContains(outer: EnforceSyntaxNode['range'], inner: EnforceSyntaxNode['range']): boolean {
	const startsBeforeOrAt = comparePositions(outer.start, inner.start) <= 0;
	const endsAfterOrAt = comparePositions(outer.end, inner.end) >= 0;
	const sameStart = comparePositions(outer.start, inner.start) === 0;
	const sameEnd = comparePositions(outer.end, inner.end) === 0;
	return startsBeforeOrAt && endsAfterOrAt && !(sameStart && sameEnd);
}

function rangeSize(range: EnforceSyntaxNode['range']): number {
	return (range.end.line - range.start.line) * 100000 + (range.end.character - range.start.character);
}

function compareRanges(left: EnforceSyntaxNode['range'], right: EnforceSyntaxNode['range']): number {
	return comparePositions(left.start, right.start) || comparePositions(left.end, right.end);
}

function comparePositions(left: EnforceSyntaxNode['range']['start'], right: EnforceSyntaxNode['range']['start']): number {
	if (left.line !== right.line) {
		return left.line - right.line;
	}
	return left.character - right.character;
}

function buildPreprocessorNodes(tokens: EnforceToken[]): EnforceSyntaxNode[] {
	const nodes: EnforceSyntaxNode[] = [];
	const stack: EnforceSyntaxNode[] = [];
	for (const token of tokens) {
		if (token.kind !== 'preprocessor') {
			continue;
		}
		const trimmed = token.text.trim();
		const directiveMatch = /^#\s*([A-Za-z_]\w*)\b\s*([A-Za-z_]\w*)?/.exec(trimmed);
		const directive = directiveMatch?.[1];
		const name = directiveMatch?.[2] ?? directive;
		const directiveNode: EnforceSyntaxNode = {
			kind: 'compilerDirective',
			name,
			signature: trimmed,
			range: tokenRangeToParserRange(token),
			selectionRange: tokenRangeToParserRange(token),
			complete: true,
			confidence: 'high',
		};
		nodes.push(directiveNode);

		if (directive === 'ifdef' || directive === 'ifndef') {
			const block: EnforceSyntaxNode = {
				kind: 'preprocessorBlock',
				name,
				signature: trimmed,
				range: tokenRangeToParserRange(token),
				selectionRange: tokenRangeToParserRange(token),
				complete: false,
				missingToken: '#endif',
				confidence: 'medium',
				children: [directiveNode],
			};
			stack.push(block);
			nodes.push(block);
		} else if (directive === 'else' && stack.length > 0) {
			stack[stack.length - 1].children?.push(directiveNode);
		} else if (directive === 'endif' && stack.length > 0) {
			const block = stack.pop();
			if (block) {
				block.range = {
					start: block.range.start,
					end: { line: token.endLine, character: token.endCharacter },
				};
				block.complete = true;
				block.missingToken = undefined;
				block.children?.push(directiveNode);
			}
		}
	}
	return nodes;
}

function scopeToSyntaxNode(scope: EnforceParserScopeFact): EnforceSyntaxNode {
	return {
		kind: scope.kind,
		name: scope.name,
		containerName: scope.containerName,
		signature: scope.name,
		valueType: scope.valueType,
		range: scope.range,
		selectionRange: scope.selectionRange,
		complete: true,
		confidence: 'high',
	};
}
