import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { languageClientCommands } from '../extensionConfig/languageClient';
import {
	workbenchCommands,
	workbenchConfig,
	workbenchDefaults,
} from '../extensionConfig/workbench';
import {
	blockCommentPairPosition,
	ifSpaceCommitContractFromCommandArguments,
} from '../languageClient/languageClient';
import { positionFromByteOffset } from '../languageClient/symbolLocationBridge';
import { registerBlockCommentPair } from '../languageClient/typingAssistTransactionBridge';
import { executeIndent, executeInsertNewline, executeInsertSpace } from '../languageClient/controlHeaderEnterBridge';
import { VersionedEditorTransaction } from '../languageClient/versionedEditorTransaction';
import { completionPresentationObservationForDocument, completionUiMiddlewareCallbacks, nestedSnippetTransactionTookOwnership } from '../languageClient/completionUiBridge';
import { formatUiAutomationPayload } from '../languageClient/suggestWidgetUiReport';
import {
	activeScopeDelimiterDecorationOptions,
	activeScopeDelimiterRangesForSnapshot,
	refreshActiveScopeDelimiterDecorationForSnapshot,
	registerActiveScopeDelimiterBridge,
} from '../languageClient/activeScopeDelimiterBridge';
import {
	applyBracketColoringEditorMode,
	bracketColoringServerArguments,
	usesCustomScopeDelimiterPresentation,
} from '../languageClient/bracketColoringBridge';
import { RestartCoordinator } from '../languageClient/restartCoordinator';

suite('extension activation', () => {
	test('renders the completion response observed by the VS Code suggest pipeline', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: '\t#' });
		completionUiMiddlewareCallbacks.begin(document, new vscode.Position(0, 2), 1);
		const callable = new vscode.CompletionItem('TestNumFun2');
		callable.insertText = new vscode.SnippetString('TestNumFun2(${1:input}, ${2:num})');
		callable.range = new vscode.Range(0, 0, 0, 4);
		callable.command = { title: 'Show parameters', command: 'editor.action.triggerParameterHints' };
		completionUiMiddlewareCallbacks.respond(
			document,
			1,
			document.version,
			undefined,
			[callable, new vscode.CompletionItem('#ifdef')],
			0,
		);

		const report = completionPresentationObservationForDocument(document.uri.toString());
		assert.match(report, /Cursor: line 0, character 2/);
		assert.match(report, /Trigger kind: 1/);
		assert.match(report, /\| 1 \| TestNumFun2 \| TestNumFun2\(\$\{1:input\}, \$\{2:num\}\) \| snippet \| plain \| editor.action.triggerParameterHints \|/);
		assert.match(report, /\| 2 \| #ifdef \|  \| label \| none \|  \|/);
	});

	test('renders accessibility-visible suggestion rows separately from the completion payload', () => {
		const report = formatUiAutomationPayload({
			status: 'ok',
			focusedElement: 'GC_Sounds.c',
			lists: [{
				name: 'Suggest', automationId: 'suggestWidget', className: 'monaco-list', isOffscreen: false, hasKeyboardFocus: true,
				bounds: { x: 100, y: 200, width: 300, height: 120 },
				verticalScrollPercent: 0,
				items: [
					{ name: 'Resource', bounds: { x: 100, y: 200, width: 300, height: 20 }, isSelected: false },
					{ name: 'ResourceName', bounds: { x: 100, y: 220, width: 300, height: 20 }, isSelected: true },
					{ name: 'ResourceManager', bounds: { x: 100, y: 240, width: 300, height: 20 }, isSelected: false },
				],
			}],
		});
		assert.match(report, /Rendered Suggest Widget \(Windows UI Automation\)/);
		assert.match(report, /Bounds: 100,200 300x120/);
		assert.match(report, /\| 2 \| yes \| ResourceName \|/);
	});

	test('refuses to guess which unrelated accessibility list is the suggest widget', () => {
		const report = formatUiAutomationPayload({ status: 'no-suggest-widget', focusedElement: 'GC_Sounds.c', lists: [] });
		assert.match(report, /No rendered rows are reported rather than guessing/);
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
		assert.ok(commands.includes(workbenchCommands.validateScripts));
		assert.ok(contributedCommands.some(command =>
			command.command === workbenchCommands.validateScripts));
	});

	test('contributes the Workbench endpoint and compiler-validation defaults', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<
			string,
			{ default?: unknown; enum?: unknown[] }
		>;
		assert.strictEqual(
			properties[`${workbenchConfig.section}.${workbenchConfig.settings.enabled}`]?.default,
			workbenchDefaults.enabled,
		);
		assert.strictEqual(
			properties[`${workbenchConfig.section}.${workbenchConfig.settings.host}`]?.default,
			workbenchDefaults.host,
		);
		assert.strictEqual(
			properties[`${workbenchConfig.section}.${workbenchConfig.settings.port}`]?.default,
			workbenchDefaults.port,
		);
		assert.strictEqual(
			properties[`${workbenchConfig.section}.${workbenchConfig.settings.validationDelaySeconds}`]?.default,
			workbenchDefaults.validationDelaySeconds,
		);
		assert.deepStrictEqual(
			properties[`${workbenchConfig.section}.${workbenchConfig.settings.validationProfile}`]?.enum,
			['WORKBENCH'],
		);
	});

	test('retains map placeholder progression unless a nested snippet takes ownership', () => {
		assert.strictEqual(nestedSnippetTransactionTookOwnership(4, 4), false);
		assert.strictEqual(nestedSnippetTransactionTookOwnership(4, undefined), false);
		assert.strictEqual(nestedSnippetTransactionTookOwnership(4, 5), true);
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
			colorizedBracketPairs: string[][];
			autoClosingPairs: Array<{ open: string; close: string; notIn?: string[] }>;
			onEnterRules?: Array<{
				beforeText?: string;
				previousLineText?: string;
				action?: { indent?: string };
			}>;
		};
		const allBracketPairs = [
			['{', '}'],
			['[', ']'],
			['(', ')'],
			['<', '>'],
		];
		assert.deepStrictEqual(configuration.colorizedBracketPairs, allBracketPairs);
		assert.deepStrictEqual(
			configuration.autoClosingPairs.find(pair => pair.open === '/*'),
			{ open: '/*', close: '*/', notIn: ['string', 'comment'] },
		);
		assert.deepStrictEqual(
			configuration.autoClosingPairs.find(pair => pair.open === '<'),
			{ open: '<', close: '>' },
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
		assert.ok(!bodyLine.test('\t{'));
		assert.ok(!bodyLine.test('\t// comment'));
		assert.ok(!bodyLine.test('\t/* comment'));

		const languageDefaults = extension.packageJSON.contributes.configurationDefaults['[enforce]'] as {
			'editor.bracketPairColorization.enabled'?: boolean;
			'editor.matchBrackets'?: string;
		};
		assert.strictEqual(languageDefaults['editor.bracketPairColorization.enabled'], false);
		assert.strictEqual(languageDefaults['editor.matchBrackets'], 'never');
	});

	test('contributes one three-mode bracket coloring setting with semantic ownership by default', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const properties = extension.packageJSON.contributes.configuration.properties as Record<string, {
			default?: unknown;
			enum?: unknown[];
			enumItemLabels?: string[];
			scope?: string;
		}>;
		const setting = properties['reforgerScriptTools.bracketColoring'];

		assert.ok(setting);
		assert.strictEqual(setting.default, 'semantic');
		assert.strictEqual(setting.scope, 'application');
		assert.deepStrictEqual(setting.enum, ['semantic', 'punctuation', 'vscode']);
		assert.deepStrictEqual(setting.enumItemLabels, [
			'Semantic Owner Colors',
			'Punctuation Color',
			'Visual Studio Code Colors',
		]);
	});

	test('contributes one Enforce semantic palette without a complete color theme', () => {
		const extension = vscode.extensions.all.find(
			candidate => candidate.packageJSON.name === 'reforger-sript-tools',
		);
		assert.ok(extension, 'development extension is discoverable');
		const contributes = extension.packageJSON.contributes as {
			themes?: unknown;
			semanticTokenTypes?: Array<{
				id: string;
				superType: string;
				description: string;
			}>;
			configurationDefaults: Record<string, Record<string, unknown>>;
		};

		assert.strictEqual(contributes.themes, undefined);
		assert.deepStrictEqual(contributes.semanticTokenTypes, [
			{
				id: 'reforgerField',
				superType: 'property',
				description: 'An Enforce Script field.',
			},
			{
				id: 'reforgerPunctuation',
				superType: 'operator',
				description: 'Enforce Script punctuation.',
			},
			{
				id: 'reforgerPreprocessor',
				superType: 'keyword',
				description: 'Enforce Script preprocessor syntax.',
			},
		]);
		assert.strictEqual(
			contributes.configurationDefaults['[enforce]']['editor.semanticHighlighting.enabled'],
			true,
		);
		assert.deepStrictEqual(
			contributes.configurationDefaults['editor.semanticTokenColorCustomizations'],
			{
				rules: {
					'class:enforce': '#40b5ac',
					'enum:enforce': '#40b5ac',
					'type:enforce': '#40b5ac',
					'typeParameter:enforce': '#40b5ac',
					'function:enforce': '#f3ad58',
					'reforgerField:enforce': '#cfcfcf',
					'variable:enforce': '#cfcfcf',
					'parameter:enforce': '#cfcfcf',
					'enumMember:enforce': '#cfcfcf',
					'number:enforce': '#cfcfcf',
					'operator:enforce': '#cfcfcf',
					'reforgerPunctuation:enforce': '#cfcfcf',
					'keyword:enforce': '#59A6E9',
					'comment:enforce': '#59aa59',
					'string:enforce': '#c178dd',
					'reforgerPreprocessor:enforce': '#d4fd95',
				},
			},
		);
	});

	test('uses the bracket coloring mode as the sole Enforce native presentation control', async () => {
		const scope = { languageId: 'enforce' };
		const editorConfiguration = () => vscode.workspace.getConfiguration('editor', scope);
		try {
			await applyBracketColoringEditorMode('vscode');
			assert.strictEqual(
				editorConfiguration().get('bracketPairColorization.enabled'),
				true,
			);
			assert.strictEqual(editorConfiguration().get('matchBrackets'), 'always');

			await applyBracketColoringEditorMode('punctuation');
			assert.strictEqual(
				editorConfiguration().get('bracketPairColorization.enabled'),
				false,
			);
			assert.strictEqual(editorConfiguration().get('matchBrackets'), 'never');

			await applyBracketColoringEditorMode('semantic');
			assert.strictEqual(
				editorConfiguration().get('bracketPairColorization.enabled'),
				false,
			);
			assert.strictEqual(editorConfiguration().get('matchBrackets'), 'never');
		} finally {
			await editorConfiguration().update(
				'bracketPairColorization.enabled',
				undefined,
				vscode.ConfigurationTarget.Global,
				true,
			);
			await editorConfiguration().update(
				'matchBrackets',
				undefined,
				vscode.ConfigurationTarget.Global,
				true,
			);
		}
	});

	test('passes each bracket mode to Rust and reserves active matching for custom modes', () => {
		assert.deepStrictEqual(
			bracketColoringServerArguments('semantic'),
			['--bracket-coloring', 'semantic'],
		);
		assert.deepStrictEqual(
			bracketColoringServerArguments('punctuation'),
			['--bracket-coloring', 'punctuation'],
		);
		assert.deepStrictEqual(
			bracketColoringServerArguments('vscode'),
			['--bracket-coloring', 'vscode'],
		);
		assert.strictEqual(usesCustomScopeDelimiterPresentation('semantic'), true);
		assert.strictEqual(usesCustomScopeDelimiterPresentation('punctuation'), true);
		assert.strictEqual(usesCustomScopeDelimiterPresentation('vscode'), false);
	});

	test('coalesces overlapping language-server restarts to the latest request', async () => {
		const coordinator = new RestartCoordinator();
		const events: string[] = [];
		let releaseFirst: (() => void) | undefined;
		const firstBlocked = new Promise<void>(resolve => {
			releaseFirst = resolve;
		});

		const first = coordinator.run(async () => {
			events.push('semantic:start');
			await firstBlocked;
			events.push('semantic:end');
		});
		const superseded = coordinator.run(async () => {
			events.push('punctuation');
		});
		const latest = coordinator.run(async () => {
			events.push('vscode');
		});
		releaseFirst?.();
		await Promise.all([first, superseded, latest]);

		assert.deepStrictEqual(events, ['semantic:start', 'semantic:end', 'vscode']);
	});

	test('uses theme bracket-match emphasis without replacing semantic foregrounds', () => {
		const options = activeScopeDelimiterDecorationOptions();
		assert.strictEqual((options.backgroundColor as vscode.ThemeColor).id, 'editorBracketMatch.background');
		assert.strictEqual((options.borderColor as vscode.ThemeColor).id, 'editorBracketMatch.border');
		assert.strictEqual(options.borderStyle, 'solid');
		assert.strictEqual(options.borderWidth, '1px');
		assert.strictEqual(options.color, undefined);
	});

	test('projects current multi-caret scope delimiter responses into editor ranges', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example\n{\n\tvoid Run() {}\n}',
		});
		const selections = [
			new vscode.Selection(2, 10, 2, 10),
			new vscode.Selection(1, 1, 1, 1),
		];
		const requests: Array<{ method: string; params: unknown }> = [];
		const ranges = await activeScopeDelimiterRangesForSnapshot(
			document,
			selections,
			{
				sendRequest: async <Result>(method: string, params: unknown) => {
					requests.push({ method, params });
					return {
						version: document.version,
						pairs: [{
							opener: { start: { line: 2, character: 9 }, end: { line: 2, character: 10 } },
							closer: { start: { line: 2, character: 10 }, end: { line: 2, character: 11 } },
						}],
					} as Result;
				},
			},
			() => true,
		);

		assert.deepStrictEqual(requests, [{
			method: 'reforger/activeScopeDelimiters',
			params: {
				textDocument: { uri: document.uri.toString() },
				version: document.version,
				positions: [
					{ line: 2, character: 10 },
					{ line: 1, character: 1 },
				],
			},
		}]);
		assert.deepStrictEqual(ranges, [
			new vscode.Range(2, 9, 2, 10),
			new vscode.Range(2, 10, 2, 11),
		]);
	});

	test('rejects stale active scope delimiter responses', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example {}',
		});
		const selection = new vscode.Selection(0, 15, 0, 15);
		assert.strictEqual(
			await activeScopeDelimiterRangesForSnapshot(
				document,
				[selection],
				{
					sendRequest: async <Result>() => ({
						version: document.version + 1,
						pairs: [],
					}) as Result,
				},
				() => true,
			),
			undefined,
		);
		assert.strictEqual(
			await activeScopeDelimiterRangesForSnapshot(
				document,
				[selection],
				{
					sendRequest: async <Result>() => ({
						version: document.version,
						pairs: [],
					}) as Result,
				},
				() => false,
			),
			undefined,
		);
	});

	test('clears active scope decorations before awaiting current or stale refreshes', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example {}',
		});
		const selection = new vscode.Selection(0, 15, 0, 15);
		let releaseResponse: ((response: unknown) => void) | undefined;
		const response = new Promise(resolve => {
			releaseResponse = resolve;
		});
		const applied: vscode.Range[][] = [];
		const refresh = refreshActiveScopeDelimiterDecorationForSnapshot(
			document,
			[selection],
			{
				sendRequest: async <Result>() => await response as Result,
			},
			() => true,
			ranges => applied.push([...ranges]),
		);
		assert.deepStrictEqual(applied, [[]], 'the prior pair clears synchronously');
		releaseResponse?.({
			version: document.version,
			pairs: [{
				opener: { start: { line: 0, character: 14 }, end: { line: 0, character: 15 } },
				closer: { start: { line: 0, character: 15 }, end: { line: 0, character: 16 } },
			}],
		});
		await refresh;
		assert.strictEqual(applied.length, 2);
		assert.deepStrictEqual(applied[0], []);
		assert.deepStrictEqual(applied[1], [
			new vscode.Range(0, 14, 0, 15),
			new vscode.Range(0, 15, 0, 16),
		]);

		const staleApplied: vscode.Range[][] = [];
		await refreshActiveScopeDelimiterDecorationForSnapshot(
			document,
			[selection],
			{
				sendRequest: async <Result>() => ({
					version: document.version + 1,
					pairs: [],
				}) as Result,
			},
			() => true,
			ranges => staleApplied.push([...ranges]),
		);
		assert.deepStrictEqual(staleApplied, [[]]);
	});

	test('reports pending scope delimiter projections for a lifecycle retry', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example {}',
		});
		const selection = new vscode.Selection(0, 15, 0, 15);
		const applied: vscode.Range[][] = [];
		const foregroundReady = await refreshActiveScopeDelimiterDecorationForSnapshot(
			document,
			[selection],
			{
				sendRequest: async <Result>() => ({
					version: document.version,
					pending: true,
					pairs: [],
				}) as Result,
			},
			() => true,
			ranges => applied.push([...ranges]),
		);

		assert.strictEqual(foregroundReady, false);
		assert.deepStrictEqual(applied, [[], []]);
	});

	test('retries active scope delimiters when foreground syntax becomes ready', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example {}',
		});
		await vscode.window.showTextDocument(document);
		let requestCount = 0;
		const registration = registerActiveScopeDelimiterBridge({
			sendRequest: async <Result>() => {
				requestCount += 1;
				return {
					version: document.version,
					pending: requestCount === 1,
					pairs: [],
				} as Result;
			},
		});
		try {
			await new Promise(resolve => setTimeout(resolve, 80));
			assert.ok(requestCount >= 2, 'pending foreground state triggers a current-snapshot retry');
		} finally {
			registration.dispose();
		}
	});

	test('refreshes active scope delimiters with caret movement and stops on disposal', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: 'class Example {}',
		});
		const editor = await vscode.window.showTextDocument(document);
		let requestCount = 0;
		const registration = registerActiveScopeDelimiterBridge({
			sendRequest: async <Result>() => {
				requestCount += 1;
				return {
					version: document.version,
					pairs: [],
				} as Result;
			},
		});
		try {
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.ok(requestCount >= 1, 'registration requests the initial caret pair');
			const beforeMove = requestCount;
			editor.selection = new vscode.Selection(0, 7, 0, 7);
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.ok(requestCount > beforeMove, 'caret movement refreshes the active pair');
			const beforeEdit = requestCount;
			await editor.edit(edit => edit.insert(new vscode.Position(0, 0), ' '));
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.ok(requestCount > beforeEdit, 'document changes refresh the active pair');

			registration.dispose();
			const afterDispose = requestCount;
			editor.selection = new vscode.Selection(0, 8, 0, 8);
			await new Promise(resolve => setTimeout(resolve, 20));
			assert.strictEqual(requestCount, afterDispose);
		} finally {
			registration.dispose();
		}
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

	test('places the caret inside a generated class declaration body', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'modded class GRAY_TEST2' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(0, document.lineAt(0).text.length);
		editor.selection = new vscode.Selection(position, position);
		await executeInsertNewline(editor, {
			sendRequest: async () => ({
				edits: [{ range: { start: { line: 0, character: position.character }, end: { line: 0, character: position.character } }, newText: '\n{\n    \n}' }],
				selection: { line: 2, character: 4 },
				owner: 'classDeclaration',
			}),
		} as never);
		assert.strictEqual(document.getText(), 'modded class GRAY_TEST2\r\n{\r\n    \r\n}');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(2, 4));
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

	test('routes Tab after a completed unbraced if body before native indentation', async () => {
		const document = await vscode.workspace.openTextDocument({
			language: 'enforce',
			content: '        if (true)\n            return;\n\n',
		});
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(2, 0);
		editor.selection = new vscode.Selection(position, position);
		const requests: unknown[] = [];
		await executeIndent(editor, {
			sendRequest: async (_method: string, request: unknown) => {
				requests.push(request);
				return {
					edits: [{ range: { start: { line: 2, character: 0 }, end: { line: 2, character: 0 } }, newText: '        ' }],
					selection: { line: 2, character: 8 },
					owner: 'unbracedIfBody',
				};
			},
		} as never);
		assert.strictEqual(document.lineAt(2).text, '        ');
		assert.deepStrictEqual(editor.selection.active, new vscode.Position(2, 8));
		assert.deepStrictEqual(requests, [{
			textDocument: { uri: document.uri.toString() },
			version: 1,
			operation: 'indent',
			trace: false,
			selections: [{ start: { line: 2, character: 0 }, end: { line: 2, character: 0 } }],
			options: { tabSize: 4, insertSpaces: true },
		}]);
	});

	test('routes an eligible collection declaration Space as one prompt-opening edit', async () => {
		const document = await vscode.workspace.openTextDocument({ language: 'enforce', content: 'class Example\n{\n\tarray<int> values\n}' });
		const editor = await vscode.window.showTextDocument(document);
		const position = new vscode.Position(2, 18);
		editor.selection = new vscode.Selection(position, position);
		const requests: unknown[] = [];
		let suggestionRequests = 0;
		await executeInsertSpace(editor, {
			sendRequest: async (_method: string, request: unknown) => {
				requests.push(request);
				return {
					edits: [{ range: { start: { line: 2, character: 18 }, end: { line: 2, character: 18 } }, newText: ' ' }],
					selection: { line: 2, character: 19 },
					owner: 'collectionDeclarationTail',
					triggerSuggest: true,
				};
			},
		} as never, undefined, async () => {
			suggestionRequests += 1;
		});
		assert.strictEqual(document.lineAt(2).text, '\tarray<int> values ');
		assert.strictEqual(suggestionRequests, 1);
		assert.deepStrictEqual(requests, [{
			textDocument: { uri: document.uri.toString() },
			version: 1,
			operation: 'insertSpace',
			trace: false,
			selections: [{ start: { line: 2, character: 18 }, end: { line: 2, character: 18 } }],
			options: { tabSize: 4, insertSpaces: false },
		}]);
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
		const routedIndent = keybindings.find(binding => binding.command === languageClientCommands.indent);
		assert.match(routedIndent?.when ?? '', /!editorReadonly/);
		assert.match(routedIndent?.when ?? '', /!suggestWidgetVisible/);
		assert.match(routedIndent?.when ?? '', /!inSnippetMode/);
		const routedSpace = keybindings.find(binding => binding.command === languageClientCommands.insertSpace);
		assert.match(routedSpace?.when ?? '', /!editorReadonly/);
		assert.match(routedSpace?.when ?? '', /!suggestWidgetVisible/);
		assert.match(routedSpace?.when ?? '', /!inSnippetMode/);
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
