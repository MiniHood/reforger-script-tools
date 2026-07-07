import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

export interface ValidateScriptsResponse {
	Errors?: ValidateScriptsIssue[];
	Warnings?: ValidateScriptsIssue[];
	Success: boolean;
}

export interface ValidateScriptsIssue {
	error: string;
	file: string;
	fileAbs?: string;
	addon?: string;
	line: number;
}

export interface FormattedValidationIssue {
	issue: ValidateScriptsIssue;
	severity: 'ERROR' | 'WARNING';
	link: string;
}

export interface ValidationOutputOptions {
	currentDocument?: vscode.TextDocument;
	durationMs?: number;
	pathResolutionOptions?: IssuePathResolutionOptions;
}

export type IssuePathResolutionKind = 'absolute' | 'workspaceRelative' | 'suffix' | 'ambiguous' | 'unmapped';

export interface IssuePathResolution {
	kind: IssuePathResolutionKind;
	uri?: vscode.Uri;
	candidates: vscode.Uri[];
}

export interface IssuePathResolutionOptions {
	candidatePaths?: string[];
	fileExists?: (filePath: string) => boolean;
	workspaceFolders?: readonly vscode.WorkspaceFolder[];
}

export function buildValidationOutputLines(
	errors: ValidateScriptsIssue[],
	warnings: ValidateScriptsIssue[],
	currentDocumentOrOptions?: vscode.TextDocument | ValidationOutputOptions,
	durationMs?: number
): string[] {
	const options = getValidationOutputOptions(currentDocumentOrOptions, durationMs);
	const hasIssues = errors.length > 0 || warnings.length > 0;
	const lines = [
		formatValidationSummary(hasIssues, errors.length, warnings.length, options.durationMs),
	];

	if (options.currentDocument && hasIssues) {
		const currentIssues = [...errors, ...warnings].filter(issue => issueBelongsToDocument(issue, options.currentDocument!));
		const currentErrors = errors.filter(issue => issueBelongsToDocument(issue, options.currentDocument!));
		const currentWarnings = warnings.filter(issue => issueBelongsToDocument(issue, options.currentDocument!));
		lines.push(`Current file: ${currentIssues.length > 0 ? formatIssueCount(currentErrors.length, currentWarnings.length) : 'no issues'}`);
	}

	const formattedIssues = [
		...errors.map(issue => formatValidationIssue(issue, 'ERROR' as const, options.pathResolutionOptions)),
		...warnings.map(issue => formatValidationIssue(issue, 'WARNING' as const, options.pathResolutionOptions)),
	];

	if (formattedIssues.length === 0) {
		return lines;
	}

	lines.push('');
	if (options.currentDocument) {
		const currentIssues = formattedIssues.filter(formattedIssue => issueBelongsToDocument(formattedIssue.issue, options.currentDocument!));
		const otherIssues = formattedIssues.filter(formattedIssue => !issueBelongsToDocument(formattedIssue.issue, options.currentDocument!));
		const currentErrors = currentIssues.filter(issue => issue.severity === 'ERROR');
		const currentWarnings = currentIssues.filter(issue => issue.severity === 'WARNING');
		const otherErrors = otherIssues.filter(issue => issue.severity === 'ERROR');
		const otherWarnings = otherIssues.filter(issue => issue.severity === 'WARNING');

		if (currentErrors.length > 0) {
			lines.push(currentErrors.length === 1 ? 'Current file error:' : 'Current file errors:');
			for (const formattedIssue of currentErrors) {
				lines.push(formatValidationIssueLine(formattedIssue));
			}
		}

		if (currentWarnings.length > 0) {
			if (currentErrors.length > 0) {
				lines.push('');
			}
			lines.push(currentWarnings.length === 1 ? 'Current file warning:' : 'Current file warnings:');
			for (const formattedIssue of currentWarnings) {
				lines.push(formatValidationIssueLine(formattedIssue));
			}
		}

		if (otherIssues.length > 0) {
			if (currentIssues.length > 0) {
				lines.push('');
			}
			lines.push(`Other project issues (${formatIssueCount(otherErrors.length, otherWarnings.length)}):`);
			for (const formattedIssue of otherIssues) {
				lines.push(formatValidationIssueLine(formattedIssue));
			}
		}

		return lines;
	}

	for (const formattedIssue of formattedIssues) {
		lines.push(formatValidationIssueLine(formattedIssue));
	}

	return lines;
}

function getValidationOutputOptions(
	currentDocumentOrOptions: vscode.TextDocument | ValidationOutputOptions | undefined,
	durationMs: number | undefined
): ValidationOutputOptions {
	if (currentDocumentOrOptions && 'uri' in currentDocumentOrOptions) {
		return {
			currentDocument: currentDocumentOrOptions,
			durationMs,
		};
	}

	return currentDocumentOrOptions ?? { durationMs };
}

function formatValidationSummary(hasIssues: boolean, errors: number, warnings: number, durationMs?: number): string {
	const timing = durationMs === undefined ? '' : ` in ${Math.max(0, Math.round(durationMs))} ms`;
	if (!hasIssues) {
		return `Reforger validation passed${timing}.`;
	}

	return `Reforger validation failed${timing}: ${formatIssueCount(errors, warnings)}`;
}

function formatValidationIssueLine(formattedIssue: FormattedValidationIssue): string {
	return `${formattedIssue.severity} ${formattedIssue.link} ${formattedIssue.issue.error}`;
}

export function formatValidationIssue(
	issue: ValidateScriptsIssue,
	severity: FormattedValidationIssue['severity'],
	options?: IssuePathResolutionOptions
): FormattedValidationIssue {
	return {
		issue,
		severity,
		link: formatIssueLink(issue, options),
	};
}

export function issueBelongsToDocument(issue: ValidateScriptsIssue, document: vscode.TextDocument): boolean {
	return issueBelongsToPath(issue, document.uri.fsPath);
}

export function issueBelongsToPath(issue: ValidateScriptsIssue, filePath: string): boolean {
	const documentPath = normalizePath(filePath);

	if (issue.fileAbs && normalizePath(issue.fileAbs) === documentPath) {
		return true;
	}

	const issuePath = normalizePath(issue.file);
	return documentPath === issuePath || documentPath.endsWith(`/${issuePath}`);
}

export function formatIssueLink(issue: ValidateScriptsIssue, options?: IssuePathResolutionOptions): string {
	const resolution = resolveIssuePath(issue, options);
	if (resolution.uri) {
		return `${resolution.uri.fsPath}:${Math.max(1, issue.line || 1)}:1`;
	}

	const addonPrefix = issue.addon ? `${issue.addon}:` : '';
	return `${addonPrefix}${issue.file}:${Math.max(1, issue.line || 1)}`;
}

export function formatIssueCount(errors: number, warnings: number): string {
	const parts: string[] = [];
	if (errors > 0) {
		parts.push(`${errors} error${errors === 1 ? '' : 's'}`);
	}
	if (warnings > 0) {
		parts.push(`${warnings} warning${warnings === 1 ? '' : 's'}`);
	}
	return parts.join(' and ') || 'no issues';
}

export function resolveIssueUri(issue: ValidateScriptsIssue, options?: IssuePathResolutionOptions): vscode.Uri | undefined {
	return resolveIssuePath(issue, options).uri;
}

export function resolveIssuePath(issue: ValidateScriptsIssue, options: IssuePathResolutionOptions = {}): IssuePathResolution {
	if (issue.fileAbs) {
		return {
			kind: 'absolute',
			uri: vscode.Uri.file(issue.fileAbs),
			candidates: [vscode.Uri.file(issue.fileAbs)],
		};
	}

	const fileExists = options.fileExists ?? fs.existsSync;
	const workspaceFolders = options.workspaceFolders ?? vscode.workspace.workspaceFolders ?? [];
	const workspaceCandidates = new Map<string, vscode.Uri>();
	for (const folder of workspaceFolders) {
		const candidatePath = path.join(folder.uri.fsPath, issue.file);
		if (fileExists(candidatePath)) {
			workspaceCandidates.set(normalizePath(candidatePath), vscode.Uri.file(candidatePath));
		}
	}

	if (workspaceCandidates.size === 1) {
		const uri = [...workspaceCandidates.values()][0];
		return {
			kind: 'workspaceRelative',
			uri,
			candidates: [uri],
		};
	}

	if (workspaceCandidates.size > 1) {
		return {
			kind: 'ambiguous',
			candidates: [...workspaceCandidates.values()],
		};
	}

	const suffixCandidates = findSuffixCandidates(issue, options.candidatePaths ?? []);
	if (suffixCandidates.length === 1) {
		return {
			kind: 'suffix',
			uri: suffixCandidates[0],
			candidates: suffixCandidates,
		};
	}

	if (suffixCandidates.length > 1) {
		return {
			kind: 'ambiguous',
			candidates: suffixCandidates,
		};
	}

	return {
		kind: 'unmapped',
		candidates: [],
	};
}

export function canMapIssueToFile(issue: ValidateScriptsIssue, options?: IssuePathResolutionOptions): boolean {
	return Boolean(resolveIssueUri(issue, options));
}

export function getTrimmedDiagnosticRange(uri: vscode.Uri, line: number): vscode.Range {
	const openDocument = vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
	const text = getOpenDocumentLine(openDocument, line) ?? readLine(uri.fsPath, line);
	if (text === undefined) {
		return new vscode.Range(line, 0, line, Number.MAX_SAFE_INTEGER);
	}

	const start = text.search(/\S/);
	if (start < 0) {
		return new vscode.Range(line, 0, line, 0);
	}

	const endMatch = /\S\s*$/.exec(text);
	const end = endMatch ? endMatch.index + 1 : text.length;
	return new vscode.Range(line, start, line, end);
}

export function normalizePath(filePath: string): string {
	return filePath.replace(/\\/g, '/').toLowerCase();
}

function findSuffixCandidates(issue: ValidateScriptsIssue, candidatePaths: string[]): vscode.Uri[] {
	const issuePath = normalizePath(issue.file);
	const matchedPaths = new Map<string, vscode.Uri>();
	for (const candidatePath of candidatePaths) {
		const normalizedCandidate = normalizePath(candidatePath);
		if (normalizedCandidate === issuePath || normalizedCandidate.endsWith(`/${issuePath}`)) {
			matchedPaths.set(normalizedCandidate, vscode.Uri.file(candidatePath));
		}
	}

	return [...matchedPaths.values()];
}

function readLine(filePath: string, line: number): string | undefined {
	try {
		return fs.readFileSync(filePath, 'utf8').split(/\r?\n/)[line];
	} catch {
		return undefined;
	}
}

function getOpenDocumentLine(document: vscode.TextDocument | undefined, line: number): string | undefined {
	if (!document || line < 0 || line >= document.lineCount) {
		return undefined;
	}

	return document.lineAt(line).text;
}
