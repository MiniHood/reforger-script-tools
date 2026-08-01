import * as assert from 'node:assert';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import {
	workbenchCommands,
	workbenchConfig,
	workbenchDefaults,
	workbenchDiagnostics,
	workbenchTestCommands,
} from '../extensionConfig/workbench';
import {
	workbenchConnectionStarted,
	type WorkbenchCompilerObservation,
} from '../workbenchNetApi/compiler/workbenchCompiler';
import { startNetApiPeer } from './netApiPeer';

const workbenchFixtureSource = 'class WorkbenchCompilerFixture\n{\n}\n';
let temporaryScriptCounter = 0;

suite('Workbench compiler validation', () => {
	teardown(async () => {
		const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
		await configuration.update(
			workbenchConfig.settings.enabled,
			false,
			vscode.ConfigurationTarget.Global,
		);
		for (const setting of Object.values(workbenchConfig.settings)) {
			if (setting === workbenchConfig.settings.enabled) {
				continue;
			}
			await configuration.update(setting, undefined, vscode.ConfigurationTarget.Global);
		}
	});

	test('manual validation publishes compiler diagnostics from the configured endpoint', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		let releaseValidation = (): void => undefined;
		const validationGate = new Promise<void>(resolve => {
			releaseValidation = resolve;
		});
		const peer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return {
					errorCode: 'Ok',
					payload: { IsRunning: true, ScriptsCompiled: true },
				};
			}
			if (payload.APIFunc === 'RST_WorkbenchLoadedAddonGraph') {
				return {
					errorCode: 'Ok',
					payload: {
						bridgeVersion: 'test',
						protocolVersion: 1,
						graphJson: '[]',
					},
				};
			}
			assert.deepStrictEqual(payload, {
				APIFunc: 'ValidateScripts',
				Configuration: 'WORKBENCH',
			});
			await validationGate;
			return {
				errorCode: 'Ok',
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
			await configurePeer(peer.port);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));

			const validationCommand = vscode.commands.executeCommand(
				workbenchCommands.validateScripts,
			);
			await waitFor(() => validationRequests(peer).length === 1 ? true : undefined);
			const waiting = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.validationOutput.includes('waiting for Workbench')
					? observation
					: undefined;
			});
			assert.strictEqual(waiting.phase, 'validating');
			assert.match(
				waiting.validationOutput,
				/^\[\d{2}:\d{2}:\d{2}\] Compilation requested — waiting for Workbench to finish\.\.\.\r?\n$/,
			);
			assert.deepStrictEqual(waiting.validationOutputLinks, []);
			releaseValidation();
			await validationCommand;

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
			assert.strictEqual(diagnostics[0].range.start.character, 0);
			assert.strictEqual(diagnostics[0].range.end.character, 1);
			const output = (await observeWorkbenchCompiler() as WorkbenchCompilerObservation & {
				validationOutput?: string;
			}).validationOutput;
			assert.ok(output);
			assert.match(
				output,
				/^\[\d{2}:\d{2}:\d{2}\] Compilation in (?:\d+ ms|\d+\.\d s) — 1 project error, 0 project warnings\r?\n/,
			);
			assert.match(output, /^\[FAILED\] Workbench reported compilation errors\.$/m);
			assert.doesNotMatch(output, /^Timing:/m);
			const findingLine = "[ERROR] Scripts/Game/Example.c:2 — Undefined function 'Run'";
			assert.ok(output.includes(findingLine));
			assert.doesNotMatch(output, new RegExp(escapeRegExp(workspace.uri.fsPath)));
			const [outputLink] = (await observeWorkbenchCompiler()).validationOutputLinks;
			assert.ok(outputLink);
			const { target, ...outputLinkRange } = outputLink;
			assert.deepStrictEqual(outputLinkRange, {
				line: 3,
				startCharacter: '[ERROR] '.length,
				endCharacter: findingLine.length,
			});
			const navigationTarget = vscode.Uri.parse(target);
			assert.strictEqual(navigationTarget.scheme, 'command');
			assert.strictEqual(
				navigationTarget.path,
				workbenchCommands.openCompilerDiagnostic,
			);
			const navigationArguments = JSON.parse(
				decodeURIComponent(navigationTarget.query),
			) as unknown[];
			assert.strictEqual(navigationArguments.length, 1);
			await vscode.commands.executeCommand(
				navigationTarget.path,
				...navigationArguments,
			);
			const navigatedEditor = vscode.window.activeTextEditor;
			assert.ok(navigatedEditor);
			assert.strictEqual(navigatedEditor.document.uri.toString(), sourceUri.toString());
			assert.deepStrictEqual(navigatedEditor.selection.start, new vscode.Position(1, 0));
			assert.deepStrictEqual(navigatedEditor.selection.end, new vscode.Position(1, 1));
			assert.deepStrictEqual(navigatedEditor.selection.active, new vscode.Position(1, 0));
		} finally {
			releaseValidation();
			await peer.close();
		}
	});

	test('replaces the waiting output when Workbench does not return a validation result', async () => {
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? {
					errorCode: 'Ok',
					payload: { IsRunning: true, ScriptsCompiled: true },
				}
				: { errorCode: 'RequestFailed', payload: {} };
		});
		try {
			await configurePeer(peer.port);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);

			const observation = await observeWorkbenchCompiler();
			assert.strictEqual(observation.phase, 'unavailable');
			assert.match(
				observation.validationOutput,
				/^\[\d{2}:\d{2}:\d{2}\] Compilation did not complete — Review Workbench state and retry the operation\.\r?\n$/,
			);
			assert.doesNotMatch(observation.validationOutput, /waiting for Workbench/);
			assert.deepStrictEqual(observation.validationOutputLinks, []);
		} finally {
			await peer.close();
		}
	});

	test('validates once after the first successful startup connection', async () => {
		const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
		await configuration.update(
			workbenchConfig.settings.enabled,
			false,
			vscode.ConfigurationTarget.Global,
		);
		await vscode.commands.executeCommand(workbenchTestCommands.restartCompiler);
		await vscode.commands.executeCommand(workbenchTestCommands.armStartupValidation);
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? {
					errorCode: 'Ok',
					payload: { IsRunning: true, ScriptsCompiled: true },
				}
				: {
					errorCode: 'Ok',
					payload: { Errors: [], Warnings: [], Success: true },
				};
		});
		try {
			await configurePeer(peer.port);
			await waitFor(() => validationRequests(peer).length === 1 ? true : undefined);
			const completed = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready'
					&& observation.validationOutput.includes(
						'[SUCCESS] Compilation completed successfully.',
					)
					? observation
					: undefined;
			});

			const operations = peer.requests.map(request =>
				(request.payload as { APIFunc?: string }).APIFunc);
			assert.deepStrictEqual(operations.slice(0, 2), [
				'IsWorkbenchRunning',
				'ValidateScripts',
			]);
			assert.match(completed.tooltip, /Validation succeeded/);
			await new Promise(resolve => setTimeout(resolve, 100));
			assert.strictEqual(validationRequests(peer).length, 1);
		} finally {
			await peer.close();
		}
	});

	test('treats Workbench starting after a reachable closed state as a new connection', () => {
		assert.strictEqual(
			workbenchConnectionStarted(
				{ isRunning: false, scriptsCompiled: true },
				{ isRunning: true, scriptsCompiled: true },
			),
			true,
		);
		assert.strictEqual(
			workbenchConnectionStarted(
				undefined,
				{ isRunning: false, scriptsCompiled: true },
			),
			false,
		);
		assert.strictEqual(
			workbenchConnectionStarted(
				{ isRunning: true, scriptsCompiled: true },
				{ isRunning: true, scriptsCompiled: false },
			),
			false,
		);
	});

	test('idle validation saves only the active script before compiling', async function () {
		this.timeout(7_000);
		const workspace = onlyWorkspaceFolder();
		const active = await createTemporaryScript(workspace, 'Active');
		const other = await createTemporaryScript(workspace, 'Other');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));
			const otherDocument = await vscode.workspace.openTextDocument(other.filePath);
			const activeDocument = await vscode.workspace.openTextDocument(active.filePath);
			await vscode.window.showTextDocument(activeDocument);
			await applyAppend(otherDocument, '// unsaved other edit');
			await applyAppend(activeDocument, '// active edit');
			assert.strictEqual(activeDocument.languageId, 'enforce');
			assert.strictEqual(activeDocument.isDirty, true);

			await waitFor(() => {
				const validated = peer.requests.some(request =>
					(request.payload as { APIFunc?: string }).APIFunc === 'ValidateScripts');
				return validated && !activeDocument.isDirty ? true : undefined;
			}, 5_000);

			assert.match(await fs.readFile(active.filePath, 'utf8'), /\/\/ active edit/);
			assert.strictEqual(await fs.readFile(other.filePath, 'utf8'), workbenchFixtureSource);
			assert.strictEqual(otherDocument.isDirty, true);
			const completed = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.tooltip.includes(
					'may be out of date because other scripts still have unsaved edits',
				)
					? observation
					: undefined;
			});
			assert.match(
				completed.tooltip,
				/may be out of date because other scripts still have unsaved edits/,
			);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(other.filePath));
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await active.remove();
			await other.remove();
			await peer.close();
		}
	});

	test('save validates immediately without waiting for the idle delay', async function () {
		this.timeout(4_000);
		const workspace = onlyWorkspaceFolder();
		const source = await createTemporaryScript(workspace, 'ImmediateSave');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port);
			await waitFor(() => peer.requests.some(request =>
				(request.payload as { APIFunc?: string }).APIFunc === 'IsWorkbenchRunning'));
			const document = await vscode.workspace.openTextDocument(source.filePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// validate immediately on save');
			assert.strictEqual(validationRequests(peer).length, 0);

			assert.strictEqual(await document.save(), true);
			await waitFor(
				() => validationRequests(peer).length === 1 ? true : undefined,
				2_000,
			);
			assert.strictEqual(document.isDirty, false);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await source.remove();
			await peer.close();
		}
	});

	test('uses the default three-second idle delay for automatic validation', async function () {
		this.timeout(7_000);
		const workspace = onlyWorkspaceFolder();
		const source = await createTemporaryScript(workspace, 'DefaultDelay');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.enabled,
				false,
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
			await configuration.update(
				workbenchConfig.settings.enabled,
				true,
				vscode.ConfigurationTarget.Global,
			);
			const document = await vscode.workspace.openTextDocument(source.filePath);
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
			await source.remove();
			await peer.close();
		}
	});

	test('does not save or validate idle edits when saved-idle validation is disabled', async function () {
		this.timeout(5_000);
		const workspace = onlyWorkspaceFolder();
		const source = await createTemporaryScript(workspace, 'SavedIdleDisabled');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.saveOnIdle,
				false,
				vscode.ConfigurationTarget.Global,
			);
			const document = await vscode.workspace.openTextDocument(source.filePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// keep dirty when saved-idle validation is disabled');

			await new Promise(resolve => setTimeout(resolve, 3_300));

			assert.strictEqual(document.isDirty, true);
			assert.strictEqual(validationRequests(peer).length, 0);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await source.remove();
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
				return { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			activeValidations += 1;
			maximumConcurrentValidations = Math.max(maximumConcurrentValidations, activeValidations);
			if (validationCount === 1) {
				await firstValidationGate;
			}
			activeValidations -= 1;
			return { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port);
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
		const source = await createTemporaryScript(workspace, 'StaleThenClean');
		const sourceUri = source.uri;
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
				return { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			return validationCount === 1
				? {
					errorCode: 'Ok',
					payload: {
						Errors: [{
							error: 'First compiler finding',
							file: source.relativePath,
							line: 1,
						}],
						Warnings: [],
						Success: false,
					},
				}
				: { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
		});
		try {
			await configurePeer(peer.port);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const document = await vscode.workspace.openTextDocument(source.filePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// newer edit');

			const stale = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current[0]?.source?.endsWith('(possibly outdated)')
					? current[0]
					: undefined;
			});
			assert.match(stale.message, /^\[Possibly outdated Workbench result/);
			const outdated = await observeWorkbenchCompiler();
			assert.strictEqual(outdated.text, '$(plug) Workbench Connected');
			assert.match(
				outdated.tooltip,
				/Compiler result may be out of date because the script has newer edits\./,
			);
			assert.match(
				outdated.tooltip,
				/It describes an earlier saved snapshot and will be replaced after the next successful validation\./,
			);
			assert.match(
				outdated.validationOutput,
				/Result status: may be out of date — the script has newer edits\./,
			);

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 0 ? true : undefined);
			assert.deepStrictEqual(workbenchDiagnosticsFor(sourceUri), []);
			assert.match(
				(await observeWorkbenchCompiler()).validationOutput,
				/^\[SUCCESS\] Compilation completed successfully\.$/m,
			);
			assert.ok(vscode.languages.getDiagnostics(sourceUri).some(
				diagnostic => diagnostic.source === 'Provisional Parser',
			));
		} finally {
			parserDiagnostics.dispose();
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await source.remove();
			await peer.close();
		}
	});

	test('applies enablement immediately and presents the configured status without probing', async () => {
		const peer = await startNetApiPeer(() => ({
			errorCode: 'Ok',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			await configurePeer(peer.port);
			const ready = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready' ? observation : undefined;
			});
			assert.match(ready.tooltip, new RegExp(`127\\.0\\.0\\.1:${peer.port}`));
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

	test('presents the Workbench API as connected when scripts did not compile', async () => {
		const peer = await startNetApiPeer(() => ({
			errorCode: 'Ok',
			payload: { IsRunning: true, ScriptsCompiled: false },
		}));
		try {
			await configurePeer(peer.port);
			const connected = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready' ? observation : undefined;
			});

			assert.match(connected.text, /Workbench Connected/);
			assert.match(connected.tooltip, /Scripts: not compiled successfully/i);
			assert.match(connected.tooltip, /validation remains available/i);
		} finally {
			await peer.close();
		}
	});

	test('does not resume polling after disposal during an in-flight probe', async function () {
		this.timeout(5_000);
		let releaseProbe: (() => void) | undefined;
		const probeGate = new Promise<void>(resolve => {
			releaseProbe = resolve;
		});
		let statusRequests = 0;
		const peer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc !== 'IsWorkbenchRunning') {
				return { errorCode: 'Ok', payload: { Errors: [], Warnings: [], Success: true } };
			}
			statusRequests += 1;
			if (statusRequests === 1) {
				await probeGate;
			}
			return {
				errorCode: 'Ok',
				payload: { IsRunning: true, ScriptsCompiled: false },
			};
		});
		try {
			await configurePeer(peer.port);
			await waitFor(() => statusRequests === 1 ? true : undefined);
			await vscode.commands.executeCommand(workbenchTestCommands.disposeCompiler);
			releaseProbe?.();

			await new Promise(resolve => setTimeout(resolve, 1_250));

			assert.strictEqual(statusRequests, 1);
		} finally {
			releaseProbe?.();
			await vscode.commands.executeCommand(workbenchTestCommands.restartCompiler);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.enabled,
				false,
				vscode.ConfigurationTarget.Global,
			);
			await peer.close();
		}
	});

	test('keeps compiler findings fresh when the connected API reports compile failure', async function () {
		this.timeout(8_000);
		const workspace = onlyWorkspaceFolder();
		const sourceUri = vscode.Uri.file(path.join(
			workspace.uri.fsPath,
			'Scripts',
			'Game',
			'Example.c',
		));
		let scriptsCompiled = true;
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return {
					errorCode: 'Ok',
					payload: { IsRunning: true, ScriptsCompiled: scriptsCompiled },
				};
			}
			return {
				errorCode: 'Ok',
				payload: {
					Errors: [{
						error: 'Finding retained while Workbench starts',
						file: 'Scripts/Game/Example.c',
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		try {
			await configurePeer(peer.port);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri)
				.some(diagnostic => diagnostic.message.includes('Finding retained while Workbench starts'))
				? true
				: undefined);
			scriptsCompiled = false;

			await waitFor(async () =>
				(await observeWorkbenchCompiler()).tooltip.includes(
					'Scripts: not compiled successfully',
				) ? true : undefined,
			6_000);

			const retained = workbenchDiagnosticsFor(sourceUri).filter(diagnostic =>
				diagnostic.message.includes('Finding retained while Workbench starts'));
			assert.strictEqual(retained.length, 1);
			assert.strictEqual(retained[0].source, workbenchDiagnostics.source);
			assert.match(
				(await observeWorkbenchCompiler()).tooltip,
				/Compiler result: current for the last saved snapshot/,
			);
		} finally {
			await peer.close();
		}
	});

	test('retains stale findings and reports save-failed when the active script cannot be saved', async () => {
		const workspace = onlyWorkspaceFolder();
		const source = await createTemporaryScript(workspace, 'SaveFailure');
		const sourceUri = source.uri;
		let validationCount = 0;
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationCount += 1;
			return {
				errorCode: 'Ok',
				payload: {
					Errors: [{
						error: 'Retained finding',
						file: source.relativePath,
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		try {
			await configurePeer(peer.port);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const document = await vscode.workspace.openTextDocument(source.filePath);
			await vscode.window.showTextDocument(document);
			await applyAppend(document, '// dirty editor version');
			await fs.writeFile(
				source.filePath,
				`${workbenchFixtureSource}// conflicting disk version`,
				'utf8',
			);

			await vscode.commands.executeCommand(workbenchCommands.validateScripts);

			assert.strictEqual(validationCount, 1);
			const retained = workbenchDiagnosticsFor(sourceUri);
			assert.strictEqual(retained.length, 1);
			assert.match(retained[0].source ?? '', /\(possibly outdated\)$/);
			assert.match((await observeWorkbenchCompiler()).tooltip, /save-failed/);
		} finally {
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await source.remove();
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
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: {
					errorCode: 'Ok',
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
			await configurePeer(peer.port);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => workbenchDiagnosticsFor(sourceUri).length === 1 ? true : undefined);
			const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
			await configuration.update(
				workbenchConfig.settings.port,
				unavailablePort,
				vscode.ConfigurationTarget.Global,
			);

			const unavailable = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'unavailable' ? observation : undefined;
			});
			const retained = workbenchDiagnosticsFor(sourceUri);
			assert.strictEqual(retained.length, 1);
			assert.match(retained[0].source ?? '', /\(possibly outdated\)$/);
			assert.match(
				unavailable.tooltip,
				/Compiler result may be out of date because Workbench is unavailable/,
			);
			assert.strictEqual(
				unavailable.backgroundColor,
				'statusBarItem.errorBackground',
			);

			await configuration.update(
				workbenchConfig.settings.port,
				peer.port,
				vscode.ConfigurationTarget.Global,
			);
			const recovered = await waitFor(async () => {
				const observation = await observeWorkbenchCompiler();
				return observation.phase === 'ready' ? observation : undefined;
			});
			assert.strictEqual(recovered.backgroundColor, undefined);
		} finally {
			await peer.close();
		}
	});

	test('projects only proven project-contained compiler locations into VS Code', async () => {
		const workspace = onlyWorkspaceFolder();
		const sourcePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', 'Example.c');
		const sourceUri = vscode.Uri.file(sourcePath);
		const externalDirectory = await fs.mkdtemp(path.join(
			os.tmpdir(),
			'reforger-script-tools-compiler-location-',
		));
		const externalPath = path.join(externalDirectory, 'ExternalCompilerLocation.c');
		await fs.writeFile(externalPath, 'class ExternalCompilerLocation {}\n', 'utf8');
		const peer = await startNetApiPeer(request => {
			const payload = request.payload as { APIFunc?: string };
			return payload.APIFunc === 'IsWorkbenchRunning'
				? { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } }
				: {
					errorCode: 'Ok',
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
						Warnings: [{
							error: 'Project warning',
							file: 'Scripts/Game/Example.c',
							line: 2,
						}, {
							error: 'Base-game warning noise',
							file: 'Scripts/Game/BaseGame.c',
							fileAbs: externalPath,
							line: 2,
						}],
						Success: false,
					},
				};
		});
		try {
			await configurePeer(peer.port);
			await vscode.commands.executeCommand(workbenchCommands.validateScripts);
			const diagnostics = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current.length > 0 ? current : undefined;
			});

			assert.deepStrictEqual(
				diagnostics.map(diagnostic => diagnostic.message),
				['Relative contained location', 'Project warning'],
			);
			assert.deepStrictEqual(workbenchDiagnosticsFor(vscode.Uri.file(externalPath)), []);
			assert.strictEqual(
				(await observeWorkbenchCompiler()).lastValidationResult?.diagnostics.length,
				6,
			);
			const output = (await observeWorkbenchCompiler()).validationOutput;
			assert.match(
				output,
				/^\[\d{2}:\d{2}:\d{2}\] Compilation in (?:\d+ ms|\d+\.\d s) — 1 project error, 1 project warning \(4 non-project findings hidden\)\r?\n/,
			);
			assert.ok(output.includes(
				'[ERROR] Scripts/Game/Example.c:1 — Relative contained location',
			));
			assert.ok(output.includes(
				'[WARNING] Scripts/Game/Example.c:2 — Project warning',
			));
			assert.doesNotMatch(output, new RegExp(escapeRegExp(workspace.uri.fsPath)));
			assert.doesNotMatch(output, /External absolute location/);
			assert.doesNotMatch(output, /Escaping relative location/);
			assert.doesNotMatch(output, /Unresolvable location/);
			assert.doesNotMatch(output, /Base-game warning noise/);
		} finally {
			await fs.rm(externalDirectory, { recursive: true, force: true });
			await peer.close();
		}
	});

	test('keeps a result stale when scripts change during its validation request', async () => {
		const workspace = onlyWorkspaceFolder();
		const source = await createTemporaryScript(workspace, 'EditDuringValidation');
		const sourceUri = source.uri;
		let releaseValidation: (() => void) | undefined;
		const validationGate = new Promise<void>(resolve => {
			releaseValidation = resolve;
		});
		let validationStarted = false;
		const peer = await startNetApiPeer(async request => {
			const payload = request.payload as { APIFunc?: string };
			if (payload.APIFunc === 'IsWorkbenchRunning') {
				return { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			validationStarted = true;
			await validationGate;
			return {
				errorCode: 'Ok',
				payload: {
					Errors: [{
						error: 'Finding for the older saved snapshot',
						file: source.relativePath,
						line: 1,
					}],
					Warnings: [],
					Success: false,
				},
			};
		});
		const publishedSources: string[][] = [];
		const diagnosticsListener = vscode.languages.onDidChangeDiagnostics(event => {
			if (!event.uris.some(uri => uri.toString() === sourceUri.toString())) {
				return;
			}
			const matching = workbenchDiagnosticsFor(sourceUri).filter(diagnostic =>
				diagnostic.message.includes('Finding for the older saved snapshot'));
			if (matching.length > 0) {
				publishedSources.push(matching.map(diagnostic => diagnostic.source ?? ''));
			}
		});
		try {
			await configurePeer(peer.port);
			const document = await vscode.workspace.openTextDocument(source.filePath);
			await vscode.window.showTextDocument(document);
			const command = vscode.commands.executeCommand(workbenchCommands.validateScripts);
			await waitFor(() => validationStarted ? true : undefined);
			await applyAppend(document, '// edit during validation');
			releaseValidation?.();
			await command;

			const stale = await waitFor(() => {
				const current = workbenchDiagnosticsFor(sourceUri);
				return current[0]?.source?.endsWith('(possibly outdated)')
					? current[0]
					: undefined;
			});
			assert.match(stale.message, /Finding for the older saved snapshot/);
			assert.match(
				(await observeWorkbenchCompiler()).tooltip,
				/may be out of date because scripts changed while validation was running/,
			);
			await waitFor(() => publishedSources.length > 0 ? true : undefined);
			assert.ok(publishedSources.every(sources =>
				sources.every(source => source.endsWith('(possibly outdated)'))));
		} finally {
			diagnosticsListener.dispose();
			releaseValidation?.();
			await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
			await source.remove();
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
				return { errorCode: 'Ok', payload: { IsRunning: true, ScriptsCompiled: true } };
			}
			oldValidationStarted = true;
			await oldValidationGate;
			return {
				errorCode: 'Ok',
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
			errorCode: 'Ok',
			payload: { IsRunning: true, ScriptsCompiled: true },
		}));
		try {
			await configurePeer(oldPeer.port);
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

async function configurePeer(port: number): Promise<void> {
	const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
	await configuration.update(workbenchConfig.settings.enabled, false, vscode.ConfigurationTarget.Global);
	await configuration.update(workbenchConfig.settings.host, '127.0.0.1', vscode.ConfigurationTarget.Global);
	await configuration.update(workbenchConfig.settings.port, port, vscode.ConfigurationTarget.Global);
	await configuration.update(
		workbenchConfig.settings.saveOnIdle,
		workbenchDefaults.saveOnIdle,
		vscode.ConfigurationTarget.Global,
	);
	await configuration.update(workbenchConfig.settings.enabled, true, vscode.ConfigurationTarget.Global);
}

function onlyWorkspaceFolder(): vscode.WorkspaceFolder {
	const folders = vscode.workspace.workspaceFolders;
	assert.ok(folders && folders.length === 1);
	return folders[0];
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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

async function createTemporaryScript(
	workspace: vscode.WorkspaceFolder,
	label: string,
): Promise<{
	filePath: string;
	relativePath: string;
	uri: vscode.Uri;
	remove: () => Promise<void>;
}> {
	temporaryScriptCounter += 1;
	const fileName = `.workbench-${label}-${process.pid}-${temporaryScriptCounter}.c`;
	const filePath = path.join(workspace.uri.fsPath, 'Scripts', 'Game', fileName);
	await fs.writeFile(filePath, workbenchFixtureSource, 'utf8');
	return {
		filePath,
		relativePath: path.relative(workspace.uri.fsPath, filePath).split(path.sep).join('/'),
		uri: vscode.Uri.file(filePath),
		remove: async () => {
			await fs.unlink(filePath).catch(() => undefined);
		},
	};
}

function workbenchDiagnosticsFor(uri: vscode.Uri): vscode.Diagnostic[] {
	return vscode.languages.getDiagnostics(uri)
		.filter(diagnostic => diagnostic.source?.startsWith(workbenchDiagnostics.source));
}

function validationRequests(peer: { requests: Array<{ payload: unknown }> }): unknown[] {
	return peer.requests.filter(request =>
		(request.payload as { APIFunc?: string }).APIFunc === 'ValidateScripts');
}
