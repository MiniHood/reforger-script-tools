import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientCommands } from '../extensionConfig/languageClient';

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
		assert.ok(clientSource.includes('snippetSuggestTraceVersion'));
	});

	test('enables local diagnostic logging by default', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<string, { default?: unknown }>;
		assert.strictEqual(properties['reforgerScriptTools.diagnostics.enabled'].default, true);
	});
});
