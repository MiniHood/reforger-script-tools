import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

export class ExtensionLogger {
	private readonly logFilePath: string;

	constructor(context: vscode.ExtensionContext) {
		const logDir = path.join(context.globalStorageUri.fsPath, 'logs');
		fs.mkdirSync(logDir, { recursive: true });
		this.logFilePath = path.join(logDir, 'reforger-script-tools.log');
	}

	get path(): string {
		return this.logFilePath;
	}

	info(message: string): void {
		this.write('INFO', message);
	}

	warn(message: string): void {
		this.write('WARN', message);
	}

	error(message: string): void {
		this.write('ERROR', message);
	}

	private write(level: string, message: string): void {
		const line = `${new Date().toISOString()} ${level} ${message}\n`;
		fs.appendFileSync(this.logFilePath, line, 'utf8');
	}
}
