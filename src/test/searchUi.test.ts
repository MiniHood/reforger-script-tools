import * as assert from 'node:assert';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { searchLimits } from '../extensionConfig/search';
import { addonScopeLabel, formatSearchKind, maxSearchPages, normalizeResourceSearchPage, normalizeSearchPage, resourceKindsFor, searchKindFilters, searchResourceKindFilters, searchToolFor, sourceContextPreview, sourceLinePreview, sourceMatchRange, sourcePreviewLine, stripSourceComments } from '../searchPrototype/mcpSearchClient';
import { semanticPreviewForLine, semanticPreviewForLines, semanticTokenSpansForLine } from '../searchPrototype/semanticPreview';

const searchUiSource = fs.readFileSync(
	path.join(__dirname, '../../src/searchPrototype/searchUiPrototype.ts'),
	'utf8',
);
const searchClientSource = fs.readFileSync(
	path.join(__dirname, '../../src/searchPrototype/mcpSearchClient.ts'),
	'utf8',
);

suite('Reforger search UI MCP mapping', () => {
	test('maps canonical Workbench resources and exposes their fixed kind filters', () => {
		assert.deepStrictEqual(resourceKindsFor('audio'), ['audio']);
		assert.deepStrictEqual(resourceKindsFor('texture'), ['texture', 'imageset']);
		assert.ok(resourceKindsFor('all').includes('prefab'));
		assert.ok(searchResourceKindFilters.some(filter => filter.value === 'script'));
		assert.deepStrictEqual(normalizeResourceSearchPage({
			results: [{
				resourceName: '{58D0FB3206B6F859}Prefabs/Props/Radio.et',
				addonGuid: '58D0FB3206B6F859',
				addonId: 'ArmaReforger',
				logicalPath: 'Prefabs/Props/Radio.et',
				name: 'Radio',
				extension: 'et',
			}],
		}), [{
			id: 'workbench-resource-0-{58D0FB3206B6F859}Prefabs/Props/Radio.et',
			source: 'workbench',
			kind: 'resource',
			title: 'Radio',
			detail: 'et',
			path: 'Prefabs/Props/Radio.et',
			excerpt: '{58D0FB3206B6F859}Prefabs/Props/Radio.et',
			matchKind: 'resource',
			readInput: {},
			resourceName: '{58D0FB3206B6F859}Prefabs/Props/Radio.et',
			addonGuid: '58D0FB3206B6F859',
			addonLabel: 'ArmaReforger',
		}]);
	});

	test('offers a resource search mode backed by the canonical Workbench tools', () => {
		assert.match(searchUiSource, /data-mode="resource">Resources/);
		assert.match(searchUiSource, /resourceResultTypes/);
		assert.match(searchUiSource, /resourceKindsFor\(typeValue\)/);
		assert.match(searchUiSource, /hit\.kind === 'resource'/);
		assert.match(searchClientSource, /workbench_search_resources/);
		assert.match(searchUiSource, /enfusion:\/\/\$\{resourceName\}/);
	});

	test('supersedes an in-flight search when a mode or type filter changes', () => {
		assert.match(searchUiSource, /function cancelInFlightSearch\(active: ActiveSearch\): void/);
		assert.match(searchUiSource, /cancelInFlightSearch\(active\);/);
		assert.match(searchUiSource, /active\.client = undefined;/);
		assert.match(searchUiSource, /previousClient\.then\(client => client\.dispose\(\)/);
		assert.match(searchUiSource, /state\.page = 1; render\(false\); search\(true\);/);
	});

	test('refreshes a text cursor once when its source revision changes between pages', () => {
		assert.match(searchClientSource, /function isInvalidTextCursor\(error: unknown\): boolean/);
		assert.match(searchClientSource, /pages\.clear\(\);[\s\S]*?return this\.searchPage\(/);
		assert.match(searchClientSource, /retryInvalidCursor = true/);
		assert.match(searchClientSource, /cursor is invalid for this text query or source revision/);
	});

	test('shows one human-facing add-on name in Search Scope', () => {
		assert.strictEqual(
			addonScopeLabel('GlobalConflictsCore (Global Conflicts CORE)', 'GlobalConflictsCore', '623555110E2B2CA0'),
			'Global Conflicts CORE',
		);
		assert.strictEqual(
			addonScopeLabel('ACE_Captives_Dev (ACE Captives Dev)', 'ACE Captives Dev', '0000000000000000'),
			'ACE Captives Dev',
		);
		assert.strictEqual(
			addonScopeLabel('core (Enfusion core data)', 'Enfusion core data', '0000000000000000'),
			'Enfusion core data',
		);
		assert.strictEqual(addonScopeLabel('Arma Reforger', 'ArmaReforger', '58D0FB3206B6F859'), 'Arma Reforger');
	});

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
				excerptMatchStart: 17,
				excerpt: '    void Run() { SCR_(); }',
				matchText: 'SCR_',
				sourceUri: 'file:///C:/Addons/Example/Scripts/Example.c',
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
		assert.strictEqual(results[0].sourceUri, 'file:///C:/Addons/Example/Scripts/Example.c');
		assert.strictEqual(results[0].readInput.catalogueRevision, 'ws1:revision');
	});

	test('queues script text matches for asynchronous semantic coloring', () => {
		assert.match(searchUiSource, /const semanticHits = previewHits\.filter\(hit => hit\.source !== 'wiki'\)/);
		assert.match(searchUiSource, /length: Math\.min\(4, semanticHits\.length\)/);
		assert.match(searchUiSource, /const rawPreview = \{ hit, document, previewLine, preview, matchRange \};[\s\S]*?queueRawPreview\(hit\.id\);[\s\S]*?queueSemanticPreview\(rawPreview\);/);
		assert.match(searchUiSource, /else if \(hit\.kind === 'text'\) \{\s*return undefined;/);
		assert.match(searchUiSource, /state\.sourcePreviews\[result\.id\] \?\? \(result\.kind === 'text' \? result\.excerpt : undefined\)/);
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
		assert.strictEqual(results[0].textMatchStart, 9);
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

	test('renders a configurable source context around the selected preview line', () => {
		const document = { content: 'one\n  two\n\nthree\nfour\nfive', startLine: 1, endLine: 6 };
		assert.strictEqual(sourceContextPreview(document, 4, 1), 'three');
		assert.strictEqual(sourceContextPreview(document, 4, 2), '  two\n\nthree\nfour\nfive');
		assert.match(searchUiSource, /data-preview-context type="number" min="1" max="249" value="' \+ previewContextLines/);
		assert.match(searchUiSource, /type: 'previewContext', contextLines: previewContextLines/);
		assert.match(searchUiSource, /clearTimeout\(previewContextTimer\);[\s\S]*?setTimeout\(\(\) => vscode\.postMessage\(\{ type: 'previewContext'/);
		assert.match(searchUiSource, /previewContextLines: numberField\(snapshot\.previewContextLines\)/);
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
		assert.match(searchUiSource, /startPreviewHydration\(active, client, requestId, result\.results, normalizedQuery\)/);
		assert.match(searchUiSource, /const matchRange = sourceMatchRange\(preview, query\);/);
	});

	test('decodes the language server semantic token legend for a preview line', () => {
		assert.deepStrictEqual(semanticTokenSpansForLine([
			0, 4, 5, 0, 0,
			1, 0, 8, 7, 0,
		], 1), [{ start: 0, length: 8, role: 'enumMember' }]);
	});

	test('keeps semantic token offsets across a multi-line preview', () => {
		assert.match(searchUiSource, /semanticPreviewForLines\(/);
		assert.match(searchUiSource, /active\.previewContextLines === 1[\s\S]*?semanticPreviewForLine/);
		assert.match(searchUiSource, /line - active\.previewContextLines/);
		assert.match(searchUiSource, /line \+ active\.previewContextLines/);
		assert.strictEqual(typeof semanticPreviewForLine, 'function');
		assert.strictEqual(typeof semanticPreviewForLines, 'function');
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
		assert.doesNotMatch(searchUiSource, /Search the indexed workspace, loaded add-ons/);
		assert.doesNotMatch(searchUiSource, /class="intro"/);
		assert.match(searchUiSource, /const modeButtons = \(\) =>/);
		assert.match(searchUiSource, /state\.mode === 'text'/);
		assert.match(searchUiSource, /data-mode="semantic"/);
		assert.match(searchUiSource, /data-mode="text"/);
		assert.match(searchUiSource, /state\.mode === 'resource' \? resourceResultTypes : resultTypes/);
		assert.match(searchUiSource, /query\.addEventListener\('input', event => \{ state\.query = event\.target\.value; pendingQuerySelection = \{ start: event\.target\.selectionStart \?\? state\.query\.length, end: event\.target\.selectionEnd \?\? state\.query\.length \}; if \(state\.mode !== 'text'\) search\(true\); \}\)/);
		assert.doesNotMatch(searchUiSource, /searchTimer|scheduleSearch/);
		assert.match(searchUiSource, /searchMode: state\.mode/);
		assert.match(searchClientSource, /search_workspace_text/);
		assert.match(searchClientSource, /search_game_data_text/);
		assert.match(searchClientSource, /paginationMode: paginationModeFor\(mode, sources\)/);
		assert.match(searchClientSource, /mode === 'semantic'[\s\S]*sources\.filter\(source => source !== 'wiki'\)/);
		assert.match(searchUiSource, /state\.scopeSources\.filter\(source => source\.kind !== 'wiki' \|\| state\.mode === 'text'\)/);
		assert.match(searchUiSource, /selectedEligibleScopeIds\(\)/);
	});

	test('focuses the search field when reopening the panel and routes unclaimed typing to it', () => {
		assert.match(searchUiSource, /activePanel\.webview\.postMessage\(\{ type: 'focusQuery' \}\)/);
		assert.match(searchUiSource, /message\.type === 'focusQuery'\) \{ document\.getElementById\('query'\)\?\.focus\(\); return; \}/);
		assert.match(searchUiSource, /event\.data\?\.type !== 'focusQuery'[\s\S]*?query\.setSelectionRange\(query\.value\.length, query\.value\.length\)/);
		assert.match(searchUiSource, /document\.activeElement !== document\.body \|\| event\.ctrlKey \|\| event\.altKey \|\| event\.metaKey \|\| event\.isComposing \|\| event\.key\.length !== 1/);
		assert.match(searchUiSource, /query\.setRangeText\(event\.key, query\.selectionStart, query\.selectionEnd, 'end'\);/);
		assert.match(searchUiSource, /query\.dispatchEvent\(new Event\('input', \{ bubbles: true \}\)\);/);
	});

	test('preserves a query selection when an asynchronous search response rerenders the page', () => {
		assert.match(searchUiSource, /function render\(focusQuery = false\)/);
		assert.match(searchUiSource, /\nrender\(true\);\n<\/script>/);
		assert.match(searchUiSource, /pendingQuerySelection = \{ start: event\.target\.selectionStart \?\? state\.query\.length, end: event\.target\.selectionEnd \?\? state\.query\.length \}/);
		assert.match(searchUiSource, /else if \(pendingQuerySelection\) \{ query\.focus\(\); query\.setSelectionRange\(pendingQuerySelection\.start, pendingQuerySelection\.end\); pendingQuerySelection = undefined; \}/);
		assert.match(searchUiSource, /query\.focus\(\);\s+query\.setSelectionRange\(query\.value\.length, query\.value\.length\);\s+query\.setRangeText\(event\.key/);
	});

	test('removes the result-limit status after a search completes', () => {
		assert.match(searchUiSource, /state\.status !== 'loading'[\s\S]*?\.source-header > div:first-child > \.muted'\)\?\.remove\(\)/);
	});

	test('focuses and preserves the selection of the selected-source filter', () => {
		assert.match(searchUiSource, /if \(state\.scopeOpen\) focusScopeFilter\(state\.scopeFilter\.length\);/);
		assert.match(searchUiSource, /const selectionStart = element\.selectionStart \?\? element\.value\.length;[\s\S]*?focusScopeFilter\(selectionStart, selectionEnd\);/);
		assert.doesNotMatch(searchUiSource, /filter\.setSelectionRange\(filter\.value\.length, filter\.value\.length\)/);
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
		assert.doesNotMatch(searchUiSource, /data-scope-refresh/);
		assert.match(searchUiSource, /async function refreshSearchScope/);
		assert.doesNotMatch(searchUiSource, /type: 'refreshScope'/);
		assert.match(searchUiSource, /scopeRefresh: Promise<void> \| undefined/);
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
		assert.match(searchUiSource, /<button class="addon-trigger"[\s\S]*?<div class="addon-chips">/);
		assert.match(searchUiSource, /\.addon-chips \{[^}]*margin-top: 6px;/);
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
		assert.match(searchUiSource, /<div class="atlas-filter-strip"><div class="control-block scope-control"><div class="group-label">SEARCH SCOPE<\/div>' \+ searchScope\(\) \+ '<\/div><div class="control-block atlas-query"><div class="group-label">SEARCH<\/div>' \+ queryField\(\) \+ textSearchOptions\(\) \+ '<\/div><div class="atlas-secondary-controls"><div class="control-block mode-control"><div class="group-label">SEARCH MODE<\/div>' \+ modeControls\(\)/);
	});

	test('promotes the corrected source atlas as the only Search presentation', () => {
		assert.match(searchUiSource, /function renderSearchUi\(webview: vscode\.Webview\): string/);
		assert.match(searchUiSource, /class="shell search-atlas"/);
		assert.doesNotMatch(searchUiSource, /prototypeSwitcher|prototypeVariants|pageRenderers|renderFocusVariant|renderInspectorVariant|renderLedgerVariant|renderConsoleVariant/);
		assert.match(searchUiSource, /\.atlas-filter-strip \{ display: grid; grid-template-columns: 180px minmax\(240px, 1fr\); grid-template-areas: "scope query" "secondary secondary"; align-items: start;/);
		assert.match(searchUiSource, /\.control-block \.group-label \{ min-height: 22px; padding: 0 0 7px;/);
		assert.match(searchUiSource, /\.atlas-filter-strip \.addon-trigger \{ width: 100%; min-height: 38px;/);
		assert.match(searchUiSource, /\.atlas-query \{ grid-area: query; min-width: 240px;/);
		assert.match(searchUiSource, /\.atlas-secondary-controls \{ grid-area: secondary; display: flex; align-items: start;/);
		assert.match(searchUiSource, /const typeControl = state\.mode === 'text' \? '' : '<div class="control-block type-control">/);
		assert.match(searchUiSource, /class="control-block scope-control"[\s\S]*?class="control-block atlas-query"[\s\S]*?class="atlas-secondary-controls"[\s\S]*?class="control-block mode-control"[\s\S]*?' \+ typeControl \+ '/);
		assert.match(searchUiSource, /\.atlas-card \{[^}]*border-left: 3px solid var\(--accent\);/);
		assert.match(searchUiSource, /const resultGroups = \(\) => \{/);
		assert.match(searchUiSource, /visibleResults\(\)\.forEach\(result => \{/);
		assert.match(searchUiSource, /<section class="atlas-group"><div class="atlas-group-head"><h2>/);
		assert.match(searchUiSource, /results\.map\(resultCard\)\.join\(''\)/);
		assert.match(searchUiSource, /\.atlas-results\.two-column \{ grid-template-columns: repeat\(2, minmax\(0, 1fr\)\);/);
		assert.match(searchUiSource, /resultBody\(resultGroups\(\)\)/);
		assert.doesNotMatch(searchUiSource, /atlas-lanes|atlas-lane/);
	});

	test('keeps opening and closing Search Scope presentation-only', () => {
		assert.doesNotMatch(searchUiSource, /type: 'refreshScope'/);
		assert.match(searchUiSource, /const scopeSelectionChanged =/);
		assert.match(searchUiSource, /const scopeRevisionChanged =/);
		assert.match(searchUiSource, /const scopeSearchChanged = scopeSelectionChanged \|\| scopeRevisionChanged/);
		assert.match(searchUiSource, /if \(message\.refreshSearch === true && scopeSearchChanged && state\.query\.trim\(\)\) search\(true\)/);
		assert.match(searchUiSource, /affectsConfiguration[\s\S]*?refreshSearchScope\(context, active\)/);
		assert.match(searchUiSource, /\[data-scope-open\][\s\S]*?state\.scopeOpen = !state\.scopeOpen; render\(false\)/);
		assert.doesNotMatch(searchUiSource, /render\(false\); if \(state\.query\.trim\(\)\) search\(true\); return;/);
	});

	test('starts the custom Search MCP process with the configured external index mode', () => {
		assert.match(searchClientSource, /externalIndexMode: ExternalIndexMode/);
		assert.match(searchClientSource, /buildMcpLaunchConfiguration\(this\.options\)/);
		assert.doesNotMatch(searchClientSource, /'--external-index-mode'/);
		assert.match(searchUiSource, /externalIndexMode: readExternalIndexMode\(\)/);
		assert.match(searchUiSource, /dependencyProjectFiles: await discoverWorkspaceProjectFiles\(\)/);
		assert.doesNotMatch(searchClientSource, /dependencyProjectFiles\.flatMap/);
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

	test('preserves add-on identity while showing one display name in game-data rows', () => {
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
				addonLabel: 'ExampleAddon (Example Add-on)',
				sourceUri: 'file:///C:/Addons/Example/Scripts/SCR_AddonClass.c',
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
		assert.strictEqual(results[0].sourceUri, 'file:///C:/Addons/Example/Scripts/SCR_AddonClass.c');
		assert.strictEqual(results[0].readInput.addonGuid, 'A1B2C3D4E5F60718');
		assert.match(results[0].id, /A1B2C3D4E5F60718/);
	});

	test('keeps the full result path above a full-width preview', () => {
		assert.match(searchUiSource, /\.atlas-card-head \{ display: flex; justify-content: space-between;/);
		assert.match(searchUiSource, /\.result-path \{[^}]*overflow-wrap: anywhere;/);
		assert.match(searchUiSource, /\.atlas-card \.result-path \{[^}]*max-width: none;[^}]*text-align: left;/);
		assert.match(searchUiSource, /<div class="atlas-card-head"><strong>[\s\S]*?<div class="result-path">[\s\S]*?resultPreview\(result\)/);
		assert.match(searchUiSource, /const highlightText = \(value, query\) =>/);
		assert.match(searchUiSource, /<mark>/);
		assert.match(searchUiSource, /highlightRange\(sourceText, matchRange\)/);
		assert.match(searchUiSource, /state\.sourcePreviews\[result\.id\]/);
		assert.doesNotMatch(searchUiSource, /return highlightRange\(result\.excerpt, matchRange\)/);
		assert.match(searchUiSource, /state\.matchRanges = \{ \.\.\.state\.matchRanges/);
		assert.match(searchUiSource, /message\.type === 'previews'/);
		assert.match(searchUiSource, /hydrateSearchPreviews/);
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
		assert.match(searchUiSource, /\.atlas-results\.two-column \{ grid-template-columns: repeat\(2, minmax\(0, 1fr\)\);/);
		assert.match(searchUiSource, /\.page-controls \[data-result-layout\] \{ display: inline-flex; align-items: center; justify-content: center; padding: 0; line-height: 0; \}/);
		assert.match(searchUiSource, /const pageControls = \(includeLayoutToggle = false\) =>/);
		assert.match(searchUiSource, /return '<div class="page-controls" aria-label="Search result pages">' \+ previewControl \+ layoutToggle \+ '<select data-page-size/);
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
		const semanticPhase = searchUiSource.indexOf('const hydrateSemanticPreview = async');
		const semanticMessage = searchUiSource.indexOf("type: 'semanticPreviews'", rawMessage);
		const semanticQueued = searchUiSource.indexOf('queueSemanticPreview(rawPreview)');
		const rawWorkersComplete = searchUiSource.indexOf('await Promise.all(Array.from({ length: Math.min(sourcePreviewWorkerCount');
		assert.ok(rawPhase >= 0);
		assert.ok(rawMessage > rawPhase);
		assert.ok(semanticMessage > rawMessage);
		assert.ok(semanticPhase > semanticMessage);
		assert.ok(semanticQueued > semanticPhase);
		assert.ok(rawWorkersComplete > semanticQueued);
		assert.match(searchUiSource, /queueRawPreview\(hit\.id\)/);
		assert.match(searchUiSource, /flushRawPreviews\(\);/);
		assert.match(searchUiSource, /active\.previewCancellation\?\.cancel\(\)/);
		assert.match(searchUiSource, /firstSemanticMs/);
		assert.doesNotMatch(searchUiSource, /pendingUpdates/);
		assert.match(searchUiSource, /searchUi\.previewRawCompleted/);
		assert.match(searchUiSource, /phase: 'raw'/);
		assert.match(searchUiSource, /phase: 'semantic'/);
		assert.match(searchUiSource, /lastSemanticMessageMs/);
		assert.match(searchUiSource, /state\.previewPerformance = message\.performance \?\? state\.previewPerformance/);
		assert.match(searchUiSource, /data-result-preview="' \+ esc\(result\.id\) \+ '"/);
		assert.match(searchUiSource, /const updateResultPreviews = ids =>/);
		assert.match(searchUiSource, /state\.matchRanges = \{ \.\.\.state\.matchRanges, \.\.\.\(message\.matches \?\? \{\}\) \}; updateResultPreviews\(Object\.keys\(message\.previews \?\? \{\}\)\);/);
		assert.match(searchUiSource, /state\.semanticPreviews = \{ \.\.\.state\.semanticPreviews, \.\.\.\(message\.previews \?\? \{\}\) \}; updateResultPreviews\(Object\.keys\(message\.previews \?\? \{\}\)\);/);
		assert.doesNotMatch(searchUiSource, /state\.matchRanges = \{ \.\.\.state\.matchRanges, \.\.\.\(message\.matches \?\? \{\}\) \}; render\(\);/);
		assert.doesNotMatch(searchUiSource, /state\.semanticPreviews = \{ \.\.\.state\.semanticPreviews, \.\.\.\(message\.previews \?\? \{\}\) \}; render\(\);/);
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
		assert.match(searchUiSource, /state\.type = element\.dataset\.type; state\.page = 1; render\(false\); search\(true\)/);
		assert.match(searchUiSource, /resultType: state\.type/);
		assert.match(searchUiSource, /message\.resultType/);
		assert.match(searchUiSource, /const resultTypes = \$\{JSON\.stringify\(searchKindFilters\.map\(\(\{ value, label \}\) => \(\{ value, label \}\)\)\)\};/);
		assert.match(searchUiSource, /searchKindsFor\(typeValue\)/);
		assert.match(searchUiSource, /isSearchKindValue\(message\.resultType\) && !isSearchResourceKindValue\(message\.resultType\)/);
		assert.match(searchClientSource, /const sourcePageSize = 100;/);
		assert.match(searchUiSource, /const sourcePreviewWorkerCount = 8;/);
		assert.match(searchUiSource, /Math\.min\(sourcePreviewWorkerCount, previewHits\.length\)/);
		assert.match(searchUiSource, /const previewUpdateBatchSize = 4;/);
		assert.match(searchUiSource, /pendingRawPreviewIds\.length >= previewUpdateBatchSize/);
		assert.match(searchUiSource, /pendingSemanticPreviewIds\.length >= previewUpdateBatchSize/);
		assert.match(searchUiSource, /previews: Object\.fromEntries\(ids\.map\(id => \[id, semanticPreviews\[id\]\]\)\)/);
		assert.doesNotMatch(searchUiSource, /setTimeout\(\(\) => search\(true\), \d+\)/);
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
		assert.match(searchUiSource, /'Page ' \+ state\.page \+ ' of ' \+ totalPages\(\)/);
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
