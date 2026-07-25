import { spawn } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { resolve } from 'node:path';

const binaryArgument = process.argv[2];
const iterations = Number.parseInt(process.argv[3] ?? '7', 10);
if (!binaryArgument || !Number.isInteger(iterations) || iterations < 1) {
	process.stderr.write('Usage: node tools/lsp-startup-baseline.mjs <binary> [iterations]\n');
	process.exit(2);
}

const binary = resolve(binaryArgument);
const measurements = [];
for (let iteration = 0; iteration < iterations; iteration += 1) {
	measurements.push(await measureInitialize(binary, iteration + 1));
}
const sorted = [...measurements].sort((left, right) => left - right);
const median = sorted[Math.floor(sorted.length / 2)];
process.stdout.write(`${JSON.stringify({
	binary,
	iterations,
	measurementsMs: measurements.map(round),
	minMs: round(sorted[0]),
	medianMs: round(median),
	maxMs: round(sorted[sorted.length - 1]),
}, null, 2)}\n`);

function measureInitialize(command, id) {
	return new Promise((resolveMeasurement, reject) => {
		const child = spawn(command, [], {
			stdio: ['pipe', 'pipe', 'pipe'],
			windowsHide: true,
		});
		const started = performance.now();
		let stdout = Buffer.alloc(0);
		let stderr = '';
		const timeout = setTimeout(() => {
			child.kill();
			reject(new Error(`LSP initialize timed out: ${stderr}`));
		}, 10_000);
		child.stderr.on('data', chunk => {
			stderr += chunk.toString();
		});
		child.stdout.on('data', chunk => {
			stdout = Buffer.concat([stdout, chunk]);
			const headerEnd = stdout.indexOf('\r\n\r\n');
			if (headerEnd < 0) {
				return;
			}
			const header = stdout.subarray(0, headerEnd).toString('ascii');
			const lengthMatch = /^Content-Length:\s*(\d+)$/im.exec(header);
			if (!lengthMatch) {
				clearTimeout(timeout);
				child.kill();
				reject(new Error(`Invalid LSP response header: ${header}`));
				return;
			}
			const contentLength = Number.parseInt(lengthMatch[1], 10);
			if (stdout.length < headerEnd + 4 + contentLength) {
				return;
			}
			clearTimeout(timeout);
			child.kill();
			resolveMeasurement(performance.now() - started);
		});
		child.on('error', error => {
			clearTimeout(timeout);
			reject(error);
		});

		const message = JSON.stringify({
			jsonrpc: '2.0',
			id,
			method: 'initialize',
			params: {
				processId: null,
				rootUri: null,
				capabilities: {},
				clientInfo: {
					name: 'lsp-startup-baseline',
					version: '1.0.0',
				},
			},
		});
		child.stdin.write(`Content-Length: ${Buffer.byteLength(message)}\r\n\r\n${message}`);
	});
}

function round(value) {
	return Math.round(value * 100) / 100;
}
