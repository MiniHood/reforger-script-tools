import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
	workbenchCommands,
	workbenchConfig,
	workbenchDiagnostics,
	workbenchTestCommands,
} from '../extensionConfig/workbench';
import type { WorkbenchCompilerObservation } from '../workbenchCompiler/workbenchCompiler';
import { startNetApiPeer } from './netApiPeer';

const workbenchFixtureSource = 'class WorkbenchCompilerFixture\n{\n}\n';

suite('Workbench compiler validation', () => {
	teardown(async () => {
		const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
		for (const setting of Object.values(workbenchConfig.settings)) {
			await configuration.update(setting, undefined, vscode.ConfigurationTarget.Global);
		}
	});

	test('manual validation publishes compiler diagnostics from the configured endpoint', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return {
					errorCode: '',
					payload: { IsRunning: true, ScriptsCompiled: true },
				};
			}
			assert.deepStrictEqual(payload, {
				APIFunc: 'ValidateScripts',
				Configuration: 'WORKBENCH',
			});
			return {
				errorCode: '',
				payload: {
					Errors: [{
						error: "Undefined function 'Run'",
						file: 'Scripts/Game/Example.c',
						fileAbs: sourcePath,
						addon: path.basename(workspace.uri.fsPath),
						line: 2,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		try {
			await configurePeer(peer.port, 0);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);

			const diagnostics = await waitFor(() => {
				const current = vscode.languages.getDiagnostics(sourceUri)
					.filter(diagnostic => diagnostic.source?.startsWith(workbenchDiagnostics.source));
				return current.length > 0 ? current : undefined;
			});
			assert.strictEqual(diagnostics.length, 1);
			assert.strictEqual(diagnostics[0].message, "Undefined function 'Run'");
			assert.strictEqual(diagnostics[0].source, workbenchDiagnostics.source);
			assert.strictEqual(diagnostics[0].severity, vscode.DiagnosticSeverity.Error);
			assert.strictEqual(diagnostics[0].range.start.line, 1);
		} finally {
			await peer.close();
		}
	});

	test('idle validation saves only the active script before compiling', async () => {
		const workspace = onlyWorkspaceFolder();
		const activePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const otherPath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Other.c');
		const originalActive = await fs.readFile(activePath, 'utf8');
		await fs.writeFile(otherPath, 'class OtherFixture {}\n', 'utf8');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: '', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port, 0.05);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));
			const otherDocument = await vscode.workspace.openTextDocument(otherPath);
			const activeDocument = await vscode.workspace.openTextDocument(activePath);
			await vscode.window.showTextDocument(activeDocument);
			await applyAppend(otherDocument, '// unsaved other edit');
			await applyAppend(activeDocument, '// active edit');
			assert.strictEqual(activeDocument.languageId, 'enforce');
			assert.strictEqual(activeDocument.isDirty, true);

			await waitFor(() => {
				const validated = peer.requests.some(request =>
					(request.payload as { APIFunc?: string }).APIFunc === 'ValidateScripts');
				return validated && !activeDocument.isDirty ? true : undefined;
			});

			assert.match(await fs.readFile(activePath, 'utf8'), /\/\/ active edit/);
			assert.strictEqual(await fs.readFile(otherPath, 'utf8'), 'class OtherFixture {}\n');
			assert.strictEqual(otherDocument.isDirty, true);
			assert.match((await observeWorkbenchCompiler()).tooltip, /Compiler result: stale/);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(otherPath));
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await fs.writeFile(activePath, originalActive, 'utf8');
			await fs.unlink(otherPath).catch(() => undefined);
			await peer.close();
		}
	});

	test('uses the default three-second idle delay for automatic validation', async function () {
		this.timeout(7_000);
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		await fs.writeFile(sourcePath, workbenchFixtureSource, 'utf8');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: '', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.enabled,
				true,
				vscode.ConfigurationTarget.Global,
			);
			await configuration.update(
				workbenchConfig.settings.host,
				'127.0.0.1',
				vscode.ConfigurationTarget.Global,
			);
			await configuration.update(
				workbenchConfig.settings.port,
				peer.port,
				vscode.ConfigurationTarget.Global,
			);
			const document = await vscode.workspace.openTextDocument(sourcePath);
			await vscode.window.showTextDocument(document);
			const editedAt = Date.now();
			await applyAppend(document, '// default idle delay');
			await new Promise(resolve => setTimeout(resolve, 500));
			assert.strictEqual(validationRequests(peer).length, 0);
			assert.strictEqual(document.isDirty, true);

			await waitFor(
				() => validationRequests(peer).length === 1 ? true : undefined,
				4_500,
			);
			await waitFor(async () =>
				(await observeWorkbenchCompiler()).phase === 'ready' ? true : undefined);
			assert.ok(Date.now() - editedAt >= 2_500);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await fs.writeFile(sourcePath, workbenchFixtureSource, 'utf8');
			await peer.close();
		}
	});

	test('coalesces triggers during a slow compile into one non-overlapping follow-up', async () => {
		let activeValidations = 0;
		let maximumConcurrentValidations = 0;
		let validationCount = 0;
		let releaseFirstValidation: (() => void) | undefined;
		const firstValidationGate = new Promise<void>(resolve => {
			releaseFirstValidation = resolve;
		});
		const peer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			activeValidations += 1;
			maximumConcurrentValidations = Math.max(maximumConcurrentValidations, activeValidations);
			if (validationCount === 1) {
				await firstValidationGate;
			}
			activeValidations -= 1;
			return { errorCode: '', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port, 0);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));

			const first = vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => validationCount === 1 ? true : undefined);
			assert.strictEqual((await observeWorkbenchCompiler()).phase, 'validating');
			const second = vscode.commands.executeCommand(workbenchCommands.validateScripts);
			releaseFirstValidation?.();
			await Promise.all([first, second]);
			await waitFor(() => validationCount === 2 ? true : undefined);

			assert.strictEqual(validationCount, 2);
			assert.strictEqual(maximumConcurrentValidations, 1);
		} finally {
			releaseFirstValidation?.();
			await peer.close();
		}
	});

	test('marks prior compiler findings stale and clears them on a clean validation', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		const original = await fs.readFile(sourcePath, 'utf8');
		const parserDiagnostics = vscode.languages.createDiagnosticCollection('provisional-parser-test');
		const provisional = new vscode.Diagnostic(
			new vscode.Range(0, 0, 0, 0),
			'Provisional parser finding',
			vscode.DiagnosticSeverity.Warning,
		);
		provisional.source = 'Provisional Parser';
		parserDiagnostics.set(sourceUri, [provisional]);
		let validationCount = 0;
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			return validationCount === 1
				? {
					errorCode: '',
					payload: {
						Errors: [{
							error: 'First compiler finding',
							file: 'Scripts/Game/Example.c',
							line: 1,
						}],
						Warnings: [],
						Success: false,
					},
				}
				: { errorCode: '', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port, 0);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const document = await vscode.workspace.openTextDocument(sourcePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// newer edit');

			const stale = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current[0]?.source?.endsWith('(stale)') ? current[0] : undefined;
			});
			assert.match(stale.message, /^\[Stale Workbench result/);
			assert.match((await observeWorkbenchCompiler()).text, /stale/i);
			assert.match((await observeWorkbenchCompiler()).tooltip, /Compiler result: stale/);

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 0 ? true : undefined);
			assert.deepStrictEqual(workbenchDiagnosticsFor(sourceUri), []);
			assert.ok(vscode.languages.getDiagnostics(sourceUri).some(
				diagnostic => diagnostic.source === 'Provisional Parser',
			));
		} finally {
			parserDiagnostics.dispose();
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await fs.writeFile(sourcePath, original, 'utf8');
			await peer.close();
		}
	});

	test('applies enablement immediately and presents the configured status without probing', async () => {
		const peer = await startNetApiPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			await configurePeer(peer.port, 0);
			const ready = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready' ? observation : undefined;
			});
			assert.match(ready.tooltip, new RegExp(`127\\.0\\.0\\.1:${peer.port}`));
			assert.match(ready.tooltip, /Profile: WORKBENCH/);
			assert.match(ready.tooltip, /cannot prove that it matches this VS Code workspace/);

			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.enabled,
				false,
				vscode.ConfigurationTarget.Global,
			);
			await waitFor(async () =>
				(await observeWorkbenchCompiler()).phase === 'disabled' ? true : undefined);
			const requestCount = peer.requests.length;
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await new Promise(resolve => setTimeout(resolve, 100));

			assert.strictEqual((await observeWorkbenchCompiler()).phase, 'disabled');
			assert.strictEqual(peer.requests.length, requestCount);
		} finally {
			await peer.close();
		}
	});

	test('presents starting while Workbench scripts are not compiled', async () => {
		const peer = await startNetApiPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: false },
		}));
		try {
			await configurePeer(peer.port, 0);
			const starting = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'starting' ? observation : undefined;
			});

			assert.match(starting.text, /starting/i);
			assert.match(starting.tooltip, /scripts are not ready/i);
		} finally {
			await peer.close();
		}
	});

	test('rejects an unsupported profile setting without calling validation', async () => {
		const peer = await startNetApiPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			await configurePeer(peer.port, 0);
			await waitFor(() => peer.requests.length > 0 ? true : undefined);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.validationProfile,
				'PC',
				vscode.ConfigurationTarget.Global,
			);
			await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready' && observation.tooltip.includes('Profile: PC')
					? true
					: undefined;
			});

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);

			assert.strictEqual(validationRequests(peer).length, 0);
			const observation = await observeWorkbenchCompiler();
			assert.strictEqual(observation.phase, 'unavailable');
			assert.match(observation.tooltip, /Profile: PC/);
			assert.match(observation.tooltip, /Failure: unsupported/);
		} finally {
			await peer.close();
		}
	});

	test('applies validation-delay changes to queued and future edits immediately', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const original = await fs.readFile(sourcePath, 'utf8');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: '', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port, 0.2);
			const document = await vscode.workspace.openTextDocument(sourcePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// queued before delay change');
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.validationDelaySeconds,
				0,
				vscode.ConfigurationTarget.Global,
			);
			await new Promise(resolve => setTimeout(resolve, 300));
			assert.strictEqual(validationRequests(peer).length, 0);
			assert.strictEqual(document.isDirty, true);

			await configuration.update(
				workbenchConfig.settings.validationDelaySeconds,
				0.05,
				vscode.ConfigurationTarget.Global,
			);
			await applyAppend(document, '// edit after delay change');
			await waitFor(() => validationRequests(peer).length === 1 ? true : undefined);
			assert.strictEqual(document.isDirty, false);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await fs.writeFile(sourcePath, original, 'utf8');
			await peer.close();
		}
	});

	test('retains stale findings and reports save-failed when the active script cannot be saved', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		const original = await fs.readFile(sourcePath, 'utf8');
		let validationCount = 0;
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			return {
				errorCode: '',
				payload: {
					Errors: [{
						error: 'Retained finding',
						file: 'Scripts/Game/Example.c',
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		try {
			await configurePeer(peer.port, 0);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const document = await vscode.workspace.openTextDocument(sourcePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// dirty editor version');
			await fs.writeFile(sourcePath, `${original}// conflicting disk version`, 'utf8');

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);

			assert.strictEqual(validationCount, 1);
			const retained = workbenchDiagnosticsFor(sourceUri);
			assert.strictEqual(retained.length, 1);
			assert.match(retained[0].source ?? '', /\(stale\)$/);
			assert.match((await observeWorkbenchCompiler()).tooltip, /save-failed/);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await fs.writeFile(sourcePath, original, 'utf8');
			await peer.close();
		}
	});

	test('retains findings as stale when the configured Workbench endpoint becomes unavailable', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourceUri = vscode.Uri.file(path.join(
			workspace.uri.fsPath,
			'Scripts',
			'Game',
			'Example.c',
		));
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } }
				: {
					errorCode: '',
					payload: {
						Errors: [{
							error: 'Finding retained through outage',
							file: 'Scripts/Game/Example.c',
							line: 1,
						}],
						Warnings: [],
						Success: false,
					},
				};
		});
		const unavailablePeer = await startNetApiPeer(() => ({ silent: true }));
		const unavailablePort = unavailablePeer.port;
		await unavailablePeer.close();
		try {
			await configurePeer(peer.port, 0);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.port,
				unavailablePort,
				vscode.ConfigurationTarget.Global,
			);

			await waitFor(async () =>
				(await observeWorkbenchCompiler()).phase === 'unavailable' ? true : undefined);
			const retained = workbenchDiagnosticsFor(sourceUri);
			assert.strictEqual(retained.length, 1);
			assert.match(retained[0].source ?? '', /\(stale\)$/);
			assert.match((await observeWorkbenchCompiler()).tooltip, /Compiler result: stale/);
		} finally {
			await peer.close();
		}
	});

	test('projects only proven project-contained compiler locations into VS Code', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		const externalPath = path.join(path.dirname(workspace.uri.fsPath), 'ExternalCompilerLocation.c');
		await fs.writeFile(externalPath, 'class ExternalCompilerLocation {}\n', 'utf8');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } }
				: {
					errorCode: '',
					payload: {
						Errors: [{
							error: 'Relative contained location',
							file: 'Scripts/Game/Example.c',
							line: 1,
						}, {
							error: 'External absolute location',
							file: 'Scripts/Game/Example.c',
							fileAbs: externalPath,
							line: 1,
						}, {
							error: 'Escaping relative location',
							file: '../ExternalCompilerLocation.c',
							line: 1,
						}, {
							error: 'Unresolvable location',
							file: 'Scripts/Game/Missing.c',
							line: 1,
						}],
						Warnings: [],
						Success: false,
					},
				};
		});
		try {
			await configurePeer(peer.port, 0);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			const diagnostics = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current.length > 0 ? current : undefined;
			});

			assert.deepStrictEqual(
				diagnostics.map(diagnostic => diagnostic.message),
				['Relative contained location'],
			);
			assert.deepStrictEqual(workbenchDiagnosticsFor(vscode.Uri.file(externalPath)), []);
			assert.strictEqual(
				(await observeWorkbenchCompiler()).lastValidationResult?.diagnostics.length,
				4,
			);
		} finally {
			await fs.unlink(externalPath).catch(() => undefined);
			await peer.close();
		}
	});

	test('keeps a result stale when scripts change during its validation request', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		let releaseValidation: (() => void) | undefined;
		const validationGate = new Promise<void>(resolve => {
			releaseValidation = resolve;
		});
		let validationStarted = false;
		const peer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationStarted = true;
			await validationGate;
			return {
				errorCode: '',
				payload: {
					Errors: [{
						error: 'Finding for the older saved snapshot',
						file: 'Scripts/Game/Example.c',
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		try {
			await configurePeer(peer.port, 0);
			const document = await vscode.workspace.openTextDocument(sourcePath);
			await vscode.window.showTextDocument(document);
			const command = vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => validationStarted ? true : undefined);
			await applyAppend(document, '// edit during validation');
			releaseValidation?.();
			await command;

			const stale = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current[0]?.source?.endsWith('(stale)') ? current[0] : undefined;
			});
			assert.match(stale.message, /Finding for the older saved snapshot/);
			assert.match((await observeWorkbenchCompiler()).tooltip, /Compiler result: stale/);
		} finally {
			releaseValidation?.();
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await peer.close();
		}
	});

	test('supersedes an in-flight result when endpoint settings change', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourceUri = vscode.Uri.file(path.join(
			workspace.uri.fsPath,
			'Scripts',
			'Game',
			'Example.c',
		));
		let releaseOldValidation: (() => void) | undefined;
		const oldValidationGate = new Promise<void>(resolve => {
			releaseOldValidation = resolve;
		});
		let oldValidationStarted = false;
		const oldPeer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: '', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			oldValidationStarted = true;
			await oldValidationGate;
			return {
				errorCode: '',
				payload: {
					Errors: [{
						error: 'Obsolete endpoint result',
						file: 'Scripts/Game/Example.c',
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		const newPeer = await startNetApiPeer(() => ({
			errorCode: '',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			await configurePeer(oldPeer.port, 0);
			const oldCommand = vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => oldValidationStarted ? true : undefined);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.port,
				newPeer.port,
				vscode.ConfigurationTarget.Global,
			);
			releaseOldValidation?.();
			await oldCommand;
			await waitFor(() => newPeer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));

			assert.match(
				(await observeWorkbenchCompiler()).tooltip,
				new RegExp(`127\\.0\\.0\\.1:${newPeer.port}`),
			);
			assert.ok(!workbenchDiagnosticsFor(sourceUri).some(
				diagnostic => diagnostic.message.includes('Obsolete endpoint result'),
			));
		} finally {
			releaseOldValidation?.();
			await Promise.all([oldPeer.close(), newPeer.close()]);
		}
	});
});

async function configurePeer(port: number, delaySeconds: number): Promise<void> {
	const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
	await configuration.update(workbenchConfig.settings.enabled, true, vscode.ConfigurationTarget.Global);
	await configuration.update(workbenchConfig.settings.host, '127.0.0.1', vscode.ConfigurationTarget.Global);
	await configuration.update(workbenchConfig.settings.port, port, vscode.ConfigurationTarget.Global);
	await configuration.update(
		workbenchConfig.settings.validationDelaySeconds,
		delaySeconds,
		vscode.ConfigurationTarget.Global,
	);
}

function onlyWorkspaceFolder(): vscode.WorkspaceFolder {
	const folders = vscode.workspace.workspaceFolders;
	assert.ok(folders && folders.length === 1);
	return folders[0];
}

async function waitFor<T>(
	read: () => T | undefined | Promise<T | undefined>,
	timeoutMs = 4_000,
): Promise<T> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		const value = await read();
		if (value !== undefined) {
			return value;
		}
		await new Promise(resolve => setTimeout(resolve, 20));
	}
	throw new Error('Timed out waiting for Workbench compiler behavior');
}

async function observeWorkbenchCompiler(): Promise<WorkbenchCompilerObservation> {
	const observation = await vscode.commands.executeCommand<WorkbenchCompilerObservation>(
		workbenchTestCommands.observeCompiler,
	);
	assert.ok(observation, 'Workbench compiler test observation command is registered');
	return observation;
}

async function applyAppend(document: vscode.TextDocument, text: string): Promise<void> {
	const edit = new vscode.WorkspaceEdit();
	edit.insert(document.uri, document.positionAt(document.getText().length), text);
	assert.strictEqual(await vscode.workspace.applyEdit(edit), true);
}

function workbenchDiagnosticsFor(uri: vscode.Uri): vscode.Diagnostic[] {
	return vscode.languages.getDiagnostics(uri)
		.filter(diagnostic => diagnostic.source?.startsWith(workbenchDiagnostics.source));
}

function validationRequests(peer: { requests: Array<{ payload: unknown }> }): unknown[] {
	return peer.requests.filter(request =>
		(request.payload as { APIFunc?: string }).APIFunc === 'ValidateScripts');
}
