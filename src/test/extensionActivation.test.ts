import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientCommands } from '../extensionConfig/languageClient';
import {
	blockCommentPairPosition,
	ifSpaceCommitContractFromCommandArguments,
} from '../languageClient/languageClient';
import { positionFromByteOffset } from '../languageClient/symbolLocationBridge';
import { registerBlockCommentPair } from '../languageClient/typingAssistTransactionBridge';
import { executeInsertNewline } from '../languageClient/controlHeaderEnterBridge';
import { VersionedEditorTransaction } from '../languageClient/versionedEditorTransaction';
import { completionPresentationObservationForDocument, completionUiMiddlewareCallbacks } from '../languageClient/completionUiBridge';

suite('extension activation', () => {
	test('renders the completion response observed by the VS Code suggest pipeline', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: '\t#' });
		completionUiMiddlewareCallbacks.begin(document, new vscode.Position(0, 2), 1);
		completionUiMiddlewareCallbacks.respond(
			document,
			1,
			document.version,
			undefined,
			[new vscode.CompletionItem('#define'), new vscode.CompletionItem('#ifdef')],
			0,
		);

		const report = completionPresentationObservationForDocument(document.uri.toString());
		assert.match(report, /Cursor: line 0, character 2/);
		assert.match(report, /Trigger kind: 1/);
		assert.match(report, /\| 1 \| #define \|/);
		assert.match(report, /\| 2 \| #ifdef \|/);
	});

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

	test('accepts only a complete Rust-authored if completion contract', () => {
		const contract = ifSpaceCommitContractFromCommandArguments([{
			expectedCommit: ' ',
			deletion: { start: { line: 3, character: 9 }, end: { line: 3, character: 10 } },
			trailingDeletion: { start: { line: 3, character: 10 }, end: { line: 3, character: 11 } },
			caret: { line: 3, character: 9 },
		}]);
		assert.deepStrictEqual(contract?.deletion, new vscode.Range(3, 9, 3, 10));
		assert.deepStrictEqual(contract?.trailingDeletion, new vscode.Range(3, 10, 3, 11));
		assert.deepStrictEqual(contract?.caret, new vscode.Position(3, 9));
		assert.strictEqual(ifSpaceCommitContractFromCommandArguments([{ expectedCommit: ' ' }]), undefined);
	});

	test('applies the Rust-authored if completion contract after a Space commit', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'if ()' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 4);
		await editor.edit(edit => edit.insert(position, ' '));
		editor.selection = new vscode.Selection(new vscode.Position(0, 5), new vscode.Position(0, 5));
		await vscode.commands.executeCommand(languageClientCommands.normalizeIfSpaceCommit, {
			expectedCommit: ' ',
			deletion: { start: { line: 0, character: 4 }, end: { line: 0, character: 5 } },
			trailingDeletion: { start: { line: 0, character: 5 }, end: { line: 0, character: 6 } },
			caret: { line: 0, character: 4 },
		});
		assert.strictEqual(document.getText(), 'if ()');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(0, 4));
	});

	test('removes a Space committed before VS Code applies the if snippet', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'if () ' });
		const editor = await vscode.window.showTextDocument(document);
		editor.selection = new vscode.Selection(new vscode.Position(0, 4), new vscode.Position(0, 4));
		await vscode.commands.executeCommand(languageClientCommands.normalizeIfSpaceCommit, {
			expectedCommit: ' ',
			deletion: { start: { line: 0, character: 4 }, end: { line: 0, character: 5 } },
			trailingDeletion: { start: { line: 0, character: 5 }, end: { line: 0, character: 6 } },
			caret: { line: 0, character: 4 },
		});
		assert.strictEqual(document.getText(), 'if ()');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(0, 4));
	});

	test('removes a Space committed after the if snippet before VS Code advances the selection', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'if ()' });
		const editor = await vscode.window.showTextDocument(document);
		editor.selection = new vscode.Selection(new vscode.Position(0, 4), new vscode.Position(0, 4));
		await vscode.commands.executeCommand(languageClientCommands.normalizeIfSpaceCommit, {
			expectedCommit: ' ',
			deletion: { start: { line: 0, character: 4 }, end: { line: 0, character: 5 } },
			trailingDeletion: { start: { line: 0, character: 5 }, end: { line: 0, character: 6 } },
			caret: { line: 0, character: 4 },
		});
		await editor.edit(edit => edit.insert(new vscode.Position(0, 4), ' '));
		await new Promise(resolve => setTimeout(resolve, 20));
		assert.strictEqual(document.getText(), 'if ()');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(0, 4));
	});

	test('rejects an if completion contract after a caret or selection change', async () => {
		const contract = {
			expectedCommit: ' ',
			deletion: { start: { line: 0, character: 4 }, end: { line: 0, character: 5 } },
			trailingDeletion: { start: { line: 0, character: 5 }, end: { line: 0, character: 6 } },
			caret: { line: 0, character: 4 },
		};
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'if ( )' });
		const editor = await vscode.window.showTextDocument(document);
		editor.selection = new vscode.Selection(new vscode.Position(0, 0), new vscode.Position(0, 0));
		await vscode.commands.executeCommand(languageClientCommands.normalizeIfSpaceCommit, contract);
		assert.strictEqual(document.getText(), 'if ( )');
		editor.selections = [
			new vscode.Selection(new vscode.Position(0, 4), new vscode.Position(0, 4)),
			new vscode.Selection(new vscode.Position(0, 4), new vscode.Position(0, 4)),
		];
		await vscode.commands.executeCommand(languageClientCommands.normalizeIfSpaceCommit, contract);
		assert.strictEqual(document.getText(), 'if ( )');
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

	test('routes a switch Enter as one atomic edit with default selected', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'switch (value)\n' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 14);
		editor.selection = new vscode.Selection(position, position);
		const requests: unknown[] = [];
		const changes: vscode.TextDocumentChangeEvent[] = [];
		let suggestionRequests = 0;
		const listener = vscode.workspace.onDidChangeTextDocument(event => {
			if (event.document.uri.toString() === document.uri.toString()) {
				changes.push(event);
			}
		});
		try {
			await executeInsertNewline(editor, {
				sendRequest: async (_method: string, request: unknown) => {
					requests.push(request);
					return {
						edits: [{ range: { start: { line: 0, character: 14 }, end: { line: 1, character: 0 } }, newText: '\n{\n\tdefault:\n\t\t\n}' }],
						selectionRange: { start: { line: 2, character: 1 }, end: { line: 2, character: 8 } },
						owner: 'controlHeader',
						triggerSuggest: true,
					};
				},
			} as never, undefined, async () => {
				suggestionRequests += 1;
			});
		} finally {
			listener.dispose();
		}
		assert.strictEqual(document.getText(), 'switch (value)\n{\n\tdefault:\n\t\t\n}');
		assert.deepStrictEqual(editor.selection.anchor, new vscode.Position(2, 1));
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(2, 8));
		assert.strictEqual(suggestionRequests, 1);
		assert.strictEqual(changes.length, 1, 'routed Enter must not trigger a post-native correction');
		assert.deepStrictEqual(requests, [{
			textDocument: { uri: document.uri.toString() },
			version: 1,
			operation: 'insertNewline',
			trace: false,
			selections: [{ start: { line: 0, character: 14 }, end: { line: 0, character: 14 } }],
			options: { tabSize: 4, insertSpaces: true },
		}]);
	});

	test('uses native fallback when an input route declines or fails', async () => {
		for (const response of [
			{ edits: [], reason: 'declined' },
			new Error('server unavailable'),
		]) {
			const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'while (true)' });
			const editor = await vscode.window.showTextDocument(document);
			const position = new vscode.Position(0, 12);
			editor.selection = new vscode.Selection(position, position);
			let fallbacks = 0;
			await executeInsertNewline(editor, {
				sendRequest: async () => {
					if (response instanceof Error) {
						throw response;
					}
					return response;
				},
			} as never, async () => {
				fallbacks += 1;
				await editor.edit(edit => edit.insert(editor.selection.active, '\n'));
			});
			assert.strictEqual(fallbacks, 1);
			assert.strictEqual(document.getText(), 'while (true)\r\n');
		}
	});

	test('dismisses completion before routing Enter without accepting its selection', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'if (tr)' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 'if (tr'.length);
		editor.selection = new vscode.Selection(position, position);
		const events: string[] = [];
		await executeInsertNewline(editor, {
			sendRequest: async () => {
				events.push('route');
				return {
					edits: [{ range: { start: { line: 0, character: 7 }, end: { line: 0, character: 7 } }, newText: '\n\t' }],
					owner: 'ifHeader',
				};
			},
		} as never, async () => {
			events.push('native');
		}, undefined, async () => {
			events.push('dismiss');
		});
		assert.deepStrictEqual(events, ['dismiss', 'route']);
		assert.strictEqual(document.getText(), 'if (tr)\r\n\t');
	});

	test('discards a stale input route before it can edit the current caret', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'while (true)' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, 12);
		editor.selection = new vscode.Selection(position, position);
		let fallbacks = 0;
		await executeInsertNewline(editor, {
			sendRequest: async () => {
				editor.selection = new vscode.Selection(new vscode.Position(0, 0), new vscode.Position(0, 0));
				return {
					edits: [{ range: { start: { line: 0, character: 12 }, end: { line: 0, character: 12 } }, newText: '\n{\n\t\n}' }],
					owner: 'controlHeader',
				};
			},
		} as never, async () => {
			fallbacks += 1;
			await editor.edit(edit => edit.insert(editor.selection.active, '\n'));
		});
		assert.strictEqual(fallbacks, 1);
		assert.strictEqual(document.getText(), '\r\nwhile (true)');
	});

	test('leaves routed Enter native when Experimental Auto Formatting is disabled', async () => {
		const configuration = vscode.workspace.getConfiguration('reforgerScriptTools');
		await configuration.update('experimentalAutoFormatting', false, vscode.ConfigurationTarget.Global);
		try {
			const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'while (true)' });
			const editor = await vscode.window.showTextDocument(document);
			const position = new vscode.Position(0, 12);
			editor.selection = new vscode.Selection(position, position);
			let requests = 0;
			let fallbacks = 0;
			await executeInsertNewline(editor, {
				sendRequest: async () => {
					requests += 1;
					return { edits: [] };
				},
			} as never, async () => {
				fallbacks += 1;
				await editor.edit(edit => edit.insert(editor.selection.active, '\n'));
			});
			assert.strictEqual(requests, 0);
			assert.strictEqual(fallbacks, 1);
			assert.strictEqual(document.getText(), 'while (true)\r\n');
		} finally {
			await configuration.update('experimentalAutoFormatting', undefined, vscode.ConfigurationTarget.Global);
		}
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

	test('keeps input-route traces out of the user configuration', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<string, { default?: unknown }>;
		assert.strictEqual(properties['reforgerScriptTools.diagnostics.enabled'].default, true);
		assert.strictEqual(properties['reforgerScriptTools.diagnostics.inputRouteTrace'], undefined);
	});

	test('enables Experimental Auto Formatting by default', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<string, {
			default?: unknown;
			title?: unknown;
		}>;
		const setting = properties['reforgerScriptTools.experimentalAutoFormatting'];
		assert.strictEqual(setting.default, true);
		assert.strictEqual(setting.title, 'Experimental: Auto Formatting');
	});

	test('applies directive separators only while Experimental Auto Formatting is enabled', async () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		await extension.activate();
		const enabled = await vscode.workspace.openTextDocument({ language: 'enforce', content: '#ifdef' });
		const enabledEditor = await vscode.window.showTextDocument(enabled);
		enabledEditor.selection = new vscode.Selection(new vscode.Position(0, 6), new vscode.Position(0, 6));
		await vscode.commands.executeCommand(languageClientCommands.applyDirectiveSeparator, '#ifdef');
		assert.strictEqual(enabled.getText(), '#ifdef ');

		const configuration = vscode.workspace.getConfiguration('reforgerScriptTools');
		await configuration.update('experimentalAutoFormatting', false, vscode.ConfigurationTarget.Global);
		try {
			const disabled = await vscode.workspace.openTextDocument({ language: 'enforce', content: '#ifndef' });
			const disabledEditor = await vscode.window.showTextDocument(disabled);
			disabledEditor.selection = new vscode.Selection(new vscode.Position(0, 7), new vscode.Position(0, 7));
			await vscode.commands.executeCommand(languageClientCommands.applyDirectiveSeparator, '#ifndef');
			assert.strictEqual(disabled.getText(), '#ifndef');
		} finally {
			await configuration.update('experimentalAutoFormatting', undefined, vscode.ConfigurationTarget.Global);
		}
	});

	test('serves directive and Macro completion through the live Extension Development Host', async function () {
		this.timeout(10_000);
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		await extension.activate();
		const folder = await fs.mkdtemp(path.join(os.tmpdir(), 'reforger-script-tools-'));
		const file = path.join(folder, 'PreprocessorCompletion.c');
		await fs.writeFile(file, '#define LIVE_FLAG\n\t#\n#ifndef ');
		try {
			const opened = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
			const document = await vscode.languages.setTextDocumentLanguage(opened, 'enforce');
			await vscode.window.showTextDocument(document);
			const directives = await completionItems(document, new vscode.Position(1, 2));
			for (const directive of ['#define', '#ifdef', '#ifndef', '#else', '#endif']) {
				assert.ok(directives.includes(directive), directive);
			}
			const macros = await completionItems(document, new vscode.Position(2, 8));
			assert.ok(macros.includes('LIVE_FLAG'));
		} finally {
			await fs.rm(folder, { recursive: true, force: true }).catch(error => {
				if ((error as NodeJS.ErrnoException).code !== 'EBUSY') {
					throw error;
				}
			});
		}
	});

	test('keeps Enter available for line breaks in Enforce suggestions', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const defaults = extension.packageJSON.contributes.configurationDefaults as Record<string, Record<string, unknown>>;
		assert.strictEqual(defaults['[enforce]']['editor.acceptSuggestionOnEnter'], 'off');
	});

	test('routes Enter only outside native editing modes', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const keybindings = extension.packageJSON.contributes.keybindings as Array<{
			command: string;
			when?: string;
			args?: { dismissSuggest?: boolean };
		}>;
		const routedEnter = keybindings.find(binding => binding.command === languageClientCommands.insertNewline && binding.when?.includes('!suggestWidgetVisible'));
		assert.match(routedEnter?.when ?? '', /!editorReadonly/);
		assert.match(routedEnter?.when ?? '', /!inSnippetMode/);
		const suggestEnter = keybindings.find(binding => binding.command === languageClientCommands.insertNewline
			&& binding.when?.includes('suggestWidgetVisible')
			&& !binding.when.includes('!suggestWidgetVisible'));
		assert.strictEqual(suggestEnter?.args?.dismissSuggest, true);
	});

});

async function completionItems(document: vscode.TextDocument, position: vscode.Position): Promise<string[]> {
	for (let attempt = 0; attempt < 20; attempt += 1) {
		const result = await vscode.commands.executeCommand<vscode.CompletionList>(
			'vscode.executeCompletionItemProvider', document.uri, position,
		);
		const labels = result?.items.map(item => typeof item.label === 'string' ? item.label : item.label.label) ?? [];
		if (labels.length > 0) {
			return labels;
		}
		await new Promise(resolve => setTimeout(resolve, 100));
	}
	return [];
}
