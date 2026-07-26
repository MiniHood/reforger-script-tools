import * as assert from 'node:assert';
import {
	WorkbenchIntegrationCoordinator,
	WorkbenchIntegrationRuntime,
	WorkbenchIntegrationUi,
} from '../workbenchNetApi/integration/workbenchIntegration';
import { WorkbenchEndpoint } from '../workbenchNetApi/gateway/workbenchGateway';

const endpoint: WorkbenchEndpoint = { host: '127.0.0.1', port: 5775 };

suite('Workbench Integration', () => {
	test('dismisses the first-install prompt without installing or prompting again this session', async () => {
		let prompts = 0;
		let installs = 0;
		const runtime = runtimeWith({
			status: async () => ({
				ok: true,
				value: { installed: false, installationAvailable: true },
			}),
			install: async () => {
				installs += 1;
				return { ok: true, value: { activated: true } };
			},
		});
		const ui = uiWith({
			confirmInstall: async () => {
				prompts += 1;
				return false;
			},
		});
		const coordinator = new WorkbenchIntegrationCoordinator(runtime, ui);

		await coordinator.onWorkbenchConnected(endpoint);
		await coordinator.onWorkbenchConnected(endpoint);

		assert.strictEqual(prompts, 1);
		assert.strictEqual(installs, 0);
	});

	test('installs after extension UI consent and reports activation', async () => {
		let installs = 0;
		const messages: string[] = [];
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => ({
					ok: true,
					value: { installed: false, installationAvailable: true },
				}),
				install: async () => {
					installs += 1;
					return { ok: true, value: { activated: true } };
				},
			}),
			uiWith({
				confirmInstall: async () => true,
				showInstalled: message => messages.push(message),
			}),
		);

		await coordinator.onWorkbenchConnected(endpoint);

		assert.strictEqual(installs, 1);
		assert.deepStrictEqual(messages, ['Reforger Workbench script tools installed.']);
	});

	test('does not prompt when a consent manifest is already installed', async () => {
		let prompts = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => ({
					ok: true,
					value: { installed: true, installationAvailable: false },
				}),
			}),
			uiWith({
				confirmInstall: async () => {
					prompts += 1;
					return true;
				},
			}),
		);

		await coordinator.onWorkbenchConnected(endpoint);

		assert.strictEqual(prompts, 0);
	});

	test('maintains once per continuous connection and checks again after reconnect', async () => {
		let statusCalls = 0;
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => {
					statusCalls += 1;
					return {
						ok: true,
						value: { installed: true, installationAvailable: false },
					};
				},
			}),
			uiWith({}),
		);

		await coordinator.onWorkbenchConnected(endpoint);
		await coordinator.onWorkbenchConnected(endpoint);
		coordinator.onWorkbenchDisconnected();
		await coordinator.onWorkbenchConnected(endpoint);

		assert.strictEqual(statusCalls, 2);
	});

	test('informs the user that a Workbench refresh activates a successful installation', async () => {
		let statusCalls = 0;
		const activationWarnings: string[] = [];
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => {
					statusCalls += 1;
					return statusCalls === 1
						? { ok: false, failure: { category: 'unavailable', recoveryHint: 'ignored' } }
						: {
							ok: true,
							value: { installed: false, installationAvailable: true },
						};
				},
				install: async () => ({ ok: true, value: { activated: false } }),
			}),
			uiWith({
				confirmInstall: async () => true,
				showActivationPending: message => activationWarnings.push(message),
			}),
		);

		await coordinator.onWorkbenchConnected(endpoint);
		await coordinator.onWorkbenchConnected(endpoint);

		assert.strictEqual(statusCalls, 2);
		assert.deepStrictEqual(
			activationWarnings,
			['Reforger Workbench script tools were installed in your Workbench profile. Refresh Workbench with Ctrl+Shift+R to activate them.'],
		);
	});

	test('reports a genuine installer failure separately from activation', async () => {
		const installFailures: string[] = [];
		const coordinator = new WorkbenchIntegrationCoordinator(
			runtimeWith({
				status: async () => ({
					ok: true,
					value: { installed: false, installationAvailable: true },
				}),
				install: async () => ({
					ok: false,
					failure: { category: 'unavailable', recoveryHint: 'ignored' },
				}),
			}),
			uiWith({
				confirmInstall: async () => true,
				showInstallFailed: message => installFailures.push(message),
			}),
		);

		await coordinator.onWorkbenchConnected(endpoint);

		assert.deepStrictEqual(
			installFailures,
			['Reforger Workbench script tools could not be installed.'],
		);
	});
});

function runtimeWith(
	overrides: Partial<WorkbenchIntegrationRuntime>,
): WorkbenchIntegrationRuntime {
	return {
		status: async () => ({
			ok: false,
			failure: { category: 'unavailable', recoveryHint: 'unavailable' },
		}),
		install: async () => ({
			ok: false,
			failure: { category: 'unavailable', recoveryHint: 'unavailable' },
		}),
		...overrides,
	};
}

function uiWith(overrides: Partial<WorkbenchIntegrationUi>): WorkbenchIntegrationUi {
	return {
		confirmInstall: async () => false,
		runInstall: task => task(),
		showInstalled: () => undefined,
		showActivationPending: () => undefined,
		showInstallFailed: () => undefined,
		...overrides,
	};
}
