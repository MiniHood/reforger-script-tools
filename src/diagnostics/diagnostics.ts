import * as fs from 'fs/promises';
import * as path from 'path';
import * as vscode from 'vscode';
import { diagnosticsConfig, diagnosticsLogs } from '../extensionConfig/diagnostics';

type DiagnosticField = string | number | boolean | undefined;

let enabled = false;
let session = '';
let logPath = '';
let writeQueue: Promise<void> = Promise.resolve();

export function initializeDiagnostics(context: vscode.ExtensionContext): void {
	enabled = vscode.workspace.getConfiguration(diagnosticsConfig.section).get<boolean>(diagnosticsConfig.settings.enabled, false);
	session = `${Date.now()}-${process.pid}`;
	logPath = path.join(context.globalStorageUri.fsPath, diagnosticsLogs.rootFolder, diagnosticsLogs.extensionFile);
	if (!enabled) {
		return;
	}
	writeQueue = writeQueue.then(() => prepareLog(logPath)).catch(() => undefined);
	diagnostic('activation', { extensionMode: context.extensionMode, workspaceFolders: vscode.workspace.workspaceFolders?.length ?? 0 });
}

export function diagnosticsEnabled(): boolean {
	return enabled;
}

export function languageServerDiagnosticPath(context: vscode.ExtensionContext): string | undefined {
	return enabled
		? path.join(context.globalStorageUri.fsPath, diagnosticsLogs.rootFolder, diagnosticsLogs.serverFile)
		: undefined;
}

export function diagnostic(event: string, fields: Record<string, DiagnosticField> = {}): void {
	if (!enabled || !logPath) {
		return;
	}
	const record = JSON.stringify({
		timestamp: new Date().toISOString(),
		component: 'extension',
		session,
		event,
		...fields,
	});
	writeQueue = writeQueue
		.then(() => fs.appendFile(logPath, `${record}\n`, 'utf8'))
		.catch(() => undefined);
}

async function prepareLog(target: string): Promise<void> {
	await fs.mkdir(path.dirname(target), { recursive: true });
	try {
		const stat = await fs.stat(target);
		if (stat.size <= diagnosticsLogs.maxBytes) {
			return;
		}
		const source = await fs.readFile(target, 'utf8');
		const retained = source.slice(-Math.floor(diagnosticsLogs.maxBytes / 2));
		const firstRecord = retained.indexOf('\n');
		await fs.writeFile(target, firstRecord >= 0 ? retained.slice(firstRecord + 1) : '', 'utf8');
	} catch {
		// A missing or unreadable support log must never affect extension behavior.
	}
}
