import * as vscode from 'vscode';
import { diagnostic } from '../../diagnostics/diagnostics';
import {
	invokeWorkbenchPrivateApi,
	WorkbenchEndpoint,
	WorkbenchGatewayResult,
	WorkbenchIntegrationBootstrap,
	WorkbenchProcessStatus,
} from '../gateway/workbenchGateway';

// V2 avoids inheriting the pre-consent implementation's implicit approval for
// users who already had a managed bridge installed.
const approvalStateKey = 'workbenchIntegrationApprovedV2';
const installChoice = 'Enable Workbench Integration';
const installFailureMessage = 'Reforger Workbench integration could not be installed.';
const restartMessage = 'Reforger Workbench integration was updated. Restart Workbench to activate it.';

export interface WorkbenchIntegrationStatus {
	installed: boolean;
	installationAvailable: boolean;
}

export interface WorkbenchIntegrationRuntime {
	status(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchIntegrationStatus>>;
	bootstrap(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchIntegrationBootstrap>>;
	maintain(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchIntegrationBootstrap>>;
	processStatus(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchProcessStatus>>;
	launchDefault(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<unknown>>;
}

export interface WorkbenchIntegrationUi {
	confirmInstall(): Promise<boolean>;
	runInstall<T>(task: () => Promise<T>): Promise<T>;
	showInstallFailed(message: string): void;
	showStartOrRestart?(running: boolean): void;
}

export interface WorkbenchIntegrationState {
	isApproved(): boolean;
	approve(): Promise<void>;
}

export class WorkbenchIntegrationCoordinator implements vscode.Disposable {
	private readonly ready: Promise<boolean>;
	private resolveReady: ((ready: boolean) => void) | undefined;
	private startup: Promise<boolean> | undefined;
	private connected = false;
	private disconnectedAfterRequiredRestart = false;
	private requiresRestart = false;
	private bootstrapFinished = false;
	private bootstrapInProgress = false;
	private profileWasMissing = false;
	private endpoint: WorkbenchEndpoint | undefined;
	private disposed = false;

	public constructor(
		private readonly state: WorkbenchIntegrationState,
		private readonly runtime: WorkbenchIntegrationRuntime,
		private readonly ui: WorkbenchIntegrationUi,
		private readonly enabled: boolean,
	) {
		this.ready = new Promise(resolve => {
			this.resolveReady = resolve;
		});
	}

	public start(): Promise<boolean> {
		if (this.startup) {
			return this.ready;
		}
		if (!this.enabled) {
			this.resolveReady?.(true);
			return this.ready;
		}
		this.startup = this.bootstrapStartup().catch(error => {
			const message = error instanceof Error ? error.message : String(error);
			diagnostic('workbenchIntegrationStartupFailed', { message });
			this.ui.showInstallFailed(installFailureMessage);
			this.bootstrapFinished = true;
			this.resolveReady?.(false);
			return false;
		});
		return this.ready;
	}

	public async onWorkbenchConnected(endpoint: WorkbenchEndpoint): Promise<void> {
		if (this.disposed || !this.enabled) {
			return;
		}
		this.endpoint = endpoint;
		this.connected = true;
		if (this.bootstrapInProgress) {
			return;
		}
		if (this.profileWasMissing) {
			await this.retryMissingProfile();
		}
		this.finishIfReady();
	}

	public onWorkbenchDisconnected(): void {
		this.connected = false;
		if (this.requiresRestart) {
			this.disconnectedAfterRequiredRestart = true;
		}
	}

	public whenReady(): Promise<boolean> {
		return this.ready;
	}

	public dispose(): void {
		this.disposed = true;
		this.resolveReady?.(false);
	}

	private async bootstrapStartup(): Promise<boolean> {
		const endpoint = this.readEndpoint();
		const status = await this.runtime.status(endpoint);
		if (!status.ok) {
			diagnostic('workbenchIntegrationStatusUnavailable', {
				category: status.failure.category,
			});
		}
		const approved = this.state.isApproved();

		let result: WorkbenchGatewayResult<WorkbenchIntegrationBootstrap>;
		if (!approved) {
			if (!(await this.ui.confirmInstall())) {
				this.bootstrapFinished = true;
				this.resolveReady?.(false);
				return false;
			}
			result = await this.ui.runInstall(() => this.runtime.bootstrap(endpoint));
		} else {
			result = await this.ui.runInstall(() => this.runtime.maintain(endpoint));
		}
		if (this.disposed) {
			return false;
		}
		if (!result.ok) {
			this.ui.showInstallFailed(installFailureMessage);
			this.bootstrapFinished = true;
			this.resolveReady?.(false);
			return false;
		}
		if (!approved) {
			await this.state.approve();
		}

		this.bootstrapFinished = true;
		this.profileWasMissing = !result.value.profileAvailable;
		if (result.value.bridgeChanged) {
			await this.handleBridgeChange(endpoint);
			return true;
		}
		await this.ensureWorkbenchProcess(endpoint);
		this.finishIfReady();
		return true;
	}

	private async retryMissingProfile(): Promise<void> {
		if (!this.endpoint || !this.profileWasMissing || this.bootstrapInProgress) {
			return;
		}
		this.bootstrapInProgress = true;
		try {
			const result = await this.ui.runInstall(() => this.runtime.maintain(this.endpoint!));
			if (!result.ok) {
				this.ui.showInstallFailed(installFailureMessage);
				this.resolveReady?.(false);
				return;
			}
			this.profileWasMissing = !result.value.profileAvailable;
			if (result.value.bridgeChanged) {
				await this.handleBridgeChange(this.endpoint);
			}
		} finally {
			this.bootstrapInProgress = false;
		}
	}

	private async handleBridgeChange(endpoint: WorkbenchEndpoint): Promise<void> {
		this.requiresRestart = true;
		this.disconnectedAfterRequiredRestart = false;
		const process = await this.runtime.processStatus(endpoint);
		if (process.ok && process.value.isOpen) {
			this.ui.showStartOrRestart?.(true);
			return;
		}
		const launched = await this.ui.runInstall(() => this.runtime.launchDefault(endpoint));
		if (!launched.ok) {
			this.ui.showInstallFailed('The default Arma Reforger Workbench project could not be started.');
			this.resolveReady?.(false);
			return;
		}
		this.requiresRestart = false;
		this.finishIfReady();
	}

	private async ensureWorkbenchProcess(endpoint: WorkbenchEndpoint): Promise<void> {
		const process = await this.runtime.processStatus(endpoint);
		if (!process.ok || process.value.isOpen) {
			return;
		}
		const launched = await this.ui.runInstall(() => this.runtime.launchDefault(endpoint));
		if (!launched.ok) {
			this.ui.showInstallFailed('The default Arma Reforger Workbench project could not be started.');
			this.resolveReady?.(false);
		}
	}

	private finishIfReady(): void {
		if (this.bootstrapFinished
			&& this.connected
			&& (!this.requiresRestart || this.disconnectedAfterRequiredRestart)
			&& !this.profileWasMissing) {
			this.resolveReady?.(true);
		}
	}

	private readEndpoint(): WorkbenchEndpoint {
		const configuration = vscode.workspace.getConfiguration('reforgerScriptTools.workbench');
		return {
			host: configuration.get('host', '127.0.0.1'),
			port: configuration.get('port', 5775),
		};
	}
}

export function createWorkbenchIntegration(
	context: vscode.ExtensionContext,
	serverPath: Promise<string | undefined>,
): WorkbenchIntegrationCoordinator {
	const configuration = vscode.workspace.getConfiguration('reforgerScriptTools.workbench');
	const invoke = (
		workbenchEndpoint: WorkbenchEndpoint,
		action: Parameters<typeof invokeWorkbenchPrivateApi>[2],
		deadline: number,
	) => invokeWorkbenchPrivateApi(serverPath, workbenchEndpoint, action, deadline);
	const runtime: WorkbenchIntegrationRuntime = {
		status: async workbenchEndpoint => decodeStatus(await invoke(workbenchEndpoint, 'integration-status', 1_500)),
		bootstrap: async workbenchEndpoint => decodeBootstrap(await invoke(workbenchEndpoint, 'bootstrap-integration', 120_000)),
		maintain: async workbenchEndpoint => decodeBootstrap(await invoke(workbenchEndpoint, 'maintain-integration', 120_000)),
		processStatus: async workbenchEndpoint => decodeProcessStatus(await invoke(workbenchEndpoint, 'process-status', 5_000)),
		launchDefault: async workbenchEndpoint => invoke(workbenchEndpoint, 'launch-default', 120_000).then(result => result.ok
			? { ok: true, value: result.value }
			: result),
	};
	const ui: WorkbenchIntegrationUi = {
		confirmInstall: async () => (await vscode.window.showInformationMessage(
				'Enable Reforger Workbench Integration? This enables Workbench\'s local integration API and installs the managed bridge.',
				installChoice,
		)) === installChoice,
		runInstall: task => Promise.resolve(vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: 'Setting up Reforger Workbench Integration…',
				cancellable: false,
			},
			task,
		)),
		showInstallFailed: message => void vscode.window.showWarningMessage(message),
		showStartOrRestart: running => {
			if (running) {
				void vscode.window.showInformationMessage(restartMessage);
			}
		},
	};
	return new WorkbenchIntegrationCoordinator(
		{
			isApproved: () => context.globalState.get<boolean>(approvalStateKey, false),
			approve: () => Promise.resolve(context.globalState.update(approvalStateKey, true)),
		},
		runtime,
		ui,
		configuration.get('autoInstallIntegration', true),
	);
}

function decodeStatus(
	result: WorkbenchGatewayResult<unknown>,
): WorkbenchGatewayResult<WorkbenchIntegrationStatus> {
	if (!result.ok) {
		return result;
	}
	const value = result.value;
	if (!isRecord(value)
		|| !isRecord(value.bridge)
		|| typeof value.bridge.installed !== 'boolean'
		|| typeof value.bridge.installationAvailable !== 'boolean') {
		return protocolFailure();
	}
	return { ok: true, value: {
		installed: value.bridge.installed,
		installationAvailable: value.bridge.installationAvailable,
	} };
}

function decodeBootstrap(
	result: WorkbenchGatewayResult<unknown>,
): WorkbenchGatewayResult<WorkbenchIntegrationBootstrap> {
	if (!result.ok) {
		return result;
	}
	const value = result.value;
	if (!isRecord(value)
		|| typeof value.netApiEnabled !== 'boolean'
		|| typeof value.netApiWritePerformed !== 'boolean'
		|| typeof value.bridgeInstalled !== 'boolean'
		|| (value.bridgeVersion !== undefined && typeof value.bridgeVersion !== 'string')
		|| typeof value.bridgeChanged !== 'boolean'
		|| typeof value.profileAvailable !== 'boolean') {
		return protocolFailure();
	}
	return { ok: true, value: {
		netApiEnabled: value.netApiEnabled,
		netApiWritePerformed: value.netApiWritePerformed,
		bridgeInstalled: value.bridgeInstalled,
		...(value.bridgeVersion === undefined ? {} : { bridgeVersion: value.bridgeVersion }),
		bridgeChanged: value.bridgeChanged,
		profileAvailable: value.profileAvailable,
	} };
}

function decodeProcessStatus(
	result: WorkbenchGatewayResult<unknown>,
): WorkbenchGatewayResult<WorkbenchProcessStatus> {
	if (!result.ok) {
		return result;
	}
	const value = result.value;
	if (!isRecord(value)
		|| typeof value.isOpen !== 'boolean'
		|| (value.processId !== undefined && typeof value.processId !== 'number')
		|| (value.projectPath !== undefined && typeof value.projectPath !== 'string')) {
		return protocolFailure();
	}
	return { ok: true, value: {
		isOpen: value.isOpen,
		...(value.processId === undefined ? {} : { processId: value.processId }),
		...(value.projectPath === undefined ? {} : { projectPath: value.projectPath }),
	} };
}

function protocolFailure(): WorkbenchGatewayResult<never> {
	return { ok: false, failure: {
		category: 'protocol',
		recoveryHint: 'Workbench integration returned an invalid response.',
	} };
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
