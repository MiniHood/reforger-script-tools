#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { performance } from 'node:perf_hooks';

const DEFAULT_JSON_OUT = 'tools/reports/mcp-runtime-performance.report.json';
const DEFAULT_MARKDOWN_OUT = 'tools/reports/mcp-runtime-performance.report.md';
const GAME_DATA_INITIALIZATION_BUDGET_MS = 120_000;
const SEMANTIC_BUDGET_MS = 5_000;
const TEXT_SEARCH_BUDGET_MS = 30_000;
const WIKI_BUDGET_MS = 5_000;

async function runReport(configuration) {
	const client = new McpClient(
		resolve(configuration.server),
		serverArguments(configuration),
		configuration.timeoutMs,
	);
	const operations = [];
	let listedToolCount = 0;
	let initialize;
	let toolsList;
	let processToInitializeMs = 0;
	let concurrency = { operation: undefined, probes: [] };
	try {
		initialize = await measure(() => client.initialize());
		processToInitializeMs = client.processElapsedMs();
		const listed = await measure(() => client.listTools());
		toolsList = {
			elapsedMs: round(listed.elapsedMs),
			responseBytes: responseBytes(listed.value),
		};
		const toolNames = new Set(
			(listed.value.tools ?? [])
				.map(tool => tool.name)
				.filter(name => typeof name === 'string' && !name.startsWith('workbench_')),
		);
		listedToolCount = toolNames.size;
		const runner = new ScenarioRunner(client, toolNames, operations, configuration);
		await runScenarios(runner);
		for (const name of toolNames) {
			if (!runner.seen.has(name)) {
				runner.skip(name, 'No report scenario exists for this newly listed non-Workbench tool.');
			}
		}
		concurrency = await runner.runConcurrencyProbe();
	} catch (error) {
		operations.push({
			name: error.phase ?? 'runner',
			family: 'Protocol',
			status: 'runner-error',
			reason: sanitizeMessage(error.message),
			firstMs: 0,
			warm: emptyDistribution(),
			responseBytes: 0,
		});
	} finally {
		await client.close();
	}

	const coldProcess = await measureColdProcesses(configuration);
	const exercised = operations.filter(operation => !['skipped', 'missing'].includes(operation.status)).length;
	const skipped = operations.filter(operation => ['skipped', 'missing'].includes(operation.status)).length;
	const correctnessFailures = operations.filter(operation => ['tool-error', 'runner-error', 'unstable-result'].includes(operation.status)).length;
	const overBudget = operations.reduce(
		(total, operation) => total
			+ (operation.status === 'over-budget' ? 1 : 0)
			+ (operation.variants ?? []).filter(variant => variant.status === 'over-budget').length,
		0,
	);
	const concurrencyFailures = concurrency.probes.reduce((total, probe) => total + probe.failed, 0);
	const failed = correctnessFailures + concurrencyFailures + coldProcess.failed + (configuration.enforceBudgets ? overBudget : 0);
	const verdict = failed > 0 ? 'fail' : skipped > 0 ? 'partial' : 'pass';
	return {
		schemaVersion: 1,
		generatedAt: new Date().toISOString(),
		scope: 'non-workbench',
		server: resolve(configuration.server),
		configuration: {
			samples: configuration.samples,
			concurrencyLevels: configuration.concurrencyLevels,
			timeoutMs: configuration.timeoutMs,
			requireAll: configuration.requireAll,
			enforceBudgets: configuration.enforceBudgets,
			coldSamples: configuration.coldSamples,
			workspaceRootCount: configuration.workspaceScripts.length,
			dependencyProjectCount: configuration.dependencyProjects.length,
			externalIndexMode: configuration.externalIndexMode,
		},
		protocol: {
			initializeMs: round(initialize?.elapsedMs ?? 0),
			processToInitializeMs: round(processToInitializeMs),
			initializeResponseBytes: responseBytes(initialize?.value),
			toolsListMs: toolsList?.elapsedMs ?? 0,
			toolsListResponseBytes: toolsList?.responseBytes ?? 0,
		},
		coverage: { listed: listedToolCount, exercised, skipped, failed, overBudget },
		operations,
		concurrency,
		coldProcess,
		verdict,
	};
}

class ScenarioRunner {
	constructor(client, toolNames, operations, configuration) {
		this.client = client;
		this.toolNames = toolNames;
		this.operations = operations;
		this.configuration = configuration;
		this.seen = new Set();
		this.concurrencyCandidate = undefined;
	}

	async exercise(name, argumentsValue, settings = {}) {
		this.seen.add(name);
		if (!this.toolNames.has(name)) {
			return undefined;
		}
		const first = await measure(() => this.client.callTool(name, argumentsValue));
		const firstSummary = summarizeResult(first.value);
		if (firstSummary.isError && (settings.unavailableCodes ?? []).includes(firstSummary.code)) {
			this.operations.push({
				...skippedOperation(name, `Tool reported ${firstSummary.code ?? 'an unavailable source'}.`),
				firstMs: round(first.elapsedMs),
				responseBytes: responseBytes(first.value),
				code: firstSummary.code,
			});
			return undefined;
		}
		const warmSamples = [];
		const fingerprints = [resultFingerprint(first.value)];
		const responseSizes = [responseBytes(first.value)];
		let last = first.value;
		for (let iteration = 0; iteration < this.configuration.samples; iteration += 1) {
			const sample = await measure(() => this.client.callTool(name, argumentsValue));
			warmSamples.push(sample.elapsedMs);
			fingerprints.push(resultFingerprint(sample.value));
			responseSizes.push(responseBytes(sample.value));
			last = sample.value;
		}
		const summary = summarizeResult(last);
		const budgetMs = operationBudget(name);
		const stable = fingerprints.every(fingerprint => fingerprint === fingerprints[0]);
		const status = summary.isError
			? 'tool-error'
			: !stable
				? 'unstable-result'
			: Math.max(first.elapsedMs, ...warmSamples) > budgetMs
				? 'over-budget'
				: 'pass';
		const operation = {
			name,
			family: toolFamily(name),
			status,
			firstMs: round(first.elapsedMs),
			warm: distribution(warmSamples),
			budgetMs,
			responseBytes: Math.max(...responseSizes),
			fingerprint: fingerprints.at(-1),
			stableFingerprint: stable,
			variants: [],
			reason: summary.isError ? toolErrorReason(summary.code) : undefined,
			...summary.public,
		};
		this.operations.push(operation);
		if (!summary.isError && !this.concurrencyCandidate && isConcurrencyCandidate(name)) {
			this.concurrencyCandidate = { name, argumentsValue };
		}
		return firstSummary.structured;
	}

	async variant(name, argumentsValue, scenario) {
		const operation = this.operations.find(candidate => candidate.name === name);
		if (!operation || ['skipped', 'missing'].includes(operation.status)) return undefined;
		const sample = await measure(() => this.client.callTool(name, argumentsValue));
		const summary = summarizeResult(sample.value);
		const budgetMs = operationBudget(name);
		const status = summary.isError ? 'tool-error' : sample.elapsedMs > budgetMs ? 'over-budget' : 'pass';
		operation.variants.push({
			scenario,
			status,
			elapsedMs: round(sample.elapsedMs),
			responseBytes: responseBytes(sample.value),
			fingerprint: resultFingerprint(sample.value),
			reason: summary.isError ? toolErrorReason(summary.code) : undefined,
			...summary.public,
		});
		if (status === 'tool-error') {
			operation.status = status;
		}
		return summary.structured;
	}

	skip(name, reason) {
		if (this.seen.has(name)) return;
		this.seen.add(name);
		if (this.toolNames.has(name)) this.operations.push(skippedOperation(name, reason));
	}

	skipMany(names, reason) {
		for (const name of names) this.skip(name, reason);
	}

	async runConcurrencyProbe() {
		const candidate = this.concurrencyCandidate;
		if (!candidate || this.configuration.concurrencyLevels.length === 0) {
			return { operation: undefined, probes: [] };
		}
		const probes = [];
		for (const requested of this.configuration.concurrencyLevels) {
			const started = performance.now();
			const calls = Array.from({ length: requested }, async () => {
				const sample = await measure(() => this.client.callTool(candidate.name, candidate.argumentsValue));
				return { elapsedMs: sample.elapsedMs, ok: sample.value?.isError !== true };
			});
			const results = await Promise.all(calls);
			const completed = results.filter(result => result.ok).length;
			probes.push({
				requested,
				completed,
				failed: requested - completed,
				elapsedMs: round(performance.now() - started),
				individual: distribution(results.map(result => result.elapsedMs)),
			});
		}
		return {
			operation: candidate.name,
			probes,
		};
	}
}

async function runScenarios(runner) {
	const gameStatus = await runner.exercise('game_data_status', {});
	const gameAvailable = gameStatus?.available === true;
	let gameSearch;
	let gameExamples;
	if (gameAvailable) {
		gameSearch = await runner.exercise('search_game_data_symbols', {
			query: runner.configuration.gameSymbolQuery,
			limit: 20,
		});
		const gameText = await runner.exercise('search_game_data_text', {
			query: runner.configuration.textQuery,
			limit: 20,
		});
		await runTextVariants(runner, 'search_game_data_text', gameText);
		gameExamples = await runner.exercise('search_game_data_examples', {
			topic: runner.configuration.exampleTopic,
			limit: 20,
		});
	} else {
		runner.skipMany([
			'search_game_data_symbols',
			'search_game_data_text',
			'search_game_data_examples',
		], 'Game Data status reports that the catalogue is unavailable.');
	}
	const gameResult = firstResult(gameSearch);
	if (gameResult?.symbolRef) {
		await runner.exercise('inspect_game_data_symbol', { symbolRef: gameResult.symbolRef });
		await runner.exercise('list_game_data_symbol_members', { symbolRef: gameResult.symbolRef, limit: 20 });
		await runner.exercise('query_game_data_symbol_relationships', { symbolRef: gameResult.symbolRef, limit: 20 });
	} else {
		runner.skipMany([
			'inspect_game_data_symbol',
			'list_game_data_symbol_members',
			'query_game_data_symbol_relationships',
		], gameAvailable ? 'The configured Game Data symbol query returned no symbol handoff.' : 'Game Data is unavailable.');
	}
	if (gameResult?.readSourceInput) {
		await runner.exercise('read_game_data_source', gameResult.readSourceInput);
		const exampleReadInput = firstResult(gameExamples)?.readSourceInput;
		if (exampleReadInput) await runner.variant('read_game_data_source', exampleReadInput, 'example-handoff');
	} else if (firstResult(gameExamples)?.readSourceInput) {
		await runner.exercise('read_game_data_source', firstResult(gameExamples).readSourceInput);
	} else {
		runner.skip('read_game_data_source', gameAvailable ? 'The configured Game Data symbol query returned no source-read handoff.' : 'Game Data is unavailable.');
	}

	const workspaceSearch = await runner.exercise(
		'search_workspace_symbols',
		{ query: runner.configuration.workspaceSymbolQuery, limit: 20 },
		{ unavailableCodes: ['workspace_unavailable', 'workspace_index_unavailable'] },
	);
	const workspaceAvailable = workspaceSearch !== undefined;
	const workspaceText = await runner.exercise(
		'search_workspace_text',
		{ query: runner.configuration.textQuery, limit: 20 },
		{ unavailableCodes: ['workspace_unavailable', 'workspace_index_unavailable'] },
	);
	if (workspaceText !== undefined) await runTextVariants(runner, 'search_workspace_text', workspaceText);
	const workspaceResult = firstResult(workspaceSearch);
	if (workspaceResult?.symbolRef) {
		await runner.exercise('inspect_workspace_symbol', { symbolRef: workspaceResult.symbolRef });
		await runner.exercise('list_workspace_symbol_members', { symbolRef: workspaceResult.symbolRef, limit: 20 });
		await runner.exercise('query_workspace_symbol_relationships', { symbolRef: workspaceResult.symbolRef, limit: 20 });
	} else {
		runner.skipMany([
			'inspect_workspace_symbol',
			'list_workspace_symbol_members',
			'query_workspace_symbol_relationships',
		], workspaceAvailable ? 'The configured workspace symbol query returned no symbol handoff.' : 'Workspace source is unavailable.');
	}
	if (workspaceResult?.readSourceInput) {
		await runner.exercise('read_workspace_source', workspaceResult.readSourceInput);
	} else {
		runner.skip('read_workspace_source', workspaceAvailable ? 'The configured workspace symbol query returned no source-read handoff.' : 'Workspace source is unavailable.');
	}

	const wikiStatus = await runner.exercise('official_wiki_status', {});
	let wikiSearch;
	if (wikiStatus?.available === true) {
		wikiSearch = await runner.exercise('search_official_wiki', {
			query: runner.configuration.wikiQuery,
			limit: 20,
		});
	} else {
		runner.skip('search_official_wiki', 'Official Wiki status reports that the corpus is unavailable.');
	}
	const wikiResult = firstResult(wikiSearch);
	if (wikiResult?.readInput) {
		await runner.exercise('read_official_wiki', wikiResult.readInput);
	} else {
		runner.skip('read_official_wiki', wikiStatus?.available === true ? 'The configured Wiki query returned no read handoff.' : 'Official Wiki is unavailable.');
	}
}

async function runTextVariants(runner, name, firstPage) {
	await runner.variant(name, {
		query: runner.configuration.broadTextQuery,
		limit: 20,
	}, 'broad-literal');
	await runner.variant(name, {
		query: runner.configuration.regexQuery,
		useRegex: true,
		limit: 20,
	}, 'regular-expression');
	if (firstPage?.nextCursor) {
		await runner.variant(name, {
			query: runner.configuration.textQuery,
			limit: 20,
			cursor: firstPage.nextCursor,
		}, 'pagination');
	}
}

async function measureColdProcesses(configuration) {
	const initializeSamples = [];
	const statusSamples = [];
	let availableCount = 0;
	let failed = 0;
	for (let iteration = 0; iteration < configuration.coldSamples; iteration += 1) {
		const client = new McpClient(resolve(configuration.server), serverArguments(configuration), configuration.timeoutMs);
		try {
			await client.initialize();
			initializeSamples.push(client.processElapsedMs());
			const listed = await client.listTools();
			if ((listed.tools ?? []).some(tool => tool.name === 'game_data_status')) {
				const status = await measure(() => client.callTool('game_data_status', {}));
				if (status.value?.isError === true) {
					failed += 1;
				} else {
					statusSamples.push(status.elapsedMs);
					if (status.value?.structuredContent?.available === true) availableCount += 1;
				}
			}
		} catch {
			failed += 1;
		} finally {
			await client.close();
		}
	}
	return {
		count: configuration.coldSamples,
		processToInitialize: distribution(initializeSamples),
		gameDataStatus: distribution(statusSamples),
		gameDataAvailable: availableCount,
		failed,
	};
}

class McpClient {
	constructor(command, argumentsValue, timeoutMs) {
		this.processStartedAt = performance.now();
		this.timeoutMs = timeoutMs;
		this.nextId = 1;
		this.pending = new Map();
		this.stderrTail = '';
		this.child = spawn(command, argumentsValue, {
			stdio: ['pipe', 'pipe', 'pipe'],
			windowsHide: true,
		});
		this.child.stderr.setEncoding('utf8');
		this.child.stderr.on('data', chunk => {
			this.stderrTail = (this.stderrTail + chunk).slice(-2048);
		});
		this.child.on('error', error => this.rejectPending(error));
		this.child.on('exit', code => {
			if (this.pending.size > 0) this.rejectPending(new Error(`MCP server exited with code ${code}.`));
		});
		this.lines = createInterface({ input: this.child.stdout });
		this.lines.on('line', line => this.acceptLine(line));
	}

	async initialize() {
		const value = await this.request('initialize', {
			protocolVersion: '2025-11-25',
			capabilities: {},
			clientInfo: { name: 'reforger-mcp-runtime-report', version: '1.0.0' },
		}, 'initialize');
		this.send({ jsonrpc: '2.0', method: 'notifications/initialized' });
		return value;
	}

	listTools() {
		return this.request('tools/list', {}, 'tools/list');
	}

	callTool(name, argumentsValue) {
		return this.request('tools/call', { name, arguments: argumentsValue }, name);
	}

	processElapsedMs() {
		return performance.now() - this.processStartedAt;
	}

	request(method, params, phase) {
		const id = this.nextId++;
		return new Promise((resolveRequest, reject) => {
			const timer = setTimeout(() => {
				this.pending.delete(id);
				reject(Object.assign(new Error(`MCP request exceeded ${this.timeoutMs} ms.`), { phase }));
			}, this.timeoutMs);
			this.pending.set(id, { resolveRequest, reject, timer, phase });
			this.send({ jsonrpc: '2.0', id, method, params });
		});
	}

	send(message) {
		this.child.stdin.write(`${JSON.stringify(message)}\n`);
	}

	acceptLine(line) {
		let message;
		try {
			message = JSON.parse(line);
		} catch {
			return;
		}
		const pending = this.pending.get(message.id);
		if (!pending) return;
		this.pending.delete(message.id);
		clearTimeout(pending.timer);
		if (message.error) {
			pending.reject(Object.assign(new Error(message.error.message ?? 'MCP protocol error.'), {
				phase: pending.phase,
				code: message.error.code,
			}));
		} else {
			pending.resolveRequest(message.result);
		}
	}

	rejectPending(error) {
		for (const pending of this.pending.values()) {
			clearTimeout(pending.timer);
			pending.reject(error);
		}
		this.pending.clear();
	}

	async close() {
		this.lines?.close();
		this.child.stdin.end();
		if (this.child.exitCode !== null) return;
		const exited = new Promise(resolveExit => this.child.once('exit', resolveExit));
		this.child.kill();
		await Promise.race([
			exited,
			new Promise(resolveTimeout => setTimeout(resolveTimeout, 250)),
		]);
	}
}

function parseArguments(args) {
	const parsed = {
		serverPrefixArgs: [],
		workspaceScripts: [],
		dependencyProjects: [],
		externalIndexMode: 'loaded',
		samples: 7,
		coldSamples: 3,
		concurrencyLevels: [1, 4, 8],
		timeoutMs: 120_000,
		jsonOut: DEFAULT_JSON_OUT,
		markdownOut: DEFAULT_MARKDOWN_OUT,
		gameSymbolQuery: 'SCR_BaseGameMode',
		workspaceSymbolQuery: 'SCR_',
		textQuery: 'SCR_BaseGameMode',
		broadTextQuery: 'class',
		regexQuery: '\\bclass\\s+[A-Za-z_][A-Za-z0-9_]*',
		wikiQuery: 'replication',
		exampleTopic: 'replication',
		requireAll: false,
		enforceBudgets: false,
	};
	for (let index = 0; index < args.length; index += 1) {
		const argument = args[index];
		const value = () => {
			const next = args[++index];
			if (!next) usage(`Missing value for ${argument}.`);
			return next;
		};
		switch (argument) {
			case '--server': parsed.server = value(); break;
			case '--server-prefix-arg': parsed.serverPrefixArgs.push(value()); break;
			case '--addon-index-storage': parsed.addonIndexStorage = value(); break;
			case '--addon-source-inventory': parsed.addonSourceInventory = value(); break;
			case '--external-index-mode': parsed.externalIndexMode = value(); break;
			case '--workspace-scripts': parsed.workspaceScripts.push(value()); break;
			case '--dependency-project': parsed.dependencyProjects.push(value()); break;
			case '--official-wiki-root': parsed.officialWikiRoot = value(); break;
			case '--samples': parsed.samples = nonNegativeInteger(value(), argument); break;
			case '--cold-samples': parsed.coldSamples = nonNegativeInteger(value(), argument); break;
			case '--concurrency': parsed.concurrencyLevels = boundedConcurrencyLevels(value(), argument); break;
			case '--concurrency-levels': parsed.concurrencyLevels = boundedConcurrencyLevels(value(), argument); break;
			case '--timeout-ms': parsed.timeoutMs = positiveInteger(value(), argument); break;
			case '--json-out': parsed.jsonOut = value(); break;
			case '--markdown-out': parsed.markdownOut = value(); break;
			case '--game-symbol-query': parsed.gameSymbolQuery = value(); break;
			case '--workspace-symbol-query': parsed.workspaceSymbolQuery = value(); break;
			case '--text-query': parsed.textQuery = value(); break;
			case '--broad-text-query': parsed.broadTextQuery = value(); break;
			case '--regex-query': parsed.regexQuery = value(); break;
			case '--wiki-query': parsed.wikiQuery = value(); break;
			case '--example-topic': parsed.exampleTopic = value(); break;
			case '--require-all': parsed.requireAll = true; break;
			case '--enforce-budgets': parsed.enforceBudgets = true; break;
			case '--help': usage(); break;
			default: usage(`Unknown argument: ${argument}`);
		}
	}
	if (!parsed.server) usage('--server is required.');
	if (!['all', 'loaded', 'none'].includes(parsed.externalIndexMode)) {
		usage('--external-index-mode must be all, loaded, or none.');
	}
	return parsed;
}

function serverArguments(configuration) {
	const args = [...configuration.serverPrefixArgs, 'mcp'];
	if (configuration.addonSourceInventory) args.push('--addon-source-inventory', resolve(configuration.addonSourceInventory));
	if (configuration.addonIndexStorage) args.push('--addon-index-storage', resolve(configuration.addonIndexStorage));
	args.push('--external-index-mode', configuration.externalIndexMode);
	for (const root of configuration.workspaceScripts) args.push('--workspace-scripts', resolve(root));
	for (const project of configuration.dependencyProjects) args.push('--dependency-project', resolve(project));
	if (configuration.officialWikiRoot) args.push('--official-wiki-root', resolve(configuration.officialWikiRoot));
	return args;
}

function summarizeResult(result) {
	const structured = result?.structuredContent ?? {};
	return {
		isError: result?.isError === true,
		code: typeof structured.code === 'string' ? structured.code : undefined,
		message: typeof structured.message === 'string' ? structured.message : undefined,
		structured,
		public: {
			code: typeof structured.code === 'string' ? structured.code : undefined,
			returned: finiteNumber(structured.returned),
			total: finiteNumber(structured.total),
			truncated: typeof structured.truncated === 'boolean' ? structured.truncated : undefined,
			stats: numericObject(structured.stats),
		},
	};
}

function resultFingerprint(result) {
	const structured = redactContent(result?.structuredContent ?? {});
	return createHash('sha256').update(JSON.stringify(structured)).digest('hex').slice(0, 16);
}

function redactContent(value, key = '') {
	if (['content', 'excerpt', 'lineText', 'text'].includes(key)) return '<redacted>';
	if (/(?:ms|timings)$/i.test(key)) return '<volatile>';
	if (Array.isArray(value)) return value.map(item => redactContent(item));
	if (value && typeof value === 'object') {
		return Object.fromEntries(Object.entries(value).map(([childKey, child]) => [childKey, redactContent(child, childKey)]));
	}
	return value;
}

function responseBytes(result) {
	return result === undefined ? 0 : Buffer.byteLength(JSON.stringify(result));
}

function firstResult(page) {
	return Array.isArray(page?.results) ? page.results[0] : undefined;
}

function operationBudget(name) {
	if (name === 'game_data_status') return GAME_DATA_INITIALIZATION_BUDGET_MS;
	if (name.includes('_text')) return TEXT_SEARCH_BUDGET_MS;
	if (name.includes('official_wiki')) return WIKI_BUDGET_MS;
	return SEMANTIC_BUDGET_MS;
}

function toolFamily(name) {
	if (name.includes('official_wiki')) return 'Official Wiki';
	if (name.includes('workspace')) return name.includes('_text') ? 'Workspace text' : 'Workspace semantic/source';
	if (name.includes('_text')) return 'Game Data text';
	return 'Game Data semantic/source';
}

function isConcurrencyCandidate(name) {
	return ['search_game_data_symbols', 'search_workspace_symbols', 'search_official_wiki'].includes(name);
}

function skippedOperation(name, reason, status = 'skipped') {
	return {
		name,
		family: toolFamily(name),
		status,
		reason: sanitizeMessage(reason),
		firstMs: 0,
		warm: emptyDistribution(),
		responseBytes: 0,
	};
}

function distribution(values) {
	if (values.length === 0) return emptyDistribution();
	const sorted = [...values].sort((left, right) => left - right);
	return {
		count: sorted.length,
		minMs: round(sorted[0]),
		medianMs: round(percentile(sorted, 50)),
		p95Ms: round(percentile(sorted, 95)),
		maxMs: round(sorted.at(-1)),
	};
}

function emptyDistribution() {
	return { count: 0, minMs: 0, medianMs: 0, p95Ms: 0, maxMs: 0 };
}

function percentile(sorted, percentileValue) {
	return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * percentileValue / 100) - 1)];
}

async function measure(operation) {
	const started = performance.now();
	const value = await operation();
	return { elapsedMs: performance.now() - started, value };
}

function renderMarkdown(report) {
	const lines = [
		'# MCP Runtime Performance Report',
		'',
		'This source-free report exercises the live non-Workbench MCP catalogue through stdio. Timings are local wall-clock diagnostics, not portable benchmark gates.',
		'',
		'## Summary',
		'',
		`- Generated: ${report.generatedAt}`,
		`- Verdict: **${report.verdict}**`,
		`- API coverage: **${report.coverage.exercised} / ${report.coverage.listed}**`,
		`- Skipped or unavailable: ${report.coverage.skipped}`,
		`- Correctness/gated failures: ${report.coverage.failed}`,
		`- Over-budget operations: ${report.coverage.overBudget}${report.configuration.enforceBudgets ? ' (enforced)' : ' (trend only)'}`,
		`- Initialize: ${formatMs(report.protocol.initializeMs)}`,
		`- Process start to initialize: ${formatMs(report.protocol.processToInitializeMs)}`,
		`- tools/list: ${formatMs(report.protocol.toolsListMs)}`,
		'',
		'## API Scorecard',
		'',
		'| Tool | Family | Status | First | Warm median | Warm p95 | Max | Bytes | Budget |',
		'| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |',
	];
	for (const operation of report.operations) {
		lines.push(`| ${escapeMarkdown(operation.name)} | ${escapeMarkdown(operation.family)} | ${escapeMarkdown(operation.status)} | ${formatMs(operation.firstMs)} | ${formatMs(operation.warm.medianMs)} | ${formatMs(operation.warm.p95Ms)} | ${formatMs(operation.warm.maxMs)} | ${operation.responseBytes} | ${operation.budgetMs ? formatMs(operation.budgetMs) : ''} |`);
	}
	const variants = report.operations.flatMap(operation => (operation.variants ?? []).map(variant => ({ tool: operation.name, ...variant })));
	lines.push('', '## Scenario Variants', '');
	if (variants.length === 0) {
		lines.push('No additional handoff or search variants were available.');
	} else {
		lines.push('| Tool | Scenario | Status | Elapsed | Bytes | Returned | Total |', '| --- | --- | --- | ---: | ---: | ---: | ---: |');
		for (const variant of variants) {
			lines.push(`| ${escapeMarkdown(variant.tool)} | ${escapeMarkdown(variant.scenario)} | ${escapeMarkdown(variant.status)} | ${formatMs(variant.elapsedMs)} | ${variant.responseBytes} | ${variant.returned ?? ''} | ${variant.total ?? ''} |`);
		}
	}
	lines.push('', '## Coverage Notes', '');
	const incomplete = report.operations.filter(operation => operation.reason);
	if (incomplete.length === 0) {
		lines.push('Every listed non-Workbench tool completed its configured scenario.');
	} else {
		for (const operation of incomplete) lines.push(`- \`${operation.name}\`: ${operation.reason}`);
	}
	lines.push(
		'',
		'## Cold Process Samples',
		'',
		`- Processes: ${report.coldProcess.count}`,
		`- Start-to-initialize median / p95: ${formatMs(report.coldProcess.processToInitialize.medianMs)} / ${formatMs(report.coldProcess.processToInitialize.p95Ms)}`,
		`- First Game Data status median / p95: ${formatMs(report.coldProcess.gameDataStatus.medianMs)} / ${formatMs(report.coldProcess.gameDataStatus.p95Ms)}`,
		`- Game Data available: ${report.coldProcess.gameDataAvailable} / ${report.coldProcess.count}`,
		`- Failed samples: ${report.coldProcess.failed}`,
		'',
		'## Concurrency Probe',
		'',
		`- Operation: ${report.concurrency.operation ?? 'Unavailable'}`,
		'',
		'| Requested | Completed | Failed | Batch | Individual p95 |',
		'| ---: | ---: | ---: | ---: | ---: |',
	);
	for (const probe of report.concurrency.probes) {
		lines.push(`| ${probe.requested} | ${probe.completed} | ${probe.failed} | ${formatMs(probe.elapsedMs)} | ${formatMs(probe.individual.p95Ms)} |`);
	}
	lines.push(
		'',
		'## Interpretation',
		'',
		'- `first` is the first invocation in the primary MCP process after prerequisite status or search handoffs.',
		'- Cold samples use fresh processes but do not claim a cold operating-system filesystem cache.',
		'- `warm` summarizes repeated identical calls in the same process.',
		'- Response bytes include the complete MCP result, while this report stores no retrieved source or documentation text.',
		'- A skipped API means its required authority or handoff was unavailable; it is not counted as a fast success.',
		'',
	);
	return `${lines.join('\n')}\n`;
}

function writeReport(path, contents) {
	const absolute = resolve(path);
	mkdirSync(dirname(absolute), { recursive: true });
	writeFileSync(absolute, contents, 'utf8');
}

function numericObject(value) {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
	const entries = Object.entries(value).filter(([, item]) => typeof item === 'number' && Number.isFinite(item));
	return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function finiteNumber(value) {
	return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function sanitizeMessage(value) {
	return String(value ?? '').replace(/[A-Za-z]:\\[^\s]+/g, '<path>').slice(0, 300);
}

function toolErrorReason(code) {
	return code ? `Tool reported ${code}.` : 'Tool returned a structured error without a stable code.';
}

function formatMs(value) {
	return `${round(value)} ms`;
}

function escapeMarkdown(value) {
	return String(value).replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function round(value) {
	return Math.round((Number(value) || 0) * 100) / 100;
}

function positiveInteger(value, argument) {
	const parsed = Number.parseInt(value, 10);
	if (!Number.isInteger(parsed) || parsed < 1) usage(`${argument} must be a positive integer.`);
	return parsed;
}

function nonNegativeInteger(value, argument) {
	const parsed = Number.parseInt(value, 10);
	if (!Number.isInteger(parsed) || parsed < 0) usage(`${argument} must be a non-negative integer.`);
	return parsed;
}

function boundedConcurrencyLevels(value, argument) {
	const levels = value.split(',').map(item => positiveInteger(item.trim(), argument));
	if (levels.some(level => level > 8)) usage(`${argument} levels must be between 1 and 8.`);
	return [...new Set(levels)].sort((left, right) => left - right);
}

function usage(error) {
	if (error) process.stderr.write(`${error}\n\n`);
	process.stderr.write(
		'Usage: node tools/mcp-runtime-performance-report.mjs --server <executable> [options]\n' +
		'  Sources: --addon-index-storage <dir> --addon-source-inventory <json> --external-index-mode <all|loaded|none>\n' +
		'           --workspace-scripts <dir> (repeatable) --dependency-project <gproj> (repeatable) --official-wiki-root <dir>\n' +
		'  Sampling: --samples <n> --cold-samples <n> --concurrency-levels <1,4,8> --timeout-ms <ms>\n' +
		'  Gates: --require-all --enforce-budgets\n' +
		'  Queries: --game-symbol-query <text> --workspace-symbol-query <text> --text-query <text> --broad-text-query <text>\n' +
		'           --regex-query <pattern> --wiki-query <text> --example-topic <text>\n' +
		'  Output: --json-out <path> --markdown-out <path>\n',
	);
	process.exit(error ? 2 : 0);
}

const options = parseArguments(process.argv.slice(2));
const report = await runReport(options);
writeReport(options.jsonOut, `${JSON.stringify(report, null, 2)}\n`);
writeReport(options.markdownOut, renderMarkdown(report));
process.stdout.write(
	`MCP runtime report: ${report.verdict}; ${report.coverage.exercised}/${report.coverage.listed} non-Workbench tools exercised.\n` +
	`JSON: ${options.jsonOut}\nMarkdown: ${options.markdownOut}\n`,
);
if (report.coverage.failed > 0 || (options.requireAll && report.coverage.exercised !== report.coverage.listed)) {
	process.exitCode = 1;
}
