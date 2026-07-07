import * as vscode from 'vscode';
import type { EnforceSymbolIndex } from '../index/symbolIndex';
import { buildLanguageModel } from '../model/languageModel';

const classInheritanceColonSetting = 'formatting.classInheritanceColon.enabled';
const autoBracketsSetting = 'formatting.autoBrackets.enabled';
const equalSignsSetting = 'formatting.equalSigns.enabled';
const semicolonsSetting = 'formatting.semicolons.enabled';
const commentsSetting = 'formatting.comments.enabled';

interface TypedFormattingSettings {
	autoBrackets: boolean;
	classInheritanceColon: boolean;
	comments: boolean;
	equalSigns: boolean;
	semicolons: boolean;
}

const bracketPairs: Record<string, string> = {
	'{': '}',
	'(': ')',
	'[': ']',
};

const closingBrackets = new Set(Object.values(bracketPairs));

export function handleTypedFormatting(event: vscode.TextDocumentChangeEvent, symbolIndex?: EnforceSymbolIndex): void {
	const editor = vscode.window.activeTextEditor;
	if (
		event.document.languageId !== 'enforce'
		|| event.document.uri.scheme !== 'file'
		|| editor?.document.uri.toString() !== event.document.uri.toString()
		|| event.contentChanges.length !== 1
		|| !editor.selection.isEmpty
	) {
		return;
	}

	const change = event.contentChanges[0];
	if (change.rangeLength !== 0) {
		return;
	}

	const typed = change.text;
	const typedNewline = isTypedFormattingNewline(typed);
	const typedBracket = typed.length === 1 && (Object.prototype.hasOwnProperty.call(bracketPairs, typed) || closingBrackets.has(typed));
	if (!typedNewline && !typedBracket && typed !== ' ' && typed !== '=' && typed !== '\t' && typed !== '*') {
		return;
	}

	const settings = getTypedFormattingSettings(event.document);

	if (typed === '*' && settings.comments && handleCommentBlockTypedFormatting(event.document, editor, change)) {
		return;
	}

	if (typedNewline && settings.comments && handleCommentEnterFormatting(event.document, editor, change)) {
		return;
	}

	if (typedNewline && settings.autoBrackets && handleBracketEnterFormatting(event.document, editor, change)) {
		return;
	}

	if (typedNewline && settings.semicolons && handleSemicolonEnterFormatting(event.document, editor, change)) {
		return;
	}

	if (typedBracket && settings.autoBrackets && handleBracketTypedFormatting(event.document, editor, change)) {
		return;
	}

	if (typed === ' ' && settings.autoBrackets && handleControlKeywordSpaceFormatting(event.document, editor, change)) {
		return;
	}

	if (typed === '=' && triggerSuggestAfterConditionAssertion(event.document, editor, change)) {
		return;
	}

	if (typed === '\t' && triggerSuggestAfterConditionOperandWhitespace(event.document, editor, change)) {
		return;
	}

	if (typed !== ' ') {
		return;
	}

	if (triggerSuggestAfterOverrideSpace(event.document, editor, change)) {
		return;
	}

	if (triggerSuggestAfterConditionOperandWhitespace(event.document, editor, change)) {
		return;
	}

	const cursor = change.range.start.translate(0, change.text.length);
	const linePrefix = event.document.lineAt(cursor.line).text.slice(0, cursor.character);
	const classInheritanceReplacement = settings.classInheritanceColon ? classInheritanceColonReplacement(linePrefix) : undefined;
	if (classInheritanceReplacement) {
		replaceTypedSpaceAndTriggerSuggest(editor, change.range.start, cursor, classInheritanceReplacement);
		return;
	}

	const equalSignReplacement = settings.equalSigns
		? declarationInitializerEqualSignReplacement(linePrefix) ?? assignmentTargetEqualSignReplacement(event.document, cursor, linePrefix, symbolIndex)
		: undefined;
	if (equalSignReplacement) {
		replaceTypedSpaceAndTriggerSuggest(editor, change.range.start, cursor, equalSignReplacement);
	}
}

function getTypedFormattingSettings(document: vscode.TextDocument): TypedFormattingSettings {
	const config = vscode.workspace.getConfiguration('reforgerScriptTools', document.uri);
	return {
		autoBrackets: config.get<boolean>(autoBracketsSetting, true),
		classInheritanceColon: config.get<boolean>(classInheritanceColonSetting, true),
		comments: config.get<boolean>(commentsSetting, true),
		equalSigns: config.get<boolean>(equalSignsSetting, true),
		semicolons: config.get<boolean>(semicolonsSetting, true),
	};
}

export function bracketTypedEdit(lineText: string, cursorCharacter: number, typed: string): { replacement: string; cursorOffset: number } | undefined {
	const open = Object.prototype.hasOwnProperty.call(bracketPairs, typed) ? typed : undefined;
	if (open) {
		const linePrefixBeforeTyped = lineText.slice(0, Math.max(0, cursorCharacter - typed.length));
		const lineSuffix = lineText.slice(cursorCharacter);
		if (!canAutoPairBracket(linePrefixBeforeTyped, lineSuffix)) {
			return undefined;
		}
		return { replacement: `${open}${bracketPairs[open]}`, cursorOffset: 1 };
	}

	if (closingBrackets.has(typed) && lineText[cursorCharacter] === typed) {
		const linePrefixBeforeTyped = lineText.slice(0, Math.max(0, cursorCharacter - typed.length));
		if (!canOvertypeClosingBracket(linePrefixBeforeTyped)) {
			return undefined;
		}
		return { replacement: '', cursorOffset: 1 };
	}

	return undefined;
}

export function controlKeywordSpaceEdit(linePrefix: string): { replacement: string; cursorOffset: number } | undefined {
	if (isIgnoredLinePrefix(linePrefix)) {
		return undefined;
	}
	return /^\s*(?:if|while|for|foreach|switch) $/.test(linePrefix)
		? { replacement: ' ()', cursorOffset: 2 }
		: undefined;
}

export function bracketEnterEdit(lineText: string, cursorCharacter: number, nextLineText: string, indentUnit = '\t', followingLineTexts: readonly string[] = []): { replacement: string; cursorLineOffset: number; cursorCharacter: number } | undefined {
	const linePrefix = lineText.slice(0, cursorCharacter);
	const lineSuffix = lineText.slice(cursorCharacter);
	if (lineSuffix.trim().length > 0) {
		return undefined;
	}
	if (isIgnoredLinePrefix(linePrefix)) {
		return undefined;
	}

	const indent = leadingWhitespace(lineText);
	const innerIndent = `${indent}${indentUnit}`;
	if (linePrefix.trimEnd().endsWith('{') && nextLineText.trimStart().startsWith('}')) {
		return {
			replacement: `\n${innerIndent}\n${indent}`,
			cursorLineOffset: 1,
			cursorCharacter: innerIndent.length,
		};
	}

	const blockHeaderPrefix = linePrefix;
	if (isCompleteControlHeaderLine(blockHeaderPrefix)) {
		return {
			replacement: `\n${innerIndent}`,
			cursorLineOffset: 1,
			cursorCharacter: innerIndent.length,
		};
	}
	if (!isBlockHeaderLine(blockHeaderPrefix)) {
		return undefined;
	}
	const existingEmptyBlock = existingEmptyBracketBlock(followingLineTexts);
	if (existingEmptyBlock) {
		return {
			replacement: '',
			cursorLineOffset: existingEmptyBlock.cursorLineOffset,
			cursorCharacter: existingEmptyBlock.cursorCharacter,
		};
	}
	if (nextLineText.trim().length > 0) {
		return undefined;
	}

	return {
		replacement: `\n${indent}{\n${innerIndent}\n${indent}}`,
		cursorLineOffset: 2,
		cursorCharacter: innerIndent.length,
	};
}

export function commentBlockEnterEdit(lineText: string, cursorCharacter: number, nextLineText: string): { replacement: string; cursorLineOffset: number; cursorCharacter: number } | undefined {
	const linePrefix = lineText.slice(0, cursorCharacter);
	const lineSuffix = lineText.slice(cursorCharacter);
	if (lineSuffix.trim().length > 0 || nextLineText.trim().length > 0) {
		return undefined;
	}
	const indent = leadingWhitespace(lineText);
	if (!/^\/\*(?:!|\*)?$/.test(linePrefix.slice(indent.length))) {
		return undefined;
	}
	return {
		replacement: `\n${indent}\n${indent}*/`,
		cursorLineOffset: 1,
		cursorCharacter: indent.length,
	};
}

export function commentBlockTypedEdit(lineText: string, cursorCharacter: number): { insertion: string; cursorLineOffset: number; cursorCharacter: number } | undefined {
	const linePrefix = lineText.slice(0, cursorCharacter);
	const lineSuffix = lineText.slice(cursorCharacter);
	if (lineSuffix.trim().length > 0) {
		return undefined;
	}
	const indent = leadingWhitespace(lineText);
	if (linePrefix.slice(indent.length) !== '/*') {
		return undefined;
	}
	return {
		insertion: `\n${indent}\n${indent}*/`,
		cursorLineOffset: 1,
		cursorCharacter: indent.length,
	};
}

export function classInheritanceColonReplacement(linePrefix: string): string | undefined {
	if (isLineComment(linePrefix) || isOpenString(linePrefix)) {
		return undefined;
	}
	const match = /^(\s*(?:sealed\s+)?class\s+[A-Za-z_][A-Za-z0-9_]*) $/.exec(linePrefix);
	return match ? ' : ' : undefined;
}

export function declarationInitializerEqualSignReplacement(linePrefix: string): string | undefined {
	if (isIgnoredLinePrefix(linePrefix)) {
		return undefined;
	}
	const trimmed = linePrefix.trim();
	if (!trimmed || /[=;{}(),]$/.test(trimmed)) {
		return undefined;
	}
	const declarationBody = stripDeclarationModifiers(trimmed);
	const declarationMatch = /^(?:[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>{};=()]+>)?(?:\s*\[\])?)\s+[A-Za-z_][A-Za-z0-9_]*$/.exec(declarationBody);
	if (!declarationMatch) {
		return undefined;
	}
	const firstWord = /^[A-Za-z_][A-Za-z0-9_]*/.exec(trimmed)?.[0] ?? '';
	if (['class', 'enum', 'modded', 'sealed', 'if', 'while', 'for', 'foreach', 'switch', 'return', 'case', 'else', 'override'].includes(firstWord)) {
		return undefined;
	}
	return ' = ';
}

export function assignmentTargetEqualSignReplacement(document: vscode.TextDocument, position: vscode.Position, linePrefix: string, symbolIndex: EnforceSymbolIndex | undefined): string | undefined {
	const name = standaloneAssignmentTargetName(linePrefix);
	if (!name || !symbolIndex || isIgnoredLinePrefix(linePrefix)) {
		return undefined;
	}
	const model = buildLanguageModel(document, symbolIndex);
	if (model.visibleLocals(position).some(local => local.name === name)) {
		return ' = ';
	}
	if (model.members('this', position).some(member => member.type === 'property' && member.name === name)) {
		return ' = ';
	}
	return undefined;
}

function standaloneAssignmentTargetName(linePrefix: string): string | undefined {
	const match = /^\s*([A-Za-z_][A-Za-z0-9_]*) $/.exec(linePrefix);
	if (!match) {
		return undefined;
	}
	const name = match[1];
	return ['class', 'enum', 'modded', 'sealed', 'if', 'while', 'for', 'foreach', 'switch', 'return', 'case', 'else', 'override', 'new', 'null', 'true', 'false', 'this', 'super'].includes(name)
		? undefined
		: name;
}

export function semicolonEnterEdit(lineText: string, cursorCharacter: number, nextLineText: string, outdentAfterNewline = ''): { replacementPrefix: string; replacementSuffix?: string } | undefined {
	const linePrefix = lineText.slice(0, cursorCharacter);
	const lineSuffix = lineText.slice(cursorCharacter);
	if (lineSuffix.trim().length > 0 || nextLineText.trim().length > 0 || isIgnoredLinePrefix(linePrefix)) {
		return undefined;
	}
	const trimmed = linePrefix.trimEnd();
	const statementText = trimmed.trim();
	if (!statementText || /[;:,]$/.test(statementText) || (/[{}]$/.test(statementText) && !isStatementTerminatorCandidate(statementText)) || isBlockHeaderLine(trimmed) || isNonTerminatedStatementHeader(trimmed)) {
		return undefined;
	}
	if (isStatementTerminatorCandidate(statementText)) {
		return outdentAfterNewline
			? { replacementPrefix: ';', replacementSuffix: outdentAfterNewline }
			: { replacementPrefix: ';' };
	}
	return undefined;
}

function stripDeclarationModifiers(value: string): string {
	let rest = value;
	while (/^(?:private|protected|public|static|const|owned|ref|autoptr|notnull)\s+/.test(rest)) {
		rest = rest.replace(/^(?:private|protected|public|static|const|owned|ref|autoptr|notnull)\s+/, '');
	}
	return rest;
}

export function shouldTriggerSuggestAfterOverrideSpace(linePrefix: string): boolean {
	return !isIgnoredLinePrefix(linePrefix) && /^\s*override $/.test(linePrefix);
}

export function shouldTriggerSuggestAfterConditionOperandSpace(linePrefix: string): boolean {
	if (isIgnoredLinePrefix(linePrefix) || !/[ \t]$/.test(linePrefix)) {
		return false;
	}
	return isConditionAssertionValueContext(linePrefix) || isConditionOperandOperatorContext(linePrefix);
}

export function conditionAssertionSpaceReplacement(linePrefix: string): string | undefined {
	if (isIgnoredLinePrefix(linePrefix) || !isConditionAssertionValueContext(linePrefix)) {
		return undefined;
	}
	return linePrefix.endsWith(' ') ? undefined : ' ';
}

function triggerSuggestAfterOverrideSpace(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const linePrefix = document.lineAt(cursor.line).text.slice(0, cursor.character);
	if (!shouldTriggerSuggestAfterOverrideSpace(linePrefix)) {
		return false;
	}
	void vscode.commands.executeCommand('editor.action.triggerSuggest');
	return true;
}

function triggerSuggestAfterConditionOperandWhitespace(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const linePrefix = document.lineAt(cursor.line).text.slice(0, cursor.character);
	if (!shouldTriggerSuggestAfterConditionOperandSpace(linePrefix)) {
		return false;
	}
	void vscode.commands.executeCommand('editor.action.triggerSuggest');
	return true;
}

function triggerSuggestAfterConditionAssertion(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const linePrefix = document.lineAt(cursor.line).text.slice(0, cursor.character);
	const replacement = conditionAssertionSpaceReplacement(linePrefix);
	if (replacement === undefined) {
		return false;
	}
	const insertPosition = cursor;
	void editor.edit(builder => builder.insert(insertPosition, replacement), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const completionPosition = insertPosition.translate(0, replacement.length);
			editor.selection = new vscode.Selection(completionPosition, completionPosition);
			void vscode.commands.executeCommand('editor.action.triggerSuggest');
		}
	});
	return true;
}

function replaceTypedSpaceAndTriggerSuggest(editor: vscode.TextEditor, start: vscode.Position, cursor: vscode.Position, replacement: string): void {
	const typedSpace = new vscode.Range(start, cursor);
	void editor.edit(builder => builder.replace(typedSpace, replacement), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const completionPosition = typedSpace.start.translate(0, replacement.length);
			editor.selection = new vscode.Selection(completionPosition, completionPosition);
			void vscode.commands.executeCommand('editor.action.triggerSuggest');
		}
	});
}

function handleControlKeywordSpaceFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const linePrefix = document.lineAt(cursor.line).text.slice(0, cursor.character);
	const edit = controlKeywordSpaceEdit(linePrefix);
	if (!edit) {
		return false;
	}

	const typedRange = new vscode.Range(change.range.start, cursor);
	void editor.edit(builder => builder.replace(typedRange, edit.replacement), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = change.range.start.translate(0, edit.cursorOffset);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

function handleCommentBlockTypedFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const lineText = document.lineAt(cursor.line).text;
	const edit = commentBlockTypedEdit(lineText, cursor.character);
	if (!edit) {
		return false;
	}

	void editor.edit(builder => builder.insert(cursor, normalizeNewlines(edit.insertion, document.eol)), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = new vscode.Position(change.range.start.line + edit.cursorLineOffset, edit.cursorCharacter);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

function handleCommentEnterFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const nextLine = change.range.start.line + 1;
	if (nextLine >= document.lineCount) {
		return false;
	}

	const lineText = document.lineAt(change.range.start.line).text;
	const nextLineText = document.lineAt(nextLine).text;
	const edit = commentBlockEnterEdit(lineText, change.range.start.character, nextLineText);
	if (!edit) {
		return false;
	}

	const typedRange = new vscode.Range(change.range.start, insertedTextEnd(change.range.start, change.text));
	void editor.edit(builder => builder.replace(typedRange, normalizeNewlines(edit.replacement, document.eol)), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = new vscode.Position(change.range.start.line + edit.cursorLineOffset, edit.cursorCharacter);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

function handleBracketEnterFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const nextLine = change.range.start.line + 1;
	if (nextLine >= document.lineCount) {
		return false;
	}

	const indentUnit = editorIndentUnit(editor);
	const lineText = document.lineAt(change.range.start.line).text;
	const nextLineText = document.lineAt(nextLine).text;
	const followingLineTexts = [
		nextLine + 1 < document.lineCount ? document.lineAt(nextLine + 1).text : '',
		nextLine + 2 < document.lineCount ? document.lineAt(nextLine + 2).text : '',
		nextLine + 3 < document.lineCount ? document.lineAt(nextLine + 3).text : '',
	];
	const edit = bracketEnterEdit(lineText, change.range.start.character, nextLineText, indentUnit, followingLineTexts);
	if (!edit) {
		return false;
	}

	const typedRange = new vscode.Range(change.range.start, insertedTextEnd(change.range.start, change.text));
	void editor.edit(builder => builder.replace(typedRange, normalizeNewlines(edit.replacement, document.eol)), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = new vscode.Position(change.range.start.line + edit.cursorLineOffset, edit.cursorCharacter);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

function handleSemicolonEnterFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const nextLine = change.range.start.line + 1;
	if (nextLine >= document.lineCount) {
		return false;
	}

	const lineText = document.lineAt(change.range.start.line).text;
	const nextLineText = document.lineAt(nextLine).text;
	const edit = semicolonEnterEdit(lineText, change.range.start.character, nextLineText);
	if (!edit) {
		return false;
	}
	const newLineIndent = unbracedControlBodyOutdent(document, change.range.start.line) ?? leadingWhitespace(lineText);

	const typedRange = new vscode.Range(change.range.start, insertedTextEnd(change.range.start, change.text));
	const replacement = `${edit.replacementPrefix}${newlineOnly(change.text)}${newLineIndent}`;
	void editor.edit(builder => builder.replace(typedRange, replacement), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = insertedTextEnd(change.range.start.translate(0, edit.replacementPrefix.length), `${newlineOnly(change.text)}${newLineIndent}`);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

function handleBracketTypedFormatting(document: vscode.TextDocument, editor: vscode.TextEditor, change: vscode.TextDocumentContentChangeEvent): boolean {
	const cursor = change.range.start.translate(0, change.text.length);
	const lineText = document.lineAt(cursor.line).text;
	const edit = bracketTypedEdit(lineText, cursor.character, change.text);
	if (!edit) {
		return false;
	}

	const typedRange = new vscode.Range(change.range.start, cursor);
	void editor.edit(builder => builder.replace(typedRange, edit.replacement), { undoStopBefore: false, undoStopAfter: false }).then(applied => {
		if (applied) {
			const selectionPosition = change.range.start.translate(0, edit.cursorOffset);
			editor.selection = new vscode.Selection(selectionPosition, selectionPosition);
		}
	});
	return true;
}

export function isTypedFormattingNewline(text: string): boolean {
	return /^\r?\n/.test(text);
}

function insertedTextEnd(start: vscode.Position, text: string): vscode.Position {
	const normalized = text.replace(/\r\n/g, '\n');
	const parts = normalized.split('\n');
	return parts.length === 1
		? start.translate(0, text.length)
		: new vscode.Position(start.line + parts.length - 1, parts[parts.length - 1].length);
}

function normalizeNewlines(text: string, eol: vscode.EndOfLine): string {
	return eol === vscode.EndOfLine.CRLF ? text.replace(/\n/g, '\r\n') : text;
}

function newlineOnly(text: string): string {
	const match = /^\r?\n/.exec(text);
	return match?.[0] ?? text;
}

function editorIndentUnit(editor: vscode.TextEditor): string {
	const insertSpaces = editor.options.insertSpaces === true || editor.options.insertSpaces === 'true';
	const tabSize = typeof editor.options.tabSize === 'number' ? editor.options.tabSize : 4;
	return insertSpaces ? ' '.repeat(tabSize) : '\t';
}

function leadingWhitespace(value: string): string {
	return /^\s*/.exec(value)?.[0] ?? '';
}

function isBlockHeaderLine(linePrefix: string): boolean {
	const trimmed = linePrefix.trim();
	if (!trimmed || /[;{}]$/.test(trimmed)) {
		return false;
	}
	if (/^(?:sealed\s+)?class\s+[A-Za-z_][A-Za-z0-9_]*(?:\s*(?::|extends)\s*[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>{};]+>)?)?$/.test(trimmed)) {
		return true;
	}
	if (/^(?:if|while|for|foreach|switch)\s*\(.+\)$/.test(trimmed)) {
		return true;
	}
	return /^(?:(?:private|protected|public|static|const|override|event|proto|native|external|owned|ref|autoptr|notnull|sealed)\s+)*(?:[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>{};]+>)?|\w+\[\])\s+[~A-Za-z_][A-Za-z0-9_]*\s*\([^;{}]*\)$/.test(trimmed);
}

function isCompleteControlHeaderLine(linePrefix: string): boolean {
	return /^(?:if|while|for|foreach|switch)\s*\(.+\)$/.test(linePrefix.trim());
}

function isNonTerminatedStatementHeader(trimmed: string): boolean {
	return /^(?:if|while|for|foreach|switch)\s*\(.*\)$/.test(trimmed)
		|| /^(?:else|do|try|catch|finally)\b/.test(trimmed)
		|| /^\[[^\]]*\]$/.test(trimmed);
}

function isStatementTerminatorCandidate(trimmed: string): boolean {
	if (/^(?:return|break|continue)(?:\s+.*)?$/.test(trimmed)) {
		return true;
	}
	return false
		|| /\b(?:new\s+)?[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>{};]+>)?(?:\s*\[\])?\s+[A-Za-z_][A-Za-z0-9_]*\s*(?:=.*)?$/.test(stripDeclarationModifiers(trimmed))
		|| /(?:^|[^=!<>])=(?!=).+$/.test(trimmed)
		|| /\w\s*\([^;{}]*\)$/.test(trimmed)
		|| /(?:\+\+|--)$/.test(trimmed);
}

function unbracedControlBodyOutdent(document: vscode.TextDocument, line: number): string | undefined {
	if (line <= 0) {
		return undefined;
	}
	const lineText = document.lineAt(line).text;
	const previousLineText = document.lineAt(line - 1).text;
	const leading = leadingWhitespace(lineText);
	const previousLeading = leadingWhitespace(previousLineText);
	if (!isCompleteControlHeaderLine(previousLineText) || !leading.startsWith(previousLeading) || leading.length <= previousLeading.length) {
		return undefined;
	}
	if (leading.startsWith(`${previousLeading}\t`)) {
		return previousLeading;
	}
	if (leading.startsWith(`${previousLeading}    `)) {
		return previousLeading;
	}
	return previousLeading;
}

function existingEmptyBracketBlock(lines: readonly string[]): { cursorLineOffset: number; cursorCharacter: number } | undefined {
	for (let index = 0; index + 2 < lines.length; index++) {
		const open = lines[index].trim();
		if (!bracketPairs[open]) {
			continue;
		}
		const close = lines[index + 2].trim();
		if (close !== bracketPairs[open] || lines[index + 1].trim().length > 0) {
			continue;
		}
		return {
			cursorLineOffset: index + 3,
			cursorCharacter: lines[index + 1].length,
		};
	}
	return undefined;
}

function canAutoPairBracket(linePrefix: string, lineSuffix: string): boolean {
	if (isIgnoredLinePrefix(linePrefix)) {
		return false;
	}
	if (lineSuffix && !/^[\s\]\)},;:]/.test(lineSuffix)) {
		return false;
	}
	return true;
}

function canOvertypeClosingBracket(linePrefix: string): boolean {
	return !isIgnoredLinePrefix(linePrefix);
}

function isIgnoredLinePrefix(linePrefix: string): boolean {
	return isPreprocessorLine(linePrefix) || isLineComment(linePrefix) || isOpenString(linePrefix) || isOpenBlockComment(linePrefix);
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

function isPreprocessorLine(linePrefix: string): boolean {
	return /^\s*#/.test(linePrefix);
}

function isLineComment(linePrefix: string): boolean {
	let inString: string | undefined;
	for (let index = 0; index < linePrefix.length - 1; index++) {
		const char = linePrefix[index];
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
		if (char === '/' && linePrefix[index + 1] === '/') {
			return true;
		}
	}
	return false;
}

function isOpenString(linePrefix: string): boolean {
	let inString: string | undefined;
	for (let index = 0; index < linePrefix.length; index++) {
		const char = linePrefix[index];
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
		}
	}
	return inString !== undefined;
}

function isOpenBlockComment(linePrefix: string): boolean {
	let inString: string | undefined;
	let blockDepth = 0;
	for (let index = 0; index < linePrefix.length - 1; index++) {
		const char = linePrefix[index];
		const next = linePrefix[index + 1];
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
		if (char === '/' && next === '*') {
			blockDepth++;
			index++;
			continue;
		}
		if (char === '*' && next === '/' && blockDepth > 0) {
			blockDepth--;
			index++;
		}
	}
	return blockDepth > 0;
}
