import * as vscode from 'vscode';
import { WorkbenchStatus, callWorkbenchNetApi } from './workbench/netApi';
import { IssuePathResolutionOptions, ValidateScriptsIssue, ValidateScriptsResponse, buildValidationOutputLines, canMapIssueToFile, formatIssueCount, getTrimmedDiagnosticRange, issueBelongsToDocument, resolveIssuePath } from './workbench/validation';
import { ExtensionLogger } from './core/logger';
import { registerLanguageFeatures } from './language/languageFeatures';
import { ensureGameScriptData, registerGameDataExportCommands } from './gameData/gameDataExport';
import { EnforceSymbolIndex } from './language/index/symbolIndex';

let workbenchMonitorStarted = false;
let workbenchDetected = false;
let diagnosticCollection: vscode.DiagnosticCollection;
let latestValidationRunId = 0;

export function activate(context: vscode.ExtensionContext) {
	const output = vscode.window.createOutputChannel('Reforger Script Tools');
	const logger = new ExtensionLogger(context);
	const validationStatus = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
	context.subscriptions.push(output);
	context.subscriptions.push(validationStatus);
	diagnosticCollection = vscode.languages.createDiagnosticCollection('reforgerScriptTools');
	context.subscriptions.push(diagnosticCollection);

	output.appendLine('Reforger Script Tools activated.');
	output.appendLine(`Log file: ${logger.path}`);
	logger.info('Extension activated.');
	const { symbolIndex } = registerLanguageFeatures(context, logger, output);
	registerGameDataExportCommands(context, output, logger, symbolIndex);

	const checkWorkbenchStatus = vscode.commands.registerCommand('reforger-script-tools.checkWorkbenchStatus', async () => {
		output.show(true);
		output.appendLine('Checking Arma Reforger Workbench...');
		logger.info('Manual Workbench status check started.');

		try {
			const result = await getWorkbenchStatus(logger);

			if (result.errorCode !== 'Ok') {
				const message = `Workbench replied with an error: ${result.errorCode}`;
				output.appendLine(message);
				logger.warn(message);
				vscode.window.showWarningMessage(message);
				return;
			}

			const status = result.payload;
			if (!status) {
				const message = 'Workbench replied without status details.';
				output.appendLine(message);
				logger.warn(message);
				vscode.window.showWarningMessage(message);
				return;
			}

			const message = formatWorkbenchStatus(status);
			output.appendLine(message);
			logger.info(message);
			vscode.window.showInformationMessage(message);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			output.appendLine('Workbench is not reachable. Open Arma Reforger Workbench and try again.');
			logger.error(`Workbench status check failed: ${message}`);
			vscode.window.showWarningMessage('Workbench is not reachable. Open Arma Reforger Workbench and try again.');
		}
	});

	const validateScriptsCommand = vscode.commands.registerCommand('reforger-script-tools.validateScripts', async () => {
		await validateCurrentScriptCommand(output, logger, validationStatus);
	});

	const saveListener = vscode.workspace.onDidSaveTextDocument(async document => {
		const settings = getSettings();
		if (!settings.validateOnSave || !isEnforceScript(document)) {
			return;
		}

		logger.info(`Validate on save triggered for ${document.uri.fsPath}`);
		await validateCurrentScriptCommand(output, logger, validationStatus, document);
	});

	context.subscriptions.push(checkWorkbenchStatus, validateScriptsCommand, saveListener);
	void runStartupWorkflow(context, output, logger, symbolIndex);
}

export function deactivate() {}

async function startWorkbenchMonitor(output: vscode.OutputChannel, logger: ExtensionLogger): Promise<void> {
	if (workbenchMonitorStarted) {
		return;
	}

	workbenchMonitorStarted = true;
	logger.info('Workbench monitor started.');

	const initialStatus = await tryGetWorkbenchStatus(logger);
	if (initialStatus?.IsRunning) {
		workbenchDetected = true;
		output.appendLine(formatWorkbenchStatus(initialStatus));
		logger.info('Workbench detected during startup check.');
		return;
	}

	vscode.window.withProgress(
		{
			location: vscode.ProgressLocation.Notification,
			title: 'Arma Reforger Workbench is not detected',
			cancellable: false,
		},
		async progress => {
			while (!workbenchDetected) {
				const status = await tryGetWorkbenchStatus(logger);
				if (status?.IsRunning) {
					workbenchDetected = true;
					const message = formatWorkbenchStatus(status);
					output.appendLine(message);
					logger.info(message);
					return;
				}

				await sleep(1000);
			}
		}
	);
}

async function runStartupWorkflow(
	context: vscode.ExtensionContext,
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	symbolIndex: EnforceSymbolIndex
): Promise<void> {
	logger.info('Startup workflow started: script data -> symbol index -> Workbench.');
	const scriptData = await ensureGameScriptData(context, output, logger, symbolIndex);
	if (scriptData.available) {
		const indexState = symbolIndex.getState();
		if ((scriptData.changed && !scriptData.indexed) || !indexState.cacheLoaded || indexState.cacheStale) {
			const stats = await symbolIndex.refresh(false, { reason: 'startup', forceGameDataRebuild: true });
			if (!stats) {
				logger.warn('Startup workflow could not build the game-data symbol index.');
			}
		} else if (!(await symbolIndex.ensureGameDataIndex())) {
			logger.warn('Startup workflow could not load the game-data symbol index.');
		}
	} else {
		logger.warn('Startup workflow continuing without script data or game-data symbol index.');
	}

	await startWorkbenchMonitor(output, logger);
	logger.info('Startup workflow complete.');
}

async function getWorkbenchStatus(logger: ExtensionLogger) {
	const settings = getSettings();
	return callWorkbenchNetApi<WorkbenchStatus>(
		'IsWorkbenchRunning',
		{},
		settings.netApiPort,
		settings.netApiHost,
		settings.netApiTimeoutMs,
		message => logger.info(message)
	);
}

async function tryGetWorkbenchStatus(logger: ExtensionLogger): Promise<WorkbenchStatus | undefined> {
	try {
		const result = await getWorkbenchStatus(logger);
		if (result.errorCode === 'Ok' && result.payload) {
			return result.payload;
		}

		logger.warn(`Workbench status returned ${result.errorCode}.`);
		return undefined;
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		logger.info(`Workbench monitor check failed: ${message}`);
		return undefined;
	}
}

function formatWorkbenchStatus(status: WorkbenchStatus): string {
	const running = status.IsRunning ? 'connected' : 'not running';
	const scripts = status.ScriptsCompiled ? 'scripts compiled' : 'script compile failed';
	return `Workbench ${running}; ${scripts}.`;
}

function sleep(milliseconds: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, milliseconds));
}

interface ValidationRun {
	id: number;
	startedAt: number;
	document: vscode.TextDocument;
	configuration: string;
}

async function validateCurrentScriptCommand(
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	statusBar: vscode.StatusBarItem,
	document = vscode.window.activeTextEditor?.document
): Promise<void> {
	if (!document) {
		vscode.window.showWarningMessage('Open a Reforger script file first.');
		return;
	}

	if (!isEnforceScript(document)) {
		vscode.window.showWarningMessage('The current file is not an Enforce script.');
		return;
	}

	const run: ValidationRun = {
		id: ++latestValidationRunId,
		startedAt: Date.now(),
		document,
		configuration: 'WORKBENCH',
	};

	statusBar.text = '$(sync~spin) Reforger validating scripts';
	statusBar.tooltip = `Validating ${document.uri.fsPath}`;
	statusBar.show();
	output.show(true);
	output.appendLine(`Workbench compiler: checking ${document.uri.fsPath}`);
	logger.info(`Validate scripts requested. run=${run.id} file=${document.uri.fsPath}`);

	try {
		const result = await getWorkbenchStatus(logger);
		if (!isLatestValidationRun(run, logger)) {
			return;
		}

		if (result.errorCode !== 'Ok' || !result.payload?.IsRunning) {
			statusBar.text = '$(warning) Reforger Workbench not reachable';
			hideStatusBarSoon(statusBar);
			vscode.window.showWarningMessage('Workbench is not reachable. Open Arma Reforger Workbench and try again.');
			return;
		}

		await validateScripts(output, logger, statusBar, run);
	} catch (error) {
		if (!isLatestValidationRun(run, logger)) {
			return;
		}

		const message = error instanceof Error ? error.message : String(error);
		statusBar.text = '$(error) Reforger validation failed';
		hideStatusBarSoon(statusBar);
		logger.error(`Validate scripts failed. run=${run.id} message=${message}`);
		vscode.window.showWarningMessage('Script validation failed. Check the Reforger Script Tools output.');
	}
}

function isEnforceScript(document: vscode.TextDocument): boolean {
	return document.uri.scheme === 'file' && document.fileName.toLowerCase().endsWith('.c');
}

interface ReforgerSettings {
	validateOnSave: boolean;
	netApiHost: string;
	netApiPort: number;
	netApiTimeoutMs: number;
	showBaseGameWarnings: boolean;
	showSuccessMessage: boolean;
}

function getSettings(): ReforgerSettings {
	const config = vscode.workspace.getConfiguration('reforgerScriptTools');
	return {
		validateOnSave: config.get<boolean>('validateOnSave', true),
		netApiHost: config.get<string>('netApiHost', '127.0.0.1'),
		netApiPort: config.get<number>('netApiPort', 5775),
		netApiTimeoutMs: config.get<number>('netApiTimeoutMs', 3000),
		showBaseGameWarnings: config.get<boolean>('showBaseGameWarnings', false),
		showSuccessMessage: config.get<boolean>('showSuccessMessage', true),
	};
}

async function validateScripts(
	output: vscode.OutputChannel,
	logger: ExtensionLogger,
	statusBar: vscode.StatusBarItem,
	run: ValidationRun
): Promise<void> {
	const settings = getSettings();
	const apiStartedAt = Date.now();
	output.appendLine(`Workbench compiler: running ${run.configuration} validation...`);
	logger.info(`ValidateScripts started. run=${run.id} configuration=${run.configuration}`);

	const result = await callWorkbenchNetApi<ValidateScriptsResponse>(
		'ValidateScripts',
		{ Configuration: run.configuration },
		settings.netApiPort,
		settings.netApiHost,
		settings.netApiTimeoutMs,
		message => logger.info(message)
	);

	if (!isLatestValidationRun(run, logger)) {
		return;
	}

	const apiDurationMs = Date.now() - apiStartedAt;
	const totalDurationMs = Date.now() - run.startedAt;

	if (result.errorCode !== 'Ok') {
		const message = `Workbench validation failed: ${result.errorCode}`;
		output.clear();
		output.appendLine(`Workbench compiler: failed in ${totalDurationMs} ms.`);
		output.appendLine(message);
		output.show(true);
		statusBar.text = '$(error) Reforger validation failed';
		hideStatusBarSoon(statusBar);
		vscode.window.showWarningMessage(message);
		return;
	}

	if (!result.payload) {
		const message = 'Workbench validation returned no results.';
		output.clear();
		output.appendLine(`Workbench compiler: failed in ${totalDurationMs} ms.`);
		output.appendLine(message);
		output.show(true);
		statusBar.text = '$(error) Reforger validation failed';
		hideStatusBarSoon(statusBar);
		vscode.window.showWarningMessage(message);
		return;
	}

	const errors = result.payload.Errors ?? [];
	const warnings = result.payload.Warnings ?? [];
	logger.info(`ValidateScripts result. run=${run.id} success=${result.payload.Success} errors=${errors.length} warnings=${warnings.length} apiMs=${apiDurationMs} totalMs=${totalDurationMs}`);
	const pathResolutionOptions = await getValidationPathResolutionOptions();
	if (!isLatestValidationRun(run, logger)) {
		return;
	}

	const visibleWarnings = settings.showBaseGameWarnings ? warnings : warnings.filter(issue => canMapIssueToFile(issue, pathResolutionOptions));
	const allVisibleIssues = [...errors, ...visibleWarnings];

	updateDiagnostics(errors, visibleWarnings, logger, pathResolutionOptions);
	writeValidationOutput(output, errors, visibleWarnings, run.document, totalDurationMs, pathResolutionOptions);

	const currentFileIssues = allVisibleIssues.filter(issue => issueBelongsToDocument(issue, run.document));

	if (currentFileIssues.length > 0) {
		const errorCount = currentFileIssues.filter(issue => errors.includes(issue)).length;
		const warningCount = currentFileIssues.length - errorCount;
		statusBar.text = `$(error) Reforger validation failed: ${formatIssueCount(errorCount, warningCount)}`;
		hideStatusBarSoon(statusBar);
		vscode.window.showWarningMessage(`Validation found ${formatIssueCount(errorCount, warningCount)} in the current file.`);
		return;
	}

	if (errors.length > 0 || visibleWarnings.length > 0) {
		statusBar.text = `$(error) Reforger validation failed: ${formatIssueCount(errors.length, visibleWarnings.length)}`;
		hideStatusBarSoon(statusBar);
		vscode.window.showWarningMessage(`Validation completed with ${formatIssueCount(errors.length, visibleWarnings.length)}.`);
		return;
	}

	statusBar.text = `$(check) Reforger validation passed in ${Math.round(totalDurationMs)} ms`;
	hideStatusBarSoon(statusBar);
	if (settings.showSuccessMessage) {
		vscode.window.showInformationMessage('Validation passed. No script errors found.');
	}
}

function isLatestValidationRun(run: ValidationRun, logger: ExtensionLogger): boolean {
	if (run.id === latestValidationRunId) {
		return true;
	}

	logger.info(`Ignored stale validation result. run=${run.id} latest=${latestValidationRunId}`);
	return false;
}

function hideStatusBarSoon(statusBar: vscode.StatusBarItem): void {
	setTimeout(() => statusBar.hide(), 3000);
}

async function getValidationPathResolutionOptions(): Promise<IssuePathResolutionOptions> {
	const workspaceUris = await vscode.workspace.findFiles('**/*.c', '**/{node_modules,.git,out}/**');
	const candidatePaths = new Set<string>();
	for (const uri of workspaceUris) {
		candidatePaths.add(uri.fsPath);
	}
	for (const document of vscode.workspace.textDocuments) {
		if (document.uri.scheme === 'file' && document.fileName.toLowerCase().endsWith('.c')) {
			candidatePaths.add(document.uri.fsPath);
		}
	}

	return {
		candidatePaths: [...candidatePaths],
	};
}

function updateDiagnostics(
	errors: ValidateScriptsIssue[],
	warnings: ValidateScriptsIssue[],
	logger: ExtensionLogger,
	pathResolutionOptions: IssuePathResolutionOptions
): void {
	const diagnosticsByUri = new Map<string, vscode.Diagnostic[]>();

	for (const issue of errors) {
		addDiagnostic(diagnosticsByUri, issue, vscode.DiagnosticSeverity.Error, logger, pathResolutionOptions);
	}

	for (const issue of warnings) {
		addDiagnostic(diagnosticsByUri, issue, vscode.DiagnosticSeverity.Warning, logger, pathResolutionOptions);
	}

	diagnosticCollection.clear();
	for (const [uriString, diagnostics] of diagnosticsByUri) {
		diagnosticCollection.set(vscode.Uri.parse(uriString), diagnostics);
	}
}

function addDiagnostic(
	diagnosticsByUri: Map<string, vscode.Diagnostic[]>,
	issue: ValidateScriptsIssue,
	severity: vscode.DiagnosticSeverity,
	logger: ExtensionLogger,
	pathResolutionOptions: IssuePathResolutionOptions
): void {
	const resolution = resolveIssuePath(issue, pathResolutionOptions);
	if (resolution.kind === 'ambiguous') {
		logger.info(`Ambiguous validation issue path: issue=${JSON.stringify(issue)} candidates=${resolution.candidates.map(uri => uri.fsPath).join('; ')}`);
		return;
	}

	if (!resolution.uri) {
		logger.info(`Could not map validation issue to local file: ${JSON.stringify(issue)}`);
		return;
	}

	const line = Math.max(0, (issue.line || 1) - 1);
	const range = getTrimmedDiagnosticRange(resolution.uri, line);
	const diagnostic = new vscode.Diagnostic(range, issue.error, severity);
	diagnostic.source = 'Reforger Workbench';

	const existing = diagnosticsByUri.get(resolution.uri.toString()) ?? [];
	existing.push(diagnostic);
	diagnosticsByUri.set(resolution.uri.toString(), existing);
}

function writeValidationOutput(
	output: vscode.OutputChannel,
	errors: ValidateScriptsIssue[],
	warnings: ValidateScriptsIssue[],
	currentDocument: vscode.TextDocument,
	durationMs: number,
	pathResolutionOptions: IssuePathResolutionOptions
): void {
	output.clear();
	for (const line of buildValidationOutputLines(errors, warnings, {
		currentDocument,
		durationMs,
		pathResolutionOptions,
	})) {
		output.appendLine(line);
	}
	output.show(true);
}
