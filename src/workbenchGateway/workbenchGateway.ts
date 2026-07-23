import * as net from 'node:net';

const protocolVersion = 1;
const clientId = 'ReforgerScriptTools';
const contentType = 'JsonRPC';
const successfulErrorCode = 'Ok';
const defaultGetStatusDeadlineMs = 1_500;
const defaultValidateScriptsDeadlineMs = 120_000;
const maximumResponseBytes = 4 * 1024 * 1024;

export interface WorkbenchEndpoint {
	host: string;
	port: number;
}

export interface WorkbenchGatewayOptions {
	enabled: boolean;
	endpoint: WorkbenchEndpoint;
	deadlines?: Partial<WorkbenchGatewayDeadlines>;
	record?: (record: WorkbenchGatewayDiagnosticRecord) => void;
}

export interface WorkbenchGatewayDeadlines {
	getStatusMs: number;
	validateScriptsMs: number;
}

export interface WorkbenchGatewayDiagnosticRecord {
	capability: 'getStatus' | 'validateScripts';
	outcome: 'success' | 'compiler-findings' | WorkbenchGatewayFailureCategory;
	durationMs: number;
}

export interface WorkbenchStatus {
	isRunning: boolean;
	scriptsCompiled: boolean;
}

export type WorkbenchValidationProfile = 'WORKBENCH';

export interface WorkbenchDiagnosticLocation {
	file: string;
	fileAbs?: string;
	addon?: string;
	line: number;
}

export interface WorkbenchCompilerDiagnostic {
	severity: 'error' | 'warning';
	message: string;
	location: WorkbenchDiagnosticLocation;
}

export interface WorkbenchValidationResult {
	profile: WorkbenchValidationProfile;
	success: boolean;
	diagnostics: WorkbenchCompilerDiagnostic[];
}

export type WorkbenchAvailability =
	| { kind: 'disabled' }
	| { kind: 'unavailable'; failure: WorkbenchGatewayFailure }
	| { kind: 'ready' };

export type WorkbenchGatewayFailureCategory =
	| 'unavailable'
	| 'timeout'
	| 'protocol'
	| 'unsupported'
	| 'workbench-error';

export interface WorkbenchGatewayFailure {
	category: WorkbenchGatewayFailureCategory;
	recoveryHint: string;
}

export type WorkbenchGatewayResult<T> =
	| { ok: true; value: T }
	| { ok: false; failure: WorkbenchGatewayFailure };

export class WorkbenchGateway {
	private readonly options: WorkbenchGatewayOptions;
	private currentAvailability: WorkbenchAvailability;

	public constructor(options: WorkbenchGatewayOptions) {
		this.options = {
			...options,
			endpoint: {
				...options.endpoint,
				host: options.endpoint.host.trim(),
			},
		};
		this.currentAvailability = options.enabled
			? { kind: 'unavailable', failure: unavailableFailure() }
			: { kind: 'disabled' };
	}

	public get availability(): WorkbenchAvailability {
		return this.currentAvailability.kind === 'unavailable'
			? {
				kind: 'unavailable',
				failure: { ...this.currentAvailability.failure },
			}
			: { ...this.currentAvailability };
	}

	public async getStatus(): Promise<WorkbenchGatewayResult<WorkbenchStatus>> {
		const startedAt = Date.now();
		const result = await this.request(
			{ APIFunc: 'IsWorkbenchRunning' },
			deadline(
				this.options.deadlines?.getStatusMs,
				defaultGetStatusDeadlineMs,
			),
		);
		if (!result.ok) {
			this.currentAvailability = this.options.enabled
				? { kind: 'unavailable', failure: result.failure }
				: { kind: 'disabled' };
			this.record('getStatus', result.failure.category, startedAt);
			return result;
		}
		const status = decodeStatus(result.value);
		if (!status.ok) {
			this.currentAvailability = { kind: 'unavailable', failure: status.failure };
			this.record('getStatus', status.failure.category, startedAt);
			return status;
		}
		this.currentAvailability = { kind: 'ready' };
		this.record('getStatus', 'success', startedAt);
		return status;
	}

	public async validateScripts(
		profile: WorkbenchValidationProfile,
	): Promise<WorkbenchGatewayResult<WorkbenchValidationResult>> {
		const startedAt = Date.now();
		if (profile !== 'WORKBENCH') {
			const result = failure(
				'unsupported',
				'Select the supported WORKBENCH validation profile.',
			);
			this.record('validateScripts', 'unsupported', startedAt);
			return result;
		}
		const result = await this.request({
			APIFunc: 'ValidateScripts',
			Configuration: profile,
		}, deadline(
			this.options.deadlines?.validateScriptsMs,
			defaultValidateScriptsDeadlineMs,
		));
		if (!result.ok) {
			this.noteFailure(result.failure);
			this.record('validateScripts', result.failure.category, startedAt);
			return result;
		}
		const validation = decodeValidation(profile, result.value);
		if (!validation.ok) {
			this.noteFailure(validation.failure);
			this.record('validateScripts', validation.failure.category, startedAt);
			return validation;
		}
		this.currentAvailability = { kind: 'ready' };
		this.record(
			'validateScripts',
			validation.value.success ? 'success' : 'compiler-findings',
			startedAt,
		);
		return validation;
	}

	private request(
		payload: Record<string, unknown>,
		deadlineMs: number,
	): Promise<WorkbenchGatewayResult<unknown>> {
		if (!this.options.enabled) {
			return Promise.resolve(failure(
				'unsupported',
				'Enable Workbench NET API integration in extension settings.',
			));
		}
		const endpointFailure = validateEndpoint(this.options.endpoint);
		if (endpointFailure) {
			return Promise.resolve({ ok: false, failure: endpointFailure });
		}
		return transact(
			this.options.endpoint,
			payload,
			deadlineMs,
		);
	}

	private noteFailure(gatewayFailure: WorkbenchGatewayFailure): void {
		this.currentAvailability = this.options.enabled
			? { kind: 'unavailable', failure: gatewayFailure }
			: { kind: 'disabled' };
	}

	private record(
		capability: WorkbenchGatewayDiagnosticRecord['capability'],
		outcome: WorkbenchGatewayDiagnosticRecord['outcome'],
		startedAt: number,
	): void {
		try {
			this.options.record?.({
				capability,
				outcome,
				durationMs: Date.now() - startedAt,
			});
		} catch {
			// Host diagnostics must never affect a Gateway capability outcome.
		}
	}
}

function decodeStatus(value: unknown): WorkbenchGatewayResult<WorkbenchStatus> {
	if (!isRecord(value)
		|| typeof value.IsRunning !== 'boolean'
		|| typeof value.ScriptsCompiled !== 'boolean') {
		return failure('protocol', 'Restart Workbench and verify that its NET API is compatible.');
	}
	return {
		ok: true,
		value: {
			isRunning: value.IsRunning,
			scriptsCompiled: value.ScriptsCompiled,
		},
	};
}

function decodeValidation(
	profile: WorkbenchValidationProfile,
	value: unknown,
): WorkbenchGatewayResult<WorkbenchValidationResult> {
	if (!isRecord(value)
		|| typeof value.Success !== 'boolean'
		|| !Array.isArray(value.Errors)
		|| !Array.isArray(value.Warnings)) {
		return failure('protocol', 'Restart Workbench and verify that its NET API is compatible.');
	}
	const errors = decodeDiagnostics(value.Errors, 'error');
	const warnings = decodeDiagnostics(value.Warnings, 'warning');
	if (!errors || !warnings) {
		return failure('protocol', 'Restart Workbench and verify that its NET API is compatible.');
	}
	const diagnostics = uniqueDiagnostics([...errors, ...warnings]);
	return {
		ok: true,
		value: {
			profile,
			success: value.Success,
			diagnostics,
		},
	};
}

function decodeDiagnostics(
	values: unknown[],
	severity: WorkbenchCompilerDiagnostic['severity'],
): WorkbenchCompilerDiagnostic[] | undefined {
	const diagnostics: WorkbenchCompilerDiagnostic[] = [];
	for (const value of values) {
		if (!isRecord(value)
			|| typeof value.error !== 'string'
			|| typeof value.file !== 'string'
			|| !Number.isInteger(value.line)
			|| (value.fileAbs !== undefined && typeof value.fileAbs !== 'string')
			|| (value.addon !== undefined && typeof value.addon !== 'string')) {
			return undefined;
		}
		diagnostics.push({
			severity,
			message: value.error,
			location: {
				file: value.file,
				...(value.fileAbs === undefined ? {} : { fileAbs: value.fileAbs }),
				...(value.addon === undefined ? {} : { addon: value.addon }),
				line: value.line as number,
			},
		});
	}
	return diagnostics;
}

function uniqueDiagnostics(
	diagnostics: WorkbenchCompilerDiagnostic[],
): WorkbenchCompilerDiagnostic[] {
	const seen = new Set<string>();
	return diagnostics.filter(diagnostic => {
		const identity = JSON.stringify([
			diagnostic.severity,
			diagnostic.message,
			diagnostic.location.file,
			diagnostic.location.fileAbs ?? '',
			diagnostic.location.addon ?? '',
			diagnostic.location.line,
		]);
		if (seen.has(identity)) {
			return false;
		}
		seen.add(identity);
		return true;
	});
}

function transact(
	endpoint: WorkbenchEndpoint,
	payload: Record<string, unknown>,
	deadlineMs: number,
): Promise<WorkbenchGatewayResult<unknown>> {
	return new Promise(resolve => {
		const socket = net.createConnection(endpoint);
		const chunks: Buffer[] = [];
		let receivedBytes = 0;
		let settled = false;
		const finish = (result: WorkbenchGatewayResult<unknown>) => {
			if (settled) {
				return;
			}
			settled = true;
			clearTimeout(deadlineTimer);
			socket.destroy();
			resolve(result);
		};
		const deadlineTimer = setTimeout(() => {
			finish(failure('timeout', 'Ensure Workbench is responsive and retry the operation.'));
		}, deadlineMs);
		socket.once('connect', () => {
			socket.end(encodeRequest(payload));
		});
		socket.on('data', (chunk: Buffer) => {
			receivedBytes += chunk.length;
			if (receivedBytes > maximumResponseBytes) {
				finish(failure('protocol', 'Restart Workbench and retry the request.'));
				return;
			}
			chunks.push(chunk);
			const decoded = decodeResponse(Buffer.concat(chunks));
			if (decoded.kind === 'incomplete') {
				return;
			}
			if (decoded.kind === 'invalid') {
				finish(failure('protocol', 'Restart Workbench and verify that its NET API is compatible.'));
				return;
			}
			if (decoded.errorCode !== successfulErrorCode) {
				finish(failure('workbench-error', 'Review Workbench state and retry the operation.'));
				return;
			}
			try {
				finish({ ok: true, value: JSON.parse(decoded.payload) as unknown });
			} catch {
				finish(failure('protocol', 'Restart Workbench and verify that its NET API is compatible.'));
			}
		});
		socket.once('error', () => {
			finish(failure('unavailable', 'Start Workbench with NET API enabled, then retry.'));
		});
		socket.once('close', () => {
			if (!settled) {
				finish(failure('protocol', 'Restart Workbench and retry the request.'));
			}
		});
	});
}

function encodeRequest(payload: Record<string, unknown>): Buffer {
	const version = Buffer.allocUnsafe(4);
	version.writeInt32LE(protocolVersion);
	return Buffer.concat([
		version,
		encodeString(clientId),
		encodeString(contentType),
		encodeString(JSON.stringify(payload)),
	]);
}

type DecodedResponse =
	| { kind: 'incomplete' }
	| { kind: 'invalid' }
	| { kind: 'complete'; errorCode: string; payload: string };

function decodeResponse(buffer: Buffer): DecodedResponse {
	const errorCode = decodeString(buffer, 0);
	if (errorCode.kind !== 'complete') {
		return errorCode;
	}
	const payload = decodeString(buffer, errorCode.offset);
	if (payload.kind !== 'complete') {
		return payload;
	}
	return {
		kind: 'complete',
		errorCode: errorCode.value,
		payload: payload.value,
	};
}

type DecodedString =
	| { kind: 'incomplete' }
	| { kind: 'invalid' }
	| { kind: 'complete'; value: string; offset: number };

function decodeString(buffer: Buffer, offset: number): DecodedString {
	if (buffer.length - offset < 4) {
		return { kind: 'incomplete' };
	}
	const length = buffer.readInt32LE(offset);
	if (length < 0 || length > maximumResponseBytes) {
		return { kind: 'invalid' };
	}
	const start = offset + 4;
	const end = start + length;
	if (buffer.length < end) {
		return { kind: 'incomplete' };
	}
	return { kind: 'complete', value: buffer.toString('utf8', start, end), offset: end };
}

function encodeString(value: string): Buffer {
	const encoded = Buffer.from(value, 'utf8');
	const length = Buffer.allocUnsafe(4);
	length.writeInt32LE(encoded.length);
	return Buffer.concat([length, encoded]);
}

function validateEndpoint(endpoint: WorkbenchEndpoint): WorkbenchGatewayFailure | undefined {
	if (!isLoopbackHost(endpoint.host)) {
		return {
			category: 'unsupported',
			recoveryHint: 'Configure a loopback Workbench host such as 127.0.0.1.',
		};
	}
	if (!Number.isInteger(endpoint.port) || endpoint.port < 1 || endpoint.port > 65_535) {
		return {
			category: 'unsupported',
			recoveryHint: 'Configure a Workbench NET API port from 1 through 65535.',
		};
	}
	return undefined;
}

function isLoopbackHost(host: string): boolean {
	const normalized = host.trim().toLowerCase();
	if (normalized === '::1') {
		return true;
	}
	const parts = normalized.split('.');
	return parts.length === 4
		&& parts[0] === '127'
		&& parts.every(part => /^\d{1,3}$/.test(part) && Number(part) <= 255);
}

function deadline(configured: number | undefined, defaultMs: number): number {
	return configured !== undefined && Number.isFinite(configured) && configured > 0
		? configured
		: defaultMs;
}

function unavailableFailure(): WorkbenchGatewayFailure {
	return {
		category: 'unavailable',
		recoveryHint: 'Start Workbench with NET API enabled, then retry.',
	};
}

function failure(
	category: WorkbenchGatewayFailureCategory,
	recoveryHint: string,
): WorkbenchGatewayResult<never> {
	return { ok: false, failure: { category, recoveryHint } };
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
