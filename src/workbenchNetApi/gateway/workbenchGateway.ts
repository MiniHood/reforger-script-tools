import { execFile } from 'node:child_process';
import * as path from 'node:path';

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
	serverPath?: Promise<string | undefined>;
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
	| 'consent-required'
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

export type WorkbenchPrivateApiCommand =
	| 'status'
	| 'validate'
	| 'integration-status'
	| 'install-bridge';

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
	return invokeWorkbenchPrivateApi(
			this.options.serverPath ?? defaultDevelopmentServerPath(),
			this.options.endpoint,
			payload.APIFunc === 'ValidateScripts' ? 'validate' : 'status',
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
		|| typeof value.isRunning !== 'boolean'
		|| typeof value.scriptsCompiled !== 'boolean') {
		return failure('protocol', 'Restart Workbench and verify that its NET API is compatible.');
	}
	return {
		ok: true,
		value: {
			isRunning: value.isRunning,
			scriptsCompiled: value.scriptsCompiled,
		},
	};
}

function decodeValidation(
	profile: WorkbenchValidationProfile,
	value: unknown,
): WorkbenchGatewayResult<WorkbenchValidationResult> {
	if (!isRecord(value)
		|| value.profile !== profile
		|| typeof value.success !== 'boolean'
		|| !Array.isArray(value.diagnostics)
		|| !value.diagnostics.every(isCompilerDiagnostic)) {
		return failure('protocol', 'Restart Workbench and verify that its NET API is compatible.');
	}
	return {
		ok: true,
		value: {
			profile,
			success: value.success,
			diagnostics: value.diagnostics,
		},
	};
}

function isCompilerDiagnostic(value: unknown): value is WorkbenchCompilerDiagnostic {
	if (!isRecord(value)
		|| (value.severity !== 'error' && value.severity !== 'warning')
		|| typeof value.message !== 'string'
		|| !isRecord(value.location)) {
		return false;
	}
	const location = value.location;
	return typeof location.file === 'string'
		&& Number.isInteger(location.line)
		&& (location.fileAbs === undefined || typeof location.fileAbs === 'string')
		&& (location.addon === undefined || typeof location.addon === 'string');
}

export async function invokeWorkbenchPrivateApi(
	serverPath: Promise<string | undefined>,
	endpoint: WorkbenchEndpoint,
	action: WorkbenchPrivateApiCommand,
	deadlineMs: number,
): Promise<WorkbenchGatewayResult<unknown>> {
	const endpointFailure = validateEndpoint(endpoint);
	if (endpointFailure) {
		return { ok: false, failure: endpointFailure };
	}
	const executable = await serverPath;
	if (!executable) {
		return failure('unavailable', 'Restart the extension and retry.');
	}
	return new Promise(resolve => {
		execFile(
			executable,
			[
				'workbench-api',
				action,
				'--host',
				endpoint.host,
				'--port',
				String(endpoint.port),
				'--deadline-ms',
				String(deadlineMs),
			],
			{
				timeout: deadlineMs + 500,
				maxBuffer: maximumResponseBytes,
				windowsHide: true,
			},
			(error, stdout) => {
				if (error) {
					resolve(failure(
						error.killed || error.code === 'ETIMEDOUT'
							? 'timeout'
							: 'unavailable',
						'Restart Workbench and retry the request.',
					));
					return;
				}
				try {
					const result = JSON.parse(stdout) as {
						ok: boolean;
						value?: unknown;
						failure?: { category?: WorkbenchGatewayFailureCategory };
					};
					if (result.ok) {
						resolve({ ok: true, value: result.value });
						return;
					}
					resolve(failure(
						result.failure?.category ?? 'protocol',
						'Review Workbench state and retry the operation.',
					));
				} catch {
					resolve(failure('protocol', 'Restart Workbench and retry the request.'));
				}
			},
		);
	});
}

function defaultDevelopmentServerPath(): Promise<string | undefined> {
	return Promise.resolve(path.resolve(
		__dirname,
		'..',
		'..',
		'..',
		'server',
		'target',
		'debug',
		process.platform === 'win32'
			? 'reforger_language_server.exe'
			: 'reforger_language_server',
	));
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
