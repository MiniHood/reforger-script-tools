import * as vscode from 'vscode';
import { tracePerformance } from '../../core/performanceTrace';
import { getParsedDocument, toVscodeRange } from '../parser/documentCache';
import type { EnforceParserPosition, EnforceParserRange, EnforceSyntaxNode, ParsedEnforceSource } from '../parser/ast';
import type { EnforceToken } from '../parser/tokens';

export type BracketDecorationRole = 'active' | 'class' | 'function' | 'variable';

export interface BracketDecorationRanges {
	active: vscode.Range[];
	activeScope: vscode.Range[];
	class: vscode.Range[];
	function: vscode.Range[];
	variable: vscode.Range[];
}

const activeBracketColor = new vscode.ThemeColor('reforgerScriptTools.bracket.active');
const activeScopeColor = new vscode.ThemeColor('reforgerScriptTools.bracket.scopeGuide');
const classBracketColor = new vscode.ThemeColor('reforgerScriptTools.bracket.class');
const functionBracketColor = new vscode.ThemeColor('reforgerScriptTools.bracket.function');
const variableBracketColor = new vscode.ThemeColor('reforgerScriptTools.bracket.variable');

const openToClose = new Map([
	['(', ')'],
	['{', '}'],
	['[', ']'],
	['<', '>'],
]);

const closeToOpen = new Map([...openToClose.entries()].map(([open, close]) => [close, open]));
const controlKeywords = new Set(['if', 'for', 'foreach', 'while', 'switch']);

export class BracketDecorationController implements vscode.Disposable {
	private readonly activeDecoration = createBracketDecoration(activeBracketColor);
	private readonly activeScopeDecoration = createActiveScopeDecoration(activeScopeColor);
	private readonly classDecoration = createBracketDecoration(classBracketColor);
	private readonly functionDecoration = createBracketDecoration(functionBracketColor);
	private readonly variableDecoration = createBracketDecoration(variableBracketColor);
	private readonly pendingUpdates = new Map<string, ReturnType<typeof setTimeout>>();
	private readonly disposables: vscode.Disposable[] = [
		this.activeDecoration,
		this.activeScopeDecoration,
		this.classDecoration,
		this.functionDecoration,
		this.variableDecoration,
	];

	constructor(context: vscode.ExtensionContext) {
		this.disposables.push(
			vscode.window.onDidChangeActiveTextEditor(() => this.updateVisibleEditors()),
			vscode.window.onDidChangeTextEditorSelection(event => this.scheduleEditorUpdate(event.textEditor)),
			vscode.window.onDidChangeVisibleTextEditors(() => this.updateVisibleEditors()),
			vscode.workspace.onDidChangeTextDocument(event => this.scheduleDocumentEditorsUpdate(event.document)),
			vscode.workspace.onDidCloseTextDocument(document => this.clearDocumentEditors(document))
		);
		context.subscriptions.push(this);
		this.updateVisibleEditors();
	}

	dispose(): void {
		for (const timeout of this.pendingUpdates.values()) {
			clearTimeout(timeout);
		}
		this.pendingUpdates.clear();
		for (const disposable of this.disposables) {
			disposable.dispose();
		}
	}

	private updateVisibleEditors(): void {
		for (const editor of vscode.window.visibleTextEditors) {
			this.updateEditor(editor);
		}
	}

	private updateDocumentEditors(document: vscode.TextDocument): void {
		for (const editor of vscode.window.visibleTextEditors.filter(candidate => candidate.document.uri.toString() === document.uri.toString())) {
			this.updateEditor(editor);
		}
	}

	private scheduleDocumentEditorsUpdate(document: vscode.TextDocument): void {
		const key = document.uri.toString();
		const existing = this.pendingUpdates.get(key);
		if (existing) {
			clearTimeout(existing);
		}
		this.pendingUpdates.set(key, setTimeout(() => {
			this.pendingUpdates.delete(key);
			this.updateDocumentEditors(document);
		}, 25));
	}

	private scheduleEditorUpdate(editor: vscode.TextEditor): void {
		if (!isDecoratedDocument(editor.document)) {
			this.updateEditor(editor);
			return;
		}
		this.scheduleDocumentEditorsUpdate(editor.document);
	}

	private clearDocumentEditors(document: vscode.TextDocument): void {
		const key = document.uri.toString();
		const existing = this.pendingUpdates.get(key);
		if (existing) {
			clearTimeout(existing);
			this.pendingUpdates.delete(key);
		}
		for (const editor of vscode.window.visibleTextEditors.filter(candidate => candidate.document.uri.toString() === document.uri.toString())) {
			this.apply(editor, emptyBracketDecorationRanges());
		}
	}

	private updateEditor(editor: vscode.TextEditor): void {
		if (!isDecoratedDocument(editor.document)) {
			this.apply(editor, emptyBracketDecorationRanges());
			return;
		}

		const ranges = tracePerformance(
			'model.bracketDecorations',
			`${editor.document.uri.fsPath.split(/[\\/]/).pop() ?? editor.document.uri.toString()} | version=${editor.document.version}`,
			() => collectBracketDecorationRanges(editor.document, editor.selection.active)
		);
		this.apply(editor, ranges);
	}

	private apply(editor: vscode.TextEditor, ranges: BracketDecorationRanges): void {
		editor.setDecorations(this.activeDecoration, ranges.active);
		editor.setDecorations(this.activeScopeDecoration, ranges.activeScope);
		editor.setDecorations(this.classDecoration, ranges.class);
		editor.setDecorations(this.functionDecoration, ranges.function);
		editor.setDecorations(this.variableDecoration, ranges.variable);
	}
}

export function registerBracketDecorations(context: vscode.ExtensionContext): void {
	new BracketDecorationController(context);
}

export function collectBracketDecorationRanges(document: vscode.TextDocument, activePosition?: vscode.Position): BracketDecorationRanges {
	return collectBracketDecorationRangesFromParsed(getParsedDocument(document), activePosition);
}

export function collectBracketDecorationRangesFromParsed(parsed: ParsedEnforceSource, activePosition?: vscode.Position): BracketDecorationRanges {
	const brackets = collectBracketPairs(parsed);
	const activeParserPosition = activePosition ? { line: activePosition.line, character: activePosition.character } : undefined;
	const activePair = activeParserPosition ? findActivePair(brackets.pairs, activeParserPosition) ?? findActiveBodyPair(parsed, activeParserPosition) : undefined;
	const activeOpen = activeParserPosition && !activePair ? findActiveUnmatchedOpen(brackets.unmatchedOpen, activeParserPosition, parsed) : undefined;
	const ranges = emptyBracketDecorationRanges();
	for (const pair of brackets.pairs) {
		const role: BracketDecorationRole = pair === activePair ? 'active' : roleForBracketPair(parsed, pair);
		ranges[role].push(toVscodeRange(pair.open.range), toVscodeRange(pair.close.range));
	}
	if (activePair && shouldDrawActiveScopeGuide(parsed, activePair)) {
		ranges.activeScope.push(...activeScopeRanges(parsed, activePair).map(toVscodeRange));
	}
	if (activeOpen) {
		ranges.active.push(toVscodeRange(activeOpen.range));
	}
	return ranges;
}

function emptyBracketDecorationRanges(): BracketDecorationRanges {
	return { active: [], activeScope: [], class: [], function: [], variable: [] };
}

function createBracketDecoration(color: string | vscode.ThemeColor): vscode.TextEditorDecorationType {
	return vscode.window.createTextEditorDecorationType({
		color,
		rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
	});
}

function createActiveScopeDecoration(color: string | vscode.ThemeColor): vscode.TextEditorDecorationType {
	return vscode.window.createTextEditorDecorationType({
		borderColor: color,
		borderStyle: 'solid',
		borderWidth: '0 0 0 1px',
		rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
	});
}

function isDecoratedDocument(document: vscode.TextDocument): boolean {
	return document.languageId === 'enforce' && document.uri.scheme === 'file';
}

interface BracketPair {
	open: BracketUnit;
	close: BracketUnit;
}

interface BracketUnit {
	text: '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>';
	token: EnforceToken;
	range: EnforceParserRange;
}

interface BracketCollection {
	pairs: BracketPair[];
	unmatchedOpen: BracketUnit[];
}

function collectBracketPairs(parsed: ParsedEnforceSource): BracketCollection {
	const pairs: BracketPair[] = [];
	const stack: BracketUnit[] = [];
	const significant = parsed.tokens.filter(token => !isTriviaOrIgnored(token));
	for (let index = 0; index < significant.length; index++) {
		for (const unit of bracketUnitsForToken(significant, index)) {
			if (openToClose.has(unit.text)) {
				stack.push(unit);
				continue;
			}
			const expectedOpen = closeToOpen.get(unit.text);
			if (!expectedOpen) {
				continue;
			}
			const openIndex = findLastOpenIndex(stack, expectedOpen);
			if (openIndex < 0) {
				continue;
			}
			const [open] = stack.splice(openIndex, 1);
			pairs.push({ open, close: unit });
		}
	}
	return { pairs, unmatchedOpen: stack };
}

function bracketUnitsForToken(tokens: readonly EnforceToken[], index: number): BracketUnit[] {
	const token = tokens[index];
	if (['(', ')', '{', '}', '[', ']'].includes(token.text)) {
		return [bracketUnit(token.text as BracketUnit['text'], token, token.character)];
	}
	if (token.text === '<' && isGenericAngleOpen(tokens, index)) {
		return [bracketUnit('<', token, token.character)];
	}
	if (token.text === '>') {
		return [bracketUnit('>', token, token.character)];
	}
	if (token.text === '>>') {
		return [bracketUnit('>', token, token.character), bracketUnit('>', token, token.character + 1)];
	}
	return [];
}

function bracketUnit(text: BracketUnit['text'], token: EnforceToken, character: number): BracketUnit {
	return {
		text,
		token,
		range: {
			start: { line: token.line, character },
			end: { line: token.line, character: character + 1 },
		},
	};
}

function isGenericAngleOpen(tokens: readonly EnforceToken[], index: number): boolean {
	const token = tokens[index];
	const previous = previousSignificantToken(tokens, index - 1);
	const next = nextSignificantToken(tokens, index + 1);
	return previous !== undefined
		&& isIdentifierLike(previous)
		&& previous.end === token.start
		&& next !== undefined
		&& isIdentifierLike(next)
		&& hasGenericAngleClose(tokens, index + 1);
}

function hasGenericAngleClose(tokens: readonly EnforceToken[], startIndex: number): boolean {
	let depth = 1;
	for (let index = startIndex; index < tokens.length; index++) {
		const token = tokens[index];
		if (isTriviaOrIgnored(token)) {
			continue;
		}
		if ([';', '{', '}', '(', ')', '='].includes(token.text)) {
			return false;
		}
		if (token.text === '<' && isGenericAngleOpen(tokens, index)) {
			depth++;
			continue;
		}
		if (token.text === '>>') {
			depth -= 2;
		} else if (token.text === '>') {
			depth--;
		}
		if (depth <= 0) {
			return true;
		}
	}
	return false;
}

function findLastOpenIndex(stack: readonly BracketUnit[], expectedOpen: string): number {
	for (let index = stack.length - 1; index >= 0; index--) {
		if (stack[index].text === expectedOpen) {
			return index;
		}
	}
	return -1;
}

function findActivePair(pairs: readonly BracketPair[], position: EnforceParserPosition): BracketPair | undefined {
	return pairs
		.filter(pair => rangeContains(pairRange(pair), position))
		.sort((left, right) => rangeSize(pairRange(left)) - rangeSize(pairRange(right)))[0];
}

function findActiveBodyPair(parsed: ParsedEnforceSource, position: EnforceParserPosition): BracketPair | undefined {
	const owner = parsed.nodes
		.filter(node => node.bodyRange && rangeContains(node.bodyRange, position))
		.sort((left, right) => rangeSize(left.bodyRange ?? left.range) - rangeSize(right.bodyRange ?? right.range))[0];
	if (!owner?.bodyRange) {
		return undefined;
	}
	const open = bracketUnitAt(parsed.tokens, owner.bodyRange.start, '{');
	const close = bracketUnitAt(parsed.tokens, { line: owner.bodyRange.end.line, character: owner.bodyRange.end.character - 1 }, '}');
	return open && close ? { open, close } : undefined;
}

function findActiveUnmatchedOpen(unmatchedOpen: readonly BracketUnit[], position: EnforceParserPosition, parsed: ParsedEnforceSource): BracketUnit | undefined {
	const end = sourceEndPosition(parsed);
	return unmatchedOpen
		.filter(open => comparePositions(open.range.start, position) <= 0 && comparePositions(position, end) <= 0)
		.sort((left, right) => comparePositions(right.range.start, left.range.start))[0];
}

function bracketUnitAt(tokens: readonly EnforceToken[], position: EnforceParserPosition, text: BracketUnit['text']): BracketUnit | undefined {
	const token = tokens.find(candidate => candidate.text === text && candidate.line === position.line && candidate.character === position.character);
	return token ? bracketUnit(text, token, position.character) : undefined;
}

function roleForBracketPair(parsed: ParsedEnforceSource, pair: BracketPair): BracketDecorationRole {
	if (pair.open.text === '<') {
		return 'class';
	}
	if (pair.open.text === '(' && isDecoratorCallableParentheses(parsed.tokens, pair)) {
		return 'class';
	}
	if (pair.open.text === '(' && isCallableParentheses(parsed.tokens, pair)) {
		return 'function';
	}
	if (pair.open.text === '{') {
		const bodyOwner = bodyOwnerForBracePair(parsed.nodes, pair);
		if (bodyOwner) {
			return declarationBodyRole(bodyOwner);
		}
	}
	return 'variable';
}

function activeScopeRanges(parsed: ParsedEnforceSource, pair: BracketPair): EnforceParserRange[] {
	const ranges: EnforceParserRange[] = [];
	const lines = parsed.sourceText.split(/\r\n|\r|\n/);
	const character = pair.open.range.start.character;
	for (let line = pair.open.range.start.line + 1; line < pair.close.range.start.line; line++) {
		const lineText = lines[line] ?? '';
		if (lineText.length <= character || !isWhitespaceOnly(lineText.slice(0, character))) {
			continue;
		}
		ranges.push({
			start: { line, character },
			end: { line, character: character + 1 },
		});
	}
	return ranges;
}

function isWhitespaceOnly(value: string): boolean {
	return /^[ \t]*$/.test(value);
}

function shouldDrawActiveScopeGuide(parsed: ParsedEnforceSource, pair: BracketPair): boolean {
	if (pair.open.text !== '{') {
		return false;
	}
	const bodyOwner = bodyOwnerForBracePair(parsed.nodes, pair);
	return bodyOwner !== undefined;
}

function isCallableParentheses(tokens: readonly EnforceToken[], pair: BracketPair): boolean {
	const tokenIndex = tokens.indexOf(pair.open.token);
	const previous = tokenIndex >= 0 ? previousSignificantToken(tokens, tokenIndex - 1) : undefined;
	return previous !== undefined && isIdentifierLike(previous) && !controlKeywords.has(previous.text);
}

function isDecoratorCallableParentheses(tokens: readonly EnforceToken[], pair: BracketPair): boolean {
	const tokenIndex = tokens.indexOf(pair.open.token);
	const previous = tokenIndex >= 0 ? previousSignificantToken(tokens, tokenIndex - 1) : undefined;
	if (!previous || !isIdentifierLike(previous)) {
		return false;
	}
	const previousIndex = tokens.indexOf(previous);
	const openAttributeBracket = previousIndex >= 0 ? previousSignificantToken(tokens, previousIndex - 1) : undefined;
	return openAttributeBracket?.text === '[';
}

function bodyOwnerForBracePair(nodes: readonly EnforceSyntaxNode[], pair: BracketPair): EnforceSyntaxNode | undefined {
	return nodes
		.filter(node =>
			node.bodyRange
			&& samePosition(node.bodyRange.start, pair.open.range.start)
			&& samePosition({ line: node.bodyRange.end.line, character: node.bodyRange.end.character - 1 }, pair.close.range.start)
		)
		.sort((left, right) => rangeSize(left.bodyRange ?? left.range) - rangeSize(right.bodyRange ?? right.range))[0];
}

function declarationBodyRole(node: EnforceSyntaxNode): BracketDecorationRole {
	if (node.kind === 'class' || node.kind === 'enum') {
		return 'class';
	}
	if (['function', 'memberFunction', 'constructor', 'destructor'].includes(node.kind)) {
		return 'function';
	}
	return 'variable';
}

function pairRange(pair: BracketPair): EnforceParserRange {
	return { start: pair.open.range.start, end: pair.close.range.end };
}

function rangeContains(range: EnforceParserRange, position: EnforceParserPosition): boolean {
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

function sourceEndPosition(parsed: ParsedEnforceSource): EnforceParserPosition {
	const lines = parsed.sourceText.split(/\r\n|\r|\n/);
	const line = Math.max(0, lines.length - 1);
	return { line, character: lines[line]?.length ?? 0 };
}

function comparePositions(left: EnforceParserPosition, right: EnforceParserPosition): number {
	return left.line !== right.line ? left.line - right.line : left.character - right.character;
}

function samePosition(left: EnforceParserPosition, right: EnforceParserPosition): boolean {
	return left.line === right.line && left.character === right.character;
}

function previousSignificantToken(tokens: readonly EnforceToken[], startIndex: number): EnforceToken | undefined {
	for (let index = startIndex; index >= 0; index--) {
		if (!isTriviaOrIgnored(tokens[index])) {
			return tokens[index];
		}
	}
	return undefined;
}

function nextSignificantToken(tokens: readonly EnforceToken[], startIndex: number): EnforceToken | undefined {
	for (let index = startIndex; index < tokens.length; index++) {
		if (!isTriviaOrIgnored(tokens[index])) {
			return tokens[index];
		}
	}
	return undefined;
}

function isIdentifierLike(token: EnforceToken | undefined): boolean {
	return token?.kind === 'identifier' || token?.kind === 'keyword';
}

function isTriviaOrIgnored(token: EnforceToken): boolean {
	return token.kind === 'whitespace'
		|| token.kind === 'newline'
		|| token.kind === 'comment'
		|| token.kind === 'string'
		|| token.kind === 'preprocessor'
		|| token.kind === 'eof';
}
