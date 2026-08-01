import { access } from 'node:fs/promises';
import * as path from 'node:path';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';

export type SearchSource = 'workspace' | 'gameData' | 'wiki';

export interface SearchHit {
	id: string;
	source: SearchSource;
	kind: 'symbol' | 'documentation';
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
}

export interface SearchResponse {
	results: SearchHit[];
	warnings: string[];
	total: number;
	page: number;
	pageSize: number;
}

export interface SearchDocument {
	content: string;
	startLine: number;
	endLine: number;
}

export interface McpSearchClientOptions {
	serverPath: string;
	indexCache: string;
	workspaceScripts: string[];
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

interface RecordValue {
	[key: string]: unknown;
}

interface CachedSearchPage {
	results: SearchHit[];
	total: number;
	nextCursor?: string;
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
		sources: SearchSource[],
		pageSize: number,
		page: number,
	): Promise<SearchResponse> {
		await this.start();
		const normalizedPageSize = Math.min(100, Math.max(1, Number.isFinite(pageSize) ? Math.floor(pageSize) : 25));
		const requestedPage = Math.max(1, Number.isFinite(page) ? Math.floor(page) : 1);
		if (sources.length === 0) {
			return { results: [], warnings: [], total: 0, page: 1, pageSize: normalizedPageSize };
		}
		const responses = await Promise.all(sources.map(async source => {
			try {
				const value = await this.searchPage(query, source, sourcePageSize, 1);
				return { source, value, warning: undefined };
			} catch (error) {
				return { source, value: undefined, warning: searchErrorMessage(source, error) };
			}
		}));

		const warnings = responses
			.map(response => response.warning)
			.filter((warning): warning is string => warning !== undefined);
		if (responses.every(response => response.value === undefined) && warnings.length === responses.length) {
			throw new Error(warnings.join(' '));
		}
		const total = responses.reduce((sum, response) => sum + (response.value?.total ?? 0), 0);
		const totalPages = Math.max(1, Math.ceil(total / normalizedPageSize));
		const normalizedPage = Math.min(requestedPage, totalPages);
		const pageStart = (normalizedPage - 1) * normalizedPageSize;
		const pageEnd = pageStart + normalizedPageSize;
		const results: SearchHit[] = [];
		let sourceOffset = 0;
		for (const response of responses) {
			const sourceTotal = response.value?.total ?? 0;
			const sourceStart = Math.max(0, pageStart - sourceOffset);
			const sourceEnd = Math.min(sourceTotal, pageEnd - sourceOffset);
			if (response.value && sourceStart < sourceEnd) {
				results.push(...await this.sourceRange(query, response.source, sourceStart, sourceEnd, sourceTotal));
			}
			sourceOffset += sourceTotal;
		}
		return { results, warnings, total, page: normalizedPage, pageSize: normalizedPageSize };
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
			'--index-cache',
			this.options.indexCache,
			'--official-wiki-root',
			this.options.officialWikiRoot,
			...this.options.workspaceScripts.flatMap(root => ['--workspace-scripts', root]),
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
		maxResults: number,
	): Promise<SearchHit[]> {
		const results: SearchHit[] = [];
		let sourceOffset = 0;
		for (let pageNumber = 1; pageNumber <= Math.max(1, Math.floor(maxResults)); pageNumber += 1) {
			const page = await this.searchPage(query, source, sourcePageSize, pageNumber);
			const pageEnd = sourceOffset + page.results.length;
			const resultStart = Math.max(0, start - sourceOffset);
			const resultEnd = Math.min(page.results.length, end - sourceOffset);
			if (resultStart < resultEnd) {
				results.push(...page.results.slice(resultStart, resultEnd));
			}
			sourceOffset = pageEnd;
			if (sourceOffset >= end || !page.nextCursor || page.results.length === 0) {
				break;
			}
		}
		return results;
	}

	private async searchPage(
		query: string,
		source: SearchSource,
		pageSize: number,
		page: number,
	): Promise<CachedSearchPage> {
		const cacheKey = `${source}\u0000${pageSize}\u0000${query}`;
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

		let previousPage: CachedSearchPage | undefined;
		for (let pageNumber = 1; pageNumber <= page; pageNumber += 1) {
			const cached = pages.get(pageNumber);
			if (cached) {
				previousPage = cached;
				continue;
			}
			if (pageNumber > 1 && !previousPage?.nextCursor) {
				return { results: [], total: previousPage?.total ?? 0 };
			}
			const argumentsValue: Record<string, unknown> = { query, limit: pageSize };
			if (previousPage?.nextCursor) {
				argumentsValue.cursor = previousPage.nextCursor;
			}
			const value = asRecord(await this.callTool(searchToolFor(source), argumentsValue));
			const results = normalizeSearchPage(source, value);
			const currentPage: CachedSearchPage = {
				results,
				total: asNumber(value.total, results.length),
				...(typeof value.nextCursor === 'string' && value.nextCursor.length > 0
					? { nextCursor: value.nextCursor }
					: {}),
			};
			pages.set(pageNumber, currentPage);
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
			previousPage = currentPage;
		}
		return previousPage ?? { results: [], total: 0 };
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

export function searchToolFor(source: SearchSource): string {
	return source === 'wiki' ? 'search_official_wiki' : source === 'gameData'
		? 'search_game_data_symbols'
		: 'search_workspace_symbols';
}

export function normalizeSearchPage(source: SearchSource, value: unknown): SearchHit[] {
	const results = asRecord(value).results;
	if (!Array.isArray(results)) {
		return [];
	}
	return results.flatMap((entry, index) => {
		const hit = asRecord(entry);
		if (source === 'wiki') {
			return normalizeWikiHit(hit, index);
		}
		return normalizeSymbolHit(source, hit, index);
	});
}

function normalizeSymbolHit(source: SearchSource, hit: RecordValue, index: number): SearchHit[] {
	const name = asString(hit.name, 'Unnamed symbol');
	const kind = asString(hit.kind, 'Symbol');
	const qualifiedName = asString(hit.qualifiedName, name);
	const relativePath = asString(hit.relativePath, 'Unknown source');
	const range = asRecord(hit.declarationRange);
	const line = asNumber(range.startLine, 0);
	const signature = asString(hit.signature, qualifiedName);
	const documentation = typeof hit.documentationSummary === 'string' ? hit.documentationSummary : '';
	const excerpt = documentation ? `${signature}\n\n${documentation}` : signature;
	const readInput = asRecord(hit.readSourceInput);
	if (!readInput.relativePath) {
		return [];
	}
	return [{
		id: `${source}-${index}-${asString(hit.symbolRef, name)}`,
		source,
		kind: 'symbol',
		title: name,
		detail: `${kind} · ${qualifiedName}`,
		path: `${relativePath}:${line}`,
		excerpt,
		matchKind: asString(hit.matchKind, 'symbol'),
		...(typeof hit.sourceUri === 'string' ? { sourceUri: hit.sourceUri } : {}),
		selectionStartLine: asNumber(asRecord(hit.selectionRange).startLine, line),
		selectionEndLine: asNumber(asRecord(hit.selectionRange).endLine, line),
		readInput,
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

function asNumber(value: unknown, fallback: number): number {
	return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function isWithinRoot(root: string, candidate: string): boolean {
	const normalizedRoot = path.resolve(root).toLowerCase();
	const normalizedCandidate = candidate.toLowerCase();
	return normalizedCandidate === normalizedRoot
		|| normalizedCandidate.startsWith(`${normalizedRoot}${path.sep}`);
}
