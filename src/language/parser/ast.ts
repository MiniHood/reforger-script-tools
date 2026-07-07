import type { EnforceSymbol } from '../index/symbolIndex';
import type { EnforceToken } from './tokens';

export interface EnforceParserPosition {
	line: number;
	character: number;
}

export interface EnforceParserRange {
	start: EnforceParserPosition;
	end: EnforceParserPosition;
}

export type EnforceSyntaxNodeKind =
	| 'sourceFile'
	| 'attribute'
	| 'preprocessorBlock'
	| 'compilerDirective'
	| 'class'
	| 'enum'
	| 'enumMember'
	| 'function'
	| 'memberFunction'
	| 'constructor'
	| 'destructor'
	| 'property'
	| 'macro'
	| 'parameter'
	| 'local'
	| 'foreach'
	| 'block'
	| 'if'
	| 'else'
	| 'while'
	| 'for'
	| 'switch'
	| 'case'
	| 'breakStatement'
	| 'continueStatement'
	| 'returnStatement'
	| 'expressionStatement'
	| 'declarationStatement'
	| 'forInitializer'
	| 'forUpdate'
	| 'argumentList'
	| 'emptyStatement'
	| 'binaryExpression'
	| 'unaryExpression'
	| 'literal'
	| 'identifier'
	| 'parenthesizedExpression'
	| 'assignmentExpression'
	| 'memberAccess'
	| 'callExpression'
	| 'indexAccess'
	| 'castExpression'
	| 'newExpression'
	| 'incompleteExpression';

export type EnforceParserConfidence = 'high' | 'medium' | 'low';

export interface EnforceSyntaxNode {
	id?: string;
	parentId?: string;
	kind: EnforceSyntaxNodeKind;
	name?: string;
	range: EnforceParserRange;
	bodyRange?: EnforceParserRange;
	selectionRange?: EnforceParserRange;
	containerName?: string;
	signature?: string;
	declarationKind?: string;
	modifiers?: string[];
	valueType?: string;
	expression?: string;
	receiver?: string;
	memberName?: string;
	operator?: string;
	accessOperator?: '.' | '::';
	complete?: boolean;
	missingToken?: string;
	unterminated?: boolean;
	incomplete?: boolean;
	recovered?: boolean;
	confidence?: EnforceParserConfidence;
	children?: EnforceSyntaxNode[];
}

export interface EnforceParserDiagnostic {
	message: string;
	range: EnforceParserRange;
	severity: 'info' | 'warning' | 'error';
}

export type EnforceParserScopeKind = 'parameter' | 'local' | 'foreach' | 'block' | 'switch' | 'case';

export interface EnforceParserScopeFact {
	kind: EnforceParserScopeKind;
	range: EnforceParserRange;
	selectionRange?: EnforceParserRange;
	containerName?: string;
	functionName?: string;
	name?: string;
	valueType?: string;
	depth?: number;
}

export interface ParsedEnforceSource {
	sourceText: string;
	tokens: EnforceToken[];
	nodes: EnforceSyntaxNode[];
	symbols: EnforceSymbol[];
	scopes: EnforceParserScopeFact[];
	diagnostics: EnforceParserDiagnostic[];
}
