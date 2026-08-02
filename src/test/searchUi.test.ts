import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { searchLimits } from '../extensionConfig/search';
import { formatSearchKind, maxSearchPages, normalizeSearchPage, searchKindFilters, searchToolFor, sourceLinePreview, sourceMatchRange, sourcePreviewLine, stripSourceComments } from '../searchPrototype/mcpSearchClient';
import { semanticTokenSpansForLine } from '../searchPrototype/semanticPreview';

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
		assert.strictEqual(searchToolFor('workspace', 'text'), 'search_workspace_text');
		assert.strictEqual(searchToolFor('gameData', 'text'), 'search_game_data_text');
		assert.strictEqual(searchToolFor('wiki', 'text'), 'search_official_wiki');
	});

	test('maps literal text matches to exact line previews and source-read handoffs', () => {
		const results = normalizeSearchPage('workspace', {
			results: [{
				relativePath: 'Scripts/Example.c',
				matchRange: { startLine: 12, startCharacter: 18, endLine: 12, endCharacter: 22 },
				excerpt: 'void Run() { SCR_(); }',
				matchText: 'SCR_',
				readSourceInput: {
					catalogueRevision: 'ws1:revision',
					relativePath: 'Scripts/Example.c',
					startLine: 12,
				},
			}],
		}, 'text');

		assert.strictEqual(results[0].kind, 'text');
		assert.strictEqual(results[0].excerpt, 'void Run() { SCR_(); }');
		assert.strictEqual(results[0].textMatchStart, 13);
		assert.strictEqual(results[0].textMatchLength, 4);
		assert.strictEqual(results[0].readInput.catalogueRevision, 'ws1:revision');
	});

	test('routes explicit text mode to corpus-specific literal search tools', () => {
		assert.strictEqual(searchToolFor('workspace', 'text'), 'search_workspace_text');
		assert.strictEqual(searchToolFor('gameData', 'text'), 'search_game_data_text');
		const results = normalizeSearchPage('workspace', {
			results: [{
				relativePath: 'Game/Use.c',
				matchRange: { startLine: 4, startCharacter: 13, endLine: 4, endCharacter: 17 },
				excerpt: '    void Run(SCR_ value);',
				matchText: 'SCR_',
				readSourceInput: { catalogueRevision: 'ws1:test', relativePath: 'Game/Use.c', startLine: 4 },
			}],
		}, 'text');
		assert.strictEqual(results[0].kind, 'text');
		assert.strictEqual(results[0].title, 'SCR_');
		assert.strictEqual(results[0].textMatchStart, 13);
		assert.strictEqual(results[0].textMatchLength, 4);
		assert.strictEqual(results[0].readInput.catalogueRevision, 'ws1:test');
	});

	test('formats compound symbol kinds for result details', () => {
		assert.strictEqual(formatSearchKind('enumMember'), 'enum Member');
		assert.strictEqual(formatSearchKind('globalField'), 'global Field');
		assert.strictEqual(formatSearchKind('method'), 'function');
		assert.strictEqual(formatSearchKind('constructor'), 'function');
		assert.strictEqual(formatSearchKind('destructor'), 'function');
	});

	test('extracts the authoritative source line for a symbol preview', () => {
		assert.strictEqual(sourceLinePreview({ content: '    class SCR_Mode\n', startLine: 18, endLine: 18 }, 18), 'class SCR_Mode');
		assert.strictEqual(sourceLinePreview({ content: 'only line', startLine: 0, endLine: 0 }, 1), 'only line');
	});

	test('anchors previews to the declaration and removes comments', () => {
		const document = { content: '// field documentation\n    SCR_Field value; // trailing note\n', startLine: 10, endLine: 11 };
		assert.strictEqual(sourcePreviewLine(document, 10, 'SCR_Field'), 11);
		assert.strictEqual(sourceLinePreview(document, 10, 'SCR_Field'), 'SCR_Field value;');
		assert.strictEqual(stripSourceComments('const url = "https://example.test"; // note'), 'const url = "https://example.test"; ');
	});

	test('finds the selected symbol occurrence instead of every query occurrence', () => {
		assert.deepStrictEqual(sourceMatchRange('void Foo(Foo value)', 'Foo'), { start: 5, length: 3 });
		assert.deepStrictEqual(sourceMatchRange('void foo()', 'FOO'), { start: 5, length: 3 });
		assert.strictEqual(sourceMatchRange('void Bar()', 'Foo'), undefined);
		assert.doesNotMatch(searchUiSource, /highlightText\(result\.excerpt, state\.query \+ ' ' \+ result\.title\)/);
		assert.match(searchUiSource, /hydrateSymbolPreviews\(active, client, requestId, result\.results, normalizedQuery\)/);
		assert.match(searchUiSource, /const matchRange = sourceMatchRange\(preview, query\);/);
	});

	test('decodes the language server semantic token legend for a preview line', () => {
		assert.deepStrictEqual(semanticTokenSpansForLine([
			0, 4, 5, 0, 0,
			1, 0, 8, 7, 0,
		], 1), [{ start: 0, length: 8, role: 'enumMember' }]);
	});

	test('defines useful symbol kind filters without a documentation duplicate', () => {
		assert.deepStrictEqual(searchKindFilters.map(filter => filter.label), [
			'All results',
			'Classes',
			'Functions',
			'Fields',
			'Enums',
		]);
		assert.deepStrictEqual(searchKindFilters.find(filter => filter.value === 'function')?.kinds, [
			'function',
			'method',
			'constructor',
			'destructor',
		]);
		assert.doesNotMatch(searchUiSource, /Documentation/);
	});

	test('keeps full-text search explicit inside the existing search UI', () => {
		assert.match(searchUiSource, /const modeButtons = \(\) =>/);
		assert.match(searchUiSource, /state\.mode === 'text'/);
		assert.match(searchUiSource, /data-mode="semantic"/);
		assert.match(searchUiSource, /data-mode="text"/);
		assert.match(searchUiSource, /state\.mode === 'text' \? '' : resultTypes/);
		assert.match(searchUiSource, /if \(state\.mode === 'text'\) return/);
		assert.match(searchUiSource, /clearTimeout\(searchTimer\); if \(state\.mode === 'text'\) return/);
		assert.match(searchUiSource, /searchMode: state\.mode/);
		assert.match(searchClientSource, /search_workspace_text/);
		assert.match(searchClientSource, /search_game_data_text/);
		assert.match(searchClientSource, /paginationMode: paginationModeFor\(mode, sources\)/);
		assert.match(searchClientSource, /mode === 'semantic'[\s\S]*sources\.filter\(source => source !== 'wiki'\)/);
		assert.match(searchUiSource, /state\.scopeSources\.filter\(source => source\.kind !== 'wiki' \|\| state\.mode === 'text'\)/);
		assert.match(searchUiSource, /selectedEligibleScopeIds\(\)/);
	});

	test('uses discovered loaded add-ons as the production Search Scope', () => {
		assert.match(searchClientSource, /public async discoverScope\(\): Promise<SearchScopeDiscovery>/);
		assert.match(searchClientSource, /this\.callTool\('game_data_status', \{\}\)/);
		assert.match(searchClientSource, /defaultSelected: addon\.defaultSelected === true/);
		assert.doesNotMatch(searchClientSource, /baseGameScopeId|enfusionCoreScopeId/);
		assert.match(searchUiSource, /scopeOpen: false/);
		assert.ok(
			searchClientSource.indexOf("...(addonGuids.length > 0 ? ['gameData' as const] : [])")
				< searchClientSource.indexOf("...(normalizedScopes.includes(workspaceScopeId) ? ['workspace' as const] : [])"),
		);
		assert.match(searchUiSource, /const searchScope = \(\) =>/);
		assert.match(searchUiSource, /data-scope-choice/);
		assert.match(searchUiSource, /data-scope-all/);
		assert.doesNotMatch(searchUiSource, /data-scope-refresh|message\.type === 'refreshScope'|refreshSearchScope/);
		assert.match(searchUiSource, /<div class="scope-actions"><input class="addon-filter"[\s\S]*?<button type="button" data-scope-all>/);
		assert.match(searchUiSource, /Select all/);
		assert.match(searchUiSource, /Unselect all/);
		assert.match(searchUiSource, /const allEligibleScopesSelected = \(\) => \{/);
		assert.match(searchUiSource, /eligible\.every\(source => state\.selectedScopeIds\.includes\(source\.id\)\)/);
		assert.match(searchUiSource, /const allSelected = allEligibleScopesSelected\(\);/);
		assert.match(searchUiSource, /allSelected \? 'Unselect all' : 'Select all'/);
		assert.match(searchUiSource, /selectedScopeIds/);
		assert.match(searchUiSource, /selectionTouched/);
		assert.match(searchUiSource, /message\.type === 'scope'/);
		assert.match(searchUiSource, /nextSources\.filter\(source => source\.defaultSelected\)/);
		assert.match(searchUiSource, /selected\.length > 3/);
		assert.match(searchUiSource, /' more<\/span>'/);
		assert.doesNotMatch(searchUiSource, /prototypeVariants/);
		assert.doesNotMatch(searchUiSource, /addonPrototypeSources/);
		assert.doesNotMatch(searchUiSource, /SEARCH IN/);
		assert.match(searchUiSource, /No search scopes selected\./);
		assert.match(searchUiSource, /scopeDiscoveryMs/);
		assert.match(searchUiSource, /entry\.addonTotals = jsonField\(source\.addonTotals\)/);
		assert.match(searchUiSource, /readMsByAddon/);
		assert.match(searchUiSource, /readFailuresByAddon/);
		assert.match(searchUiSource, /unavailableScopeIds/);
		assert.match(searchClientSource, /unavailableScopeIds/);
		assert.doesNotMatch(searchUiSource, /\.addon-choice\.workspace/);
		assert.match(searchUiSource, /source\.pinned && !filtered\[index \+ 1\]\?\.pinned/);
		assert.match(searchUiSource, /pinned-boundary/);
		assert.match(searchUiSource, /\.scope-actions \[data-scope-all\] \{ flex: 0 0 76px; width: 76px; box-sizing: border-box;/);
		assert.match(searchUiSource, /document\.addEventListener\('click', event => \{/);
		assert.match(searchUiSource, /event\.target\.closest\('\.search-scope'\)/);
		assert.match(searchUiSource, /document\.querySelector\('\.addon-menu'\)\?\.remove\(\)/);
		assert.match(searchClientSource, /workspaceScopeId[\s\S]*?wikiScopeId[\s\S]*?\.\.\.addonSources/);
		assert.match(searchClientSource, /wikiScopeId[^\n]+pinned: true/);
		assert.match(searchUiSource, /SEARCH SCOPE<\/div>' \+ searchScope\(\) \+ '<div class="group-label">SEARCH MODE<\/div>' \+ modeButtons\(\)/);
	});

	test('starts the custom Search MCP process with the configured external index mode', () => {
		assert.match(searchClientSource, /externalIndexMode: ExternalIndexMode/);
		assert.match(searchClientSource, /'--external-index-mode',[\s\S]*?this\.options\.externalIndexMode/);
		assert.match(searchUiSource, /externalIndexMode: readExternalIndexMode\(\)/);
		assert.match(searchUiSource, /affectsConfiguration\(`\$\{workbenchConfig\.section\}\.\$\{workbenchConfig\.settings\.externalIndexMode\}`\)/);
		assert.match(searchUiSource, /restartSearchScopeForIndexMode\(context, active\)/);
		assert.match(searchUiSource, /\(await previousClient\)\.dispose\(\)/);
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
			detail: 'class',
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

	test('preserves add-on identity in game-data rows and source handoffs', () => {
		const results = normalizeSearchPage('gameData', {
			results: [{
				name: 'SCR_AddonClass',
				kind: 'class',
				qualifiedName: 'SCR_AddonClass',
				signature: 'class SCR_AddonClass',
				relativePath: 'Scripts/SCR_AddonClass.c',
				declarationRange: { startLine: 7 },
				symbolRef: 'sr2:addon-symbol',
				addonGuid: 'A1B2C3D4E5F60718',
				addonLabel: 'Example Add-on',
				readSourceInput: {
					catalogueRevision: 'gd2:revision',
					addonGuid: 'A1B2C3D4E5F60718',
					relativePath: 'Scripts/SCR_AddonClass.c',
					startLine: 7,
				},
			}],
		});

		assert.strictEqual(results[0].addonGuid, 'A1B2C3D4E5F60718');
		assert.strictEqual(results[0].addonLabel, 'Example Add-on');
		assert.strictEqual(results[0].readInput.addonGuid, 'A1B2C3D4E5F60718');
		assert.match(results[0].id, /A1B2C3D4E5F60718/);
	});

	test('keeps the full result path above a full-width preview', () => {
		assert.match(searchUiSource, /\.source-row \{ display: grid; grid-template-columns: 28px minmax\(0, 1fr\);/);
		assert.doesNotMatch(searchUiSource, /\.source-row \{ display: grid; grid-template-columns: 28px 1fr auto;/);
		assert.match(searchUiSource, /\.result-head \{ display: flex; justify-content: space-between;/);
		assert.match(searchUiSource, /\.result-path \{[^}]*overflow-wrap: anywhere;[^}]*text-align: right;/);
		assert.match(searchUiSource, /<div class="result-head"><h3>/);
		assert.match(searchUiSource, /const highlightText = \(value, query\) =>/);
		assert.match(searchUiSource, /<mark>/);
		assert.match(searchUiSource, /highlightRange\(sourceText, matchRange\)/);
		assert.match(searchUiSource, /state\.sourcePreviews\[result\.id\]/);
		assert.doesNotMatch(searchUiSource, /return highlightRange\(result\.excerpt, matchRange\)/);
		assert.match(searchUiSource, /state\.matchRanges = \{ \.\.\.state\.matchRanges/);
		assert.match(searchUiSource, /message\.type === 'previews'/);
		assert.match(searchUiSource, /hydrateSymbolPreviews/);
		assert.match(searchUiSource, /provideLanguageServerSemanticTokens/);
		assert.match(searchUiSource, /semanticPreviewText/);
		assert.match(searchUiSource, /message\.type === 'semanticPreviews'/);
		assert.match(searchUiSource, /previewMatchText/);
		assert.match(searchUiSource, /semanticTokenRoles/);
		assert.match(searchUiSource, /semanticForegrounds/);
		assert.match(searchUiSource, /previewDiagnostics/);
	});

	test('toggles a top-only two-column result grid without rerunning the search', () => {
		assert.match(searchUiSource, /resultColumns: 1/);
		assert.match(searchUiSource, /\.source-rows\.two-column \{ grid-template-columns: repeat\(2, minmax\(0, 1fr\)\);/);
		assert.match(searchUiSource, /\.page-controls \[data-result-layout\] \{ display: inline-flex; align-items: center; justify-content: center; padding: 0; line-height: 0; \}/);
		assert.match(searchUiSource, /const pageControls = \(includeLayoutToggle = false\) =>/);
		assert.match(searchUiSource, /return '<div class="page-controls" aria-label="Search result pages">' \+ layoutToggle \+ '<select data-page-size/);
		assert.match(searchUiSource, /pageControls\(true\)/);
		assert.match(searchUiSource, /data-result-layout/);
		assert.match(searchUiSource, /aria-pressed="' \+ \(state\.resultColumns === 2\) \+ '"/);
		assert.match(searchUiSource, /state\.resultColumns = state\.resultColumns === 2 \? 1 : 2; render\(false\);/);
		assert.doesNotMatch(searchUiSource, /data-result-layout[^\n]*search\(/);
		assert.match(searchUiSource, /resultColumns: state\.resultColumns/);
	});

	test('publishes raw previews before semantic coloring completes', () => {
		const rawPhase = searchUiSource.indexOf('const flushRawPreviews = (): void =>');
		const rawMessage = searchUiSource.indexOf("type: 'previews',", rawPhase);
		const semanticPhase = searchUiSource.indexOf("const semanticWorker = async");
		const semanticMessage = searchUiSource.indexOf("type: 'semanticPreviews'", semanticPhase);
		assert.ok(rawPhase >= 0);
		assert.ok(rawMessage > rawPhase);
		assert.ok(semanticPhase > rawMessage);
		assert.ok(semanticMessage > semanticPhase);
		assert.match(searchUiSource, /queueRawPreview\(hit\.id\)/);
		assert.match(searchUiSource, /flushRawPreviews\(\);/);
		assert.match(searchUiSource, /searchUi\.previewRawCompleted/);
		assert.match(searchUiSource, /phase: 'raw'/);
		assert.match(searchUiSource, /phase: 'semantic'/);
		assert.match(searchUiSource, /lastSemanticMessageMs/);
		assert.match(searchUiSource, /state\.previewPerformance = message\.performance \?\? state\.previewPerformance/);
	});

	test('opens result cards while preserving text selection and the Wiki page action', () => {
		assert.match(searchUiSource, /data-open="' \+ esc\(result\.id\) \+ '" tabindex="0" role="button"/);
		assert.doesNotMatch(searchUiSource, /<button class="open" data-open=/);
		assert.match(searchUiSource, /const hasTextSelection = \(\) => Boolean\(window\.getSelection\(\)\?\.toString\(\)\);/);
		assert.match(searchUiSource, /if \(event\.target\.closest\('\[data-external\]'\) \|\| hasTextSelection\(\)\) return;/);
		assert.match(searchUiSource, /keydown', event => \{ if \(event\.target\.closest\('\[data-external\]'\)\) return;/);
		assert.match(searchUiSource, /data-external="' \+ esc\(result\.id\) \+ '">Open official page/);
	});

	test('supports random-access result pages, cursor-compatible MCP responses, and selectable page sizes', () => {
		assert.strictEqual(maxSearchPages, searchLimits.maxPages);
		assert.match(searchClientSource, /public async search\([\s\S]*?pageSize: number,[\s\S]*?page: number/);
		assert.match(searchClientSource, /symbolKinds\?: readonly SearchSymbolKind\[\]/);
		assert.match(searchClientSource, /kinds: symbolKinds/);
		assert.match(searchClientSource, /limit: pageSize/);
		assert.match(searchClientSource, /nextCursor/);
		assert.match(searchClientSource, /nextCursor/);
		assert.match(searchClientSource, /total: number/);
		assert.match(searchClientSource, /totalBySource: Partial<Record<SearchSource, number>>/);
		assert.match(searchUiSource, /const pageSizeOptions = \[25, 50, 100\];/);
		assert.match(searchUiSource, /data-page-input/);
		assert.match(searchUiSource, /data-page-prev/);
		assert.match(searchUiSource, /data-page-next/);
		assert.match(searchUiSource, /data-page-size/);
		assert.match(searchUiSource, /<select data-page-size aria-label="Total results per page"/);
		assert.match(searchUiSource, /data-page-size aria-label="Total results per page"' \+ \(state\.status === 'loading' \? ' disabled' : ''\)/);
		assert.match(searchUiSource, /<span class="page-arrows"><button type="button" data-page-prev[\s\S]*data-page-next/);
		assert.match(searchUiSource, /<select data-page-size[\s\S]*<span class="page-status"><span class="muted">Page<\/span>/);
		assert.match(searchUiSource, /\.page-status \{ display: inline-flex; flex: 0 0 150px; align-items: center; justify-content: flex-end;/);
		assert.match(searchUiSource, /value="' \+ state\.page \+ '"/);
		assert.match(searchUiSource, /of ' \+ pageTotal \+ '<\/span>/);
		assert.match(searchUiSource, /state\.type = element\.dataset\.type; state\.page = 1; search\(true\)/);
		assert.match(searchUiSource, /resultType: state\.type/);
		assert.match(searchUiSource, /message\.resultType/);
		assert.match(searchUiSource, /const resultTypes = \$\{JSON\.stringify\(searchKindFilters\.map\(\(\{ value, label \}\) => \(\{ value, label \}\)\)\)\};/);
		assert.match(searchUiSource, /searchKindsFor\(typeValue\)/);
		assert.match(searchUiSource, /if \(!isSearchKindValue\(message\.resultType\)\) \{/);
		assert.match(searchClientSource, /const sourcePageSize = 100;/);
		assert.match(searchUiSource, /const sourcePreviewWorkerCount = 8;/);
		assert.match(searchUiSource, /Math\.min\(sourcePreviewWorkerCount, previewHits\.length\)/);
		assert.match(searchUiSource, /const previewUpdateBatchSize = 4;/);
		assert.match(searchUiSource, /pendingRawPreviewIds\.length >= previewUpdateBatchSize/);
		assert.match(searchUiSource, /setTimeout\(\(\) => search\(true\), 100\)/);
		assert.doesNotMatch(searchUiSource, /setTimeout\(\(\) => search\(true\), 260\)/);
		assert.match(searchClientSource, /export const maxSearchPages = searchLimits\.maxPages;/);
		assert.match(searchClientSource, /paginationMode: 'offset'/);
		assert.match(searchClientSource, /export interface SearchPerformance/);
		assert.match(searchClientSource, /remoteRequests: number/);
		assert.match(searchClientSource, /cacheHits: number/);
		assert.match(searchClientSource, /performance: finishSearchPerformance/);
		assert.match(searchClientSource, /let sourceOffset = 0;/);
		assert.match(searchClientSource, /this\.searchPageCaches\.clear\(\);/);
		assert.match(searchUiSource, /const maxSearchPages = \$\{searchLimits\.maxPages\};/);
		assert.match(searchUiSource, /Math\.min\(maxSearchPages, Math\.max\(1, Math\.ceil\(state\.total \/ state\.pageSize\)\)\)/);
		assert.doesNotMatch(searchUiSource, /results per source/);
		assert.match(searchUiSource, /Showing up to ' \+ state\.pageSize \+ ' total results/);
		assert.match(searchUiSource, /<div class="empty">No results match this search\.<\/div>/);
		assert.doesNotMatch(searchUiSource, /Enter a symbol, concept, or documentation term to search\./);
		assert.doesNotMatch(searchUiSource, /function search\(resetPagination\) \{ if \(resetPagination\) \{ state\.page = 1; state\.total = 0; \}/);
		assert.doesNotMatch(searchUiSource, /function search\(resetPagination\)[\s\S]*?state\.uiPerformance\.lastPreviewMessageMs = 0; render\(\); vscode\.postMessage/);
		assert.doesNotMatch(searchUiSource, /if \(message\.type === 'loading'\) \{[^}]*render\(\); \}/);
		assert.match(searchUiSource, /event\.ctrlKey && event\.key === 'F3'/);
		assert.match(searchUiSource, /if \(state\.status === 'loading'\) return;/);
		assert.match(searchUiSource, /state\.lastSearchKey/);
		assert.match(searchUiSource, /function requestPage\(value\) \{ if \(state\.status === 'loading'\) return;/);
		assert.match(searchClientSource, /const cached = pages\.get\(page\);/);
		assert.match(searchClientSource, /offset: \(page - 1\) \* pageSize/);
		assert.match(searchClientSource, /const firstPageNumber = Math\.floor\(start \/ sourcePageSize\) \+ 1;/);
		assert.match(searchClientSource, /const lastPageNumber = Math\.floor\(\(end - 1\) \/ sourcePageSize\) \+ 1;/);
		assert.match(searchUiSource, /type: 'debugSnapshot'/);
		assert.match(searchUiSource, /message\.type === 'debugSnapshot'/);
		assert.match(searchUiSource, /searchUi\.snapshot/);
		assert.match(searchUiSource, /searchPerformance: state\.searchPerformance/);
		assert.match(searchUiSource, /lastSearchResponseMs/);
		assert.match(searchUiSource, /state\.previewPerformance = message\.performance/);
		assert.match(searchUiSource, /totalBySource: state\.totalBySource/);
		assert.match(searchUiSource, /selectionStartLine: result\.selectionStartLine/);
		assert.match(searchUiSource, /excerptLength: typeof result\.excerpt === 'string'/);
		assert.match(searchUiSource, /function snapshotResults\(value: unknown\)/);
		assert.match(searchUiSource, /resultsTruncated: Array\.isArray\(snapshot\.results\) && snapshot\.results\.length > 100/);
		assert.match(searchUiSource, /const modeButtons = \(\) =>/);
		assert.match(searchUiSource, /data-mode="text"/);
		assert.match(searchUiSource, /state\.mode === 'text'/);
		assert.match(searchUiSource, /searchMode: state\.mode/);
		assert.match(searchUiSource, /matchCase: false, matchWholeWord: false, useRegex: false/);
		assert.match(searchUiSource, /data-text-option="matchCase"/);
		assert.match(searchUiSource, />Match case<\/label>/);
		assert.match(searchUiSource, /data-text-option="matchWholeWord"/);
		assert.match(searchUiSource, />Match whole word<\/label>/);
		assert.match(searchUiSource, /data-text-option="useRegex"/);
		assert.match(searchUiSource, />Regular expression<\/label>/);
		assert.match(searchUiSource, /matchCase: state\.matchCase, matchWholeWord: state\.matchWholeWord, useRegex: state\.useRegex/);
		assert.match(searchClientSource, /matchCase: textOptions\.matchCase/);
		assert.match(searchClientSource, /matchWholeWord: textOptions\.matchWholeWord/);
		assert.match(searchClientSource, /useRegex: textOptions\.useRegex/);
		assert.match(searchClientSource, /textOptions\.matchCase.*textOptions\.matchWholeWord.*textOptions\.useRegex/);
		assert.match(searchClientSource, /search_game_data_text/);
		assert.match(searchClientSource, /search_workspace_text/);
	});
});
