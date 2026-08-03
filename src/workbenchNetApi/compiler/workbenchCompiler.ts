import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { diagnostic } from '../../diagnostics/diagnostics';
import {
	workbenchCommands,
	workbenchConfig,
	workbenchDefaults,
	workbenchDiagnostics,
	workbenchTestCommands,
} from '../../extensionConfig/workbench';
import {
	WorkbenchCompilerDiagnostic,
	WorkbenchGateway,
	WorkbenchGatewayFailureCategory,
	WorkbenchStatus,
	WorkbenchValidationResult,
} from '../gateway/workbenchGateway';
import {
	onDidChangeWorkbenchFailure,
	resetWorkbenchFailureNotification,
	updateWorkbenchFailureNotification,
} from '../workbenchFailureNotification';
import {
	WorkbenchDiagnosticRange,
	workbenchDiagnosticProjection,
} from './workbenchDiagnosticSpan';
import { resolveLanguageServerPath } from '../../languageClient/serverPath';
import {
	WorkbenchIntegrationCoordinator,
} from '../integration/workbenchIntegration';

const unavailableRetryMs = 1_000;
const readyHeartbeatMs = 5_000;
const workbenchValidation = {
	idleDelaySeconds: 3,
	profile: 'WORKBENCH',
} as const;

type WorkbenchUiPhase =
	| 'disabled'
	| 'connecting'
	| 'ready'
	| 'validating'
	| 'unavailable';

export interface WorkbenchCompilerObservation {
	phase: WorkbenchUiPhase;
	text: string;
	tooltip: string;
	backgroundColor?: string;
	validationOutput: string;
	validationOutputLinks: Array<{
		line: number;
		startCharacter: number;
		endCharacter: number;
		target: string;
	}>;
	lastValidationResult?: WorkbenchValidationResult;
}

interface WorkbenchConfiguration {
	enabled: boolean;
	host: string;
	port: number;
	saveOnIdle: boolean;
}

interface RenderedDiagnosticSet {
	uri: vscode.Uri;
	diagnostics: vscode.Diagnostic[];
}

interface ValidationRequest {
	generation: number;
	trigger: 'edit' | 'save' | 'manual' | 'startup';
	requestedAtMs: number;
	documentToSave?: vscode.TextDocument;
	revealOutput?: boolean;
}

interface ValidationTiming {
	trigger: ValidationRequest['trigger'];
	requestedAtMs: number;
	queuedAtMs: number;
	validationStartedAtMs: number;
	completedAtMs: number;
}

interface ValidationOutputLink {
	id: string;
	line: number;
	lineText: string;
	startCharacter: number;
	sourceUri: vscode.Uri;
	sourceRange: vscode.Range;
	tooltip: string;
}

export function workbenchConnectionStarted(
	previous: WorkbenchStatus | undefined,
	current: WorkbenchStatus,
): boolean {
	return current.isRunning && previous?.isRunning !== true;
}

export function shouldRefreshWorkbenchGraph(
	previous: WorkbenchStatus | undefined,
	current: WorkbenchStatus,
	bridgeInactive: boolean,
): boolean {
	return workbenchConnectionStarted(previous, current)
		|| (current.isRunning && bridgeInactive);
}

interface WorkbenchCompilerFailure {
	category: WorkbenchGatewayFailureCategory | 'save-failed';
	recoveryHint: string;
}

export function registerWorkbenchCompilerFeatures(
	context: vscode.ExtensionContext,
	integration?: WorkbenchIntegrationCoordinator,
	onWorkbenchGraphRefreshRequested?: () => void,
): void {
	const startupValidationEnabled = context.extensionMode !== vscode.ExtensionMode.Test;
	const serverPath = resolveLanguageServerPath(context);
	let controller = new WorkbenchCompilerController(
		startupValidationEnabled,
		serverPath,
		integration,
		onWorkbenchGraphRefreshRequested,
	);
	controller.start(context.extensionMode);
	context.subscriptions.push(controller);
	if (context.extensionMode === vscode.ExtensionMode.Test) {
		context.subscriptions.push(
			vscode.commands.registerCommand(workbenchTestCommands.disposeCompiler, () => {
				controller.dispose();
			}),
			vscode.commands.registerCommand(workbenchTestCommands.restartCompiler, () => {
				controller.dispose();
				controller = new WorkbenchCompilerController(false, serverPath);
				controller.start(context.extensionMode);
				context.subscriptions.push(controller);
			}),
			vscode.commands.registerCommand(workbenchTestCommands.armStartupValidation, () => {
				controller.armStartupValidation();
			}),
			vscode.commands.registerCommand(workbenchTestCommands.resetFailureNotification, () => {
				resetWorkbenchFailureNotification();
			}),
		);
	}
}

class WorkbenchCompilerController implements vscode.Disposable {
	private configuration = readConfiguration();
	private gateway: WorkbenchGateway;
	private readonly compilerDiagnostics = vscode.languages.createDiagnosticCollection(
		workbenchDiagnostics.collectionName,
	);
	private readonly validationOutput = vscode.window.createOutputChannel(
		workbenchDiagnostics.outputChannelName,
		workbenchDiagnostics.outputLanguageId,
	);
	private readonly statusItem = vscode.window.createStatusBarItem(
		vscode.StatusBarAlignment.Right,
		100,
	);
	private readonly disposables: vscode.Disposable[] = [];
	private readonly retainedDiagnostics = new Map<string, RenderedDiagnosticSet>();
	private probeTimer: NodeJS.Timeout | undefined;
	private validationTimer: NodeJS.Timeout | undefined;
	private pendingValidation: ValidationRequest | undefined;
	private readonly savesStartedByValidation = new Set<string>();
	private configurationGeneration = 0;
	private scriptEditGeneration = 0;
	private validating = false;
	private phase: WorkbenchUiPhase = 'connecting';
	private lastOutcome = 'No validation has completed.';
	private lastFailure: WorkbenchCompilerFailure | undefined;
	private lastStatus: WorkbenchStatus | undefined;
	private lastValidationResult: WorkbenchValidationResult | undefined;
	private lastValidationTiming: ValidationTiming | undefined;
	private latestValidationOutput = '';
	private latestValidationOutputLinks: ValidationOutputLink[] = [];
	private validationOutputGeneration = 0;
	private activeValidationStartedAtMs: number | undefined;
	private startupValidationAttempted = false;
	private validationCompletedThisSession = false;
	private staleReason: string | undefined;
	private bridgeInactive = false;
	private disposed = false;

	public constructor(
		private startupValidationEnabled: boolean,
		private readonly serverPath: Promise<string | undefined>,
		private readonly integration?: WorkbenchIntegrationCoordinator,
		private readonly onWorkbenchGraphRefreshRequested?: () => void,
	) {
		this.gateway = this.createGatewayForCurrentConfiguration();
	}

	public start(extensionMode: vscode.ExtensionMode): void {
		this.statusItem.name = 'Reforger Workbench';
		this.statusItem.command = workbenchCommands.validateScripts;
		this.statusItem.show();
		this.disposables.push(
			this.statusItem,
			this.compilerDiagnostics,
			this.validationOutput,
			onDidChangeWorkbenchFailure(diagnosis => {
				this.bridgeInactive = diagnosis === 'bridge-inactive';
				this.setPhase(this.phase);
			}),
			vscode.languages.registerDocumentLinkProvider(
				{ language: workbenchDiagnostics.outputLanguageId },
				{
					provideDocumentLinks: document => this.provideValidationOutputLinks(document),
				},
			),
			vscode.commands.registerCommand(
				workbenchCommands.validateScripts,
				() => this.requestManualValidation(),
			),
			vscode.commands.registerCommand(
				workbenchCommands.openCompilerDiagnostic,
				id => this.openValidationOutputLink(id),
			),
			vscode.workspace.onDidChangeConfiguration(event => {
				if (event.affectsConfiguration(workbenchConfig.section)) {
					this.applyConfiguration();
				}
			}),
			vscode.workspace.onDidChangeTextDocument(event => {
				this.onDocumentChanged(event.document);
			}),
			vscode.workspace.onDidSaveTextDocument(document => {
				this.onDocumentSaved(document);
			}),
		);
		if (extensionMode === vscode.ExtensionMode.Test) {
			this.disposables.push(vscode.commands.registerCommand(
				workbenchTestCommands.observeCompiler,
				() => this.observation(),
			));
		}
		this.applyConfiguration();
	}

	public armStartupValidation(): void {
		this.startupValidationEnabled = true;
	}

	public dispose(): void {
		if (this.disposed) {
			return;
		}
		this.disposed = true;
		this.configurationGeneration += 1;
		this.pendingValidation = undefined;
		this.activeValidationStartedAtMs = undefined;
		this.clearProbeTimer();
		this.clearValidationTimer();
		for (const disposable of this.disposables.splice(0)) {
			disposable.dispose();
		}
		this.integration?.dispose();
	}

	private applyConfiguration(): void {
		if (this.disposed) {
			return;
		}
		this.configurationGeneration += 1;
		this.configuration = readConfiguration();
		this.gateway = this.createGatewayForCurrentConfiguration();
		this.integration?.onWorkbenchConfigurationChanged(
			this.configuration.enabled,
			isWorkbenchEnablementExplicitlyDisabled(),
		);
		this.clearProbeTimer();
		this.clearValidationTimer();
		this.pendingValidation = undefined;
		this.activeValidationStartedAtMs = undefined;
		this.markDiagnosticsStale('Workbench configuration changed');
		this.lastFailure = undefined;
		this.lastStatus = undefined;
		if (!this.configuration.enabled) {
			this.setPhase('disabled');
			return;
		}
		this.setPhase('connecting');
		this.scheduleProbe(0, this.configurationGeneration);
	}

	private async requestManualValidation(): Promise<void> {
		if (!this.configuration.enabled) {
			this.setPhase('disabled');
			this.markDiagnosticsStale('Workbench NET API integration is disabled');
			return;
		}
		if (!onlyAddonWorkspace()) {
			this.noteFailure({
				category: 'unsupported',
				recoveryHint: 'Open one Reforger addon project as the VS Code workspace.',
			});
			return;
		}
		const activeDocument = eligibleActiveDocument();
		await this.queueValidation({
			generation: this.configurationGeneration,
			trigger: 'manual',
			requestedAtMs: Date.now(),
			revealOutput: true,
			...(activeDocument?.isDirty ? { documentToSave: activeDocument } : {}),
		});
	}

	private onDocumentChanged(document: vscode.TextDocument): void {
		if (!eligibleDocument(document)) {
			return;
		}
		this.scriptEditGeneration += 1;
		this.markDiagnosticsStale('the script has newer edits');
		if (this.configuration.saveOnIdle
			&& vscode.window.activeTextEditor?.document.uri.toString() === document.uri.toString()) {
			this.scheduleValidation({
				generation: this.configurationGeneration,
				trigger: 'edit',
				requestedAtMs: Date.now(),
				documentToSave: document,
			});
		}
	}

	private onDocumentSaved(document: vscode.TextDocument): void {
		if (!eligibleDocument(document)
			|| this.savesStartedByValidation.has(document.uri.toString())
			|| this.disposed
			|| !this.configuration.enabled) {
			return;
		}
		this.clearValidationTimer();
		const request: ValidationRequest = {
			generation: this.configurationGeneration,
			trigger: 'save',
			requestedAtMs: Date.now(),
		};
		diagnostic('workbenchValidationScheduled', {
			trigger: request.trigger,
			delayMs: 0,
		});
		void this.queueValidation(request);
	}

	private scheduleValidation(request: ValidationRequest): void {
		if (this.disposed || !this.configuration.enabled) {
			return;
		}
		this.clearValidationTimer();
		if (this.validating) {
			this.pendingValidation = undefined;
		}
		const delayMs = workbenchValidation.idleDelaySeconds * 1_000;
		diagnostic('workbenchValidationScheduled', {
			trigger: request.trigger,
			delayMs,
		});
		this.validationTimer = setTimeout(() => {
			this.validationTimer = undefined;
			if (request.generation !== this.configurationGeneration) {
				return;
			}
			if (request.documentToSave
				&& vscode.window.activeTextEditor?.document.uri.toString()
					!== request.documentToSave.uri.toString()) {
				return;
			}
			void this.queueValidation(request);
		}, delayMs);
	}

	private async queueValidation(request: ValidationRequest): Promise<void> {
		if (this.disposed || request.generation !== this.configurationGeneration) {
			return;
		}
		if (this.validating) {
			this.pendingValidation = request;
			return;
		}
		const queuedAtMs = Date.now();
		this.validating = true;
		this.clearProbeTimer();
		this.setPhase('validating');
		try {
			const saved = !request.documentToSave?.isDirty
				|| await this.saveForValidation(request.documentToSave);
			if (request.generation !== this.configurationGeneration) {
				return;
			}
			if (!saved) {
				this.lastOutcome = 'Validation skipped because the active script could not be saved.';
				this.noteFailure({
					category: 'save-failed',
					recoveryHint: 'Save the active script, then retry validation.',
				});
				this.scheduleProbe(readyHeartbeatMs, request.generation);
				return;
			}
			await this.validate(
				request,
				this.scriptEditGeneration,
				queuedAtMs,
			);
		} finally {
			this.validating = false;
			if (this.disposed) {
				return;
			}
			const pending = this.pendingValidation;
			this.pendingValidation = undefined;
			if (pending && pending.generation === this.configurationGeneration) {
				void this.queueValidation(pending);
			} else if (request.generation !== this.configurationGeneration
				&& this.configuration.enabled) {
				this.scheduleProbe(0, this.configurationGeneration);
			}
		}
	}

	private async saveForValidation(document: vscode.TextDocument): Promise<boolean> {
		const key = document.uri.toString();
		this.savesStartedByValidation.add(key);
		try {
			return await document.save();
		} catch {
			return false;
		} finally {
			this.savesStartedByValidation.delete(key);
		}
	}

	private async validate(
		request: ValidationRequest,
		editGeneration: number,
		queuedAtMs: number,
	): Promise<void> {
		if (this.disposed || request.generation !== this.configurationGeneration) {
			return;
		}
		const hadDirtyScriptsAtRequest = hasDirtyEligibleDocuments();
		const validationStartedAtMs = Date.now();
		this.activeValidationStartedAtMs = validationStartedAtMs;
		const validation = this.gateway.validateScripts(workbenchValidation.profile);
		this.publishValidationPending(
			validationStartedAtMs,
			request.revealOutput === true,
		);
		const result = await validation;
		const completedAtMs = Date.now();
		if (this.activeValidationStartedAtMs === validationStartedAtMs) {
			this.activeValidationStartedAtMs = undefined;
		}
		if (this.disposed || request.generation !== this.configurationGeneration) {
			return;
		}
		if (!result.ok) {
			this.lastOutcome = `Validation failed: ${result.failure.category}.`;
			this.noteFailure(result.failure);
			this.publishValidationState(
				`[${formatValidationClockTime(completedAtMs)}] `
					+ `Compilation did not complete — ${result.failure.recoveryHint}`,
				request.revealOutput === true,
			);
			this.scheduleProbe(unavailableRetryMs, request.generation);
			return;
		}
		this.validationCompletedThisSession = true;
		const staleReason = editGeneration !== this.scriptEditGeneration
			? 'scripts changed while validation was running'
			: hadDirtyScriptsAtRequest || hasDirtyEligibleDocuments()
				? 'other scripts still have unsaved edits'
				: undefined;
		this.publishValidationResult(
			result.value,
			{
				trigger: request.trigger,
				requestedAtMs: request.requestedAtMs,
				queuedAtMs,
				validationStartedAtMs,
				completedAtMs,
			},
			staleReason,
			request.revealOutput === true,
		);
		this.lastFailure = undefined;
		this.lastOutcome = staleReason
			? `Validation completed for an older saved snapshot at ${new Date().toLocaleTimeString()}.`
			: result.value.success
				? `Validation succeeded at ${new Date().toLocaleTimeString()}.`
				: `Validation completed with ${result.value.diagnostics.length} finding(s) at ${new Date().toLocaleTimeString()}.`;
		this.setPhase('ready');
		diagnostic('workbenchCompilerDiagnosticSet', {
			outcome: staleReason
				? 'stale-result'
				: result.value.success ? 'success' : 'compiler-findings',
			diagnosticCount: result.value.diagnostics.length,
		});
		diagnostic('workbenchValidationTiming', {
			trigger: request.trigger,
			totalDurationMs: completedAtMs - request.requestedAtMs,
			idleQueueDurationMs: queuedAtMs - request.requestedAtMs,
			savePreparationDurationMs: validationStartedAtMs - queuedAtMs,
			workbenchDurationMs: completedAtMs - validationStartedAtMs,
			presentationDurationMs: Date.now() - completedAtMs,
		});
		this.scheduleProbe(readyHeartbeatMs, request.generation);
	}

	private publishValidationResult(
		result: WorkbenchValidationResult,
		timing: ValidationTiming,
		staleReason?: string,
		revealOutput = false,
	): void {
		const projected = projectDiagnostics(result.diagnostics);
		const next = new Map<string, RenderedDiagnosticSet>();
		for (const item of projected.located) {
			const key = item.uri.toString();
			const existing = next.get(key);
			if (existing) {
				existing.diagnostics.push(item.diagnostic);
			} else {
				next.set(key, { uri: item.uri, diagnostics: [item.diagnostic] });
			}
		}
		const entries: Array<[vscode.Uri, readonly vscode.Diagnostic[] | undefined]> = [];
		for (const previous of this.retainedDiagnostics.values()) {
			entries.push([previous.uri, undefined]);
		}
		for (const current of next.values()) {
			entries.push([
				current.uri,
				staleReason
					? current.diagnostics.map(diagnostic =>
						renderStaleDiagnostic(diagnostic, staleReason))
					: current.diagnostics,
			]);
		}
		this.compilerDiagnostics.set(entries);
		this.retainedDiagnostics.clear();
		for (const [key, value] of next) {
			this.retainedDiagnostics.set(key, value);
		}
		this.lastValidationResult = cloneValidationResult(result);
		this.lastValidationTiming = timing;
		this.staleReason = staleReason;
		this.publishValidationOutput(
			projected,
			timing,
			result.success,
			staleReason,
			revealOutput || projected.located.length > 0,
		);
		if (projected.unresolved.length > 0) {
			diagnostic('workbenchUnresolvedDiagnosticLocations', {
				count: projected.unresolved.length,
			});
		}
	}

	private publishValidationOutput(
		projected: ProjectedDiagnostics,
		timing: ValidationTiming,
		successful: boolean,
		staleReason: string | undefined,
		revealOutput: boolean,
	): void {
		const projectErrorCount = projected.located.filter(item =>
			item.compilerDiagnostic.severity === 'error').length;
		const projectWarningCount = projected.located.length - projectErrorCount;
		const hiddenFindingCount = projected.unresolved.length;
		const hiddenSummary = hiddenFindingCount > 0
			? ` (${hiddenFindingCount} non-project finding${hiddenFindingCount === 1 ? '' : 's'} hidden)`
			: '';
		const workbenchDurationMs = timing.completedAtMs - timing.validationStartedAtMs;
		const lines = [
			`[${formatValidationClockTime(timing.completedAtMs)}] `
				+ `Compilation in ${formatValidationDuration(workbenchDurationMs)} — `
				+ `${projectErrorCount} project error${projectErrorCount === 1 ? '' : 's'}, `
				+ `${projectWarningCount} project warning${projectWarningCount === 1 ? '' : 's'}`
				+ hiddenSummary,
			successful
				? '[SUCCESS] Compilation completed successfully.'
				: '[FAILED] Workbench reported compilation errors.',
			...(staleReason ? [`Result status: may be out of date — ${staleReason}.`] : []),
		];
		if (projected.located.length > 0) {
			lines.push('');
		}
		const links: ValidationOutputLink[] = [];
		const outputGeneration = ++this.validationOutputGeneration;
		for (const item of projected.located) {
			const severity = `[${item.compilerDiagnostic.severity.toUpperCase()}]`;
			const relativePath = vscode.workspace.asRelativePath(item.uri, false)
				.split(path.sep)
				.join('/');
			const location = `${relativePath}:${item.compilerDiagnostic.location.line}`;
			const lineText = `${severity} ${location} — ${item.compilerDiagnostic.message}`;
			links.push({
				id: `${outputGeneration}:${links.length}`,
				line: lines.length,
				lineText,
				startCharacter: severity.length + 1,
				sourceUri: item.uri,
				sourceRange: item.diagnostic.range,
				tooltip: `Open and select ${location}`,
			});
			lines.push(lineText);
		}
		this.latestValidationOutput = `${lines.join('\n')}\n`;
		this.latestValidationOutputLinks = links;
		this.validationOutput.replace(this.latestValidationOutput);
		if (revealOutput) {
			this.validationOutput.show(true);
		}
	}

	private publishValidationPending(startedAtMs: number, revealOutput: boolean): void {
		this.publishValidationState(
			`[${formatValidationClockTime(startedAtMs)}] `
				+ 'Compilation requested — waiting for Workbench to finish...',
			revealOutput,
		);
	}

	private publishValidationState(message: string, revealOutput: boolean): void {
		this.validationOutputGeneration += 1;
		this.latestValidationOutput = `${message}\n`;
		this.latestValidationOutputLinks = [];
		this.validationOutput.replace(this.latestValidationOutput);
		if (revealOutput) {
			this.validationOutput.show(true);
		}
	}

	private provideValidationOutputLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
		const links: vscode.DocumentLink[] = [];
		for (const candidate of this.latestValidationOutputLinks) {
			if (candidate.line >= document.lineCount
				|| document.lineAt(candidate.line).text !== candidate.lineText) {
				continue;
			}
			const link = new vscode.DocumentLink(
				new vscode.Range(
					candidate.line,
					candidate.startCharacter,
					candidate.line,
					candidate.lineText.length,
				),
				this.validationOutputLinkTarget(candidate),
			);
			link.tooltip = candidate.tooltip;
			links.push(link);
		}
		return links;
	}

	private validationOutputLinkTarget(candidate: ValidationOutputLink): vscode.Uri {
		const argumentsJson = encodeURIComponent(JSON.stringify([candidate.id]));
		return vscode.Uri.parse(
			`command:${workbenchCommands.openCompilerDiagnostic}?${argumentsJson}`,
		);
	}

	private async openValidationOutputLink(id: unknown): Promise<void> {
		if (this.disposed || typeof id !== 'string') {
			return;
		}
		const candidate = this.latestValidationOutputLinks.find(link => link.id === id);
		if (!candidate) {
			return;
		}
		try {
			const editor = await vscode.window.showTextDocument(candidate.sourceUri, {
				preview: true,
			});
			editor.selection = new vscode.Selection(
				candidate.sourceRange.end,
				candidate.sourceRange.start,
			);
			editor.revealRange(
				candidate.sourceRange,
				vscode.TextEditorRevealType.InCenterIfOutsideViewport,
			);
		} catch {
			diagnostic('workbenchDiagnosticNavigationFailed', {
				category: 'source-unavailable',
			});
		}
	}

	private markDiagnosticsStale(reason: string): void {
		if (!this.lastValidationResult || !this.lastValidationTiming) {
			return;
		}
		this.staleReason = reason;
		const entries: Array<[vscode.Uri, readonly vscode.Diagnostic[]]> = [];
		for (const set of this.retainedDiagnostics.values()) {
			entries.push([
				set.uri,
				set.diagnostics.map(original => renderStaleDiagnostic(original, reason)),
			]);
		}
		this.compilerDiagnostics.set(entries);
		if (this.activeValidationStartedAtMs !== undefined) {
			this.publishValidationPending(
				this.activeValidationStartedAtMs,
				false,
			);
		} else {
			this.publishValidationOutput(
				projectDiagnostics(this.lastValidationResult.diagnostics),
				this.lastValidationTiming,
				this.lastValidationResult.success,
				reason,
				false,
			);
		}
		this.setPhase(this.phase);
	}

	private async probe(generation: number): Promise<void> {
		if (this.disposed
			|| generation !== this.configurationGeneration
			|| !this.configuration.enabled) {
			return;
		}
		if (this.validating) {
			this.scheduleProbe(unavailableRetryMs, generation);
			return;
		}
		const result = await this.gateway.getStatus();
		if (this.disposed
			|| generation !== this.configurationGeneration
			|| this.validating) {
			return;
		}
		if (!result.ok) {
			this.integration?.onWorkbenchDisconnected();
			this.lastStatus = undefined;
			this.noteFailure(result.failure);
			this.scheduleProbe(unavailableRetryMs, generation);
			return;
		}
		const refreshWorkbenchGraph = shouldRefreshWorkbenchGraph(
			this.lastStatus,
			result.value,
			this.bridgeInactive,
		);
		this.lastFailure = undefined;
		this.lastStatus = result.value;
		if (!result.value.isRunning) {
			this.integration?.onWorkbenchDisconnected();
			this.setPhase('connecting');
			this.scheduleProbe(readyHeartbeatMs, generation);
			return;
		}
		this.setPhase('ready');
		void this.integration?.onWorkbenchConnected({
			host: this.configuration.host,
			port: this.configuration.port,
		});
		if (refreshWorkbenchGraph) {
			this.onWorkbenchGraphRefreshRequested?.();
		}
		if (this.shouldRequestStartupValidation()) {
			this.startupValidationAttempted = true;
			const request: ValidationRequest = {
				generation,
				trigger: 'startup',
				requestedAtMs: Date.now(),
			};
			diagnostic('workbenchValidationScheduled', {
				trigger: request.trigger,
				delayMs: 0,
			});
			void this.queueValidation(request);
			return;
		}
		this.scheduleProbe(readyHeartbeatMs, generation);
	}

	private shouldRequestStartupValidation(): boolean {
		return this.startupValidationEnabled
			&& !this.startupValidationAttempted
			&& !this.validationCompletedThisSession
			&& onlyAddonWorkspace() !== undefined;
	}

	private noteFailure(
		failure: WorkbenchCompilerFailure,
		outcome: string = failure.category,
	): void {
		this.lastFailure = failure;
		this.lastStatus = undefined;
		this.markDiagnosticsStale(
			failure.category === 'save-failed'
				? 'the active script could not be saved'
				: 'Workbench is unavailable',
		);
		this.setPhase('unavailable');
		diagnostic('workbenchStateOutcome', {
			state: 'unavailable',
			outcome,
			category: failure.category,
		});
	}

	private scheduleProbe(delayMs: number, generation: number): void {
		if (this.disposed) {
			return;
		}
		this.clearProbeTimer();
		this.probeTimer = setTimeout(() => {
			this.probeTimer = undefined;
			void this.probe(generation);
		}, delayMs);
	}

	private clearProbeTimer(): void {
		if (this.probeTimer) {
			clearTimeout(this.probeTimer);
			this.probeTimer = undefined;
		}
	}

	private updateBridgeFailureNotification(
		diagnosis: 'bridge-inactive' | undefined,
	): void {
		updateWorkbenchFailureNotification(diagnosis);
	}

	private createGatewayForCurrentConfiguration(): WorkbenchGateway {
		const generation = this.configurationGeneration;
		return createGateway(
			this.configuration,
			this.serverPath,
			diagnosis => {
				if (!this.disposed && generation === this.configurationGeneration) {
					this.updateBridgeFailureNotification(diagnosis);
				}
			},
		);
	}

	private clearValidationTimer(): void {
		if (this.validationTimer) {
			clearTimeout(this.validationTimer);
			this.validationTimer = undefined;
		}
	}

	private setPhase(phase: WorkbenchUiPhase): void {
		if (this.phase !== phase) {
			diagnostic('workbenchStateTransition', {
				from: this.phase,
				to: phase,
			});
		}
		this.phase = phase;
		this.statusItem.backgroundColor = phase === 'unavailable' || this.bridgeInactive
			? new vscode.ThemeColor('statusBarItem.errorBackground')
			: undefined;
		const presentation = statusPresentation(phase);
		const baseText = this.bridgeInactive
			? '$(error) Workbench API inactive'
			: phase === 'unavailable' && this.lastFailure?.category === 'save-failed'
			? '$(warning) Workbench save failed'
			: presentation.text;
		this.statusItem.text = baseText;
		const detail = this.bridgeInactive
			? 'Workbench NET API bridge inactive. Fix script compilation errors.'
			: phase === 'unavailable' && this.lastFailure?.category === 'save-failed'
			? 'Compiler validation stopped because the active script could not be saved.'
			: presentation.detail;
		const endpoint = `${this.configuration.host}:${this.configuration.port}`;
		this.statusItem.tooltip = [
			detail,
			`Endpoint: ${endpoint}`,
			...(!this.bridgeInactive && this.lastStatus?.isRunning
				? [
					'Workbench API: connected.',
					this.lastStatus.scriptsCompiled
						? 'Scripts: compiled successfully.'
						: 'Scripts: not compiled successfully; validation remains available.',
				]
				: []),
			this.staleReason
				? `Compiler result may be out of date because ${this.staleReason}. `
					+ 'It describes an earlier saved snapshot and will be replaced '
					+ 'after the next successful validation.'
				: this.lastValidationResult
					? 'Compiler result: current for the last saved snapshot.'
					: 'Compiler result: not yet available.',
			this.lastOutcome,
			...(this.lastFailure
				? [`Failure: ${this.lastFailure.category}. ${this.lastFailure.recoveryHint}`]
				: []),
			'Workbench validates its currently open project; the built-in API cannot prove that it matches this VS Code workspace.',
			'Select to validate scripts now.',
		].join('\n\n');
	}

	private observation(): WorkbenchCompilerObservation {
		return {
			phase: this.phase,
			text: this.statusItem.text,
			tooltip: typeof this.statusItem.tooltip === 'string'
				? this.statusItem.tooltip
				: '',
			...(this.statusItem.backgroundColor
				? { backgroundColor: this.statusItem.backgroundColor.id }
				: {}),
			validationOutput: this.latestValidationOutput,
			validationOutputLinks: this.latestValidationOutputLinks.map(link => ({
				line: link.line,
				startCharacter: link.startCharacter,
				endCharacter: link.lineText.length,
				target: this.validationOutputLinkTarget(link).toString(),
			})),
			...(this.lastValidationResult
				? { lastValidationResult: cloneValidationResult(this.lastValidationResult) }
				: {}),
		};
	}
}

function readConfiguration(): WorkbenchConfiguration {
	const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
	return {
		enabled: configuration.get(workbenchConfig.settings.enabled, workbenchDefaults.enabled),
		host: configuration.get(workbenchConfig.settings.host, workbenchDefaults.host),
		port: configuration.get(workbenchConfig.settings.port, workbenchDefaults.port),
		saveOnIdle: configuration.get(
			workbenchConfig.settings.saveOnIdle,
			workbenchDefaults.saveOnIdle,
		),
	};
}

function isWorkbenchEnablementExplicitlyDisabled(): boolean {
	const configuration = vscode.workspace.getConfiguration(workbenchConfig.section);
	if (configuration.get(workbenchConfig.settings.enabled, workbenchDefaults.enabled)) {
		return false;
	}
	const inspected = configuration.inspect<boolean>(workbenchConfig.settings.enabled);
	return inspected?.globalValue === false
		|| inspected?.workspaceValue === false
		|| inspected?.workspaceFolderValue === false
		|| inspected?.globalLanguageValue === false
		|| inspected?.workspaceLanguageValue === false
		|| inspected?.workspaceFolderLanguageValue === false;
}

function createGateway(
	configuration: WorkbenchConfiguration,
	serverPath: Promise<string | undefined>,
	onNetApiFailure: (diagnosis: 'bridge-inactive') => void,
): WorkbenchGateway {
	return new WorkbenchGateway({
		enabled: configuration.enabled,
		serverPath,
		endpoint: {
			host: configuration.host,
			port: configuration.port,
		},
		record: record => {
			diagnostic('workbenchGatewayDiagnosticRecord', {
				capability: record.capability,
				outcome: record.outcome,
				durationMs: record.durationMs,
				timing: record.timing ? JSON.stringify(record.timing) : undefined,
			});
		},
		onNetApiFailure,
	});
}


function cloneValidationResult(result: WorkbenchValidationResult): WorkbenchValidationResult {
	return {
		...result,
		diagnostics: result.diagnostics.map(compilerDiagnostic => ({
			...compilerDiagnostic,
			location: { ...compilerDiagnostic.location },
		})),
	};
}

function statusPresentation(phase: WorkbenchUiPhase): { text: string; detail: string } {
	switch (phase) {
		case 'disabled':
			return { text: '$(circle-slash) Workbench disabled', detail: 'Workbench NET API integration is disabled.' };
		case 'connecting':
			return { text: '$(sync~spin) Workbench connecting', detail: 'Connecting to the configured Workbench endpoint.' };
		case 'ready':
			return { text: '$(plug) Workbench Connected', detail: 'Workbench NET API is connected and compiler validation is available.' };
		case 'validating':
			return { text: '$(sync~spin) Workbench validating', detail: 'Workbench is validating scripts.' };
		case 'unavailable':
			return { text: '$(warning) Workbench unavailable', detail: 'Workbench is unavailable; retrying the configured endpoint.' };
	}
}

function formatValidationDuration(durationMs: number): string {
	const roundedDurationMs = Math.max(0, Math.round(durationMs));
	return roundedDurationMs < 1_000
		? `${roundedDurationMs} ms`
		: `${(roundedDurationMs / 1_000).toFixed(1)} s`;
}

function formatValidationClockTime(timestampMs: number): string {
	const completedAt = new Date(timestampMs);
	return [
		completedAt.getHours(),
		completedAt.getMinutes(),
		completedAt.getSeconds(),
	].map(part => part.toString().padStart(2, '0')).join(':');
}

interface ProjectedDiagnostics {
	located: Array<{
		uri: vscode.Uri;
		diagnostic: vscode.Diagnostic;
		compilerDiagnostic: WorkbenchCompilerDiagnostic;
	}>;
	unresolved: WorkbenchCompilerDiagnostic[];
}

function projectDiagnostics(diagnostics: WorkbenchCompilerDiagnostic[]): ProjectedDiagnostics {
	const workspace = onlyAddonWorkspace();
	if (!workspace) {
		return { located: [], unresolved: diagnostics };
	}
	const located: ProjectedDiagnostics['located'] = [];
	const unresolved: WorkbenchCompilerDiagnostic[] = [];
	const sourceLinesByFile = new Map<string, string[]>();
	for (const compilerDiagnostic of diagnostics) {
		const uri = projectLocation(workspace, compilerDiagnostic);
		if (!uri) {
			unresolved.push(compilerDiagnostic);
			continue;
		}
		const line = Math.max(0, compilerDiagnostic.location.line - 1);
		const projection = workbenchDiagnosticProjection(
			readSourceLines(uri.fsPath, sourceLinesByFile),
			line,
			compilerDiagnostic.message,
		);
		const rendered = new vscode.Diagnostic(
			asVscodeRange(projection.primaryRange),
			compilerDiagnostic.message,
			compilerDiagnostic.severity === 'error'
				? vscode.DiagnosticSeverity.Error
				: vscode.DiagnosticSeverity.Warning,
		);
		rendered.source = workbenchDiagnostics.source;
		if (projection.recoveryContextRange) {
			rendered.relatedInformation = [
				new vscode.DiagnosticRelatedInformation(
					new vscode.Location(
						uri,
						asVscodeRange(projection.recoveryContextRange),
					),
					'Previous non-blank source line before Workbench recovered.',
				),
			];
		}
		located.push({ uri, diagnostic: rendered, compilerDiagnostic });
	}
	return { located, unresolved };
}

function asVscodeRange(range: WorkbenchDiagnosticRange): vscode.Range {
	return new vscode.Range(
		range.startLine,
		range.startCharacter,
		range.endLine,
		range.endCharacter,
	);
}

function readSourceLines(
	filePath: string,
	sourceLinesByFile: Map<string, string[]>,
): string[] {
	let lines = sourceLinesByFile.get(filePath);
	if (!lines) {
		try {
			lines = fs.readFileSync(filePath, 'utf8').split(/\r\n|\n|\r/u);
		} catch {
			lines = [];
		}
		sourceLinesByFile.set(filePath, lines);
	}
	return lines;
}

function projectLocation(
	workspace: vscode.WorkspaceFolder,
	diagnostic: WorkbenchCompilerDiagnostic,
): vscode.Uri | undefined {
	const root = realPath(workspace.uri.fsPath);
	const candidate = diagnostic.location.fileAbs ?? diagnostic.location.file;
	if (!root || candidate.length === 0) {
		return undefined;
	}
	if (!diagnostic.location.fileAbs
		&& diagnostic.location.addon
		&& diagnostic.location.addon.toLowerCase() !== workspace.name.toLowerCase()) {
		return undefined;
	}
	const resolved = path.isAbsolute(candidate)
		? path.resolve(candidate)
		: path.resolve(root, candidate);
	const canonical = realPath(resolved);
	return canonical && isContained(root, canonical)
		? vscode.Uri.file(canonical)
		: undefined;
}

function isContained(root: string, candidate: string): boolean {
	const relative = path.relative(root, candidate);
	return relative === ''
		|| (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

function realPath(candidate: string): string | undefined {
	try {
		return fs.realpathSync.native(candidate);
	} catch {
		return undefined;
	}
}

function onlyAddonWorkspace(): vscode.WorkspaceFolder | undefined {
	const folders = vscode.workspace.workspaceFolders;
	return folders?.length === 1 ? folders[0] : undefined;
}

function eligibleActiveDocument(): vscode.TextDocument | undefined {
	const document = vscode.window.activeTextEditor?.document;
	return document && eligibleDocument(document) ? document : undefined;
}

function eligibleDocument(document: vscode.TextDocument): boolean {
	const workspace = onlyAddonWorkspace();
	if (!workspace || document.languageId !== 'enforce' || document.uri.scheme !== 'file') {
		return false;
	}
	const root = realPath(workspace.uri.fsPath);
	const candidate = realPath(document.uri.fsPath);
	return Boolean(root && candidate && isContained(root, candidate));
}

function hasDirtyEligibleDocuments(): boolean {
	return vscode.workspace.textDocuments.some(document =>
		document.isDirty && eligibleDocument(document));
}

function renderStaleDiagnostic(
	original: vscode.Diagnostic,
	reason: string,
): vscode.Diagnostic {
	const stale = new vscode.Diagnostic(
		original.range,
		`[Possibly outdated Workbench result — ${reason}] ${original.message}`,
		original.severity,
	);
	stale.source = `${workbenchDiagnostics.source} (possibly outdated)`;
	stale.code = original.code;
	stale.relatedInformation = original.relatedInformation;
	return stale;
}
