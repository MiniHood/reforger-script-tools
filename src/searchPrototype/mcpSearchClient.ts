import { access } from 'node:fs/promises';
import * as path from 'node:path';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { searchLimits } from '../extensionConfig/search';
import type { ExternalIndexMode } from '../extensionConfig/workbench';

export type SearchSource = 'workspace' | 'gameData' | 'wiki';
export type SearchMode = 'semantic' | 'text';
export const workspaceScopeId = 'workspace';
export const wikiScopeId = 'wiki';

export interface SearchScopeSource {
	id: string;
	label: string;
	detail: string;
	kind: 'workspace' | 'addon' | 'wiki';
	pinned: boolean;
	defaultSelected: boolean;
}

export interface SearchScopeDiscovery {
	scopeRevision?: string;
	scopeAuthority?: string;
	discoveryMs: number;
	unavailableScopeIds: string[];
	sources: SearchScopeSource[];
}
export interface TextSearchOptions {
	matchCase: boolean;
	matchWholeWord: boolean;
	useRegex: boolean;
}

export const defaultTextSearchOptions: TextSearchOptions = {
	matchCase: false,
	matchWholeWord: false,
	useRegex: false,
};
export type SearchSymbolKind =
	| 'class'
	| 'constructor'
	| 'destructor'
	| 'enum'
	| 'enumMember'
	| 'field'
	| 'function'
	| 'globalField'
	| 'method'
	| 'preprocessorMacro'
	| 'typedef';

export interface SearchKindFilter {
	value: string;
	label: string;
	kinds?: readonly SearchSymbolKind[];
}

export const searchKindFilters: readonly SearchKindFilter[] = [
	{ value: 'all', label: 'All results' },
	{ value: 'class', label: 'Classes', kinds: ['class'] },
	{ value: 'function', label: 'Functions', kinds: ['function', 'method', 'constructor', 'destructor'] },
	{ value: 'field', label: 'Fields', kinds: ['field', 'globalField'] },
	{ value: 'enum', label: 'Enums', kinds: ['enum', 'enumMember'] },
];

export interface SearchHit {
	id: string;
	source: SearchSource;
	kind: 'symbol' | 'documentation' | 'text';
	title: string;
	detail: string;
	path: string;
	excerpt: string;
	matchKind?: string;
	sourceUrl?: string;
	sourceUri?: string;
	selectionStartLine?: number;
	selectionEndLine?: number;
	readInput: Record<string, unknown>;
	addonGuid?: string;
	addonLabel?: string;
	textMatchStart?: number;
	textMatchLength?: number;
}

export interface SearchResponse {
	results: SearchHit[];
	warnings: string[];
	total: number;
	totalBySource: Partial<Record<SearchSource, number>>;
	page: number;
	pageSize: number;
	performance: SearchPerformance;
}

export interface SearchSourcePerformance {
	source: SearchSource;
	initialMs: number;
	rangeMs: number;
	remoteMs: number;
	pagesVisited: number;
	remoteRequests: number;
	cacheHits: number;
	firstPage: number | undefined;
	lastPage: number | undefined;
	cacheSize: number;
	addonTotals?: Record<string, number>;
	textStats?: Record<string, unknown>;
}

export interface SearchPerformance {
	totalMs: number;
	startupMs: number;
	initialSearchMs: number;
	rangeSearchMs: number;
	mergeMs: number;
	requestedPage: number;
	paginationMode: 'offset' | 'cursor' | 'mixed';
	searchMode: SearchMode;
	textOptions: TextSearchOptions;
	pageSize: number;
	sourcePageSize: number;
	sources: SearchSourcePerformance[];
	selectedScopeIds: string[];
	addonGuids: string[];
}

export interface SearchDocument {
	content: string;
	startLine: number;
	endLine: number;
}

export function sourcePreviewLine(
	document: SearchDocument,
	line: number | undefined,
	needle?: string,
): number {
	const lines = document.content.split(/\r?\n/);
	const startLine = document.startLine > 0 ? document.startLine : line ?? 1;
	const requestedIndex = Math.max(0, Math.min(lines.length - 1, (line ?? startLine) - startLine));
	const candidateIndexes = [
		...Array.from({ length: Math.max(0, lines.length - requestedIndex) }, (_, index) => requestedIndex + index),
		...Array.from({ length: Math.max(0, requestedIndex) }, (_, index) => index),
	];
	const normalizedNeedle = needle?.trim().toLowerCase();
	const matchingIndex = normalizedNeedle
		? candidateIndexes.find(index => {
			const value = stripSourceComments(lines[index] ?? '');
			return value.trim().length > 0 && value.toLowerCase().includes(normalizedNeedle);
		})
		: undefined;
	const contentIndex = matchingIndex
		?? candidateIndexes.find(index => stripSourceComments(lines[index] ?? '').trim().length > 0)
		?? requestedIndex;
	return startLine + contentIndex;
}

export function sourceLinePreview(
	document: SearchDocument,
	line: number | undefined,
	needle?: string,
): string {
	const lines = document.content.split(/\r?\n/);
	const startLine = document.startLine > 0 ? document.startLine : line ?? 1;
	const selectedLine = sourcePreviewLine(document, line, needle);
	const lineIndex = Math.max(0, selectedLine - startLine);
	return stripSourceComments(lines[lineIndex] ?? lines[0] ?? '').trimStart().trimEnd();
}

/** Removes comments while preserving quoted strings. */
export function stripSourceComments(value: string): string {
	let result = '';
	let quote: '"' | "'" | undefined;
	let escaped = false;
	let blockComment = false;
	for (let index = 0; index < value.length; index += 1) {
		const character = value[index];
		const next = value[index + 1];
		if (blockComment) {
			if (character === '*' && next === '/') {
				blockComment = false;
				index += 1;
			}
			continue;
		}
		if (quote) {
			result += character;
			if (escaped) {
				escaped = false;
			} else if (character === '\\') {
				escaped = true;
			} else if (character === quote) {
				quote = undefined;
			}
			continue;
		}
		if (character === '"' || character === "'") {
			quote = character;
			result += character;
		} else if (character === '/' && next === '/') {
			break;
		} else if (character === '/' && next === '*') {
			blockComment = true;
			index += 1;
		} else {
			result += character;
		}
	}
	return result;
}

export interface SourceMatchRange {
	start: number;
	length: number;
}

export function sourceMatchRange(text: string, title: string): SourceMatchRange | undefined {
	if (!text || !title) {
		return undefined;
	}
	const exactStart = text.indexOf(title);
	if (exactStart >= 0) {
		return { start: exactStart, length: title.length };
	}
	const foldedStart = text.toLowerCase().indexOf(title.toLowerCase());
	return foldedStart >= 0 ? { start: foldedStart, length: title.length } : undefined;
}

export interface McpSearchClientOptions {
	serverPath: string;
	addonSourceInventory: string;
	addonIndexStorage: string;
	externalIndexMode: ExternalIndexMode;
	workspaceScripts: string[];
	dependencyProjectFiles: string[];
	officialWikiRoot: string;
}

interface JsonRpcResult {
	result?: {
		isError?: boolean;
		structuredContent?: unknown;
	};
	error?: {
		message?: string;
	};
}

interface PendingRequest {
	resolve: (value: JsonRpcResult) => void;
	reject: (error: Error) => void;
	timeout: ReturnType<typeof setTimeout>;
}

const requestTimeoutMs = 135_000;
const maxSearchPageCaches = 32;
const maxCachedPagesPerSearch = 32;
const sourcePageSize = 100;
export const maxSearchPages = searchLimits.maxPages;

interface RecordValue {
	[key: string]: unknown;
}

interface CachedSearchPage {
	results: SearchHit[];
	total: number;
	nextCursor?: string;
	stats?: Record<string, unknown>;
	addonTotals?: Record<string, number>;
}

interface SearchPerformanceTrace {
	startedAt: number;
	startupMs: number;
	initialSearchMs: number;
	rangeSearchStartedAt: number | undefined;
	rangeSearchMs: number;
	mergeMs: number;
	sources: Map<SearchSource, SearchSourcePerformance>;
}

function createSearchPerformanceTrace(): SearchPerformanceTrace {
	return {
		startedAt: performance.now(),
		startupMs: 0,
		initialSearchMs: 0,
		rangeSearchStartedAt: undefined,
		rangeSearchMs: 0,
		mergeMs: 0,
		sources: new Map(),
	};
}

function sourcePerformanceFor(
	trace: SearchPerformanceTrace,
	source: SearchSource,
): SearchSourcePerformance {
	let value = trace.sources.get(source);
	if (!value) {
		value = {
			source,
			initialMs: 0,
			rangeMs: 0,
			remoteMs: 0,
			pagesVisited: 0,
			remoteRequests: 0,
			cacheHits: 0,
			firstPage: undefined,
			lastPage: undefined,
			cacheSize: 0,
		};
		trace.sources.set(source, value);
	}
	return value;
}

function recordVisitedPage(trace: SearchSourcePerformance, page: number): void {
	trace.pagesVisited += 1;
	trace.firstPage = trace.firstPage === undefined ? page : Math.min(trace.firstPage, page);
	trace.lastPage = trace.lastPage === undefined ? page : Math.max(trace.lastPage, page);
}

function finishSearchPerformance(
	trace: SearchPerformanceTrace,
	requestedPage: number,
	pageSize: number,
	sources: readonly SearchSource[],
	mode: SearchMode,
	textOptions: TextSearchOptions,
	selectedScopeIds: readonly string[],
	addonGuids: readonly string[],
): SearchPerformance {
	return {
		totalMs: performance.now() - trace.startedAt,
		startupMs: trace.startupMs,
		initialSearchMs: trace.initialSearchMs,
		rangeSearchMs: trace.rangeSearchMs,
		mergeMs: trace.mergeMs,
		requestedPage,
		paginationMode: paginationModeFor(mode, sources),
		searchMode: mode,
		textOptions,
		pageSize,
		sourcePageSize,
		sources: sources.map(source => sourcePerformanceFor(trace, source)),
		selectedScopeIds: [...selectedScopeIds],
		addonGuids: [...addonGuids],
	};
}

export class McpSearchClient {
	private process: ChildProcessWithoutNullStreams | undefined;
	private receiveBuffer = Buffer.alloc(0);
	private nextRequestId = 1;
	private readonly pending = new Map<number, PendingRequest>();
	private initialized: Promise<void> | undefined;
	private readonly searchPageCaches = new Map<string, Map<number, CachedSearchPage>>();

	public constructor(private readonly options: McpSearchClientOptions) {}

	public async search(
		query: string,
		selectedScopeIds: string[],
		pageSize: number,
		page: number,
		symbolKinds?: readonly SearchSymbolKind[],
		mode: SearchMode = 'semantic',
		textOptions: TextSearchOptions = defaultTextSearchOptions,
	): Promise<SearchResponse> {
		const trace = createSearchPerformanceTrace();
		await this.start();
		trace.startupMs = performance.now() - trace.startedAt;
		const normalizedPageSize = Math.min(100, Math.max(1, Number.isFinite(pageSize) ? Math.floor(pageSize) : 25));
		const requestedPage = Math.min(maxSearchPages, Math.max(1, Number.isFinite(page) ? Math.floor(page) : 1));
		const normalizedScopes = [...new Set(selectedScopeIds)];
		const addonGuids = normalizedScopes
			.filter(value => /^[0-9a-f]{16}$/i.test(value))
			.map(value => value.toUpperCase())
			.sort();
		const sources: SearchSource[] = [
			...(addonGuids.length > 0 ? ['gameData' as const] : []),
			...(normalizedScopes.includes(workspaceScopeId) ? ['workspace' as const] : []),
			...(normalizedScopes.includes(wikiScopeId) ? ['wiki' as const] : []),
		];
		const searchableSources = mode === 'semantic'
			? sources.filter(source => source !== 'wiki')
			: sources;
		if (searchableSources.length === 0) {
			return { results: [], warnings: [], total: 0, totalBySource: {}, page: 1, pageSize: normalizedPageSize, performance: finishSearchPerformance(trace, 1, normalizedPageSize, [], mode, textOptions, normalizedScopes, addonGuids) };
		}
		const initialSearchStartedAt = performance.now();
		const responses = await Promise.all(searchableSources.map(async source => {
			const sourceTrace = sourcePerformanceFor(trace, source);
			const startedAt = performance.now();
			try {
				const value = await this.searchPage(query, source, sourcePageSize, 1, addonGuids, symbolKinds, trace, mode, textOptions);
				if (value.stats) {
					sourceTrace.textStats = value.stats;
				}
				sourceTrace.initialMs += performance.now() - startedAt;
				return { source, value, warning: undefined };
			} catch (error) {
				sourceTrace.initialMs += performance.now() - startedAt;
				return { source, value: undefined, warning: searchErrorMessage(source, error) };
			}
		}));
		trace.initialSearchMs = performance.now() - initialSearchStartedAt;

		const warnings = responses
			.map(response => response.warning)
			.filter((warning): warning is string => warning !== undefined);
		if (responses.every(response => response.value === undefined) && warnings.length === responses.length) {
			throw new Error(warnings.join(' '));
		}
		const total = responses.reduce((sum, response) => sum + (response.value?.total ?? 0), 0);
		const totalBySource: Partial<Record<SearchSource, number>> = {};
		for (const response of responses) {
			if (response.value) {
				totalBySource[response.source] = response.value.total;
			}
		}
		const totalPages = Math.max(1, Math.ceil(total / normalizedPageSize));
		const normalizedPage = Math.min(requestedPage, totalPages);
		const pageStart = (normalizedPage - 1) * normalizedPageSize;
		const pageEnd = pageStart + normalizedPageSize;
		const results: SearchHit[] = [];
		let sourceOffset = 0;
		const mergeStartedAt = performance.now();
		trace.rangeSearchStartedAt = performance.now();
		for (const response of responses) {
			const sourceTotal = response.value?.total ?? 0;
			const sourceStart = Math.max(0, pageStart - sourceOffset);
			const sourceEnd = Math.min(sourceTotal, pageEnd - sourceOffset);
			if (response.value && sourceStart < sourceEnd) {
				results.push(...await this.sourceRange(query, response.source, sourceStart, sourceEnd, addonGuids, symbolKinds, trace, mode, textOptions));
			}
			sourceOffset += sourceTotal;
		}
		trace.rangeSearchMs = performance.now() - (trace.rangeSearchStartedAt ?? performance.now());
		trace.mergeMs = performance.now() - mergeStartedAt - trace.rangeSearchMs;
		return {
			results,
			warnings,
			total,
			totalBySource,
			page: normalizedPage,
			pageSize: normalizedPageSize,
			performance: finishSearchPerformance(trace, requestedPage, normalizedPageSize, searchableSources, mode, textOptions, normalizedScopes, addonGuids),
		};
	}

	public async discoverScope(): Promise<SearchScopeDiscovery> {
		const startedAt = performance.now();
		await this.start();
		const status = asRecord(await this.callTool('game_data_status', {}));
		const addons = Array.isArray(status.addons) ? status.addons : [];
		const unavailableScopeIds = addons.flatMap(value => {
			const addon = asRecord(value);
			const id = asString(addon.addonGuid, '').toUpperCase();
			return /^[0-9A-F]{16}$/.test(id) && addon.available === false ? [id] : [];
		});
		const addonSources = addons.flatMap(value => {
			const addon = asRecord(value);
			const id = asString(addon.addonGuid, '').toUpperCase();
			if (!/^[0-9A-F]{16}$/.test(id) || addon.available === false) {
				return [];
			}
			const title = asString(addon.title, asString(addon.displayId, id));
			const scriptCount = asNumber(addon.scriptCount, 0);
			return [{
				id,
				label: title,
				detail: `${scriptCount.toLocaleString()} scripts`,
				kind: 'addon' as const,
				pinned: addon.pinned === true,
				defaultSelected: addon.defaultSelected === true,
			}];
		});
		return {
			scopeRevision: asOptionalString(status.scopeRevision),
			scopeAuthority: asOptionalString(status.scopeAuthority),
			discoveryMs: performance.now() - startedAt,
			unavailableScopeIds,
			sources: [
				{ id: workspaceScopeId, label: 'Workspace', detail: 'Live', kind: 'workspace', pinned: true, defaultSelected: true },
				{ id: wikiScopeId, label: 'Official Wiki', detail: 'Text search', kind: 'wiki', pinned: true, defaultSelected: true },
				...addonSources,
			],
		};
	}

	public async read(hit: SearchHit): Promise<SearchDocument> {
		await this.start();
		const tool = hit.source === 'wiki' ? 'read_official_wiki' : hit.source === 'gameData'
			? 'read_game_data_source'
			: 'read_workspace_source';
		const value = await this.callTool(tool, hit.readInput);
		const record = asRecord(value);
		return {
			content: asString(record.content, 'The source read returned no content.'),
			startLine: asNumber(record.startLine, 0),
			endLine: asNumber(record.endLine, 0),
		};
	}

	public async resolveSourcePath(hit: SearchHit): Promise<string | undefined> {
		const relativePath = typeof hit.readInput.relativePath === 'string'
			? hit.readInput.relativePath
			: undefined;
		if (!relativePath || hit.source === 'gameData') {
			return undefined;
		}
		const roots = hit.source === 'wiki'
			? [this.options.officialWikiRoot]
			: this.options.workspaceScripts;
		for (const root of roots) {
			const candidate = path.resolve(root, relativePath);
			if (!isWithinRoot(root, candidate)) {
				continue;
			}
			try {
				await access(candidate);
				return candidate;
			} catch {
				// Try the next configured source root.
			}
		}
		return undefined;
	}

	public async start(): Promise<void> {
		if (this.initialized) {
			return this.initialized;
		}
		this.initialized = this.startProcess();
		try {
			await this.initialized;
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public dispose(): void {
		const activeProcess = this.process;
		this.process = undefined;
		this.initialized = undefined;
		this.searchPageCaches.clear();
		const error = new Error('The Reforger search session was closed.');
		for (const request of this.pending.values()) {
			request.reject(error);
		}
		this.pending.clear();
		if (activeProcess && !activeProcess.killed) {
			activeProcess.stdin.end();
			activeProcess.kill();
		}
	}

	private async startProcess(): Promise<void> {
		const args = [
			'mcp',
			'--addon-source-inventory',
			this.options.addonSourceInventory,
			'--addon-index-storage',
			this.options.addonIndexStorage,
			'--external-index-mode',
			this.options.externalIndexMode,
			'--official-wiki-root',
			this.options.officialWikiRoot,
			...this.options.workspaceScripts.flatMap(root => ['--workspace-scripts', root]),
			...this.options.dependencyProjectFiles.flatMap(projectFile => ['--dependency-project', projectFile]),
		];
		const child = spawn(this.options.serverPath, args, {
			stdio: ['pipe', 'pipe', 'pipe'],
			windowsHide: true,
		});
		this.process = child;
		child.stdout.on('data', chunk => this.consumeOutput(Buffer.from(chunk)));
		child.on('error', error => this.failPending(error));
		child.on('exit', () => {
			this.process = undefined;
			this.initialized = undefined;
			this.searchPageCaches.clear();
			this.failPending(new Error('The Reforger search server stopped.'));
		});
		child.stdin.on('error', error => this.failPending(error));
		child.stderr.on('data', () => undefined);

		await this.request('initialize', {
			protocolVersion: '2025-11-25',
			capabilities: {},
			clientInfo: { name: 'reforger-search-ui', version: '0.1.0' },
		});
		this.notify('notifications/initialized', {});
	}

	private async callTool(tool: string, argumentsValue: Record<string, unknown>): Promise<unknown> {
		const response = await this.request('tools/call', {
			name: tool,
			arguments: argumentsValue,
		});
		if (response.result?.isError) {
			const structured = asRecord(response.result.structuredContent);
			throw new Error(asString(structured.message, `MCP tool ${tool} failed.`));
		}
		return response.result?.structuredContent ?? {};
	}

	private async sourceRange(
		query: string,
		source: SearchSource,
		start: number,
		end: number,
		addonGuids: readonly string[],
		symbolKinds?: readonly SearchSymbolKind[],
		trace?: SearchPerformanceTrace,
		mode: SearchMode = 'semantic',
		textOptions: TextSearchOptions = defaultTextSearchOptions,
	): Promise<SearchHit[]> {
		const sourceStartedAt = performance.now();
		const results: SearchHit[] = [];
		const firstPageNumber = Math.floor(start / sourcePageSize) + 1;
		const lastPageNumber = Math.floor((end - 1) / sourcePageSize) + 1;
		for (let pageNumber = firstPageNumber; pageNumber <= lastPageNumber; pageNumber += 1) {
			const page = await this.searchPage(query, source, sourcePageSize, pageNumber, addonGuids, symbolKinds, trace, mode, textOptions);
			const pageStart = (pageNumber - 1) * sourcePageSize;
			const resultStart = Math.max(0, start - pageStart);
			const resultEnd = Math.min(page.results.length, end - pageStart);
			if (resultStart < resultEnd) {
				results.push(...page.results.slice(resultStart, resultEnd));
			}
			if (!page.nextCursor || page.results.length === 0) {
				break;
			}
		}
		if (trace) {
			sourcePerformanceFor(trace, source).rangeMs += performance.now() - sourceStartedAt;
		}
		return results;
	}

	private async searchPage(
		query: string,
		source: SearchSource,
		pageSize: number,
		page: number,
		addonGuids: readonly string[],
		symbolKinds?: readonly SearchSymbolKind[],
		trace?: SearchPerformanceTrace,
		mode: SearchMode = 'semantic',
		textOptions: TextSearchOptions = defaultTextSearchOptions,
	): Promise<CachedSearchPage> {
		const sourceTrace = trace ? sourcePerformanceFor(trace, source) : undefined;
		const cacheKey = `${mode}\u0000${source}\u0000${pageSize}\u0000${query}\u0000${addonGuids.join(',')}\u0000${symbolKinds?.join(',') ?? ''}\u0000${textOptions.matchCase}\u0000${textOptions.matchWholeWord}\u0000${textOptions.useRegex}`;
		let pages = this.searchPageCaches.get(cacheKey);
		if (!pages) {
			if (this.searchPageCaches.size >= maxSearchPageCaches) {
				const oldest = this.searchPageCaches.keys().next().value;
				if (oldest !== undefined) {
					this.searchPageCaches.delete(oldest);
				}
			}
			pages = new Map<number, CachedSearchPage>();
			this.searchPageCaches.set(cacheKey, pages);
		}

		const cached = pages.get(page);
		if (cached) {
			if (sourceTrace) {
				sourceTrace.cacheHits += 1;
				recordVisitedPage(sourceTrace, page);
				sourceTrace.cacheSize = pages.size;
			}
			return cached;
		}
		const usesCursor = mode === 'text' && source !== 'wiki';
		if (usesCursor && page > 1 && !pages.has(page - 1)) {
			await this.searchPage(query, source, pageSize, page - 1, addonGuids, symbolKinds, trace, mode, textOptions);
		}
		const previousPage = usesCursor && page > 1 ? pages.get(page - 1) : undefined;
		const argumentsValue: Record<string, unknown> = usesCursor ? {
			query,
			...(source === 'gameData' ? { addonGuids } : {}),
			limit: pageSize,
			matchCase: textOptions.matchCase,
			matchWholeWord: textOptions.matchWholeWord,
			useRegex: textOptions.useRegex,
			...(previousPage?.nextCursor ? { cursor: previousPage.nextCursor } : {}),
		} : {
			query,
			...(source === 'gameData' ? { addonGuids } : {}),
			limit: pageSize,
			offset: (page - 1) * pageSize,
			...(source !== 'wiki' && symbolKinds?.length ? { kinds: symbolKinds } : {}),
		};
		const remoteStartedAt = performance.now();
		const value = asRecord(await this.callTool(searchToolFor(source, mode), argumentsValue));
		if (sourceTrace) {
			sourceTrace.remoteRequests += 1;
			sourceTrace.remoteMs += performance.now() - remoteStartedAt;
			recordVisitedPage(sourceTrace, page);
		}
		const results = normalizeSearchPage(source, value, mode);
		const currentPage: CachedSearchPage = {
			results,
			total: asNumber(value.total, results.length),
			...(typeof value.nextCursor === 'string' && value.nextCursor.length > 0
				? { nextCursor: value.nextCursor }
				: {}),
			...(mode === 'text' && value.stats && typeof value.stats === 'object' ? { stats: asRecord(value.stats) } : {}),
			...(value.totalsByAddon && typeof value.totalsByAddon === 'object'
				? { addonTotals: numberRecord(value.totalsByAddon) }
				: {}),
		};
		if (sourceTrace && currentPage.addonTotals) {
			sourceTrace.addonTotals = currentPage.addonTotals;
		}
		if (sourceTrace && mode === 'text' && currentPage.stats) {
			sourceTrace.textStats = currentPage.stats;
		}
		pages.set(page, currentPage);
		while (pages.size > maxCachedPagesPerSearch) {
			const oldest = pages.keys().next().value;
			if (oldest !== 1) {
				if (oldest !== undefined) {
					pages.delete(oldest);
				}
				continue;
			}
			const remaining = pages.keys();
			remaining.next();
			const nextOldest = remaining.next().value;
			if (nextOldest === undefined) {
				break;
			}
			pages.delete(nextOldest);
		}
		if (sourceTrace) {
			sourceTrace.cacheSize = pages.size;
		}
		return currentPage;
	}

	private request(method: string, params: unknown): Promise<JsonRpcResult> {
		const activeProcess = this.process;
		if (!activeProcess || activeProcess.killed) {
			return Promise.reject(new Error('The Reforger search server is not running.'));
		}
		const id = this.nextRequestId++;
		const request = { jsonrpc: '2.0', id, method, params };
		const message = `${JSON.stringify(request)}\n`;
		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pending.delete(id);
				reject(new Error('The Reforger search server did not respond in time.'));
				this.dispose();
			}, requestTimeoutMs);
			this.pending.set(id, { resolve, reject, timeout });
			activeProcess.stdin.write(message, 'utf8');
		});
	}

	private notify(method: string, params: unknown): void {
		const activeProcess = this.process;
		if (!activeProcess || activeProcess.killed) {
			return;
		}
		activeProcess.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method, params })}\n`, 'utf8');
	}

	private consumeOutput(chunk: Buffer): void {
		this.receiveBuffer = Buffer.concat([this.receiveBuffer, chunk]);
		while (true) {
			const lineEnd = this.receiveBuffer.indexOf(0x0a);
			if (lineEnd < 0) {
				return;
			}
			const body = this.receiveBuffer.subarray(0, lineEnd).toString('utf8').trim();
			this.receiveBuffer = this.receiveBuffer.subarray(lineEnd + 1);
			if (!body) {
				continue;
			}
			try {
				this.resolveMessage(JSON.parse(body) as JsonRpcResult & { id?: number });
			} catch (error) {
				this.failPending(error instanceof Error ? error : new Error(String(error)));
				return;
			}
		}
	}

	private resolveMessage(message: JsonRpcResult & { id?: number }): void {
		if (message.id === undefined) {
			return;
		}
		const request = this.pending.get(message.id);
		if (!request) {
			return;
		}
		this.pending.delete(message.id);
		clearTimeout(request.timeout);
		if (message.error) {
			request.reject(new Error(message.error.message ?? 'MCP request failed.'));
		} else {
			request.resolve(message);
		}
	}

	private failPending(error: Error): void {
		for (const request of this.pending.values()) {
			clearTimeout(request.timeout);
			request.reject(error);
		}
		this.pending.clear();
	}
}

export function searchToolFor(source: SearchSource, mode: SearchMode = 'semantic'): string {
	if (source === 'wiki') {
		return 'search_official_wiki';
	}
	if (mode === 'text') {
		return source === 'gameData' ? 'search_game_data_text' : 'search_workspace_text';
	}
	return source === 'gameData'
		? 'search_game_data_symbols'
		: 'search_workspace_symbols';
}

function paginationModeFor(mode: SearchMode, sources: readonly SearchSource[]): 'offset' | 'cursor' | 'mixed' {
	if (mode === 'semantic' || sources.every(source => source === 'wiki')) {
		return 'offset';
	}
	return sources.includes('wiki') ? 'mixed' : 'cursor';
}

export function normalizeSearchPage(source: SearchSource, value: unknown, mode: SearchMode = 'semantic'): SearchHit[] {
	const results = asRecord(value).results;
	if (!Array.isArray(results)) {
		return [];
	}
	return results.flatMap((entry, index) => {
		const hit = asRecord(entry);
		if (source === 'wiki') {
			return normalizeWikiHit(hit, index);
		}
		return mode === 'text' ? normalizeTextHit(source, hit, index) : normalizeSymbolHit(source, hit, index);
	});
}

export function formatSearchKind(kind: string): string {
	if (kind === 'method' || kind === 'constructor' || kind === 'destructor') {
		return 'function';
	}
	return kind.replace(/([a-z])([A-Z])/g, '$1 $2');
}

function normalizeSymbolHit(source: SearchSource, hit: RecordValue, index: number): SearchHit[] {
	const name = asString(hit.name, 'Unnamed symbol');
	const kind = formatSearchKind(asString(hit.kind, 'Symbol'));
	const qualifiedName = asString(hit.qualifiedName, name);
	const relativePath = asString(hit.relativePath, 'Unknown source');
	const range = asRecord(hit.declarationRange);
	const line = asNumber(range.startLine, 0);
	const signature = asString(hit.signature, qualifiedName);
	const documentation = typeof hit.documentationSummary === 'string' ? hit.documentationSummary : '';
	const excerpt = documentation ? `${signature}\n\n${documentation}` : signature;
	const readInput = asRecord(hit.readSourceInput);
	const addonGuid = asOptionalString(hit.addonGuid);
	if (!readInput.relativePath) {
		return [];
	}
	return [{
		id: `${source}-${addonGuid ? `${addonGuid}-` : ''}${index}-${asString(hit.symbolRef, name)}`,
		source,
		kind: 'symbol',
		title: name,
		detail: kind,
		path: `${relativePath}:${line}`,
		excerpt,
		matchKind: asString(hit.matchKind, 'symbol'),
		...(typeof hit.sourceUri === 'string' ? { sourceUri: hit.sourceUri } : {}),
		selectionStartLine: asNumber(asRecord(hit.selectionRange).startLine, line),
		selectionEndLine: asNumber(asRecord(hit.selectionRange).endLine, line),
		readInput,
		...(addonGuid ? { addonGuid } : {}),
		...(asOptionalString(hit.addonLabel) ? { addonLabel: asOptionalString(hit.addonLabel) } : {}),
	}];
}

function normalizeTextHit(source: SearchSource, hit: RecordValue, index: number): SearchHit[] {
	const relativePath = asString(hit.relativePath, 'Unknown source');
	const range = asRecord(hit.matchRange);
	const startLine = asNumber(range.startLine, 0);
	const excerpt = asString(hit.excerpt, '');
	const matchText = asString(hit.matchText, '');
	const readInput = asRecord(hit.readSourceInput);
	const addonGuid = asOptionalString(hit.addonGuid);
	if (!readInput.relativePath) {
		return [];
	}
	const matchStart = matchText ? excerpt.indexOf(matchText) : -1;
	return [{
		id: `${source}-text-${addonGuid ? `${addonGuid}-` : ''}${index}-${relativePath}-${startLine}`,
		source,
		kind: 'text',
		title: matchText || 'Text match',
		detail: 'text',
		path: `${relativePath}:${startLine}`,
		excerpt,
		matchKind: 'text',
		selectionStartLine: startLine,
		selectionEndLine: asNumber(range.endLine, startLine),
		readInput,
		...(addonGuid ? { addonGuid } : {}),
		...(asOptionalString(hit.addonLabel) ? { addonLabel: asOptionalString(hit.addonLabel) } : {}),
		textMatchStart: matchStart >= 0 ? matchStart : asNumber(range.startCharacter, 0),
		textMatchLength: matchText.length,
	}];
}

function normalizeWikiHit(hit: RecordValue, index: number): SearchHit[] {
	const title = asString(hit.title, 'Official Wiki');
	const relativePath = asString(hit.relativePath, 'Official Wiki');
	const line = hit.matchedLine ?? hit.startLine;
	const readInput = asRecord(hit.readInput);
	if (!readInput.relativePath) {
		return [];
	}
	return [{
		id: `wiki-${index}-${relativePath}`,
		source: 'wiki',
		kind: 'documentation',
		title,
		detail: `${asString(hit.heading, 'Documentation')} · ${asString(hit.matchKind, 'match')}`,
		path: `${relativePath}:${typeof line === 'number' ? line : 0}`,
		excerpt: asString(hit.excerpt, ''),
		matchKind: asString(hit.matchKind, 'body'),
		selectionStartLine: asNumber(hit.matchedLine, asNumber(hit.excerptStartLine, asNumber(line, 0))),
		selectionEndLine: asNumber(hit.matchedLine, asNumber(hit.excerptEndLine, asNumber(line, 0))),
		sourceUrl: typeof hit.sourceUrl === 'string' ? hit.sourceUrl : undefined,
		readInput,
	}];
}

function searchErrorMessage(source: SearchSource, error: unknown): string {
	const label = source === 'wiki' ? 'Official Wiki' : source === 'gameData' ? 'Game Data' : 'workspace';
	return `${label} search unavailable: ${error instanceof Error ? error.message : String(error)}`;
}

function asRecord(value: unknown): RecordValue {
	return value && typeof value === 'object' && !Array.isArray(value) ? value as RecordValue : {};
}

function asString(value: unknown, fallback: string): string {
	return typeof value === 'string' && value.length > 0 ? value : fallback;
}

function asOptionalString(value: unknown): string | undefined {
	return typeof value === 'string' && value.length > 0 ? value : undefined;
}

function asNumber(value: unknown, fallback: number): number {
	return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function numberRecord(value: unknown): Record<string, number> {
	const record = asRecord(value);
	return Object.fromEntries(
		Object.entries(record)
			.filter((entry): entry is [string, number] => typeof entry[1] === 'number' && Number.isFinite(entry[1])),
	);
}

function isWithinRoot(root: string, candidate: string): boolean {
	const normalizedRoot = path.resolve(root).toLowerCase();
	const normalizedCandidate = candidate.toLowerCase();
	return normalizedCandidate === normalizedRoot
		|| normalizedCandidate.startsWith(`${normalizedRoot}${path.sep}`);
}
