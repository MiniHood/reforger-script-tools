import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientCommands } from '../extensionConfig/languageClient';
import { isCurrentSingleSemicolonCaret, semicolonAfterEnterPosition } from '../languageClient/languageClient';

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

	test('keeps the Rust snippet bridge command aligned with the extension contract', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const completionSource = await fs.readFile(
			path.join(extension.extensionPath, 'server', 'src', 'lsp', 'completion.rs'),
			'utf8',
		);
		assert.ok(completionSource.includes(languageClientCommands.triggerSuggestAtSnippetPlaceholder));
		assert.ok(completionSource.includes('RPL_RPC_ENUM_PLACEHOLDER_DEFAULTS'));
		const clientSource = await fs.readFile(
			path.join(extension.extensionPath, 'src', 'languageClient', 'languageClient.ts'),
			'utf8',
		);
		assert.ok(clientSource.includes('expectedSelectionTexts'));
		assert.ok(clientSource.includes('advanceSnippetSuggestTransaction'));
		assert.ok(clientSource.includes('wrapBridgeCompletionCommands'));
		assert.ok(clientSource.includes('jumpToNextSnippetPlaceholder'));
		assert.ok(clientSource.includes('registerEmptyCompletionRefresh'));
		assert.ok(clientSource.includes('isRefreshableEmptyCompletion'));
		assert.ok(clientSource.includes('completionLifecycleTraceForDocument'));
		assert.ok(clientSource.includes('snippetSuggestTraceVersion'));
		assert.ok(clientSource.includes('registerSemicolonAfterEnter'));
		assert.ok(clientSource.includes('isSinglePlainEnter'));
		assert.ok(clientSource.includes('semicolonAfterEnterPosition'));
		assert.ok(clientSource.includes('onDidChangeTextEditorSelection'));
		assert.ok(!clientSource.includes('registerOnTypeFormattingEditProvider'));
	});

	test('contributes a context-safe native multi-line comment pair', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const configuration = JSON.parse(await fs.readFile(
			path.join(extension.extensionPath, 'language-configuration.json'),
			'utf8',
		)) as { autoClosingPairs: Array<{ open: string; close: string; notIn?: string[] }> };
		assert.deepStrictEqual(
			configuration.autoClosingPairs.find(pair => pair.open === '/*'),
			{ open: '/*', close: '*/', notIn: ['string', 'comment'] },
		);
	});

	test('derives the Rust request position from the accepted Enter edit', () => {
		const position = semicolonAfterEnterPosition([{
			range: new vscode.Range(new vscode.Position(20, 61), new vscode.Position(20, 61)),
			rangeLength: 0,
			text: '\n\t',
		} as vscode.TextDocumentContentChangeEvent]);
		assert.deepStrictEqual(position, new vscode.Position(21, 1));

		const crlfPosition = semicolonAfterEnterPosition([{
			range: new vscode.Range(new vscode.Position(7, 12), new vscode.Position(7, 12)),
			rangeLength: 0,
			text: '\r\n',
		} as vscode.TextDocumentContentChangeEvent]);
		assert.deepStrictEqual(crlfPosition, new vscode.Position(8, 0));

		assert.strictEqual(semicolonAfterEnterPosition([{
			range: new vscode.Range(0, 0, 0, 1),
			rangeLength: 1,
			text: '\n',
		} as vscode.TextDocumentContentChangeEvent]), undefined);

		const plainEnter = {
			range: new vscode.Range(0, 0, 0, 0),
			rangeLength: 0,
			text: '\n',
		} as vscode.TextDocumentContentChangeEvent;
		assert.strictEqual(semicolonAfterEnterPosition([plainEnter, plainEnter]), undefined);
		assert.strictEqual(semicolonAfterEnterPosition([{
			...plainEnter,
			text: '\ntext',
		}]), undefined);
	});

	test('applies a semicolon response only at the original single caret and revision', () => {
		const position = new vscode.Position(8, 4);
		assert.strictEqual(isCurrentSingleSemicolonCaret(12, 12, 1, true, position, position), true);
		assert.strictEqual(isCurrentSingleSemicolonCaret(13, 12, 1, true, position, position), false);
		assert.strictEqual(isCurrentSingleSemicolonCaret(12, 12, 2, true, position, position), false);
		assert.strictEqual(isCurrentSingleSemicolonCaret(12, 12, 1, false, position, position), false);
		assert.strictEqual(
			isCurrentSingleSemicolonCaret(12, 12, 1, true, new vscode.Position(8, 5), position),
			false,
		);
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
	});

});
