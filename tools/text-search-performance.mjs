import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { performance } from 'node:perf_hooks';
import { resolve } from 'node:path';

async function main() {
	const options = parseArguments(process.argv.slice(2));
	const serverPath = resolve(options.server);
	const serverArguments = ['mcp'];
	if (options.source === 'game-data') {
		serverArguments.push('--index-cache', resolve(options.indexCache));
	} else {
		for (const root of options.workspaceScripts) {
			serverArguments.push('--workspace-scripts', resolve(root));
		}
	}

	const client = new McpStdioClient(serverPath, serverArguments);
	const report = {
		schemaVersion: 1,
		source: options.source,
		queryCharacters: [...options.query].length,
		limit: options.limit,
		budgetMs: options.budgetMs,
		timeoutMs: options.timeoutMs,
		server: serverPath,
		workspaceRootCount: options.workspaceScripts.length,
	};

	try {
		const startup = await measure(() => client.initialize(options.timeoutMs));
		report.initializeMs = round(startup.elapsedMs);

		if (options.source === 'game-data') {
			const status = await measure(() => client.callTool('game_data_status', {}, options.timeoutMs));
			report.gameDataStatusMs = round(status.elapsedMs);
			report.gameDataStatus = summarizeToolResult(status.value);
			if (status.value.isError) {
				throw new ReportFailure('game-data-status', status.value);
			}
		}

		const toolName = options.source === 'game-data'
			? 'search_game_data_text'
			: 'search_workspace_text';
		const search = await measure(() => client.callTool(toolName, {
			query: options.query,
			limit: options.limit,
		}, options.timeoutMs));
		report.searchMs = round(search.elapsedMs);
		report.search = summarizeToolResult(search.value);
		report.verdict = search.value.isError
			? 'tool-error'
			: search.elapsedMs > options.budgetMs
				? 'over-budget'
				: 'pass';
		if (search.value.isError || search.elapsedMs > options.budgetMs) {
			process.exitCode = 1;
		}
	} catch (error) {
		report.verdict = error instanceof RequestTimeoutError ? 'timeout' : 'runner-error';
		report.failedPhase = error.phase ?? 'unknown';
		report.error = sanitizeError(error);
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
			clientInfo: {
				name: 'reforger-text-search-performance',
				version: '1.0.0',
			},
		}, timeoutMs, 'initialize');
		this.send({ jsonrpc: '2.0', method: 'notifications/initialized' });
		return result;
	}

	callTool(name, argumentsValue, timeoutMs) {
		return this.request('tools/call', {
			name,
			arguments: argumentsValue,
		}, timeoutMs, name);
	}

	async request(method, params, timeoutMs, phase) {
		const id = this.nextId++;
		const deadline = performance.now() + timeoutMs;
		this.send({ jsonrpc: '2.0', id, method, params });
		while (true) {
			const remainingMs = Math.max(1, deadline - performance.now());
			const next = await withTimeout(this.messages.next(), remainingMs, phase, timeoutMs);
			if (next.done) {
				throw Object.assign(new Error('MCP server closed stdout before responding.'), { phase });
			}
			let message;
			try {
				message = JSON.parse(next.value);
			} catch {
				continue;
			}
			if (message.id !== id) {
				continue;
			}
			if (message.error) {
				throw Object.assign(new Error(message.error.message ?? 'MCP request failed.'), {
					phase,
					code: message.error.code,
				});
			}
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

class ReportFailure extends Error {
	constructor(phase, result) {
		super('MCP tool returned a structured error.');
		this.name = 'ReportFailure';
		this.phase = phase;
		this.code = result?.structuredContent?.code;
	}
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

function summarizeToolResult(result) {
	const payload = result?.structuredContent ?? {};
	return {
		isError: result?.isError === true,
		code: payload.code,
		message: payload.message,
		returned: Array.isArray(payload.results) ? payload.results.length : undefined,
		total: payload.total,
		truncated: payload.truncated,
		stats: payload.stats,
	};
}

function sanitizeError(error) {
	return {
		name: error?.name ?? 'Error',
		message: error?.message ?? String(error),
		code: error?.code,
	};
}

function parseArguments(args) {
	const parsed = {
		workspaceScripts: [],
		limit: 25,
		budgetMs: 5000,
		timeoutMs: 35000,
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
			case '--source': parsed.source = value(); break;
			case '--index-cache': parsed.indexCache = value(); break;
			case '--workspace-scripts': parsed.workspaceScripts.push(value()); break;
			case '--query': parsed.query = value(); break;
			case '--limit': parsed.limit = positiveInteger(value(), argument); break;
			case '--budget-ms': parsed.budgetMs = positiveInteger(value(), argument); break;
			case '--timeout-ms': parsed.timeoutMs = positiveInteger(value(), argument); break;
			case '--help': usage(); break;
			default: usage(`Unknown argument: ${argument}`);
		}
	}
	if (!parsed.server || !parsed.query || !['workspace', 'game-data'].includes(parsed.source)) {
		usage('--server, --source, and --query are required.');
	}
	if (parsed.limit > 100) usage('--limit must be between 1 and 100.');
	if (parsed.source === 'game-data' && !parsed.indexCache) {
		usage('--index-cache is required for game-data searches.');
	}
	if (parsed.source === 'workspace' && parsed.workspaceScripts.length === 0) {
		usage('At least one --workspace-scripts root is required for workspace searches.');
	}
	return parsed;
}

function positiveInteger(value, argument) {
	const parsed = Number.parseInt(value, 10);
	if (!Number.isInteger(parsed) || parsed < 1) usage(`${argument} must be a positive integer.`);
	return parsed;
}

function usage(error) {
	if (error) process.stderr.write(`${error}\n\n`);
	process.stderr.write(
		'Usage: node tools/text-search-performance.mjs --server <exe> --source <workspace|game-data> --query <literal> [options]\n' +
		'  Workspace: --workspace-scripts <root> (repeatable)\n' +
		'  Game Data: --index-cache <symbols.bin>\n' +
		'  Options: --limit <1-100> --budget-ms <ms> --timeout-ms <ms>\n',
	);
	process.exit(error ? 2 : 0);
}

function round(value) {
	return Math.round(value * 100) / 100;
}

await main();
