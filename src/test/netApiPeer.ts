import * as assert from 'node:assert';
import * as net from 'node:net';

export interface NetApiPeerRequest {
	protocolVersion: number;
	clientId: string;
	contentType: string;
	payload: unknown;
}

export interface NetApiPeerResponse {
	errorCode: string;
	payload: unknown;
}

export interface NetApiPeer {
	port: number;
	requests: NetApiPeerRequest[];
	close: () => Promise<void>;
}

export async function startNetApiPeer(
	handle: (request: NetApiPeerRequest) => NetApiPeerResponse | Promise<NetApiPeerResponse>,
): Promise<NetApiPeer> {
	const requests: NetApiPeerRequest[] = [];
	const sockets = new Set<net.Socket>();
	const server = net.createServer({ allowHalfOpen: true }, socket => {
		sockets.add(socket);
		socket.once('close', () => sockets.delete(socket));
		const chunks: Buffer[] = [];
		socket.on('data', chunk => chunks.push(chunk));
		socket.on('end', () => {
			const request = decodeRequest(Buffer.concat(chunks));
			requests.push(request);
			void Promise.resolve(handle(request)).then(response => {
				socket.end(Buffer.concat([
					encodeString(response.errorCode),
					encodeString(JSON.stringify(response.payload)),
				]));
			});
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
		requests,
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

function decodeRequest(buffer: Buffer): NetApiPeerRequest {
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
