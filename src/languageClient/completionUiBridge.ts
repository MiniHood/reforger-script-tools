import * as vscode from 'vscode';
import { experimentalAutoFormattingEnabled } from '../extensionConfig/experimentalAutoFormatting';
import { languageClientCommands, languageClientCompletion, languageClientLanguage } from '../extensionConfig/languageClient';
import { diagnostic } from '../diagnostics/diagnostics';
import { completionItemCount, completionPresentationItems, completionPresentationMetadata, isCompletionListIncomplete, type CompletionMiddlewareCallbacks, type CompletionPresentationItem } from './completionMiddleware';

let completionTransactionSequence = 0;
let pendingSnippetSuggestTransaction: SnippetSuggestTransaction | undefined;
let pendingEmptyCompletionRefresh: EmptyCompletionRefresh | undefined;
let pendingIfSpaceCommit: IfSpaceCommit | undefined;
let latestEditorDocumentChange: EditorDocumentChange | undefined;
const completionLifecycleTraceLimit = 80;
const completionLifecycleTrace: CompletionLifecycleTraceEvent[] = [];
const completionPresentationObservations = new Map<string, CompletionPresentationObservation>();

// TEMPORARY: release-gated forensic trace for the RplRpc multi-placeholder
// bridge. OpenSpec task 3.3 tracks removing this once live editor behavior is
// proven. It records only counts, lengths, and state transitions.
const snippetSuggestTraceVersion = 3;
const maxSnippetSuggestSelectionProbes = 8;

interface SnippetSuggestTransaction {
	id: number;
	documentUri: string;
	expectedSelectionTexts: readonly string[];
	nextPlaceholderIndex: number;
	selectionProbeCount: number;
	selectionListener: vscode.Disposable;
	cleanupTimer?: ReturnType<typeof setTimeout>;
	suggestDispatchScheduled: boolean;
	awaitingCompletionResponse: boolean;
	awaitingSnippetAdvance: boolean;
	finalTabstop: boolean;
}

interface SnippetSuggestCommandOptions {
	finalTabstop?: boolean;
}

interface EmptyCompletionRefresh {
	documentUri: string;
	requestVersion: number;
}

interface IfSpaceCommit {
	documentUri: string;
	version: number;
	expectedCommit: string;
	deletion: vscode.Range;
	caret: vscode.Position;
}

interface EditorDocumentChange {
	documentUri: string;
	version: number;
	hasDeletion: boolean;
}

interface CompletionLifecycleTraceEvent {
	documentUri: string;
	event: string;
	fields: Record<string, string | number | boolean | undefined>;
}

interface CompletionPresentationObservation {
	requestVersion: number;
	responseVersion: number;
	line: number;
	character: number;
	triggerKind: number;
	items: CompletionPresentationItem[];
}

function registerEmptyCompletionRefresh(): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		const documentUri = event.document.uri.toString();
		const hasDeletion = event.contentChanges.some(change => change.rangeLength > change.text.length);
		if (event.document.languageId === languageClientLanguage.id) {
			recordCompletionLifecycle(documentUri, 'documentChange', {
				version: event.document.version,
				changeCount: event.contentChanges.length,
				hasDeletion,
				insertedCharacters: event.contentChanges.reduce((total, change) => total + change.text.length, 0),
				deletedCharacters: event.contentChanges.reduce((total, change) => total + change.rangeLength, 0),
				activeDocument: isActiveEnforceDocument(event.document),
			});
		}
		latestEditorDocumentChange = {
			documentUri,
			version: event.document.version,
			hasDeletion,
		};

		const refresh = pendingEmptyCompletionRefresh;
		if (!refresh || refresh.documentUri !== documentUri || event.document.version <= refresh.requestVersion) {
			return;
		}
		pendingEmptyCompletionRefresh = undefined;
		if (!hasDeletion || !isActiveEnforceDocument(event.document)) {
			recordCompletionLifecycle(documentUri, 'emptyRefreshCancelled', {
				reason: hasDeletion ? 'inactiveDocument' : 'nonDeletion',
			});
			diagnostic('completion.emptyRefresh.cancelled', {
				reason: hasDeletion ? 'inactiveDocument' : 'nonDeletion',
			});
			return;
		}
		dispatchEmptyCompletionRefresh(event.document, 'deletion');
	});
}

function armEmptyCompletionRefresh(
	document: vscode.TextDocument,
	requestVersion: number,
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): void {
	if (!isRefreshableEmptyCompletion(result)) {
		if (pendingEmptyCompletionRefresh?.documentUri === document.uri.toString()) {
			pendingEmptyCompletionRefresh = undefined;
		}
		return;
	}

	const documentUri = document.uri.toString();
	const latestChange = latestEditorDocumentChange;
	if (latestChange?.documentUri === documentUri
		&& latestChange.version > requestVersion
		&& latestChange.hasDeletion
		&& isActiveEnforceDocument(document)) {
		dispatchEmptyCompletionRefresh(document, 'staleEmptyResponseAfterDeletion');
		return;
	}
	pendingEmptyCompletionRefresh = { documentUri, requestVersion };
	recordCompletionLifecycle(documentUri, 'emptyRefreshArmed', { requestVersion });
	diagnostic('completion.emptyRefresh.armed', { requestVersion });
}

function isRefreshableEmptyCompletion(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
): result is vscode.CompletionList {
	return result !== null
		&& result !== undefined
		&& 'items' in result
		&& result.items.length === 0
		&& result.isIncomplete === true;
}

function recordCompletionLifecycle(
	documentUri: string,
	event: string,
	fields: Record<string, string | number | boolean | undefined>,
): void {
	completionLifecycleTrace.push({ documentUri, event, fields });
	if (completionLifecycleTrace.length > completionLifecycleTraceLimit) {
		completionLifecycleTrace.shift();
	}
	diagnostic(`completion.lifecycle.${event}`, fields);
}

function isActiveEnforceDocument(document: vscode.TextDocument): boolean {
	return document.languageId === languageClientLanguage.id
		&& vscode.window.activeTextEditor?.document.uri.toString() === document.uri.toString();
}

function dispatchEmptyCompletionRefresh(document: vscode.TextDocument, source: 'deletion' | 'staleEmptyResponseAfterDeletion'): void {
	recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshDispatchRequested', { source });
	diagnostic('completion.emptyRefresh.dispatched', { source });
	queueMicrotask(() => {
		if (!isActiveEnforceDocument(document)) {
			recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshCancelled', { reason: 'activeEditorChanged' });
			diagnostic('completion.emptyRefresh.cancelled', { reason: 'activeEditorChanged' });
			return;
		}
		void vscode.commands.executeCommand('editor.action.triggerSuggest').then(
			() => {
				recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshSuggestDispatched', { source });
				diagnostic('completion.emptyRefresh.suggestDispatched', { source });
			},
			() => {
				recordCompletionLifecycle(document.uri.toString(), 'emptyRefreshSuggestDispatchError', { source });
				diagnostic('completion.emptyRefresh.suggestDispatchError', { source });
			},
		);
	});
}

function triggerSuggestAtSnippetPlaceholder(...expectedSelectionTexts: unknown[]): void {
	const commandOptions = expectedSelectionTexts.at(-1);
	const finalTabstop = isSnippetSuggestCommandOptions(commandOptions)
		&& commandOptions.finalTabstop === true;
	const placeholderArguments = finalTabstop
		? expectedSelectionTexts.slice(0, -1)
		: expectedSelectionTexts;
	diagnostic('completion.transaction.commandReceived', {
		traceVersion: snippetSuggestTraceVersion,
		placeholderCount: placeholderArguments.length,
		finalTabstop,
	});
	if (placeholderArguments.length === 0
		|| placeholderArguments.some(text => typeof text !== 'string')) {
		diagnostic('completion.transaction.ignored', { reason: 'invalidPlaceholderArgument' });
		return;
	}
	const expectedSelectionTextSequence = placeholderArguments as string[];
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== languageClientLanguage.id) {
		diagnostic('completion.transaction.ignored', { reason: 'noActiveEnforceEditor' });
		return;
	}

	clearSnippetSuggestTransaction();
	const id = ++completionTransactionSequence;
	const documentUri = editor.document.uri.toString();
	const tryTrigger = (candidate: vscode.TextEditor, source: 'command' | 'selection'): void => {
		const transaction = pendingSnippetSuggestTransaction;
		if (!transaction || transaction.id !== id || candidate.document.uri.toString() !== documentUri) {
			return;
		}
		if (transaction.suggestDispatchScheduled || transaction.awaitingCompletionResponse) {
			return;
		}
		const expectedText = transaction.expectedSelectionTexts[transaction.nextPlaceholderIndex];
		if (source === 'selection'
			&& expectedText.length === 0
			&& transaction.nextPlaceholderIndex > 0
			&& !transaction.awaitingSnippetAdvance) {
			// Completing an empty tabstop leaves an ordinary empty caret at the
			// end of its inserted text. It is not the following tabstop.
			diagnostic('completion.transaction.selectionIgnored', {
				transactionId: id,
				reason: 'nextEmptyPlaceholderNotAdvanced',
				placeholderIndex: transaction.nextPlaceholderIndex,
			});
			return;
		}
		const selectionCount = candidate.selections.length;
		const selectionLength = candidate.selection.end.character - candidate.selection.start.character;
		const matchesExpected = selectionCount === 1
			&& (expectedText.length === 0
				? candidate.selection.isEmpty
				: !candidate.selection.isEmpty
					&& candidate.document.getText(candidate.selection) === expectedText);
		if (!matchesExpected) {
			if (transaction.selectionProbeCount < maxSnippetSuggestSelectionProbes) {
				transaction.selectionProbeCount += 1;
				diagnostic('completion.transaction.selectionIgnored', {
					transactionId: id,
					source,
					placeholderIndex: transaction.nextPlaceholderIndex,
					selectionCount,
					selectionLength,
					expectedLength: expectedText.length,
					probeCount: transaction.selectionProbeCount,
				});
			}
			return;
		}
		diagnostic('completion.transaction.placeholderObserved', {
			transactionId: id,
			source,
			placeholderIndex: transaction.nextPlaceholderIndex,
			placeholderCount: transaction.expectedSelectionTexts.length,
			selectionLength,
		});
		transaction.suggestDispatchScheduled = true;
		transaction.awaitingCompletionResponse = true;
		transaction.awaitingSnippetAdvance = false;
		resetSnippetSuggestTransactionTimeout(transaction, 'completionResponseNotObserved');
		queueMicrotask(() => {
			if (pendingSnippetSuggestTransaction?.id !== id) {
				return;
			}
			void vscode.commands.executeCommand('editor.action.triggerSuggest').then(
				() => diagnostic('completion.transaction.suggestDispatched', { transactionId: id }),
				() => diagnostic('completion.transaction.suggestDispatchError', { transactionId: id }),
			);
		});
	};

	const selectionListener = vscode.window.onDidChangeTextEditorSelection(event => {
		tryTrigger(event.textEditor, 'selection');
	});
	pendingSnippetSuggestTransaction = {
		id,
		documentUri,
		expectedSelectionTexts: expectedSelectionTextSequence,
		nextPlaceholderIndex: 0,
		selectionProbeCount: 0,
		selectionListener,
		suggestDispatchScheduled: false,
		awaitingCompletionResponse: false,
		awaitingSnippetAdvance: false,
		finalTabstop,
	};
	resetSnippetSuggestTransactionTimeout(pendingSnippetSuggestTransaction, 'placeholderNotObserved');
	diagnostic('completion.transaction.armed', {
		transactionId: id,
		traceVersion: snippetSuggestTraceVersion,
		placeholderCount: expectedSelectionTextSequence.length,
	});
	// An empty tabstop can be published after the completion command can run.
	// Defer one probe so it observes the tabstop rather than the pre-snippet
	// caret, which would open a competing value-context request.
	if (expectedSelectionTextSequence[0].length > 0) {
		tryTrigger(editor, 'command');
	} else {
		queueMicrotask(() => {
			const activeEditor = vscode.window.activeTextEditor;
			if (activeEditor) {
				tryTrigger(activeEditor, 'command');
			}
		});
	}
}

function wrapBridgeCompletionCommands(
	result: vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined,
	transactionId: number,
): void {
	const items = !result ? [] : ('items' in result ? result.items : result);
	for (const item of items) {
		let originalCommand = item.command;
		const transaction = pendingSnippetSuggestTransaction;
		let moveToFinalTabstop = transaction?.finalTabstop === true
			&& transaction.nextPlaceholderIndex + 1 >= transaction.expectedSelectionTexts.length;
		if (transaction?.expectedSelectionTexts[transaction.nextPlaceholderIndex - 1] === 'default'
			&& item.label === 'case') {
			item.insertText = new vscode.SnippetString('case ${1:value}');
			originalCommand = {
				title: 'Suggest switch case value',
				command: languageClientCommands.triggerSuggestAtSnippetPlaceholder,
				arguments: ['value'],
			};
			moveToFinalTabstop = false;
		}
		item.command = {
			title: 'Advance enum snippet placeholder',
			command: languageClientCommands.advanceSnippetPlaceholderAfterAccept,
			arguments: [transactionId, originalCommand, moveToFinalTabstop],
		};
	}
}

async function advanceSnippetPlaceholderAfterAccept(
	transactionId: unknown,
	originalCommand: unknown,
	moveToFinalTabstop: unknown,
): Promise<void> {
	if (isVscodeCommand(originalCommand)) {
		await vscode.commands.executeCommand(
			originalCommand.command,
			...(originalCommand.arguments ?? []),
		);
	}

	const transaction = pendingSnippetSuggestTransaction;
	if (moveToFinalTabstop === true
		&& nestedSnippetTransactionTookOwnership(transactionId, transaction?.id)) {
		diagnostic('completion.transaction.nestedSnippetTookOwnership', {
			transactionId: typeof transactionId === 'number' ? transactionId : undefined,
		});
		return;
	}
	if (typeof transactionId !== 'number') {
		return;
	}
	if (transaction?.id !== transactionId
		|| transaction.nextPlaceholderIndex >= transaction.expectedSelectionTexts.length) {
		if (moveToFinalTabstop === true) {
			await moveToFinalSnippetTabstop(transactionId);
		}
		return;
	}

	diagnostic('completion.transaction.accepted', {
		transactionId,
		placeholderIndex: transaction.nextPlaceholderIndex - 1,
	});
	transaction.awaitingSnippetAdvance = true;
	try {
		await vscode.commands.executeCommand('jumpToNextSnippetPlaceholder');
		diagnostic('completion.transaction.nextPlaceholderDispatched', {
			transactionId,
			placeholderIndex: transaction.nextPlaceholderIndex,
		});
	} catch {
		transaction.awaitingSnippetAdvance = false;
		diagnostic('completion.transaction.nextPlaceholderDispatchError', {
			transactionId,
			placeholderIndex: transaction.nextPlaceholderIndex,
		});
	}
}

/** A nested generic completion owns its own selected type slot. Its parent
 * must not advance the active snippet to a final tabstop after that handoff. */
export function nestedSnippetTransactionTookOwnership(
	expectedTransactionId: unknown,
	activeTransactionId: number | undefined,
): boolean {
	return activeTransactionId !== undefined && activeTransactionId !== expectedTransactionId;
}

async function moveToFinalSnippetTabstop(transactionId: number): Promise<void> {
	try {
		await vscode.commands.executeCommand('jumpToNextSnippetPlaceholder');
		diagnostic('completion.transaction.finalTabstopDispatched', { transactionId });
	} catch {
		diagnostic('completion.transaction.finalTabstopDispatchError', { transactionId });
	}
}

function isSnippetSuggestCommandOptions(value: unknown): value is SnippetSuggestCommandOptions {
	return typeof value === 'object'
		&& value !== null
		&& 'finalTabstop' in value;
}

function isVscodeCommand(value: unknown): value is vscode.Command {
	return typeof value === 'object'
		&& value !== null
		&& 'command' in value
		&& typeof value.command === 'string';
}

/**
 * Removes only the commit character that VS Code appends after accepting the
 * Rust-authored `if ($0)` snippet with Space. This is a completion UI adapter,
 * not TypeScript syntax recognition: Rust attaches this command exclusively to
 * that item, and the exact caret-local postcondition is the whole admission
 * contract.
 */
async function normalizeIfSpaceCommit(args: readonly unknown[]): Promise<void> {
	if (!experimentalAutoFormattingEnabled()) {
		return;
	}
	const contract = ifSpaceCommitContractFromCommandArguments(args);
	if (!contract) {
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor
		|| editor.document.languageId !== languageClientLanguage.id
		|| editor.selections.length !== 1
		|| !editor.selection.isEmpty) {
		return;
	}
	// Current VS Code can insert Space before applying the snippet edit. In that
	// ordering the commit character moves after `if ()`, while the snippet
	// caret remains inside the parentheses. Remove only that exact character.
	if (editor.selection.active.isEqual(contract.caret)
		&& editor.document.getText(contract.trailingDeletion) === contract.expectedCommit) {
		diagnostic('completion.ifSpaceCommit', { outcome: 'preCompletionCommitObserved' });
		await removeIfSpaceCommitCharacter(editor, contract.trailingDeletion, contract.caret);
		return;
	}
	if (editor.selection.active.isEqual(contract.deletion.end)
		&& latestEditorDocumentChange?.documentUri === editor.document.uri.toString()
		&& latestEditorDocumentChange.version === editor.document.version
		&& !latestEditorDocumentChange.hasDeletion) {
		diagnostic('completion.ifSpaceCommit', { outcome: 'postCommitContractObserved' });
		await removeIfSpaceCommitCharacter(editor, contract.deletion, contract.caret);
		return;
	}
	if (!editor.selection.active.isEqual(contract.deletion.start)) {
		diagnostic('completion.ifSpaceCommit', { outcome: 'ignored' });
		return;
	}
	pendingIfSpaceCommit = {
		documentUri: editor.document.uri.toString(),
		version: editor.document.version,
		...contract,
	};
	diagnostic('completion.ifSpaceCommit', { outcome: 'awaitingCommitCharacter' });
}

/** Applies the Rust-authored separator only when the user has enabled editor
 * automatic edits and the completion left the caret after that exact directive. */
async function applyDirectiveSeparator(directive: unknown): Promise<void> {
	if (typeof directive !== 'string' || !experimentalAutoFormattingEnabled()) {
		return;
	}
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== languageClientLanguage.id
		|| editor.selections.length !== 1 || !editor.selection.isEmpty) {
		return;
	}
	const caret = editor.selection.active;
	const linePrefix = editor.document.lineAt(caret.line).text.slice(0, caret.character);
	if (!linePrefix.endsWith(directive)) {
		return;
	}
	await editor.edit(edit => edit.insert(caret, ' '));
}

interface IfSpaceCommitContract {
	expectedCommit: string;
	deletion: vscode.Range;
	trailingDeletion: vscode.Range;
	caret: vscode.Position;
}

export function ifSpaceCommitContractFromCommandArguments(args: readonly unknown[]): IfSpaceCommitContract | undefined {
	const [value] = args;
	if (typeof value !== 'object' || value === null) {
		return undefined;
	}
	const contract = value as {
		expectedCommit?: unknown;
		deletion?: { start?: unknown; end?: unknown };
		trailingDeletion?: { start?: unknown; end?: unknown };
		caret?: unknown;
	};
	const position = (candidate: unknown): vscode.Position | undefined => {
		if (typeof candidate !== 'object' || candidate === null) {
			return undefined;
		}
		const { line, character } = candidate as { line?: unknown; character?: unknown };
		return typeof line === 'number' && Number.isSafeInteger(line) && line >= 0
			&& typeof character === 'number' && Number.isSafeInteger(character) && character >= 0
			? new vscode.Position(line, character) : undefined;
	};
	const start = position(contract.deletion?.start);
	const end = position(contract.deletion?.end);
	const trailingStart = position(contract.trailingDeletion?.start);
	const trailingEnd = position(contract.trailingDeletion?.end);
	const caret = position(contract.caret);
	if (typeof contract.expectedCommit !== 'string' || contract.expectedCommit.length !== 1
		|| !start || !end || !trailingStart || !trailingEnd || !caret) {
		return undefined;
	}
	return {
		expectedCommit: contract.expectedCommit,
		deletion: new vscode.Range(start, end),
		trailingDeletion: new vscode.Range(trailingStart, trailingEnd),
		caret,
	};
}

function registerIfSpaceCommitCleanup(): vscode.Disposable {
	return vscode.workspace.onDidChangeTextDocument(event => {
		const pending = pendingIfSpaceCommit;
		if (!pending || event.document.uri.toString() !== pending.documentUri) {
			return;
		}
		pendingIfSpaceCommit = undefined;
		const [change] = event.contentChanges;
		if (event.document.version !== pending.version + 1
			|| event.contentChanges.length !== 1
			|| !change.range.isEmpty
		|| change.text !== pending.expectedCommit
		|| !change.range.start.isEqual(pending.deletion.start)) {
			diagnostic('completion.ifSpaceCommit', { outcome: 'unexpectedChange' });
			return;
		}
		const editor = vscode.window.activeTextEditor;
		if (!editor
			|| editor.document.uri.toString() !== pending.documentUri) {
			diagnostic('completion.ifSpaceCommit', { outcome: 'postCommitStateMismatch' });
			return;
		}
		diagnostic('completion.ifSpaceCommit', { outcome: 'commitObserved' });
	void removeIfSpaceCommitCharacter(editor, pending.deletion, pending.caret);
	});
}

async function removeIfSpaceCommitCharacter(
	editor: vscode.TextEditor,
	deletion: vscode.Range,
	caret: vscode.Position,
): Promise<void> {
	const applied = await editor.edit(edit => edit.delete(deletion), {
		undoStopBefore: false,
		undoStopAfter: false,
	});
	if (applied) {
		editor.selection = new vscode.Selection(caret, caret);
		diagnostic('completion.ifSpaceCommit', { outcome: 'normalized' });
	}
}

function advanceSnippetSuggestTransaction(id: number): void {
	const transaction = pendingSnippetSuggestTransaction;
	if (!transaction || transaction.id !== id) {
		return;
	}
	transaction.suggestDispatchScheduled = false;
	transaction.awaitingCompletionResponse = false;
	transaction.nextPlaceholderIndex += 1;
	if (transaction.nextPlaceholderIndex >= transaction.expectedSelectionTexts.length) {
		clearSnippetSuggestTransaction(id);
		return;
	}
	resetSnippetSuggestTransactionTimeout(transaction, 'nextPlaceholderNotObserved');
	diagnostic('completion.transaction.awaitingNextPlaceholder', {
		transactionId: id,
		placeholderIndex: transaction.nextPlaceholderIndex,
		placeholderCount: transaction.expectedSelectionTexts.length,
	});
}

function resetSnippetSuggestTransactionTimeout(
	transaction: SnippetSuggestTransaction,
	reason: string,
): void {
	if (transaction.cleanupTimer) {
		clearTimeout(transaction.cleanupTimer);
	}
	transaction.cleanupTimer = setTimeout(() => {
		if (pendingSnippetSuggestTransaction?.id === transaction.id) {
			diagnostic('completion.transaction.abandoned', {
				transactionId: transaction.id,
				placeholderIndex: transaction.nextPlaceholderIndex,
				reason,
			});
			clearSnippetSuggestTransaction(transaction.id);
		}
	}, languageClientCompletion.snippetSuggestTransactionTimeoutMs);
}

function clearSnippetSuggestTransaction(expectedId?: number): void {
	const transaction = pendingSnippetSuggestTransaction;
	if (!transaction || (expectedId !== undefined && transaction.id !== expectedId)) {
		return;
	}
	transaction.selectionListener.dispose();
	if (transaction.cleanupTimer) {
		clearTimeout(transaction.cleanupTimer);
	}
	pendingSnippetSuggestTransaction = undefined;
}

export function completionLifecycleTraceForDocument(documentUri: string): string {
	const events = completionLifecycleTrace.filter(event => event.documentUri === documentUri);
	const lines = [
		'## Extension Completion Lifecycle Trace (temporary)',
		'',
		'Bounded to the latest 80 Enforce events in this extension host. It records no source text, cursor text, or completion payloads.',
		'',
	];
	if (events.length === 0) {
		lines.push('No lifecycle events were captured for this document.');
		return lines.join('\n');
	}
	lines.push('| Event | Fields |', '| --- | --- |');
	for (const event of events) {
		const fields = Object.entries(event.fields)
			.filter(([, value]) => value !== undefined)
			.map(([key, value]) => `${key}=${String(value)}`)
			.join(', ');
		lines.push(`| ${event.event} | ${fields || '<none>'} |`);
	}
	return lines.join('\n');
}

/** Diagnostic-only snapshot of the latest completion result handed to VS Code's suggest pipeline. */
export function completionPresentationObservationForDocument(documentUri: string): string {
	const observation = completionPresentationObservations.get(documentUri);
	const lines = [
		'## [DEBUG-ac-7e31] Observed Suggest Response',
		'',
		'This is the exact result returned through the language-client completion middleware to VS Code. It is not a server debug query.',
		'',
	];
	if (!observation) {
		lines.push('No completion response has been observed for this document in this extension host.');
		return lines.join('\n');
	}
	lines.push(
		`- Request version: ${observation.requestVersion}`,
		`- Response document version: ${observation.responseVersion}`,
		`- Cursor: line ${observation.line}, character ${observation.character}`,
		`- Trigger kind: ${observation.triggerKind}`,
		`- Item count: ${observation.items.length}`,
		'',
		'| # | Label | Insert Text | Insert Kind | Range | Command |',
		'| ---: | --- | --- | --- | --- | --- |',
	);
	for (const [index, item] of observation.items.entries()) {
		const escapeCell = (value: string) => value.replaceAll('|', '\\|').replaceAll('`', '\\`');
		lines.push(`| ${index + 1} | ${escapeCell(item.label)} | ${escapeCell(item.insertText)} | ${item.insertTextKind} | ${item.rangeKind} | ${escapeCell(item.command)} |`);
	}
	return lines.join('\n');
}

export const completionUiMiddlewareCallbacks: CompletionMiddlewareCallbacks = {
	begin: (document, position, triggerKind) => {
		const transaction = pendingSnippetSuggestTransaction;
		recordCompletionLifecycle(document.uri.toString(), 'request', {
			requestVersion: document.version,
			line: position.line,
			character: position.character,
			triggerKind,
		});
		return { transactionId: transaction?.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse ? transaction.id : undefined };
	},
	respond: (document, triggerKind, requestVersion, transactionId, result, elapsedMs) => {
		const latestRequest = [...completionLifecycleTrace]
			.reverse()
			.find(event => event.documentUri === document.uri.toString() && event.event === 'request');
		const line = typeof latestRequest?.fields.line === 'number' ? latestRequest.fields.line : -1;
		const character = typeof latestRequest?.fields.character === 'number' ? latestRequest.fields.character : -1;
		const items = completionPresentationItems(result);
		completionPresentationObservations.set(document.uri.toString(), {
			requestVersion,
			responseVersion: document.version,
			line,
			character,
			triggerKind,
			items,
		});
		recordCompletionLifecycle(document.uri.toString(), 'response', { requestVersion, currentVersion: document.version, triggerKind, itemCount: completionItemCount(result), isIncomplete: isCompletionListIncomplete(result), elapsedMs, ...completionPresentationMetadata(result) });
		armEmptyCompletionRefresh(document, requestVersion, result);
		const transaction = pendingSnippetSuggestTransaction;
		if (transaction && transaction.id === transactionId && transaction.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse) {
			diagnostic('completion.transaction.response', { transactionId: transaction.id, triggerKind, itemCount: completionItemCount(result), elapsedMs, ...completionPresentationMetadata(result) });
			wrapBridgeCompletionCommands(result, transaction.id);
			advanceSnippetSuggestTransaction(transaction.id);
		}
	},
	fail: (document, triggerKind, requestVersion, transactionId, elapsedMs) => {
		recordCompletionLifecycle(document.uri.toString(), 'responseError', { requestVersion, triggerKind, elapsedMs });
		const transaction = pendingSnippetSuggestTransaction;
		if (transaction && transaction.id === transactionId && transaction.documentUri === document.uri.toString() && transaction.awaitingCompletionResponse) {
			diagnostic('completion.transaction.responseError', { transactionId: transaction.id, triggerKind, elapsedMs });
			clearSnippetSuggestTransaction(transaction.id);
		}
	},
};

export function registerCompletionUiBridge(): vscode.Disposable[] {
	return [
		registerEmptyCompletionRefresh(),
		registerIfSpaceCommitCleanup(),
		vscode.commands.registerCommand(languageClientCommands.triggerSuggestAtSnippetPlaceholder, (...expectedSelectionTexts: unknown[]) => triggerSuggestAtSnippetPlaceholder(...expectedSelectionTexts)),
		vscode.commands.registerCommand(languageClientCommands.advanceSnippetPlaceholderAfterAccept, (transactionId: unknown, originalCommand: unknown, moveToFinalTabstop: unknown) => advanceSnippetPlaceholderAfterAccept(transactionId, originalCommand, moveToFinalTabstop)),
		vscode.commands.registerCommand(languageClientCommands.normalizeIfSpaceCommit, (...args: unknown[]) => normalizeIfSpaceCommit(args)),
		vscode.commands.registerCommand(languageClientCommands.applyDirectiveSeparator, (directive: unknown) => applyDirectiveSeparator(directive)),
	];
}
