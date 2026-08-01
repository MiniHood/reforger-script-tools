import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { normalizeSearchPage, searchToolFor } from '../searchPrototype/mcpSearchClient';

const searchUiSource = fs.readFileSync(
	path.join(__dirname, '../../src/searchPrototype/searchUiPrototype.ts'),
	'utf8',
);

suite('Reforger search UI MCP mapping', () => {
	test('routes each source to its authoritative search tool', () => {
		assert.strictEqual(searchToolFor('workspace'), 'search_workspace_symbols');
		assert.strictEqual(searchToolFor('gameData'), 'search_game_data_symbols');
		assert.strictEqual(searchToolFor('wiki'), 'search_official_wiki');
	});

	test('maps symbol search handoffs into source-browser rows', () => {
		const results = normalizeSearchPage('gameData', {
			results: [{
				name: 'SCR_BaseGameMode',
				kind: 'class',
				qualifiedName: 'SCR_BaseGameMode',
				signature: 'class SCR_BaseGameMode',
				relativePath: 'Game/GameMode/SCR_BaseGameMode.c',
				declarationRange: { startLine: 18 },
				symbolRef: 'gd1:symbol',
				readSourceInput: {
					catalogueRevision: 'gd1:revision',
					relativePath: 'Game/GameMode/SCR_BaseGameMode.c',
					startLine: 18,
				},
				sourceUri: 'reforger-pak://58D0FB3206B6F859/current/Game/GameMode/SCR_BaseGameMode.c',
			}],
		});

		assert.deepStrictEqual(results[0], {
			id: 'gameData-0-gd1:symbol',
			source: 'gameData',
			kind: 'symbol',
			title: 'SCR_BaseGameMode',
			detail: 'class · SCR_BaseGameMode',
			path: 'Game/GameMode/SCR_BaseGameMode.c:18',
			excerpt: 'class SCR_BaseGameMode',
			matchKind: 'symbol',
			selectionStartLine: 18,
			selectionEndLine: 18,
			sourceUri: 'reforger-pak://58D0FB3206B6F859/current/Game/GameMode/SCR_BaseGameMode.c',
			readInput: {
				catalogueRevision: 'gd1:revision',
				relativePath: 'Game/GameMode/SCR_BaseGameMode.c',
				startLine: 18,
			},
		});
	});

	test('maps Wiki excerpts and citation links into documentation rows', () => {
		const results = normalizeSearchPage('wiki', {
			results: [{
				title: 'Game Master',
				heading: 'Overview',
				matchKind: 'body',
				relativePath: 'Modding/Game Master/Overview.md',
				matchedLine: 12,
				excerpt: 'Game Master controls the scenario.',
				sourceUrl: 'https://community.bistudio.com/wiki/Arma_Reforger:Game_Master',
				readInput: {
					corpusRevision: 'ow1:revision',
					relativePath: 'Modding/Game Master/Overview.md',
					startLine: 12,
					lineCount: 12,
				},
			}],
		});

		assert.strictEqual(results[0].kind, 'documentation');
		assert.strictEqual(results[0].path, 'Modding/Game Master/Overview.md:12');
		assert.strictEqual(results[0].sourceUrl, 'https://community.bistudio.com/wiki/Arma_Reforger:Game_Master');
		assert.strictEqual(results[0].excerpt, 'Game Master controls the scenario.');
		assert.strictEqual(results[0].selectionStartLine, 12);
		assert.strictEqual(results[0].selectionEndLine, 12);
	});

	test('keeps the full result path above a full-width preview', () => {
		assert.match(searchUiSource, /\.source-row \{ display: grid; grid-template-columns: 28px minmax\(0, 1fr\);/);
		assert.doesNotMatch(searchUiSource, /\.source-row \{ display: grid; grid-template-columns: 28px 1fr auto;/);
		assert.match(searchUiSource, /\.result-head \{ display: flex; justify-content: space-between;/);
		assert.match(searchUiSource, /\.result-path \{[^}]*overflow-wrap: anywhere;[^}]*text-align: right;/);
		assert.match(searchUiSource, /<div class="result-head"><h3>/);
	});
});
