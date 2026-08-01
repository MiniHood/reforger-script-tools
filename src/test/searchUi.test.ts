import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { normalizeSearchPage, searchToolFor } from '../searchPrototype/mcpSearchClient';

const searchUiSource = fs.readFileSync(
	path.join(__dirname, '../../src/searchPrototype/searchUiPrototype.ts'),
	'utf8',
);
const searchClientSource = fs.readFileSync(
	path.join(__dirname, '../../src/searchPrototype/mcpSearchClient.ts'),
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

	test('opens result cards while preserving text selection and the Wiki page action', () => {
		assert.match(searchUiSource, /data-open="' \+ esc\(result\.id\) \+ '" tabindex="0" role="button"/);
		assert.doesNotMatch(searchUiSource, /<button class="open" data-open=/);
		assert.match(searchUiSource, /const hasTextSelection = \(\) => Boolean\(window\.getSelection\(\)\?\.toString\(\)\);/);
		assert.match(searchUiSource, /if \(event\.target\.closest\('\[data-external\]'\) \|\| hasTextSelection\(\)\) return;/);
		assert.match(searchUiSource, /keydown', event => \{ if \(event\.target\.closest\('\[data-external\]'\)\) return;/);
		assert.match(searchUiSource, /data-external="' \+ esc\(result\.id\) \+ '">Open official page/);
	});

	test('supports cursor-backed result pages and selectable page sizes', () => {
		assert.match(searchClientSource, /public async search\([\s\S]*?pageSize: number,[\s\S]*?page: number/);
		assert.match(searchClientSource, /limit: pageSize/);
		assert.match(searchClientSource, /cursor/);
		assert.match(searchClientSource, /nextCursor/);
		assert.match(searchClientSource, /total: number/);
		assert.match(searchUiSource, /const pageSizeOptions = \[25, 50, 100\];/);
		assert.match(searchUiSource, /data-page-input/);
		assert.match(searchUiSource, /data-page-prev/);
		assert.match(searchUiSource, /data-page-next/);
		assert.match(searchUiSource, /data-page-size/);
		assert.match(searchUiSource, /<select data-page-size aria-label="Total results per page">/);
		assert.match(searchUiSource, /<span class="page-arrows"><button type="button" data-page-prev[\s\S]*data-page-next/);
		assert.match(searchUiSource, /<select data-page-size[\s\S]*<span class="muted">Page<\/span>/);
		assert.match(searchUiSource, /<span class="muted">-<\/span><span class="muted">Page<\/span>/);
		assert.match(searchUiSource, /pageTotal \+ '<\/span><span class="muted">-<\/span><span class="page-arrows">/);
		assert.match(searchUiSource, /value="' \+ state\.page \+ '"/);
		assert.match(searchUiSource, /\/ ' \+ pageTotal \+ '<\/span>/);
		assert.match(searchUiSource, /state\.type = element\.dataset\.type; state\.page = 1; search\(true\)/);
		assert.match(searchUiSource, /resultType: state\.type/);
		assert.match(searchUiSource, /message\.resultType/);
		assert.match(searchClientSource, /const sourcePageSize = 100;/);
		assert.match(searchClientSource, /let sourceOffset = 0;/);
		assert.match(searchClientSource, /this\.searchPageCaches\.clear\(\);/);
		assert.doesNotMatch(searchUiSource, /maxPageNumber/);
		assert.doesNotMatch(searchUiSource, /results per source/);
		assert.match(searchUiSource, /Showing up to ' \+ state\.pageSize \+ ' total results/);
	});
});
