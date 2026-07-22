import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientCommands } from '../extensionConfig/languageClient';
import {
	blockCommentPairPosition,
	enterAfterPosition,
	ifSpaceCommitPositionFromCommandArguments,
	isCurrentSingleTypingAssistCaret,
	tabAfterPosition,
} from '../languageClient/languageClient';
import { positionFromByteOffset } from '../languageClient/symbolLocationBridge';
import { registerEnterTypingAssist } from '../languageClient/typingAssistTransactionBridge';
import { registerBlockCommentPair } from '../languageClient/typingAssistTransactionBridge';
import { VersionedEditorTransaction } from '../languageClient/versionedEditorTransaction';

suite('extension activation', () => {
	test('registers editor-facing commands', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		await extension.activate();

		const commands = await vscode.commands.getCommands(true);
		assert.ok(commands.includes(languageClientCommands.debugHoverAtCursor));
		assert.ok(commands.includes(languageClientCommands.debugCompletionAtCursor));
		assert.ok(commands.includes(languageClientCommands.triggerSuggestAtSnippetPlaceholder));
		assert.ok(commands.includes(languageClientCommands.advanceSnippetPlaceholderAfterAccept));
		const contributedCommands = extension.packageJSON.contributes.commands as Array<{ command: string }>;
		assert.ok(contributedCommands.some(command =>
			command.command === languageClientCommands.triggerSuggestAtSnippetPlaceholder));
	});

	test('accepts only Rust-authored if completion coordinates', () => {
		assert.deepStrictEqual(
			ifSpaceCommitPositionFromCommandArguments(['3', '9']),
			new vscode.Position(3, 9),
		);
		assert.strictEqual(ifSpaceCommitPositionFromCommandArguments([3, 9]), undefined);
		assert.strictEqual(ifSpaceCommitPositionFromCommandArguments(['3', '-1']), undefined);
	});

	test('maps Rust byte offsets to VS Code UTF-16 positions for symbol navigation', () => {
		assert.deepStrictEqual(positionFromByteOffset('class \u{1F600}\nRun', 10), new vscode.Position(0, 8));
		assert.deepStrictEqual(positionFromByteOffset('class \u{1F600}\nRun', 11), new vscode.Position(1, 0));
	});

	test('contributes native pairs and narrow if-family indentation', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const configuration = JSON.parse(await fs.readFile(
			path.join(extension.extensionPath, 'language-configuration.json'),
			'utf8',
		)) as {
			autoClosingPairs: Array<{ open: string; close: string; notIn?: string[] }>;
			onEnterRules?: Array<{
				beforeText?: string;
				previousLineText?: string;
				action?: { indent?: string };
			}>;
		};
		assert.deepStrictEqual(
			configuration.autoClosingPairs.find(pair => pair.open === '/*'),
			{ open: '/*', close: '*/', notIn: ['string', 'comment'] },
		);
		const onEnterHeader = configuration.onEnterRules?.[0];
		assert.strictEqual(onEnterHeader?.action?.indent, 'indent');
		assert.ok(onEnterHeader?.beforeText);
		const indentHeader = new RegExp(onEnterHeader.beforeText);
		for (const header of ['if (enabled)', 'else if (enabled)', 'else']) {
			assert.ok(indentHeader.test(header), header);
		}
		for (const ineligible of [
			'if (enabled) {',
			'if (enabled);',
			'if (enabled) Run();',
			'if (enabled) // comment',
			'if (enabled',
			'// if (enabled)',
		]) {
			assert.ok(!indentHeader.test(ineligible), ineligible);
		}
		const outdentAfterBody = configuration.onEnterRules?.[1];
		assert.strictEqual(outdentAfterBody?.action?.indent, 'outdent');
		assert.strictEqual(outdentAfterBody?.previousLineText, onEnterHeader.beforeText);
		assert.ok(outdentAfterBody?.beforeText);
		const bodyLine = new RegExp(outdentAfterBody.beforeText);
		assert.ok(bodyLine.test('\tRun();'));
		assert.ok(!bodyLine.test('\t// comment'));
		assert.ok(!bodyLine.test('\t/* comment'));
	});

	test('uses the native block-comment pair event as a narrow typing-assist trigger', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: '' });
		await vscode.window.showTextDocument(document);
		const observed: string[][] = [];
		const listener = vscode.workspace.onDidChangeTextDocument(event => {
			if (event.document.uri.toString() === document.uri.toString()) {
				observed.push(event.contentChanges.map(change => change.text));
			}
		});
		try {
			await vscode.commands.executeCommand('type', { text: '/' });
			await vscode.commands.executeCommand('type', { text: '*' });
			assert.deepStrictEqual(observed, [['/'], ['**/']]);
		} finally {
			listener.dispose();
		}
	});

	test('derives the Rust pair request position only from the native pair event', () => {
		const pair = {
			range: new vscode.Range(new vscode.Position(4, 7), new vscode.Position(4, 7)),
			rangeLength: 0,
			text: '**/',
		} as vscode.TextDocumentContentChangeEvent;
		assert.deepStrictEqual(blockCommentPairPosition([pair]), new vscode.Position(4, 8));
		assert.strictEqual(blockCommentPairPosition([{ ...pair, text: '*' }]), undefined);
		assert.strictEqual(blockCommentPairPosition([{ ...pair, rangeLength: 1 }]), undefined);
	});

	test('derives the Rust request position from the accepted Enter edit', () => {
		const position = enterAfterPosition([{
			range: new vscode.Range(new vscode.Position(20, 61), new vscode.Position(20, 61)),
			rangeLength: 0,
			text: '\n\t',
		} as vscode.TextDocumentContentChangeEvent]);
		assert.deepStrictEqual(position, new vscode.Position(21, 1));

		const crlfPosition = enterAfterPosition([{
			range: new vscode.Range(new vscode.Position(7, 12), new vscode.Position(7, 12)),
			rangeLength: 0,
			text: '\r\n',
		} as vscode.TextDocumentContentChangeEvent]);
		assert.deepStrictEqual(crlfPosition, new vscode.Position(8, 0));

		assert.strictEqual(enterAfterPosition([{
			range: new vscode.Range(0, 0, 0, 1),
			rangeLength: 1,
			text: '\n',
		} as vscode.TextDocumentContentChangeEvent]), undefined);

		const plainEnter = {
			range: new vscode.Range(0, 0, 0, 0),
			rangeLength: 0,
			text: '\n',
		} as vscode.TextDocumentContentChangeEvent;
		assert.strictEqual(enterAfterPosition([plainEnter, plainEnter]), undefined);
		assert.strictEqual(enterAfterPosition([{
			...plainEnter,
			text: '\ntext',
		}]), undefined);
	});

	test('applies an Enter assist response only at the original single caret and revision', () => {
		const position = new vscode.Position(8, 4);
		assert.strictEqual(isCurrentSingleTypingAssistCaret(12, 12, 1, true, position, position), true);
		assert.strictEqual(isCurrentSingleTypingAssistCaret(13, 12, 1, true, position, position), false);
		assert.strictEqual(isCurrentSingleTypingAssistCaret(12, 12, 2, true, position, position), false);
		assert.strictEqual(isCurrentSingleTypingAssistCaret(12, 12, 1, false, position, position), false);
		assert.strictEqual(
			isCurrentSingleTypingAssistCaret(12, 12, 1, true, new vscode.Position(8, 5), position),
			false,
		);
	});

	test('applies one current versioned assist response and preserves its selection', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'value' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 5);
		editor.selection = new vscode.Selection(position, position);
		const transaction = new VersionedEditorTransaction(document, document.version, position, position);
		assert.strictEqual(transaction.accept({
			edits: [{ range: { start: { line: 0, character: 5 }, end: { line: 0, character: 5 } }, newText: ';' }],
			selection: { line: 0, character: 6 },
		}), true);
		assert.strictEqual(await transaction.apply(), 'applied');
		assert.strictEqual(document.getText(), 'value;');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(0, 6));
		assert.strictEqual(await transaction.apply(), 'pending');
	});

	test('rejects empty and stale versioned assist responses', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'value' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 5);
		editor.selection = new vscode.Selection(position, position);
		const transaction = new VersionedEditorTransaction(document, document.version, position, position);
		assert.strictEqual(transaction.accept({ edits: [] }), false);
		assert.strictEqual(transaction.accept({
			edits: [{ range: { start: { line: 0, character: 5 }, end: { line: 0, character: 5 } }, newText: ';' }],
		}), true);
		editor.selections = [
			new vscode.Selection(position, position),
			new vscode.Selection(position, position),
		];
		assert.strictEqual(await transaction.apply(), 'stale');
		assert.strictEqual(document.getText(), 'value');
	});

	test('applies a Rust-authored Tab assist through the editor bridge', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'value' });
		const editor = await vscode.window.showTextDocument(document);
		editor.selection = new vscode.Selection(new vscode.Position(0, 5), new vscode.Position(0, 5));
		const requests: unknown[] = [];
		const disposable = registerEnterTypingAssist(() => ({
			sendRequest: async (_method: string, params: unknown) => {
				requests.push(params);
				return {
					edits: [{ range: { start: { line: 0, character: 6 }, end: { line: 0, character: 6 } }, newText: ';' }],
				};
			},
		} as never));
		try {
			await vscode.commands.executeCommand('type', { text: '\t' });
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.strictEqual(document.getText(), 'value\t;');
			assert.strictEqual(requests.length, 1);
		} finally {
			disposable.dispose();
		}
	});

	test('applies a Rust-authored block-comment assist through the native pair event', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: '' });
		await vscode.window.showTextDocument(document);
		const requests: unknown[] = [];
		const disposable = registerBlockCommentPair(() => ({
			sendRequest: async (_method: string, params: unknown) => {
				requests.push(params);
				return {
					edits: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 4 } }, newText: '/** */' }],
				};
			},
		} as never));
		try {
			await vscode.commands.executeCommand('type', { text: '/' });
			await vscode.commands.executeCommand('type', { text: '*' });
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.strictEqual(document.getText(), '/** */');
			assert.strictEqual(requests.length, 1);
		} finally {
			disposable.dispose();
		}
	});

	test('admits only literal Tab indentation edits for the Rust typing assist', () => {
		const tab = {
			range: new vscode.Range(new vscode.Position(8, 4), new vscode.Position(8, 4)),
			rangeLength: 0,
			text: '\t',
		} as vscode.TextDocumentContentChangeEvent;
		assert.deepStrictEqual(tabAfterPosition([tab]), new vscode.Position(8, 5));
		assert.strictEqual(tabAfterPosition([{ ...tab, text: '    ' }]), undefined);
		assert.strictEqual(tabAfterPosition([{ ...tab, text: '  ' }]), undefined);
		assert.strictEqual(tabAfterPosition([{ ...tab, rangeLength: 1 }]), undefined);
	});

	test('enables local diagnostic logging by default', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<string, { default?: unknown }>;
		assert.strictEqual(properties['reforgerScriptTools.diagnostics.enabled'].default, true);
	});

	test('keeps Enter available for line breaks in Enforce suggestions', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const defaults = extension.packageJSON.contributes.configurationDefaults as Record<string, Record<string, unknown>>;
		assert.strictEqual(defaults['[enforce]']['editor.acceptSuggestionOnEnter'], 'off');
		assert.strictEqual(defaults['[enforce]']['editor.autoIndent'], 'full');
	});

	test('resolves full auto indentation for an Enforce editor', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: '' });
		assert.strictEqual(vscode.workspace.getConfiguration('editor', document.uri).get('autoIndent'), 'full');
	});

});
