import * as path from 'node:path';
import * as vscode from 'vscode';
import { diagnostic } from '../diagnostics/diagnostics';
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
	diagnostic('searchUi.registered');
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
	if (message.type === 'webviewReady') {
		diagnostic('searchUi.webviewReady', {
			width: numberField(message.width),
			height: numberField(message.height),
			devicePixelRatio: numberField(message.devicePixelRatio),
		});
		return;
	}
	if (message.type === 'webviewError') {
		diagnostic('searchUi.webviewError', {
			message: textField(message.message),
			source: textField(message.source),
			line: numberField(message.line),
			column: numberField(message.column),
		});
		return;
	}
	if (message.type === 'search' && typeof message.query === 'string') {
		await runSearch(
			context,
			active,
			message.query,
			message.source,
			message.resultType,
			numberField(message.page) ?? 1,
			numberField(message.pageSize) ?? 25,
		);
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
	typeValue: unknown,
	page: number,
	pageSize: number,
): Promise<void> {
	const normalizedQuery = query.trim();
	const requestId = ++active.requestSequence;
	const startedAt = Date.now();
	diagnostic('searchUi.searchStarted', {
		requestId,
		queryLength: normalizedQuery.length,
		source: typeof sourceValue === 'string' ? sourceValue : 'all',
	});
	if (!normalizedQuery) {
		active.latestResults.clear();
		active.panel.webview.postMessage({ type: 'results', requestId, results: [], warnings: [], total: 0, page, pageSize });
		return;
	}

	active.panel.webview.postMessage({ type: 'loading', requestId });
	try {
		const client = await getClient(context, active);
		const result = await client.search(normalizedQuery, sourcesFor(sourceValue, typeValue), pageSize, page);
		if (active.disposed || requestId !== active.requestSequence) {
			return;
		}
		active.latestResults = new Map(result.results.map(hit => [hit.id, hit]));
		diagnostic('searchUi.searchCompleted', {
			requestId,
			resultCount: result.results.length,
			warningCount: result.warnings.length,
			elapsedMs: Date.now() - startedAt,
		});
		active.panel.webview.postMessage({
			type: 'results',
			requestId,
			results: result.results,
			warnings: result.warnings,
			total: result.total,
			page: result.page,
			pageSize: result.pageSize,
		});
	} catch (error) {
		if (!active.disposed && requestId === active.requestSequence) {
			diagnostic('searchUi.searchFailed', {
				requestId,
				elapsedMs: Date.now() - startedAt,
				message: error instanceof Error ? error.message : String(error),
			});
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
		diagnostic('searchUi.resultOpenStarted', { source: hit.source, kind: hit.kind });
		const sourcePath = await client.resolveSourcePath(hit);
		let opened: vscode.TextDocument;
		let boundedDocument: SearchDocument | undefined;
		if (hit.sourceUri) {
			opened = await vscode.workspace.openTextDocument(vscode.Uri.parse(hit.sourceUri, true));
		} else if (sourcePath) {
			opened = await vscode.workspace.openTextDocument(vscode.Uri.file(sourcePath));
		} else {
			boundedDocument = await client.read(hit);
			const key = `document-${++documentSequence}`;
			while (searchDocuments.size >= maxSearchDocuments) {
				const oldest = searchDocuments.keys().next().value;
				if (oldest === undefined) {
					break;
				}
				searchDocuments.delete(oldest);
			}
			searchDocuments.set(key, boundedDocument.content);
			const uri = vscode.Uri.parse(
				`${searchScheme}:/${key}/${encodeURIComponent(hit.path)}`,
			);
			opened = await vscode.workspace.openTextDocument(uri);
		}
		const language = hit.source === 'wiki' ? 'markdown' : 'enforce';
		const documentWithLanguage = await vscode.languages.setTextDocumentLanguage(opened, language);
		if (hit.source === 'wiki') {
			await vscode.commands.executeCommand('markdown.showPreview', documentWithLanguage.uri);
			diagnostic('searchUi.resultOpenCompleted', {
				source: hit.source,
				physicalDocument: Boolean(sourcePath),
				markdownPreview: true,
			});
			return;
		}
		const editor = await vscode.window.showTextDocument(documentWithLanguage);
		const range = selectionRange(documentWithLanguage, hit, boundedDocument?.startLine ?? 1);
		editor.selection = new vscode.Selection(range.start, range.end);
		editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
		diagnostic('searchUi.resultOpenCompleted', {
			source: hit.source,
			physicalDocument: Boolean(sourcePath),
			markdownPreview: false,
		});
	} catch (error) {
		diagnostic('searchUi.resultOpenFailed', {
			source: hit.source,
			message: error instanceof Error ? error.message : String(error),
		});
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


function sourcesFor(value: unknown, typeValue: unknown): SearchSource[] {
	const sources: SearchSource[] = (() => {
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
	})();
	if (typeValue === 'symbol') {
		return sources.filter(source => source !== 'wiki');
	}
	if (typeValue === 'documentation') {
		return sources.filter(source => source === 'wiki');
	}
	return sources;
}

function selectionRange(
	document: vscode.TextDocument,
	hit: SearchHit,
	contentStartLine: number,
): vscode.Range {
	const startLine = Math.max(0, (hit.selectionStartLine ?? contentStartLine) - contentStartLine);
	const endLine = Math.min(
		document.lineCount - 1,
		Math.max(startLine, (hit.selectionEndLine ?? hit.selectionStartLine ?? contentStartLine) - contentStartLine),
	);
	return new vscode.Range(
		startLine,
		0,
		endLine,
		document.lineAt(endLine).text.length,
	);
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
.toolbar { display: flex; gap: 8px; align-items: center; max-width: 680px; margin-bottom: 14px; }
.toolbar input { flex: 1 1 620px; width: 620px; max-width: 100%; min-width: 160px; border: 1px solid var(--border); background: var(--alt); padding: 9px 11px; outline: none; }
.toolbar input:focus { border-color: var(--accent); }
.layout { display: grid; grid-template-columns: 170px 1fr; gap: 18px; }
.source-rail { height: fit-content; border: 1px solid var(--border); padding: 10px; background: var(--panel); }
.group-label { padding: 0 4px 6px; color: var(--muted); font-size: 11px; font-weight: 700; letter-spacing: .07em; }
.source-rail button { display: block; width: 100%; margin: 3px 0; padding: 9px 10px; text-align: left; border: 0; background: transparent; }
.source-rail button.active { background: var(--selected); color: var(--selected-text); }
.source-header { display: flex; justify-content: space-between; align-items: end; border-bottom: 1px solid var(--border); padding-bottom: 10px; }
.page-controls { display: flex; align-items: center; justify-content: flex-end; flex-wrap: wrap; gap: 6px; }
.page-arrows { display: inline-flex; gap: 2px; }
.page-controls button, .page-controls input, .page-controls select { min-height: 28px; }
.page-controls button { min-width: 28px; padding: 3px 7px; }
.page-controls input { width: 48px; padding: 3px 5px; text-align: center; border: 1px solid var(--border); background: var(--alt); outline: none; }
.page-controls input:focus, .page-controls select:focus { border-color: var(--accent); }
.page-controls select { padding: 3px 5px; border: 1px solid var(--border); background: var(--alt); }
.page-bottom { display: flex; justify-content: flex-end; margin-top: 12px; }
.muted { color: var(--muted); }
.tag { border-radius: 12px; padding: 3px 8px; background: var(--alt); color: var(--muted); font-size: 11px; }
.source-rows { display: grid; gap: 10px; margin-top: 12px; }
.source-row { display: grid; grid-template-columns: 28px minmax(0, 1fr); gap: 12px; align-items: start; padding: 13px; border: 1px solid var(--border); background: var(--panel); cursor: pointer; user-select: text; }
.source-row:hover { border-color: var(--accent); }
.source-row:focus-visible { outline: 1px solid var(--accent); outline-offset: 2px; }
.source-row.selected { border-color: var(--accent); }
.source-icon { color: var(--accent); font-weight: 700; text-align: center; }
.result-content { min-width: 0; }
.result-head { display: flex; justify-content: space-between; align-items: baseline; gap: 16px; min-width: 0; }
.result-head h3 { min-width: 0; margin-bottom: 0; }
.result-detail, .result-path { display: block; color: var(--muted); font-size: 12px; }
.result-path { max-width: 50%; margin-left: auto; overflow-wrap: anywhere; text-align: right; }
.snippet { margin: 9px 0 0; padding: 10px; overflow: auto; background: var(--alt); border: 1px solid var(--border); font: 12px/1.5 var(--vscode-editor-font-family); white-space: pre-wrap; }
.md-preview { margin: 9px 0 0; padding: 10px 12px; overflow: auto; background: var(--alt); border: 1px solid var(--border); line-height: 1.5; }
.md-preview h1, .md-preview h2, .md-preview h3, .md-preview h4, .md-preview h5, .md-preview h6 { margin: 0 0 7px; font-size: 14px; }
.md-preview p { margin: 0 0 8px; }
.md-preview p:last-child, .md-preview ul:last-child, .md-preview blockquote:last-child { margin-bottom: 0; }
.md-preview ul { margin: 0 0 8px; padding-left: 20px; }
.md-preview blockquote { margin: 0 0 8px; padding-left: 10px; border-left: 2px solid var(--accent); color: var(--muted); }
.md-preview a { color: var(--accent); }
.md-preview code { padding: 1px 4px; background: var(--panel); font-family: var(--vscode-editor-font-family); }
.md-preview .md-code { margin: 0 0 8px; padding: 8px; overflow: auto; background: var(--panel); font: 12px/1.45 var(--vscode-editor-font-family); white-space: pre-wrap; }
.result-actions { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
.result-actions button { padding: 5px 8px; }
.status { margin: 12px 0; color: var(--muted); }
.error { padding: 10px 12px; border: 1px solid var(--vscode-inputValidation-errorBorder); color: var(--vscode-errorForeground); background: var(--vscode-inputValidation-errorBackground); }
.warning { padding: 8px 10px; border-left: 2px solid var(--vscode-editorWarning-foreground); color: var(--muted); }
.empty { padding: 30px 14px; border: 1px dashed var(--border); color: var(--muted); }
@media (max-width: 720px) { .shell { padding: 18px 14px 60px; } .layout { grid-template-columns: 1fr; } .toolbar { flex-wrap: wrap; } .toolbar input { flex-basis: 100%; } .source-header { align-items: flex-start; gap: 10px; } .source-row { grid-template-columns: 26px 1fr; } .result-head { align-items: flex-start; flex-wrap: wrap; } .result-path { max-width: 100%; margin-left: 0; text-align: left; } }
</style>
</head>
<body>
<main id="app"></main>
<script nonce="${nonce}">
window.__reforgerSearchVscode = acquireVsCodeApi();
const reportWebviewError = (message, source, line, column) => window.__reforgerSearchVscode.postMessage({ type: 'webviewError', message: String(message ?? 'Unknown webview error'), source, line, column });
window.addEventListener('error', event => reportWebviewError(event.message, event.filename, event.lineno, event.colno));
window.addEventListener('unhandledrejection', event => reportWebviewError(event.reason?.message ?? event.reason, 'unhandledrejection'));
window.__reforgerSearchVscode.postMessage({ type: 'webviewReady', width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio });
</script>
<script nonce="${nonce}">
const vscode = window.__reforgerSearchVscode;
const state = { query: '', source: 'all', type: 'all', results: [], warnings: [], status: 'idle', error: '', requestId: 0, selected: '', page: 1, pageSize: 25, total: 0 };
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
const resultTypes = [{ value: 'all', label: 'All result types' }, { value: 'symbol', label: 'Symbols' }, { value: 'documentation', label: 'Documentation' }];
const typeButtons = () => resultTypes.map(type => '<button class="' + (state.type === type.value ? 'active' : '') + '" data-type="' + esc(type.value) + '">' + esc(type.label) + '</button>').join('');
const pageSizeOptions = [25, 50, 100];
const totalMatches = () => state.total;
const totalPages = () => Math.max(1, Math.ceil(state.total / state.pageSize));
const pageControls = () => {
  const navigationDisabled = !state.query.trim() || state.status === 'loading';
  const pageTotal = totalPages();
  const sizes = pageSizeOptions.map(size => '<option value="' + size + '"' + (state.pageSize === size ? ' selected' : '') + '>' + size + ' results</option>').join('');
  return '<div class="page-controls" aria-label="Search result pages"><select data-page-size aria-label="Total results per page">' + sizes + '</select><span class="muted">Page</span><input data-page-input type="number" min="1" max="' + pageTotal + '" value="' + state.page + '" aria-label="Current result page"' + (navigationDisabled ? ' disabled' : '') + '><span class="muted">of ' + pageTotal + '</span><span class="page-arrows"><button type="button" data-page-prev' + (navigationDisabled || state.page <= 1 ? ' disabled' : '') + ' aria-label="Previous page">‹</button><button type="button" data-page-next' + (navigationDisabled || state.page >= pageTotal ? ' disabled' : '') + ' aria-label="Next page">›</button></span></div>';
};
const inlineMarkdown = value => value
  .replace(/\\\\([*_])/g, '$1')
  .replace(/\`([^\`]+)\`/g, '<code>$1</code>')
  .replace(/!\\[([^\\]]*)\\]\\((?:[^()\\n]|\\([^()\\n]*\\))*\\)/g, '$1')
  .replace(/\\[\\]\\((?:[^()\\n]|\\([^()\\n]*\\))*\\)/g, '')
  .replace(/\\[([^\\]]+)\\]\\((?:[^()\\n]|\\([^()\\n]*\\))*\\)/g, '$1')
  .replace(/\\*\\*([^*]+)\\*\\*/g, '<strong>$1</strong>')
  .replace(/__([^_]+)__/g, '<strong>$1</strong>')
  .replace(/\\*([^*]+)\\*/g, '<em>$1</em>')
  .replace(/_([^_]+)_/g, '<em>$1</em>');
const renderMarkdown = value => {
  const lines = esc(value).split('\\n');
  const output = [];
  let paragraph = [];
  let list = false;
  let code = false;
  const flushParagraph = () => { if (paragraph.length) { output.push('<p>' + inlineMarkdown(paragraph.join(' ')) + '</p>'); paragraph = []; } };
  const closeList = () => { if (list) { output.push('</ul>'); list = false; } };
  lines.forEach(line => {
    if (line.trim().startsWith('\`\`\`')) {
      flushParagraph(); closeList();
      if (code) { output.push('</code></pre>'); } else { output.push('<pre class="md-code"><code>'); }
      code = !code;
      return;
    }
    if (code) { output.push(line + '\\n'); return; }
    if (!line.trim()) { flushParagraph(); closeList(); return; }
    const heading = line.match(/^(#{1,6})\\s+(.+)$/);
    if (heading) { flushParagraph(); closeList(); output.push('<h' + heading[1].length + '>' + inlineMarkdown(heading[2]) + '</h' + heading[1].length + '>'); return; }
    const item = line.match(/^\\s*[-*+]\\s+(.+)$/);
    if (item) { flushParagraph(); if (!list) { output.push('<ul>'); list = true; } output.push('<li>' + inlineMarkdown(item[1]) + '</li>'); return; }
    if (line.match(/^>\\s?/)) { flushParagraph(); closeList(); output.push('<blockquote>' + inlineMarkdown(line.replace(/^>\\s?/, '')) + '</blockquote>'); return; }
    closeList(); paragraph.push(line.trim());
  });
  flushParagraph(); closeList();
  if (code) { output.push('</code></pre>'); }
  return output.join('');
};
const resultRows = () => visibleResults().map(result => {
  const selected = state.selected === result.id;
  const external = result.sourceUrl ? '<button data-external="' + esc(result.id) + '">Open official page</button>' : '';
  const preview = result.kind === 'documentation' ? '<div class="md-preview">' + renderMarkdown(result.excerpt) + '</div>' : '<pre class="snippet">' + esc(result.excerpt) + '</pre>';
  return '<article class="source-row ' + (selected ? 'selected' : '') + '" data-open="' + esc(result.id) + '" tabindex="0" role="button"><div class="source-icon">' + (result.kind === 'documentation' ? 'W' : 'S') + '</div><div class="result-content"><div class="result-head"><h3>' + esc(result.title) + '</h3><div class="result-path">' + esc(result.path) + '</div></div><div class="result-detail">' + esc(result.detail) + ' · ' + esc(sourceLabel(result.source)) + '</div>' + preview + '<div class="result-actions">' + external + '</div></div></article>';
}).join('');
const hasTextSelection = () => Boolean(window.getSelection()?.toString());
const openResult = element => {
  if (!element.dataset.open || hasTextSelection()) return;
  state.selected = element.dataset.open;
  vscode.postMessage({ type: 'open', id: element.dataset.open });
  render();
};
function render() {
  const results = visibleResults();
  const body = state.error ? '<div class="error">' + esc(state.error) + '</div>' : results.length ? '<div class="source-rows">' + resultRows() + '</div>' : '<div class="empty">' + (state.status === 'idle' ? 'Enter a symbol, concept, or documentation term to search.' : 'No results match this search.') + '</div>';
  const warnings = state.warnings.map(warning => '<div class="warning">' + esc(warning) + '</div>').join('');
  const bottomPager = state.query.trim() && totalMatches() > 0 ? '<div class="page-bottom">' + pageControls() + '</div>' : '';
  document.getElementById('app').innerHTML = '<div class="shell"><div class="eyebrow">Source browser · live MCP search</div><h1>Find usage in Reforger</h1><p class="intro">Search the indexed workspace, shipped Game Data, and Official Wiki together. Select a result to open the exact source document and highlight the matching lines.</p><div class="toolbar"><input id="query" value="' + esc(state.query) + '" placeholder="Search a symbol, concept, or phrase..." aria-label="Search query"></div><div class="layout"><aside class="source-rail"><div class="group-label">SEARCH IN</div>' + sourceButtons() + '<div class="group-label">RESULT TYPE</div>' + typeButtons() + '</aside><section><div class="source-header"><div><h2>' + totalMatches() + ' matches</h2><span class="muted">' + (state.status === 'loading' ? 'Searching...' : 'Showing up to ' + state.pageSize + ' total results') + '</span></div>' + pageControls() + '</div><div class="status">' + (state.status === 'error' ? 'Search failed' : '') + '</div>' + warnings + body + bottomPager + '</section></div></div>';
  const query = document.getElementById('query');
  query.focus();
  query.setSelectionRange(state.query.length, state.query.length);
  query.addEventListener('input', event => { state.query = event.target.value; scheduleSearch(); });
  query.addEventListener('keydown', event => { if (event.key === 'Enter') { event.preventDefault(); search(true); } });
  document.querySelectorAll('[data-type]').forEach(element => element.addEventListener('click', () => { state.type = element.dataset.type; state.page = 1; search(true); }));
  document.querySelectorAll('[data-source]').forEach(element => element.addEventListener('click', () => { state.source = element.dataset.source; search(true); }));
  document.querySelectorAll('[data-page-prev]').forEach(element => element.addEventListener('click', () => requestPage(state.page - 1)));
  document.querySelectorAll('[data-page-next]').forEach(element => element.addEventListener('click', () => requestPage(state.page + 1)));
  document.querySelectorAll('[data-page-size]').forEach(element => element.addEventListener('change', event => { state.pageSize = Number(event.target.value); search(true); }));
  document.querySelectorAll('[data-page-input]').forEach(element => {
    element.addEventListener('change', event => requestPage(event.target.value));
    element.addEventListener('keydown', event => { if (event.key === 'Enter') { event.preventDefault(); requestPage(event.target.value); } });
  });
  document.querySelectorAll('[data-open]').forEach(element => {
    element.addEventListener('click', event => { if (event.target.closest('[data-external]') || hasTextSelection()) return; openResult(element); });
    element.addEventListener('keydown', event => { if (event.target.closest('[data-external]')) return; if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); openResult(element); } });
  });
  document.querySelectorAll('[data-external]').forEach(element => element.addEventListener('click', event => { event.stopPropagation(); vscode.postMessage({ type: 'external', id: element.dataset.external }); }));
}
let searchTimer;
function scheduleSearch() { clearTimeout(searchTimer); searchTimer = setTimeout(() => search(true), 260); }
function requestPage(value) { const requested = Number.parseInt(value, 10); if (!Number.isFinite(requested)) return; state.page = Math.min(totalPages(), Math.max(1, requested)); search(false); }
function search(resetPagination) { if (resetPagination) { state.page = 1; state.total = 0; } state.error = ''; state.warnings = []; state.status = state.query.trim() ? 'loading' : 'idle'; state.selected = ''; render(); vscode.postMessage({ type: 'search', query: state.query, source: state.source, resultType: state.type, page: state.page, pageSize: state.pageSize }); }
window.addEventListener('message', event => { const message = event.data; if (!message || message.requestId < state.requestId) return; state.requestId = message.requestId; if (message.type === 'loading') { state.status = 'loading'; state.error = ''; render(); } if (message.type === 'results') { state.status = 'ready'; state.error = ''; state.results = message.results ?? []; state.warnings = message.warnings ?? []; state.total = message.total ?? 0; state.page = message.page ?? state.page; state.pageSize = message.pageSize ?? state.pageSize; render(); } if (message.type === 'error') { state.status = 'error'; state.error = message.message ?? 'Search failed.'; state.results = []; render(); } });
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

function textField(value: unknown): string | undefined {
	return typeof value === 'string' ? value.slice(0, 500) : undefined;
}

function numberField(value: unknown): number | undefined {
	return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}
