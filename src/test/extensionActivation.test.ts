import * as assert from 'node:assert';
import * as vscode from 'vscode';

suite('extension activation', () => {
	test('registers editor-facing commands', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		await extension.activate();

		const commands = await vscode.commands.getCommands(true);
		assert.ok(commands.includes('reforger-sript-tools.debug.hoverAtCursor'));
		assert.ok(commands.includes('reforger-sript-tools.debug.completionAtCursor'));
		assert.ok(commands.includes('reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholderEnd'));
	});
});
