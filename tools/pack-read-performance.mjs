import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { performance } from 'node:perf_hooks';
import { resolve } from 'node:path';

async function main() {
	const options = parseArguments(process.argv.slice(2));
	const serverPath = resolve(options.server);
	const serverArguments = ['mcp'];
	if (options.indexCache) serverArguments.push('--index-cache', resolve(options.indexCache));
	if (options.addonIndexStorage) serverArguments.push('--addon-index-storage', resolve(options.addonIndexStorage));
	if (options.addonSourceInventory) serverArguments.push('--addon-source-inventory', resolve(options.addonSourceInventory));
	serverArguments.push('--external-index-mode', options.externalIndexMode);
	for (const project of options.dependencyProjects) serverArguments.push('--dependency-project', resolve(project));
	for (const root of options.workspaceScripts) serverArguments.push('--workspace-scripts', resolve(root));

	const client = new McpStdioClient(serverPath, serverArguments);
	const report = {
		schemaVersion: 1,
		server: serverPath,
		symbolQuery: options.symbolQuery,
		sourceAddonGuid: options.sourceAddonGuid,
		textQueries: options.textQueries,
		readRepeat: options.readRepeat,
		postSearchReadRepeat: options.postSearchReadRepeat,
		intervalMs: options.intervalMs,
		repeatedSearchBudgetMs: options.repeatedSearchBudgetMs,
		repeatedReadBudgetMs: options.repeatedReadBudgetMs,
		postSearchReadBudgetMs: options.postSearchReadBudgetMs,
	};

	try {
		report.initializeMs = round((await measure(() => client.initialize(options.timeoutMs))).elapsedMs);
		const status = await measure(() => client.callTool('game_data_status', {}, options.timeoutMs));
		report.gameDataStatusMs = round(status.elapsedMs);
		assertToolResult(status.value, 'game_data_status');

		const symbolSearch = await measure(() => client.callTool('search_game_data_symbols', {
			query: options.symbolQuery,
			addonGuids: [options.sourceAddonGuid],
			limit: 1,
		}, options.timeoutMs));
		report.symbolSearchMs = round(symbolSearch.elapsedMs);
		assertToolResult(symbolSearch.value, 'search_game_data_symbols');
		const hit = symbolSearch.value.structuredContent?.results?.[0];
		if (!hit?.readSourceInput?.addonGuid) {
			throw Object.assign(new Error('Symbol search did not return a packed Game Data source handoff.'), {
				phase: 'search_game_data_symbols',
			});
		}
		report.source = {
			addonGuid: hit.readSourceInput.addonGuid,
			relativePath: hit.readSourceInput.relativePath,
		};

		const readSource = () => client.callTool('read_game_data_source', {
			...hit.readSourceInput,
			lineCount: 500,
		}, options.timeoutMs);
		const readRuns = [];
		for (let index = 0; index < options.readRepeat; index += 1) {
			if (index > 0 && options.intervalMs > 0) await delay(options.intervalMs);
			const read = await measure(readSource);
			assertToolResult(read.value, 'read_game_data_source');
			readRuns.push({
				run: index + 1,
				elapsedMs: round(read.elapsedMs),
				contentCharacters: read.value.structuredContent?.content?.length ?? 0,
			});
		}
		report.sourceReads = readRuns;
		report.firstSourceReadMs = readRuns[0]?.elapsedMs;
		report.repeatedSourceReadMs = summarize(readRuns.slice(1).map(run => run.elapsedMs));

		const textSearches = [];
		for (const query of options.textQueries) {
			const search = await measure(() => client.callTool('search_game_data_text', {
				query,
				limit: 25,
			}, options.timeoutMs));
			assertToolResult(search.value, 'search_game_data_text');
			const payload = search.value.structuredContent ?? {};
			textSearches.push({
				query,
				elapsedMs: round(search.elapsedMs),
				sourceReadMs: payload.stats?.sourceReadMs,
				scanMs: payload.stats?.scanMs,
				filesConsidered: payload.stats?.filesConsidered,
				total: payload.total,
				truncated: payload.truncated,
			});
		}
		report.textSearches = textSearches;
		report.repeatedTextSearchMs = summarize(textSearches.slice(1).map(run => run.elapsedMs));
		report.repeatedTextSourceReadMs = summarize(
			textSearches.slice(1).map(run => run.sourceReadMs).filter(Number.isFinite),
		);
		const postSearchReads = [];
		for (let index = 0; index < options.postSearchReadRepeat; index += 1) {
			const read = await measure(readSource);
			assertToolResult(read.value, 'read_game_data_source');
			postSearchReads.push(round(read.elapsedMs));
		}
		report.postSearchSourceReadMs = summarize(postSearchReads);

		const failures = [];
		if (options.repeatedReadBudgetMs !== undefined
			&& report.repeatedSourceReadMs.median > options.repeatedReadBudgetMs) {
			failures.push('repeated-source-read');
		}
		if (options.repeatedSearchBudgetMs !== undefined
			&& report.repeatedTextSearchMs.median > options.repeatedSearchBudgetMs) {
			failures.push('repeated-text-search');
		}
		if (options.postSearchReadBudgetMs !== undefined
			&& report.postSearchSourceReadMs.median > options.postSearchReadBudgetMs) {
			failures.push('post-search-source-read');
		}
		report.verdict = failures.length === 0 ? 'pass' : 'over-budget';
		report.failedBudgets = failures;
		if (failures.length > 0) process.exitCode = 1;
	} catch (error) {
		report.verdict = error instanceof RequestTimeoutError ? 'timeout' : 'runner-error';
		report.failedPhase = error.phase ?? 'unknown';
		report.error = {
			name: error?.name ?? 'Error',
			message: error?.message ?? String(error),
		};
		process.exitCode = 1;
	} finally {
		report.stderrTail = client.stderrTail || undefined;
		client.close();
		process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
	}
}

class McpStdioClient {
	constructor(command, args) {
		this.child = spawn(command, args, {
			stdio: ['pipe', 'pipe', 'pipe'],
			windowsHide: true,
		});
		this.lines = createInterface({ input: this.child.stdout });
		this.messages = this.lines[Symbol.asyncIterator]();
		this.nextId = 1;
		this.stderrTail = '';
		this.child.stderr.setEncoding('utf8');
		this.child.stderr.on('data', chunk => {
			this.stderrTail = (this.stderrTail + chunk).slice(-2048);
		});
	}

	async initialize(timeoutMs) {
		const result = await this.request('initialize', {
			protocolVersion: '2025-11-25',
			capabilities: {},
			clientInfo: { name: 'reforger-pack-read-performance', version: '1.0.0' },
		}, timeoutMs, 'initialize');
		this.send({ jsonrpc: '2.0', method: 'notifications/initialized' });
		return result;
	}

	callTool(name, argumentsValue, timeoutMs) {
		return this.request('tools/call', { name, arguments: argumentsValue }, timeoutMs, name);
	}

	async request(method, params, timeoutMs, phase) {
		const id = this.nextId++;
		const deadline = performance.now() + timeoutMs;
		this.send({ jsonrpc: '2.0', id, method, params });
		while (true) {
			const next = await withTimeout(this.messages.next(), Math.max(1, deadline - performance.now()), phase, timeoutMs);
			if (next.done) throw Object.assign(new Error('MCP server closed stdout before responding.'), { phase });
			let message;
			try {
				message = JSON.parse(next.value);
			} catch {
				continue;
			}
			if (message.id !== id) continue;
			if (message.error) throw Object.assign(new Error(message.error.message ?? 'MCP request failed.'), { phase });
			return message.result;
		}
	}

	send(message) {
		this.child.stdin.write(`${JSON.stringify(message)}\n`);
	}

	close() {
		this.lines.close();
		this.child.stdin.end();
		this.child.kill();
	}
}

class RequestTimeoutError extends Error {
	constructor(phase, timeoutMs) {
		super(`MCP phase exceeded the runner timeout of ${timeoutMs} ms.`);
		this.name = 'RequestTimeoutError';
		this.phase = phase;
	}
}

function assertToolResult(result, phase) {
	if (!result?.isError) return;
	throw Object.assign(new Error(result.structuredContent?.message ?? `${phase} returned an error.`), { phase });
}

function withTimeout(promise, remainingMs, phase, configuredTimeoutMs) {
	return new Promise((resolvePromise, reject) => {
		const timer = setTimeout(
			() => reject(new RequestTimeoutError(phase, configuredTimeoutMs)),
			remainingMs,
		);
		promise.then(
			value => {
				clearTimeout(timer);
				resolvePromise(value);
			},
			error => {
				clearTimeout(timer);
				reject(error);
			},
		);
	});
}

async function measure(operation) {
	const started = performance.now();
	const value = await operation();
	return { elapsedMs: performance.now() - started, value };
}

function summarize(values) {
	if (values.length === 0) return { count: 0 };
	const sorted = [...values].sort((left, right) => left - right);
	return {
		count: values.length,
		minimum: round(sorted[0]),
		median: round(sorted[Math.floor(sorted.length / 2)]),
		maximum: round(sorted.at(-1)),
		mean: round(values.reduce((sum, value) => sum + value, 0) / values.length),
	};
}

function delay(milliseconds) {
	return new Promise(resolvePromise => setTimeout(resolvePromise, milliseconds));
}

function parseArguments(args) {
	const parsed = {
		workspaceScripts: [],
		dependencyProjects: [],
		textQueries: [],
		externalIndexMode: 'loaded',
		symbolQuery: 'SCR_BaseGameMode',
		sourceAddonGuid: '58D0FB3206B6F859',
		readRepeat: 9,
		postSearchReadRepeat: 5,
		intervalMs: 25,
		timeoutMs: 120000,
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
			case '--index-cache': parsed.indexCache = value(); break;
			case '--addon-index-storage': parsed.addonIndexStorage = value(); break;
			case '--addon-source-inventory': parsed.addonSourceInventory = value(); break;
			case '--dependency-project': parsed.dependencyProjects.push(value()); break;
			case '--workspace-scripts': parsed.workspaceScripts.push(value()); break;
			case '--external-index-mode': parsed.externalIndexMode = value(); break;
			case '--symbol-query': parsed.symbolQuery = value(); break;
			case '--source-addon-guid': parsed.sourceAddonGuid = value(); break;
			case '--text-query': parsed.textQueries.push(value()); break;
			case '--read-repeat': parsed.readRepeat = positiveInteger(value(), argument); break;
			case '--post-search-read-repeat': parsed.postSearchReadRepeat = positiveInteger(value(), argument); break;
			case '--interval-ms': parsed.intervalMs = nonNegativeInteger(value(), argument); break;
			case '--timeout-ms': parsed.timeoutMs = positiveInteger(value(), argument); break;
			case '--repeated-search-budget-ms': parsed.repeatedSearchBudgetMs = positiveNumber(value(), argument); break;
			case '--repeated-read-budget-ms': parsed.repeatedReadBudgetMs = positiveNumber(value(), argument); break;
			case '--post-search-read-budget-ms': parsed.postSearchReadBudgetMs = positiveNumber(value(), argument); break;
			case '--help': usage(); break;
			default: usage(`Unknown argument: ${argument}`);
		}
	}
	if (!parsed.server || (!parsed.indexCache && !parsed.addonIndexStorage)) {
		usage('--server and --index-cache or --addon-index-storage are required.');
	}
	if (parsed.textQueries.length < 2) usage('Provide at least two distinct --text-query values.');
	if (parsed.readRepeat < 2) usage('--read-repeat must be at least 2.');
	if (!['all', 'loaded', 'none'].includes(parsed.externalIndexMode)) {
		usage('--external-index-mode must be all, loaded, or none.');
	}
	return parsed;
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

function positiveNumber(value, argument) {
	const parsed = Number.parseFloat(value);
	if (!Number.isFinite(parsed) || parsed <= 0) usage(`${argument} must be positive.`);
	return parsed;
}

function usage(error) {
	if (error) process.stderr.write(`${error}\n\n`);
	process.stderr.write(
		'Usage: node tools/pack-read-performance.mjs --server <exe> (--index-cache <file> | --addon-index-storage <dir>) --text-query <query> --text-query <query> [options]\n' +
		'  Scope: --addon-source-inventory <json> --dependency-project <gproj> --workspace-scripts <root> --external-index-mode <all|loaded|none>\n' +
		'  Reads: --symbol-query <query> --source-addon-guid <guid> --read-repeat <n> --post-search-read-repeat <n> --interval-ms <ms>\n' +
		'  Verdict: --repeated-search-budget-ms <ms> --repeated-read-budget-ms <ms> --post-search-read-budget-ms <ms> --timeout-ms <ms>\n',
	);
	process.exit(error ? 2 : 0);
}

function round(value) {
	return Math.round(value * 100) / 100;
}

await main();
