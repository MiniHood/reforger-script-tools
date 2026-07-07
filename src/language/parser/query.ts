import type {
	EnforceParserPosition,
	EnforceParserRange,
	EnforceParserScopeFact,
	EnforceSyntaxNode,
	ParsedEnforceSource,
} from './ast';

export interface EnforceReceiverExpression {
	text: string;
	range: EnforceParserRange;
	memberName?: string;
}

export interface EnforceExpectedType {
	valueType?: string;
	context: 'assignment' | 'return' | 'argument' | 'case' | 'unknown';
}

export interface EnforceSwitchContext {
	expression?: string;
	range: EnforceParserRange;
}

export interface EnforceReferenceShape {
	kind: 'declaration' | 'member' | 'staticMember' | 'call' | 'typeUsage' | 'local' | 'unknown';
	name?: string;
	receiver?: string;
	range?: EnforceParserRange;
}

export function getNodeAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	const matches = parsed.nodes.filter(node => rangeContainsPosition(node.range, position));
	const structuralMatches = matches.filter(node => node.kind !== 'identifier' && node.kind !== 'literal');
	return (structuralMatches.length > 0 ? structuralMatches : matches)
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

export function getEnclosingClass(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	return getEnclosingNode(parsed, position, ['class']);
}

export function getEnclosingFunction(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	return getEnclosingNode(parsed, position, ['function', 'memberFunction', 'constructor', 'destructor']);
}

export function getVisibleLocals(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceParserScopeFact[] {
	const enclosingFunction = getEnclosingFunction(parsed, position);
	if (!enclosingFunction) {
		return [];
	}

	const functionName = enclosingFunction.name;
	const containerName = enclosingFunction.containerName;
	return parsed.scopes.filter(scope =>
		(scope.kind === 'parameter' || scope.kind === 'local' || scope.kind === 'foreach')
		&& scope.name !== undefined
		&& scope.functionName === functionName
		&& (containerName === undefined || scope.containerName === containerName)
		&& (rangeContainsPosition(scope.range, position) || (
			rangeStartsBeforeOrAt(scope.range, position)
			&& (!scopeLifetimeEnd(scope, parsed) || comparePositions(position, scopeLifetimeEnd(scope, parsed)!) <= 0)
		))
	);
}

export function getScopeAtPosition(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceParserScopeFact | undefined {
	return parsed.scopes
		.filter(scope => rangeContainsPosition(scope.range, position) || (scope.selectionRange !== undefined && rangeContainsPosition(scope.selectionRange, position)))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

export function getExpressionAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	const expressionKinds = [
		'assignmentExpression',
		'binaryExpression',
		'unaryExpression',
		'literal',
		'identifier',
		'parenthesizedExpression',
		'argumentList',
		'memberAccess',
		'callExpression',
		'indexAccess',
		'castExpression',
		'newExpression',
		'incompleteExpression',
		'returnStatement',
		'expressionStatement',
	];
	return parsed.nodes
		.filter(node => expressionKinds.includes(node.kind) && rangeContainsPosition(node.range, position))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

export function getMemberAccessAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	return parsed.nodes
		.filter(node => (node.kind === 'memberAccess' || node.kind === 'incompleteExpression') && rangeContainsPosition(node.range, position))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

export function getCallAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	return parsed.nodes
		.filter(node => (node.kind === 'callExpression' || node.kind === 'castExpression') && rangeContainsPosition(node.range, position))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

export function getSwitchContext(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSwitchContext | undefined {
	const node = parsed.nodes
		.filter(candidate => candidate.kind === 'switch' && rangeContainsPosition(candidate.range, position))
		.sort((left, right) => Number(Boolean(right.expression)) - Number(Boolean(left.expression)) || rangeSize(left.range) - rangeSize(right.range))[0];
	if (!node) {
		return undefined;
	}
	const expression = /\bswitch\s*\((.*)\)/s.exec(node.expression ?? node.signature ?? '')?.[1]?.trim();
	return { expression, range: node.range };
}

export function getTypeUsageAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	const node = getNodeAt(parsed, position);
	if (!node) {
		return undefined;
	}
	return ['class', 'enum', 'property', 'local', 'parameter', 'foreach', 'function', 'memberFunction', 'newExpression', 'castExpression'].includes(node.kind)
		? node
		: undefined;
}

export function getReferenceShapeAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceReferenceShape {
	const declaration = getDeclarationAt(parsed, position);
	if (declaration && rangeContainsPosition(declaration.selectionRange ?? declaration.range, position)) {
		return { kind: 'declaration', name: declaration.name, range: declaration.selectionRange ?? declaration.range };
	}
	const member = getMemberAccessAt(parsed, position);
	if (member) {
		return {
			kind: member.accessOperator === '::' ? 'staticMember' : 'member',
			name: member.memberName ?? member.name,
			receiver: member.receiver,
			range: member.selectionRange ?? member.range,
		};
	}
	const call = getCallAt(parsed, position);
	if (call) {
		return { kind: 'call', name: call.name, range: call.selectionRange ?? call.range };
	}
	const typeUsage = getTypeUsageAt(parsed, position);
	if (typeUsage) {
		return { kind: 'typeUsage', name: typeUsage.name, range: typeUsage.selectionRange ?? typeUsage.range };
	}
	return { kind: 'unknown' };
}

export function getReceiverExpression(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceReceiverExpression | undefined {
	if (isIgnoredPosition(parsed, position)) {
		return undefined;
	}
	const accessNode = getMemberAccessAt(parsed, position);
	if (accessNode?.receiver) {
		return {
			text: accessNode.receiver,
			range: accessNode.range,
			memberName: accessNode.memberName,
		};
	}
	return undefined;
}

export function getExpectedType(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceExpectedType {
	if (isIgnoredPosition(parsed, position)) {
		return { context: 'unknown' };
	}
	const expression = getExpressionAt(parsed, position);
	if (expression?.kind === 'returnStatement') {
		return { context: 'return', valueType: getFunctionReturnType(getEnclosingFunction(parsed, position)) };
	}
	if (findOpenReturnStatement(parsed, position)) {
		return { context: 'return', valueType: getFunctionReturnType(getEnclosingFunction(parsed, position)) };
	}
	if (expression?.kind === 'assignmentExpression') {
		return { context: 'assignment', valueType: getAssignmentValueType(expression.expression) };
	}
	if (getCallAt(parsed, position)) {
		return { context: 'argument' };
	}
	if (parsed.nodes.some(node => node.kind === 'case' && rangeContainsPosition(node.range, position))) {
		return { context: 'case' };
	}

	return { context: 'unknown' };
}

function findOpenReturnStatement(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	return parsed.nodes
		.filter(node =>
			node.kind === 'returnStatement'
			&& node.range.start.line === position.line
			&& comparePositions(node.range.start, position) <= 0
			&& !/;\s*$/.test(node.expression ?? '')
		)
		.sort((left, right) => comparePositions(right.range.start, left.range.start))[0];
}

export function getDeclarationAt(parsed: ParsedEnforceSource, position: EnforceParserPosition): EnforceSyntaxNode | undefined {
	const node = getNodeAt(parsed, position);
	if (!node) {
		return undefined;
	}
	if (isDeclarationNode(node)) {
		return node;
	}
	return getEnclosingNode(parsed, position, ['property', 'local', 'parameter', 'foreach', 'function', 'memberFunction', 'class', 'enum', 'enumMember', 'macro']);
}

function getEnclosingNode(parsed: ParsedEnforceSource, position: EnforceParserPosition, kinds: string[]): EnforceSyntaxNode | undefined {
	return parsed.nodes
		.filter(node => kinds.includes(node.kind) && rangeContainsPosition(node.range, position))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))[0];
}

function isDeclarationNode(node: EnforceSyntaxNode): boolean {
	return [
		'class',
		'enum',
		'enumMember',
		'function',
		'memberFunction',
		'constructor',
		'destructor',
		'property',
		'macro',
		'parameter',
		'local',
		'foreach',
	].includes(node.kind);
}

function getFunctionReturnType(enclosingFunction: EnforceSyntaxNode | undefined): string | undefined {
	if (!enclosingFunction?.signature) {
		return undefined;
	}
	const beforeParen = enclosingFunction.signature.slice(0, enclosingFunction.signature.indexOf('(')).trim();
	const parts = beforeParen.split(/\s+/).filter(Boolean);
	if (parts.length < 2) {
		return undefined;
	}
	return parts[parts.length - 2];
}

function getAssignmentValueType(expression: string | undefined): string | undefined {
	if (!expression) {
		return undefined;
	}
	const assignmentIndex = expression.indexOf('=');
	if (assignmentIndex < 0) {
		return undefined;
	}
	const left = expression.slice(0, assignmentIndex).trim();
	const match = /^(?:[A-Za-z_]\w*\s+)*([A-Za-z_]\w*(?:\s*<[^=;]+>)?(?:\s*\[[^\]]*\])?)\s+[A-Za-z_]\w*$/.exec(left);
	return match?.[1]?.replace(/\s+/g, ' ').trim();
}

export function isIgnoredPosition(parsed: ParsedEnforceSource, position: EnforceParserPosition): boolean {
	return parsed.tokens.some(token =>
		(token.kind === 'string' || token.kind === 'comment')
		&& comparePositions({ line: token.line, character: token.character }, position) <= 0
		&& comparePositions(position, { line: token.endLine, character: token.endCharacter }) <= 0
	);
}

function scopeLifetimeEnd(scope: EnforceParserScopeFact, parsed: ParsedEnforceSource): EnforceParserPosition | undefined {
	if (scope.kind === 'parameter') {
		return getEnclosingFunction(parsed, scope.range.start)?.range.end;
	}
	const containingBlock = [
		...parsed.nodes.filter(node => node.kind === 'block').map(node => node.range),
		...parsed.scopes.filter(candidate => candidate.kind === 'block').map(candidate => candidate.range),
	]
		.filter(range => rangeContainsPosition(range, scope.range.start))
		.sort((left, right) => rangeSize(left) - rangeSize(right))[0];
	return containingBlock?.end ?? getEnclosingFunction(parsed, scope.range.start)?.range.end;
}

function rangeContainsPosition(range: EnforceParserRange, position: EnforceParserPosition): boolean {
	return comparePositions(range.start, position) <= 0 && comparePositions(position, range.end) <= 0;
}

function rangeStartsBeforeOrAt(range: EnforceParserRange, position: EnforceParserPosition): boolean {
	return comparePositions(range.start, position) <= 0;
}

function rangeSize(range: EnforceParserRange): number {
	return (range.end.line - range.start.line) * 100000 + (range.end.character - range.start.character);
}

function comparePositions(left: EnforceParserPosition, right: EnforceParserPosition): number {
	if (left.line !== right.line) {
		return left.line - right.line;
	}
	return left.character - right.character;
}
