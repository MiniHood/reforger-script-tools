import * as assert from 'node:assert';
import * as net from 'node:net';
import { WorkbenchGateway } from '../workbenchGateway/workbenchGateway';

suite('Workbench Gateway', () => {
	test('gets compiler readiness through the documented NET API framing', async () => {
		const peer = await startPeer(request => {
			assert.strictEqual(request.protocolVersion, 1);
			assert.strictEqual(request.clientId, 'ReforgerScriptTools');
			assert.strictEqual(request.contentType, 'JsonRPC');
			assert.deepStrictEqual(request.payload, { APIFunc: 'IsWorkbenchRunning' });
			return {
				errorCode: '',
				payload: { IsRunning: true, ScriptsCompiled: true },
			};
		});
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			assert.deepStrictEqual(await gateway.getStatus(), {
				ok: true,
				value: { isRunning: true, scriptsCompiled: true },
			});
			assert.deepStrictEqual(gateway.availability, { kind: 'ready' });
		} finally {
			await peer.close();
		}
	});

	test('validates the named WORKBENCH profile and normalizes compiler diagnostics', async () => {
		const peer = await startPeer(request => {
			assert.deepStrictEqual(request.payload, {
				APIFunc: 'ValidateScripts',
				Configuration: 'WORKBENCH',
			});
			return {
				errorCode: '',
				payload: {
					Errors: [{
						error: "Undefined function 'Run'",
						file: 'scripts/Game/Example.c',
						fileAbs: 'C:\\Addon\\scripts\\Game\\Example.c',
						addon: 'ExampleAddon',
						line: 12,
					}],
					Warnings: [{
						error: "Variable 'unused' is not used",
						file: 'scripts/Game/Other.c',
						line: 4,
					}],
					Success: false,
				},
			};
		});
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			assert.deepStrictEqual(await gateway.validateScripts('WORKBENCH'), {
				ok: true,
				value: {
					profile: 'WORKBENCH',
					success: false,
					diagnostics: [{
						severity: 'error',
						message: "Undefined function 'Run'",
						location: {
							file: 'scripts/Game/Example.c',
							fileAbs: 'C:\\Addon\\scripts\\Game\\Example.c',
							addon: 'ExampleAddon',
							line: 12,
						},
					}, {
						severity: 'warning',
						message: "Variable 'unused' is not used",
						location: {
							file: 'scripts/Game/Other.c',
							line: 4,
						},
					}],
				},
			});
		} finally {
			await peer.close();
		}
	});

	test('reports only a sanitized named-capability outcome to its host', async () => {
		const records: unknown[] = [];
		const peer = await startPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
				record: record => records.push(record),
			});

			await gateway.getStatus();

			assert.strictEqual(records.length, 1);
			assert.deepStrictEqual(records[0], {
				capability: 'getStatus',
				outcome: 'success',
				durationMs: (records[0] as { durationMs: number }).durationMs,
			});
			assert.ok(Number.isFinite((records[0] as { durationMs: number }).durationMs));
			const serialized = JSON.stringify(records[0]);
			assert.ok(!serialized.includes(String(peer.port)));
			assert.ok(!serialized.includes('IsWorkbenchRunning'));
			assert.ok(!serialized.includes('127.0.0.1'));
		} finally {
			await peer.close();
		}
	});

	test('rejects a non-loopback endpoint without network discovery', async () => {
		const gateway = new WorkbenchGateway({
			enabled: true,
			endpoint: { host: '192.0.2.10', port: 5775 },
		});

		assert.deepStrictEqual(await gateway.getStatus(), {
			ok: false,
			failure: {
				category: 'unsupported',
				recoveryHint: 'Configure a loopback Workbench host such as 127.0.0.1.',
			},
		});
	});

	test('performs no transaction while the Gateway is disabled', async () => {
		const gateway = new WorkbenchGateway({
			enabled: false,
			endpoint: { host: '127.0.0.1', port: 1 },
		});

		const result = await gateway.getStatus();

		assert.strictEqual(result.ok, false);
		assert.deepStrictEqual(gateway.availability, { kind: 'disabled' });
	});

	test('rejects an unnamed validation profile before opening a connection', async () => {
		const gateway = new WorkbenchGateway({
			enabled: true,
			endpoint: { host: '127.0.0.1', port: 1 },
		});

		assert.deepStrictEqual(
			await gateway.validateScripts('PC' as never),
			{
				ok: false,
				failure: {
					category: 'unsupported',
					recoveryHint: 'Select the supported WORKBENCH validation profile.',
				},
			},
		);
	});

	test('categorizes a Workbench error code separately from compiler findings', async () => {
		const peer = await startPeer(() => ({
			errorCode: 'InvalidRequest',
			payload: {},
		}));
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			const result = await gateway.validateScripts('WORKBENCH');

			assert.strictEqual(result.ok, false);
			if (!result.ok) {
				assert.strictEqual(result.failure.category, 'workbench-error');
			}
		} finally {
			await peer.close();
		}
	});

	test('categorizes a truncated response as a protocol failure', async () => {
		const peer = await startPeer(() => ({
			errorCode: '',
			payload: {},
			raw: encodeString(''),
		}));
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			const result = await gateway.getStatus();

			assert.strictEqual(result.ok, false);
			if (!result.ok) {
				assert.strictEqual(result.failure.category, 'protocol');
			}
		} finally {
			await peer.close();
		}
	});

	test('categorizes malformed JSON as a protocol failure', async () => {
		const peer = await startPeer(() => ({
			errorCode: '',
			payload: {},
			raw: Buffer.concat([encodeString(''), encodeString('{not-json')]),
		}));
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			const result = await gateway.getStatus();

			assert.strictEqual(result.ok, false);
			if (!result.ok) {
				assert.strictEqual(result.failure.category, 'protocol');
			}
		} finally {
			await peer.close();
		}
	});

	test('reports starting until Workbench says scripts are compiled', async () => {
		const peer = await startPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: false },
		}));
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
			});

			assert.strictEqual((await gateway.getStatus()).ok, true);
			assert.deepStrictEqual(gateway.availability, { kind: 'starting' });
		} finally {
			await peer.close();
		}
	});

	test('categorizes an unresponsive endpoint as a timeout', async () => {
		const peer = await startSilentPeer();
		try {
			const gateway = new WorkbenchGateway({
				enabled: true,
				endpoint: { host: '127.0.0.1', port: peer.port },
				deadlines: { getStatusMs: 30 },
			});

			const result = await gateway.getStatus();

			assert.strictEqual(result.ok, false);
			if (!result.ok) {
				assert.strictEqual(result.failure.category, 'timeout');
			}
		} finally {
			await peer.close();
		}
	});

	test('categorizes a refused configured endpoint as unavailable', async () => {
		const peer = await startSilentPeer();
		const port = peer.port;
		await peer.close();
		const gateway = new WorkbenchGateway({
			enabled: true,
			endpoint: { host: '127.0.0.1', port },
			deadlines: { getStatusMs: 100 },
		});

		const result = await gateway.getStatus();

		assert.strictEqual(result.ok, false);
		if (!result.ok) {
			assert.strictEqual(result.failure.category, 'unavailable');
		}
	});
});

interface PeerRequest {
	protocolVersion: number;
	clientId: string;
	contentType: string;
	payload: unknown;
}

interface PeerResponse {
	errorCode: string;
	payload: unknown;
	raw?: Buffer;
}

async function startPeer(
	handle: (request: PeerRequest) => PeerResponse,
): Promise<{ port: number; close: () => Promise<void> }> {
	const server = net.createServer(socket => {
		const chunks: Buffer[] = [];
		socket.on('data', chunk => chunks.push(chunk));
		socket.on('end', () => {
			const request = decodeRequest(Buffer.concat(chunks));
			const response = handle(request);
			socket.end(response.raw ?? Buffer.concat([
				encodeString(response.errorCode),
				encodeString(JSON.stringify(response.payload)),
			]));
		});
	});
	await new Promise<void>((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', () => resolve());
	});
	const address = server.address();
	assert.ok(address && typeof address !== 'string');
	return {
		port: address.port,
		close: () => new Promise<void>((resolve, reject) => {
			server.close(error => error ? reject(error) : resolve());
		}),
	};
}

async function startSilentPeer(): Promise<{ port: number; close: () => Promise<void> }> {
	const sockets = new Set<net.Socket>();
	const server = net.createServer({ allowHalfOpen: true }, socket => {
		sockets.add(socket);
		socket.once('close', () => sockets.delete(socket));
		socket.resume();
	});
	await new Promise<void>((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', () => resolve());
	});
	const address = server.address();
	assert.ok(address && typeof address !== 'string');
	return {
		port: address.port,
		close: async () => {
			for (const socket of sockets) {
				socket.destroy();
			}
			await new Promise<void>((resolve, reject) => {
				server.close(error => error ? reject(error) : resolve());
			});
		},
	};
}

function decodeRequest(buffer: Buffer): PeerRequest {
	let offset = 0;
	const protocolVersion = buffer.readInt32LE(offset);
	offset += 4;
	const clientId = decodeString(buffer, offset);
	offset = clientId.offset;
	const contentType = decodeString(buffer, offset);
	offset = contentType.offset;
	const payload = decodeString(buffer, offset);
	offset = payload.offset;
	assert.strictEqual(offset, buffer.length);
	return {
		protocolVersion,
		clientId: clientId.value,
		contentType: contentType.value,
		payload: JSON.parse(payload.value) as unknown,
	};
}

function decodeString(buffer: Buffer, offset: number): { value: string; offset: number } {
	const length = buffer.readInt32LE(offset);
	const start = offset + 4;
	const end = start + length;
	return { value: buffer.toString('utf8', start, end), offset: end };
}

function encodeString(value: string): Buffer {
	const encoded = Buffer.from(value, 'utf8');
	const length = Buffer.allocUnsafe(4);
	length.writeInt32LE(encoded.length);
	return Buffer.concat([length, encoded]);
}
