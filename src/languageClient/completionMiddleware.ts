import * as vscode from 'vscode';
import type { LanguageClientOptions } from 'vscode-languageclient/node';

type CompletionResult = vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined;

export interface CompletionPresentationItem {
	label: string;
	insertText: string;
	insertTextKind: 'snippet' | 'plain' | 'label';
	rangeKind: 'plain' | 'insertReplace' | 'none';
	command: string;
}

export function completionItemLabels(result: CompletionResult): string[] {
	const items = !result ? [] : ('items' in result ? result.items : result);
	return items.map(item => typeof item.label === 'string' ? item.label : item.label.label);
}

export function completionItemCount(result: CompletionResult): number {
	if (!result) {
		return 0;
	}
	return 'items' in result ? result.items.length : result.length;
}

/**
 * Captures the editor-native shape after vscode-languageclient has converted
 * the LSP response. This is deliberately separate from the server's debug
 * report: it proves what the VS Code suggest pipeline can actually insert.
 */
export function completionPresentationItems(result: CompletionResult): CompletionPresentationItem[] {
	const items = !result ? [] : ('items' in result ? result.items : result);
	return items.map(item => {
		const insertText = item.insertText;
		const insertTextKind = insertText instanceof vscode.SnippetString
			? 'snippet'
			: typeof insertText === 'string'
				? 'plain'
				: 'label';
		const rangeKind = !item.range
			? 'none'
			: item.range instanceof vscode.Range
				? 'plain'
				: 'insertReplace';
		return {
			label: typeof item.label === 'string' ? item.label : item.label.label,
			insertText: insertText instanceof vscode.SnippetString ? insertText.value : insertText ?? '',
			insertTextKind,
			rangeKind,
			command: item.command?.command ?? '',
		};
	});
}

export function isCompletionListIncomplete(result: CompletionResult): boolean {
	return result !== null && result !== undefined && 'items' in result && result.isIncomplete === true;
}

export function completionPresentationMetadata(result: CompletionResult): Record<string, string | number> {
	const items = !result ? [] : ('items' in result ? result.items : result);
	let plainRangeCount = 0;
	let insertReplaceRangeCount = 0;
	let invalidInsertReplaceRangeCount = 0;
	const firstRangeKinds: string[] = [];
	const firstFilterTextLengths: string[] = [];

	for (const item of items) {
		const range = item.range;
		if (!range) {
			continue;
		}
		if (range instanceof vscode.Range) {
			plainRangeCount += 1;
			if (firstRangeKinds.length < 3) {
				firstRangeKinds.push('plain');
				firstFilterTextLengths.push(String(item.filterText?.length ?? 0));
			}
			continue;
		}

		insertReplaceRangeCount += 1;
		if (!range.inserting.start.isEqual(range.replacing.start)
			|| !range.inserting.end.isBeforeOrEqual(range.replacing.end)) {
			invalidInsertReplaceRangeCount += 1;
		}
		if (firstRangeKinds.length < 3) {
			firstRangeKinds.push('insertReplace');
			firstFilterTextLengths.push(String(item.filterText?.length ?? 0));
		}
	}

	return {
		plainRangeCount,
		insertReplaceRangeCount,
		invalidInsertReplaceRangeCount,
		firstRangeKinds: firstRangeKinds.join(','),
		firstFilterTextLengths: firstFilterTextLengths.join(','),
	};
}

export interface CompletionMiddlewareCallbacks {
	begin(document: vscode.TextDocument, position: vscode.Position, triggerKind: number): { transactionId?: number };
	respond(document: vscode.TextDocument, triggerKind: number, requestVersion: number, transactionId: number | undefined, result: CompletionResult, elapsedMs: number): void;
	fail(document: vscode.TextDocument, triggerKind: number, requestVersion: number, transactionId: number | undefined, elapsedMs: number): void;
}

/** The editor-facing completion transaction boundary. Rust remains completion authority. */
export function createCompletionMiddleware(
	callbacks: CompletionMiddlewareCallbacks,
): Pick<NonNullable<LanguageClientOptions['middleware']>, 'provideCompletionItem'> {
	return {
		provideCompletionItem: async (document, position, completionContext, token, next) => {
			const requestVersion = document.version;
			const startedAt = Date.now();
			const { transactionId } = callbacks.begin(document, position, completionContext.triggerKind);
			try {
				const result = await next(document, position, completionContext, token);
				callbacks.respond(
					document,
					completionContext.triggerKind,
					requestVersion,
					transactionId,
					result,
					Date.now() - startedAt,
				);
				return result;
			} catch (error) {
				callbacks.fail(document, completionContext.triggerKind, requestVersion, transactionId, Date.now() - startedAt);
				throw error;
			}
		},
	};
}
