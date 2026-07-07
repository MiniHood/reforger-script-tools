import * as net from 'node:net';

export interface WorkbenchStatus {
	IsRunning: boolean;
	ScriptsCompiled: boolean;
}

export interface NetApiResult<T> {
	errorCode: string;
	payload: T | undefined;
	rawPayload: string;
}

export type NetApiLogger = (message: string) => void;

const protocolVersion = 1;
const contentType = 'JsonRPC';
const clientId = 'Reforger Script Tools';

export async function callWorkbenchNetApi<T>(
	apiFunc: string,
	params: Record<string, unknown> = {},
	port = 5775,
	host = '127.0.0.1',
	timeoutMs = 3000,
	logger?: NetApiLogger
): Promise<NetApiResult<T>> {
	const payload = JSON.stringify({
		...params,
		APIFunc: apiFunc,
	});

	logger?.(`Net API request ${host}:${port} ${payload}`);

	return new Promise((resolve, reject) => {
		const socket = net.createConnection({ host, port });
		let responseBuffer = Buffer.alloc(0);
		let resolved = false;

		const finish = (callback: () => void) => {
			if (resolved) {
				return;
			}

			resolved = true;
			socket.destroy();
			callback();
		};

		socket.setTimeout(timeoutMs);

		socket.on('connect', () => {
			const request = createRequest(payload);
			logger?.(`Net API connected; sending ${request.length} bytes.`);
			socket.write(request);
		});

		socket.on('data', chunk => {
			logger?.(`Net API received ${chunk.length} bytes.`);
			responseBuffer = Buffer.concat([responseBuffer, chunk]);

			const response = tryParseResponse<T>(responseBuffer);
			if (response) {
				finish(() => {
					logger?.(`Net API response errorCode=${response.errorCode} rawPayload=${response.rawPayload}`);
					resolve(response);
				});
			}
		});

		socket.on('end', () => {
			finish(() => {
				try {
					logger?.(`Net API socket ended with ${responseBuffer.length} buffered bytes.`);
					resolve(parseResponse<T>(responseBuffer));
				} catch (error) {
					reject(error);
				}
			});
		});

		socket.on('timeout', () => {
			logger?.(`Net API timed out after ${timeoutMs} ms with ${responseBuffer.length} buffered bytes.`);
			finish(() => reject(new Error(`Timed out connecting to Workbench Net API at ${host}:${port}`)));
		});

		socket.on('error', error => {
			logger?.(`Net API socket error: ${error.message}`);
			finish(() => reject(error));
		});

		socket.on('close', () => {
			logger?.(`Net API socket closed; resolved=${resolved}; buffered=${responseBuffer.length} bytes.`);
			if (!resolved && responseBuffer.length > 0) {
				finish(() => {
					try {
						resolve(parseResponse<T>(responseBuffer));
					} catch (error) {
						reject(error);
					}
				});
			}
		});
	});
}

function createRequest(payload: string): Buffer {
	return Buffer.concat([
		writeUInt32(protocolVersion),
		writeString(clientId),
		writeString(contentType),
		writeString(payload),
	]);
}

function tryParseResponse<T>(buffer: Buffer): NetApiResult<T> | undefined {
	if (buffer.length < 8) {
		return undefined;
	}

	const errorCodeLength = buffer.readUInt32LE(0);
	const payloadLengthOffset = 4 + errorCodeLength;
	if (buffer.length < payloadLengthOffset + 4) {
		return undefined;
	}

	const payloadLength = buffer.readUInt32LE(payloadLengthOffset);
	const totalLength = payloadLengthOffset + 4 + payloadLength;
	if (buffer.length < totalLength) {
		return undefined;
	}

	return parseResponse<T>(buffer.subarray(0, totalLength));
}

function parseResponse<T>(buffer: Buffer): NetApiResult<T> {
	let offset = 0;
	const errorCodeResult = readString(buffer, offset);
	offset = errorCodeResult.offset;

	const payloadResult = readString(buffer, offset);
	const rawPayload = payloadResult.value;

	return {
		errorCode: errorCodeResult.value,
		rawPayload,
		payload: rawPayload ? JSON.parse(rawPayload) as T : undefined,
	};
}

function writeUInt32(value: number): Buffer {
	const buffer = Buffer.alloc(4);
	buffer.writeUInt32LE(value, 0);
	return buffer;
}

function writeString(value: string): Buffer {
	const content = Buffer.from(value, 'utf8');
	return Buffer.concat([writeUInt32(content.length), content]);
}

function readString(buffer: Buffer, offset: number): { value: string; offset: number } {
	if (offset + 4 > buffer.length) {
		throw new Error('Workbench Net API response ended before string length.');
	}

	const length = buffer.readUInt32LE(offset);
	const start = offset + 4;
	const end = start + length;

	if (end > buffer.length) {
		throw new Error('Workbench Net API response ended before string content.');
	}

	return {
		value: buffer.toString('utf8', start, end),
		offset: end,
	};
}
