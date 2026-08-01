import * as path from 'node:path';
import * as vscode from 'vscode';
import { resolveBaseGameIndexCache } from '../gameData/baseGameIndexCache';
import { searchCommands, searchContext } from '../extensionConfig/search';
import { discoverWorkspaceScriptRoots } from '../languageClient/workspaceWatchBridge';
import { resolveLanguageServerPath } from '../languageClient/serverPath';
import {
	McpSearchClient,
	type SearchDocument,
	type SearchHit,
	type SearchSource,
} from './mcpSearchClient';

const searchScheme = 'reforger-search';
const maxSearchDocuments = 32;
let activePanel: vscode.WebviewPanel | undefined;
let documentSequence = 0;
const searchDocuments = new Map<string, string>();

interface ActiveSearch {
	panel: vscode.WebviewPanel;
	client: Promise<McpSearchClient> | undefined;
	latestResults: Map<string, SearchHit>;
	requestSequence: number;
	disposed: boolean;
}

export function registerSearchUi(context: vscode.ExtensionContext): void {
	void vscode.commands.executeCommand('setContext', searchContext.key, true);
	context.subscriptions.push(
		vscode.workspace.registerTextDocumentContentProvider(searchScheme, {
			provideTextDocumentContent: uri => {
				const key = uri.path.split('/')[1];
				return searchDocuments.get(key) ?? '';
			},
		}),
		vscode.commands.registerCommand(searchCommands.open, () => openSearchPanel(context)),
		new vscode.Disposable(() => {
			for (const key of searchDocuments.keys()) {
				searchDocuments.delete(key);
			}
		}),
	);
}

function openSearchPanel(context: vscode.ExtensionContext): void {
	if (activePanel) {
		activePanel.reveal(vscode.ViewColumn.One);
		return;
	}

	const panel = vscode.window.createWebviewPanel(
		'reforgerSearchUi',
		'Reforger Search',
		vscode.ViewColumn.One,
		{
			enableScripts: true,
			retainContextWhenHidden: true,
		},
	);
	const active: ActiveSearch = {
		panel,
		client: undefined,
		latestResults: new Map(),
		requestSequence: 0,
		disposed: false,
	};
	activePanel = panel;
	panel.webview.html = renderSearchUi(panel.webview);
	panel.webview.onDidReceiveMessage(message => {
		void handleMessage(context, active, message);
	}, undefined, context.subscriptions);
	panel.onDidDispose(() => {
		active.disposed = true;
		if (active.client) {
			void active.client.then(client => client.dispose(), () => undefined);
		}
		if (activePanel === panel) {
			activePanel = undefined;
		}
	}, undefined, context.subscriptions);
}

async function handleMessage(
	context: vscode.ExtensionContext,
	active: ActiveSearch,
	message: unknown,
): Promise<void> {
	if (!isRecord(message) || typeof message.type !== 'string' || active.disposed) {
		return;
	}
	if (message.type === 'search' && typeof message.query === 'string') {
		await runSearch(context, active, message.query, message.source);
		return;
	}
	if (message.type === 'open' && typeof message.id === 'string') {
		await openSearchResult(active, message.id);
		return;
	}
	if (message.type === 'external' && typeof message.id === 'string') {
		const hit = active.latestResults.get(message.id);
		if (hit?.sourceUrl) {
			await vscode.env.openExternal(vscode.Uri.parse(hit.sourceUrl));
		}
	}
}

async function runSearch(
	context: vscode.ExtensionContext,
	active: ActiveSearch,
	query: string,
	sourceValue: unknown,
): Promise<void> {
	const normalizedQuery = query.trim();
	const requestId = ++active.requestSequence;
	if (!normalizedQuery) {
		active.latestResults.clear();
		active.panel.webview.postMessage({ type: 'results', requestId, results: [], warnings: [] });
		return;
	}

	active.panel.webview.postMessage({ type: 'loading', requestId });
	try {
		const client = await getClient(context, active);
		const result = await client.search(normalizedQuery, sourcesFor(sourceValue));
		if (active.disposed || requestId !== active.requestSequence) {
			return;
		}
		active.latestResults = new Map(result.results.map(hit => [hit.id, hit]));
		active.panel.webview.postMessage({
			type: 'results',
			requestId,
			results: result.results,
			warnings: result.warnings,
		});
	} catch (error) {
		if (!active.disposed && requestId === active.requestSequence) {
			active.panel.webview.postMessage({
				type: 'error',
				requestId,
				message: error instanceof Error ? error.message : String(error),
			});
		}
	}
}

async function getClient(
	context: vscode.ExtensionContext,
	active: ActiveSearch,
): Promise<McpSearchClient> {
	if (!active.client) {
		active.client = createClient(context).catch(error => {
			active.client = undefined;
			throw error;
		});
	}
	return active.client;
}

async function createClient(context: vscode.ExtensionContext): Promise<McpSearchClient> {
	const serverPath = await resolveLanguageServerPath(context);
	if (!serverPath) {
		throw new Error('The bundled Reforger language server is not available yet.');
	}
	return new McpSearchClient({
		serverPath,
		indexCache: await resolveBaseGameIndexCache(context.globalStorageUri.fsPath),
		workspaceScripts: await discoverWorkspaceScriptRoots(),
		officialWikiRoot: path.join(context.extensionPath, 'data', 'official-wiki'),
	});
}

async function openSearchResult(active: ActiveSearch, id: string): Promise<void> {
	const hit = active.latestResults.get(id);
	if (!hit) {
		return;
	}
	try {
		const client = await getClientFromActive(active);
		const document = await client.read(hit);
		const key = `document-${++documentSequence}`;
		while (searchDocuments.size >= maxSearchDocuments) {
			const oldest = searchDocuments.keys().next().value;
			if (oldest === undefined) {
				break;
			}
			searchDocuments.delete(oldest);
		}
		searchDocuments.set(key, document.content);
		const uri = vscode.Uri.parse(
			`${searchScheme}:/${key}/${encodeURIComponent(hit.path)}`,
		);
		const opened = await vscode.workspace.openTextDocument(uri);
		const language = hit.source === 'wiki' ? 'markdown' : 'enforce';
		const documentWithLanguage = await vscode.languages.setTextDocumentLanguage(opened, language);
		const editor = await vscode.window.showTextDocument(documentWithLanguage);
		const startLine = Math.max(0, document.startLine - 1);
		const endLine = Math.max(startLine, document.endLine - 1);
		editor.revealRange(
			new vscode.Range(startLine, 0, endLine, 0),
			vscode.TextEditorRevealType.InCenterIfOutsideViewport,
		);
	} catch (error) {
		await vscode.window.showErrorMessage(
			`Could not open ${hit.title}: ${error instanceof Error ? error.message : String(error)}`,
		);
	}
}

async function getClientFromActive(active: ActiveSearch): Promise<McpSearchClient> {
	if (!active.client) {
		throw new Error('The search session is not ready.');
	}
	return active.client;
}

function sourcesFor(value: unknown): SearchSource[] {
	switch (value) {
		case 'workspace':
			return ['workspace'];
		case 'gameData':
			return ['gameData'];
		case 'wiki':
			return ['wiki'];
		default:
			return ['workspace', 'gameData', 'wiki'];
	}
}

function renderSearchUi(webview: vscode.Webview): string {
	const nonce = createNonce();
	return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<title>Reforger Search</title>
<style>
:root { color-scheme: dark; --bg: var(--vscode-editor-background); --panel: var(--vscode-sideBar-background); --alt: var(--vscode-editorWidget-background); --border: var(--vscode-panel-border); --text: var(--vscode-foreground); --muted: var(--vscode-descriptionForeground); --accent: var(--vscode-textLink-foreground); --selected: var(--vscode-list-activeSelectionBackground); --selected-text: var(--vscode-list-activeSelectionForeground); }
* { box-sizing: border-box; }
body { margin: 0; color: var(--text); background: var(--bg); font: 13px var(--vscode-font-family); }
button, input, select { font: inherit; color: inherit; }
button { border: 1px solid var(--border); background: var(--alt); cursor: pointer; }
button:hover { border-color: var(--accent); }
.shell { min-height: 100vh; padding: 24px 28px 70px; }
.eyebrow { color: var(--accent); font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
h1, h2, h3, p { margin-top: 0; }
h1 { font-size: 24px; margin: 6px 0 8px; }
h2 { font-size: 16px; margin-bottom: 6px; }
h3 { font-size: 13px; margin: 0 0 4px; }
.intro { max-width: 780px; color: var(--muted); line-height: 1.5; margin-bottom: 20px; }
.toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 14px; }
.toolbar input { flex: 1; min-width: 160px; border: 1px solid var(--border); background: var(--alt); padding: 9px 11px; outline: none; }
.toolbar input:focus, .toolbar select:focus { border-color: var(--accent); }
.toolbar select { border: 1px solid var(--border); background: var(--alt); padding: 8px 9px; }
.search-button { padding: 8px 12px; color: var(--selected-text); background: var(--selected); border-color: transparent; }
.layout { display: grid; grid-template-columns: 170px 1fr; gap: 18px; }
.source-rail { height: fit-content; border: 1px solid var(--border); padding: 10px; background: var(--panel); }
.group-label { padding: 0 4px 6px; color: var(--muted); font-size: 11px; font-weight: 700; letter-spacing: .07em; }
.source-rail button { display: block; width: 100%; margin: 3px 0; padding: 9px 10px; text-align: left; border: 0; background: transparent; }
.source-rail button.active { background: var(--selected); color: var(--selected-text); }
.source-header { display: flex; justify-content: space-between; align-items: end; border-bottom: 1px solid var(--border); padding-bottom: 10px; }
.muted { color: var(--muted); }
.tag { border-radius: 12px; padding: 3px 8px; background: var(--alt); color: var(--muted); font-size: 11px; }
.source-rows { display: grid; gap: 10px; margin-top: 12px; }
.source-row { display: grid; grid-template-columns: 28px 1fr auto; gap: 12px; align-items: start; padding: 13px; border: 1px solid var(--border); background: var(--panel); }
.source-row.selected { border-color: var(--accent); }
.source-icon { color: var(--accent); font-weight: 700; text-align: center; }
.result-detail, .result-path { display: block; color: var(--muted); font-size: 12px; }
.result-path { max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.snippet { margin: 9px 0 0; padding: 10px; overflow: auto; background: var(--alt); border: 1px solid var(--border); font: 12px/1.5 var(--vscode-editor-font-family); white-space: pre-wrap; }
.result-actions { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
.result-actions button { padding: 5px 8px; }
.open { color: var(--selected-text); background: var(--selected); border-color: transparent !important; }
.status { margin: 12px 0; color: var(--muted); }
.error { padding: 10px 12px; border: 1px solid var(--vscode-inputValidation-errorBorder); color: var(--vscode-errorForeground); background: var(--vscode-inputValidation-errorBackground); }
.warning { padding: 8px 10px; border-left: 2px solid var(--vscode-editorWarning-foreground); color: var(--muted); }
.empty { padding: 30px 14px; border: 1px dashed var(--border); color: var(--muted); }
@media (max-width: 720px) { .shell { padding: 18px 14px 60px; } .layout { grid-template-columns: 1fr; } .toolbar { flex-wrap: wrap; } .toolbar input { flex-basis: 100%; } .source-row { grid-template-columns: 26px 1fr; } .result-path { grid-column: 2; max-width: none; } }
</style>
</head>
<body>
<main id="app"></main>
<script nonce="${nonce}">
const vscode = acquireVsCodeApi();
const state = { query: '', source: 'all', type: 'all', results: [], warnings: [], status: 'idle', error: '', requestId: 0, selected: '' };
const sources = [
  { value: 'all', label: 'All sources' },
  { value: 'workspace', label: 'Workspace' },
  { value: 'gameData', label: 'Game Data' },
  { value: 'wiki', label: 'Official Wiki' },
];
const esc = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const sourceLabel = value => sources.find(source => source.value === value)?.label ?? value;
const visibleResults = () => state.results.filter(result => state.type === 'all' || result.kind === state.type);
const sourceButtons = () => sources.map(source => '<button class="' + (state.source === source.value ? 'active' : '') + '" data-source="' + esc(source.value) + '">' + esc(source.label) + '</button>').join('');
const typeOptions = () => [['all', 'All result types'], ['symbol', 'Symbols'], ['documentation', 'Documentation']].map(([value, label]) => '<option value="' + value + '">' + label + '</option>').join('');
const resultRows = () => visibleResults().map(result => {
  const selected = state.selected === result.id;
  const external = result.sourceUrl ? '<button data-external="' + esc(result.id) + '">Open official page</button>' : '';
  return '<article class="source-row ' + (selected ? 'selected' : '') + '"><div class="source-icon">' + (result.kind === 'documentation' ? 'W' : 'S') + '</div><div><h3>' + esc(result.title) + '</h3><div class="result-detail">' + esc(result.detail) + ' · ' + esc(sourceLabel(result.source)) + '</div><pre class="snippet">' + esc(result.excerpt) + '</pre><div class="result-actions"><button class="open" data-open="' + esc(result.id) + '">Open source</button>' + external + '</div></div><div class="result-path">' + esc(result.path) + '</div></article>';
}).join('');
function render() {
  const results = visibleResults();
  const body = state.error ? '<div class="error">' + esc(state.error) + '</div>' : results.length ? '<div class="source-rows">' + resultRows() + '</div>' : '<div class="empty">' + (state.status === 'idle' ? 'Enter a symbol, concept, or documentation term to search.' : 'No results match this search.') + '</div>';
  const warnings = state.warnings.map(warning => '<div class="warning">' + esc(warning) + '</div>').join('');
  document.getElementById('app').innerHTML = '<div class="shell"><div class="eyebrow">Source browser · live MCP search</div><h1>Find usage in Reforger</h1><p class="intro">Search the indexed workspace, shipped Game Data, and Official Wiki together. Select a result to open the exact bounded source passage or documentation returned by the authoritative search API.</p><div class="toolbar"><input id="query" value="' + esc(state.query) + '" placeholder="Search a symbol, concept, or phrase..." aria-label="Search query"><select id="type" aria-label="Result type">' + typeOptions() + '</select><button class="search-button" id="search">Search</button></div><div class="layout"><aside class="source-rail"><div class="group-label">SEARCH IN</div>' + sourceButtons() + '</aside><section><div class="source-header"><div><h2>' + results.length + ' matches</h2><span class="muted">' + (state.status === 'loading' ? 'Searching...' : 'Showing up to 12 results per source') + '</span></div><span class="tag">' + (state.status === 'loading' ? 'loading' : 'ready') + '</span></div><div class="status">' + (state.status === 'error' ? 'Search failed' : '') + '</div>' + warnings + body + '</section></div></div>';
  const query = document.getElementById('query');
  query.focus();
  query.setSelectionRange(state.query.length, state.query.length);
  document.getElementById('type').value = state.type;
  query.addEventListener('input', event => { state.query = event.target.value; scheduleSearch(); });
  query.addEventListener('keydown', event => { if (event.key === 'Enter') { event.preventDefault(); search(); } });
  document.getElementById('search').addEventListener('click', search);
  document.getElementById('type').addEventListener('change', event => { state.type = event.target.value; render(); });
  document.querySelectorAll('[data-source]').forEach(element => element.addEventListener('click', () => { state.source = element.dataset.source; search(); }));
  document.querySelectorAll('[data-open]').forEach(element => element.addEventListener('click', event => { event.stopPropagation(); state.selected = element.dataset.open; vscode.postMessage({ type: 'open', id: element.dataset.open }); render(); }));
  document.querySelectorAll('[data-external]').forEach(element => element.addEventListener('click', event => { event.stopPropagation(); vscode.postMessage({ type: 'external', id: element.dataset.external }); }));
}
let searchTimer;
function scheduleSearch() { clearTimeout(searchTimer); searchTimer = setTimeout(search, 260); }
function search() { state.error = ''; state.warnings = []; state.status = state.query.trim() ? 'loading' : 'idle'; state.selected = ''; render(); vscode.postMessage({ type: 'search', query: state.query, source: state.source }); }
window.addEventListener('message', event => { const message = event.data; if (!message || message.requestId < state.requestId) return; state.requestId = message.requestId; if (message.type === 'loading') { state.status = 'loading'; state.error = ''; render(); } if (message.type === 'results') { state.status = 'ready'; state.error = ''; state.results = message.results ?? []; state.warnings = message.warnings ?? []; render(); } if (message.type === 'error') { state.status = 'error'; state.error = message.message ?? 'Search failed.'; state.results = []; render(); } });
render();
</script>
</body>
</html>`;
}

function createNonce(): string {
	const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
	let value = '';
	for (let index = 0; index < 32; index += 1) {
		value += alphabet.charAt(Math.floor(Math.random() * alphabet.length));
	}
	return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === 'object' && !Array.isArray(value);
}
