import * as vscode from 'vscode';
import { diagnostic } from '../../diagnostics/diagnostics';
import {
	invokeWorkbenchPrivateApi,
	WorkbenchEndpoint,
	WorkbenchGatewayResult,
} from '../gateway/workbenchGateway';

const installChoice = 'Install Workbench Integration';
const installFailureMessage = 'Reforger Workbench script tools could not be installed.';
const activationPendingMessage = 'Reforger Workbench script tools were installed in your Workbench profile. Refresh Workbench with Ctrl+Shift+R to activate them.';

export interface WorkbenchIntegrationStatus {
	installed: boolean;
	installationAvailable: boolean;
}

export interface WorkbenchIntegrationInstallResult {
	activated: boolean;
}

export interface WorkbenchIntegrationRuntime {
	status(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchIntegrationStatus>>;
	install(
		endpoint: WorkbenchEndpoint,
	): Promise<WorkbenchGatewayResult<WorkbenchIntegrationInstallResult>>;
}

export interface WorkbenchIntegrationUi {
	confirmInstall(): Promise<boolean>;
	runInstall<T>(task: () => Promise<T>): Promise<T>;
	showInstalled(message: string): void;
	showActivationPending(message: string): void;
	showInstallFailed(message: string): void;
}

export class WorkbenchIntegrationCoordinator implements vscode.Disposable {
	private promptAttempted = false;
	private checking = false;
	private connectionHandled = false;
	private connectionGeneration = 0;
	private disposed = false;

	public constructor(
		private readonly runtime: WorkbenchIntegrationRuntime,
		private readonly ui: WorkbenchIntegrationUi,
	) {}

	public async onWorkbenchConnected(endpoint: WorkbenchEndpoint): Promise<void> {
		if (this.disposed || this.connectionHandled || this.checking) {
			return;
		}
		const generation = this.connectionGeneration;
		this.checking = true;
		try {
			const status = await this.runtime.status(endpoint);
			if (this.disposed
				|| generation !== this.connectionGeneration
				|| !status.ok) {
				return;
			}
			this.connectionHandled = true;
			if (status.value.installed) {
				return;
			}
			if (!status.value.installationAvailable || this.promptAttempted) {
				return;
			}
			this.promptAttempted = true;
			const approved = await this.ui.confirmInstall();
			if (this.disposed || !approved) {
				return;
			}
			const installed = await this.ui.runInstall(() => this.runtime.install(endpoint));
			if (this.disposed) {
				return;
			}
			if (installed.ok) {
				if (installed.value.activated) {
					this.ui.showInstalled('Reforger Workbench script tools installed.');
					diagnostic('workbenchIntegrationInstall', { outcome: 'activated' });
				} else {
					this.ui.showActivationPending(activationPendingMessage);
					diagnostic('workbenchIntegrationInstall', { outcome: 'reload-required' });
				}
				return;
			}
			this.ui.showInstallFailed(installFailureMessage);
			diagnostic('workbenchIntegrationInstall', {
				outcome: installed.failure.category,
			});
		} finally {
			this.checking = false;
		}
	}

	public onWorkbenchDisconnected(): void {
		this.connectionGeneration += 1;
		this.connectionHandled = false;
	}

	public dispose(): void {
		this.disposed = true;
		this.connectionGeneration += 1;
	}
}

export function createWorkbenchIntegration(
	serverPath: Promise<string | undefined>,
): WorkbenchIntegrationCoordinator {
	const runtime: WorkbenchIntegrationRuntime = {
		status: async endpoint => decodeStatus(
			await invokeWorkbenchPrivateApi(
				serverPath,
				endpoint,
				'integration-status',
				1_500,
			),
		),
		install: async endpoint => decodeInstall(
			await invokeWorkbenchPrivateApi(
				serverPath,
				endpoint,
				'install-bridge',
				120_000,
			),
		),
	};
	const ui: WorkbenchIntegrationUi = {
		confirmInstall: async () => (
			await vscode.window.showInformationMessage(
				'Install Reforger Script Tools to enhance Workbench integration?',
				installChoice,
			)
		) === installChoice,
		runInstall: task => Promise.resolve(vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: 'Installing Reforger Workbench script tools…',
				cancellable: false,
			},
			task,
		)),
		showInstalled: message => {
			void vscode.window.showInformationMessage(message);
		},
		showActivationPending: message => {
			void vscode.window.showWarningMessage(message);
		},
		showInstallFailed: message => {
			void vscode.window.showWarningMessage(message);
		},
	};
	return new WorkbenchIntegrationCoordinator(runtime, ui);
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
	return {
		ok: true,
		value: {
			installed: value.bridge.installed,
			installationAvailable: value.bridge.installationAvailable,
		},
	};
}

function decodeInstall(
	result: WorkbenchGatewayResult<unknown>,
): WorkbenchGatewayResult<WorkbenchIntegrationInstallResult> {
	if (!result.ok) {
		return result;
	}
	if (!isRecord(result.value) || typeof result.value.activated !== 'boolean') {
		return protocolFailure();
	}
	return { ok: true, value: { activated: result.value.activated } };
}

function protocolFailure(): WorkbenchGatewayResult<never> {
	return {
		ok: false,
		failure: {
			category: 'protocol',
			recoveryHint: 'Workbench integration returned an invalid response.',
		},
	};
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
