import * as assert from 'node:assert';
import {
	WorkbenchIntegrationCoordinator,
	WorkbenchIntegrationRuntime,
	WorkbenchIntegrationStatus,
	WorkbenchIntegrationState,
	WorkbenchIntegrationUi,
} from '../workbenchNetApi/integration/workbenchIntegration';
import { WorkbenchEndpoint } from '../workbenchNetApi/gateway/workbenchGateway';

const endpoint: WorkbenchEndpoint = { host: '127.0.0.1', port: 5775 };

suite('Workbench Integration', () => {
	test('declining approval resolves startup without changing Workbench', async () => {
		let prompts = 0;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			stateWith(false),
			runtimeWith({
				status: async () => statusResult({ installed: false, installationAvailable: true }),
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({
				confirmInstall: async () => {
					prompts += 1;
					return false;
				},
			}),
			true,
		);

		await coordinator.start();

		assert.strictEqual(prompts, 1);
		assert.strictEqual(bootstraps, 0);
	});

	test('approval enables the integration and prompts to restart an open Workbench', async () => {
		let bootstraps = 0;
		let restartPrompts = 0;
		const state = stateWith(false);
		const coordinator = new WorkbenchIntegrationCoordinator(
			state,
			runtimeWith({
				status: async () => statusResult({ installed: false, installationAvailable: true }),
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult({ bridgeChanged: true });
				},
				processStatus: async () => ({ ok: true, value: { isOpen: true } }),
			}),
			uiWith({
				confirmInstall: async () => true,
				showStartOrRestart: running => {
					if (running) {
						restartPrompts += 1;
					}
				},
			}),
			true,
		);

		const ready = coordinator.start();
		await waitUntil(() => bootstraps === 1);
		coordinator.onWorkbenchConnected(endpoint);
		await waitUntil(() => restartPrompts === 1);

		assert.strictEqual(state.isApproved(), true);
		assert.strictEqual(bootstraps, 1);
		assert.strictEqual(restartPrompts, 1);

		coordinator.onWorkbenchDisconnected();
		coordinator.onWorkbenchConnected(endpoint);
		await ready;
	});

	test('approved integration maintains the bridge without prompting', async () => {
		let prompts = 0;
		let maintenance = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			stateWith(true),
			runtimeWith({
				status: async () => statusResult({ maintenanceRequired: true }),
				maintain: async () => {
					maintenance += 1;
					return bootstrapResult({ bridgeChanged: false });
				},
				processStatus: async () => ({ ok: true, value: { isOpen: true } }),
			}),
			uiWith({ confirmInstall: async () => {
				prompts += 1;
				return true;
			} }),
			true,
		);

		const ready = coordinator.start();
		await waitUntil(() => maintenance === 1);
		coordinator.onWorkbenchConnected(endpoint);
		await ready;

		assert.strictEqual(prompts, 0);
		assert.strictEqual(maintenance, 1);
	});

	test('an installed bridge still requires the new one-time approval', async () => {
		let prompts = 0;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			stateWith(false),
			runtimeWith({
				status: async () => statusResult({ maintenanceRequired: true }),
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({
				confirmInstall: async () => {
					prompts += 1;
					return false;
				},
			}),
			true,
		);

		await coordinator.start();

		assert.strictEqual(prompts, 1);
		assert.strictEqual(bootstraps, 0);
	});

	test('approved integration launches the default project when Workbench is closed', async () => {
		let launches = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			stateWith(true),
			runtimeWith({
				status: async () => statusResult({ maintenanceRequired: true, workbenchRunning: false }),
				maintain: async () => bootstrapResult({ bridgeChanged: false }),
				processStatus: async () => ({ ok: true, value: { isOpen: false } }),
				launchDefault: async () => {
					launches += 1;
					return { ok: true, value: {} };
				},
			}),
			uiWith({}),
			true,
		);

		const ready = coordinator.start();
		await waitUntil(() => launches === 1);
		coordinator.onWorkbenchConnected(endpoint);
		await ready;

		assert.strictEqual(launches, 1);
	});

	test('approved current integration skips warm maintenance and process probing', async () => {
		let maintenance = 0;
		let processStatus = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			stateWith(true),
			runtimeWith({
				status: async () => statusResult(),
				maintain: async () => {
					maintenance += 1;
					return bootstrapResult();
				},
				processStatus: async () => {
					processStatus += 1;
					return { ok: true, value: { isOpen: true } };
				},
			}),
			uiWith({}),
			true,
		);

		const ready = coordinator.start();
		coordinator.onWorkbenchConnected(endpoint);
		await ready;

		assert.strictEqual(maintenance, 0);
		assert.strictEqual(processStatus, 0);
	});
});

function statusResult(
	overrides: Partial<WorkbenchIntegrationStatus> = {},
) {
	return {
		ok: true as const,
		value: {
			installed: overrides.installed ?? true,
			installationAvailable: overrides.installationAvailable ?? false,
			maintenanceRequired: overrides.maintenanceRequired ?? false,
			profileAvailable: overrides.profileAvailable ?? true,
			workbenchRunning: overrides.workbenchRunning ?? true,
		},
	};
}

function bootstrapResult(
	overrides: Partial<{
		bridgeChanged: boolean;
		profileAvailable: boolean;
	}> = {},
) {
	return {
		ok: true as const,
		value: {
			netApiEnabled: true,
			netApiWritePerformed: true,
			bridgeInstalled: true,
			bridgeVersion: '1.52.12',
			bridgeChanged: overrides.bridgeChanged ?? false,
			profileAvailable: overrides.profileAvailable ?? true,
		},
	};
}

async function waitUntil(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 20 && !predicate(); attempt += 1) {
		await new Promise(resolve => setTimeout(resolve, 0));
	}
	assert.strictEqual(predicate(), true);
}

function stateWith(approved: boolean): WorkbenchIntegrationState {
	let value = approved;
	return {
		isApproved: () => value,
		approve: async () => {
			value = true;
		},
	};
}

function runtimeWith(
	overrides: Partial<WorkbenchIntegrationRuntime>,
): WorkbenchIntegrationRuntime {
	return {
		status: async () => ({ ok: false, failure: { category: 'unavailable', recoveryHint: 'unavailable' } }),
		bootstrap: async () => bootstrapResult(),
		maintain: async () => bootstrapResult(),
		processStatus: async () => ({ ok: true, value: { isOpen: true } }),
		launchDefault: async () => ({ ok: true, value: {} }),
		...overrides,
	};
}

function uiWith(overrides: Partial<WorkbenchIntegrationUi>): WorkbenchIntegrationUi {
	return {
		confirmInstall: async () => false,
		runInstall: task => task(),
		showInstallFailed: () => undefined,
		...overrides,
	};
}
