import * as assert from 'node:assert';
import {
	WorkbenchIntegrationCoordinator,
	WorkbenchIntegrationRuntime,
	WorkbenchIntegrationStatus,
	WorkbenchIntegrationUi,
	workbenchStartupPolicy,
} from '../workbenchNetApi/integration/workbenchIntegration';
import { WorkbenchEndpoint } from '../workbenchNetApi/gateway/workbenchGateway';

const endpoint: WorkbenchEndpoint = { host: '127.0.0.1', port: 5775 };

suite('Workbench Integration', () => {
	test('does not contact Workbench before editor feature startup requests it', async () => {
		let statuses = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => {
					statuses += 1;
					return statusResult({ workbenchRunning: false });
				},
			}),
			uiWith({}),
			true,
		);

		coordinator.onWorkbenchConfigurationChanged(true);
		await Promise.resolve();
		assert.strictEqual(statuses, 0);

		await coordinator.start();
		assert.strictEqual(statuses, 1);
	});

	test('startup leaves an unset Workbench setting disabled and eligible for first approval', () => {
		assert.deepStrictEqual(workbenchStartupPolicy(false, false), {
			enabled: false,
			promptWhenDisabled: true,
		});
	});

	test('keeps feature startup gated until the first approval prompt is answered', async () => {
		let answerPrompt = (_approved: boolean): void => undefined;
		let prompted = false;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({
				confirmInstall: () => new Promise<boolean>(resolve => {
					prompted = true;
					answerPrompt = resolve;
				}),
			}),
			false,
		);

		void coordinator.start();
		await waitUntil(() => prompted);
		let consentSettled = false;
		void coordinator.whenConsentSettled().then(() => {
			consentSettled = true;
		});
		await Promise.resolve();
		assert.strictEqual(consentSettled, false);
		assert.strictEqual(bootstraps, 0);

		answerPrompt(true);
		assert.strictEqual(await coordinator.whenConsentSettled(), true);
		await waitUntil(() => bootstraps === 1);
	});

	test('disabled Workbench prompts for consent and enables the setting after approval', async () => {
		let enabled = false;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({ installed: false, installationAvailable: true }),
				bootstrap: async () => {
					assert.strictEqual(enabled, true);
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({ confirmInstall: async () => true }),
			false,
			async () => {
				enabled = true;
			},
		);

		const ready = coordinator.start();
		await waitUntil(() => bootstraps === 1);
		coordinator.onWorkbenchConnected(endpoint);
		await ready;

		assert.strictEqual(enabled, true);
	});

	test('explicitly disabled Workbench does not prompt or install', async () => {
		let prompts = 0;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({ confirmInstall: async () => {
				prompts += 1;
				return true;
			} }),
			false,
			undefined,
			false,
		);

		await coordinator.start();

		assert.strictEqual(prompts, 0);
		assert.strictEqual(bootstraps, 0);
	});

	test('click-requested enablement prompts after an explicit disable', async () => {
		let enabled = false;
		let prompts = 0;
		let bootstraps = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({
					installed: false,
					installationAvailable: true,
					workbenchRunning: false,
				}),
				bootstrap: async () => {
					assert.strictEqual(enabled, true);
					bootstraps += 1;
					return bootstrapResult();
				},
			}),
			uiWith({
				confirmInstall: async () => {
					prompts += 1;
					return true;
				},
			}),
			false,
			async () => {
				enabled = true;
			},
			false,
		);

		await coordinator.start();
		assert.strictEqual(prompts, 0);

		assert.strictEqual(await coordinator.requestEnablement(), true);
		assert.strictEqual(prompts, 1);
		assert.strictEqual(bootstraps, 1);
	});

	test('disabled Workbench stays dormant when the first-install prompt is not eligible', async () => {
		let statuses = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => {
					statuses += 1;
					return statusResult();
				},
			}),
			uiWith({}),
			false,
			undefined,
			false,
		);

		await coordinator.start();
		assert.strictEqual(statuses, 0);
	});

	test('enabling the setting later retries a declined installation without another prompt', async () => {
		let prompts = 0;
		let maintenance = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				maintain: async () => {
					maintenance += 1;
					return bootstrapResult();
				},
			}),
			uiWith({
				confirmInstall: async () => {
					prompts += 1;
					return false;
				},
			}),
			false,
		);

		await coordinator.start();
		coordinator.onWorkbenchConfigurationChanged(true);
		await waitUntil(() => maintenance === 1);

		assert.strictEqual(prompts, 1);
	});

	test('declining approval disables Workbench integration without installing it', async () => {
		let prompts = 0;
		let bootstraps = 0;
		let disabled = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
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
			false,
			undefined,
			true,
			async () => {
				disabled += 1;
			},
		);

		await coordinator.start();

		assert.strictEqual(prompts, 1);
		assert.strictEqual(bootstraps, 0);
		assert.strictEqual(disabled, 1);
		assert.strictEqual(await coordinator.whenConsentSettled(), false);
	});

	test('approval enables the integration and prompts to restart an open Workbench', async () => {
		let bootstraps = 0;
		let restartPrompts = 0;
		let enabled = false;
		const coordinator = new WorkbenchIntegrationCoordinator(
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
			false,
			async () => {
				enabled = true;
			},
		);

		const ready = coordinator.start();
		await waitUntil(() => bootstraps === 1);
		coordinator.onWorkbenchConnected(endpoint);
		await waitUntil(() => restartPrompts === 1);

		assert.strictEqual(enabled, true);
		assert.strictEqual(bootstraps, 1);
		assert.strictEqual(restartPrompts, 1);

		coordinator.onWorkbenchDisconnected();
		coordinator.onWorkbenchConnected(endpoint);
		await ready;
	});

	test('enabled integration maintains the bridge without prompting', async () => {
		let prompts = 0;
		let maintenance = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({
					maintenanceRequired: true,
					workbenchRunning: false,
				}),
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

	test('the enabled setting approves maintenance of an installed bridge', async () => {
		let prompts = 0;
		let maintenance = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({
					maintenanceRequired: true,
					workbenchRunning: false,
				}),
				maintain: async () => {
					maintenance += 1;
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

		assert.strictEqual(prompts, 0);
		assert.strictEqual(maintenance, 1);
	});

	test('enabled integration stays ready without launching a closed Workbench', async () => {
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({ maintenanceRequired: true, workbenchRunning: false }),
				maintain: async () => bootstrapResult({ bridgeChanged: false }),
				processStatus: async () => ({ ok: true, value: { isOpen: false } }),
			}),
			uiWith({}),
			true,
		);

		assert.strictEqual(await coordinator.start(), true);
	});

	test('first approval installs without launching a closed Workbench', async () => {
		let enabled = false;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({
					installed: false,
					installationAvailable: true,
					workbenchRunning: false,
				}),
				bootstrap: async () => bootstrapResult({ bridgeChanged: true }),
				processStatus: async () => ({ ok: true, value: { isOpen: false } }),
			}),
			uiWith({ confirmInstall: async () => true }),
			false,
			async () => {
				enabled = true;
			},
		);

		assert.strictEqual(await coordinator.start(), true);
		assert.strictEqual(enabled, true);
	});

	test('enabled current integration skips warm maintenance and process probing', async () => {
		let maintenance = 0;
		let processStatus = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
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

	test('enabled integration repairs a missing enfusion protocol registration', async () => {
		let bootstraps = 0;
		let maintenance = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => statusResult({ enfusionProtocolRegistered: false }),
				bootstrap: async () => {
					bootstraps += 1;
					return bootstrapResult();
				},
				maintain: async () => {
					maintenance += 1;
					return bootstrapResult();
				},
			}),
			uiWith({}),
			true,
		);

		const ready = coordinator.start();
		coordinator.onWorkbenchConnected(endpoint);
		await ready;

		assert.strictEqual(bootstraps, 1);
		assert.strictEqual(maintenance, 0);
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
			enfusionProtocolRegistered: overrides.enfusionProtocolRegistered ?? true,
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
			enfusionProtocolRegistered: true,
			enfusionProtocolWritePerformed: true,
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

function runtimeWith(
	overrides: Partial<WorkbenchIntegrationRuntime>,
): WorkbenchIntegrationRuntime {
	return {
		status: async () => ({ ok: false, failure: { category: 'unavailable', recoveryHint: 'unavailable' } }),
		bootstrap: async () => bootstrapResult(),
		maintain: async () => bootstrapResult(),
		processStatus: async () => ({ ok: true, value: { isOpen: true } }),
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
