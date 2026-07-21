import * as vscode from 'vscode';
import type { LanguageClientOptions } from 'vscode-languageclient/node';

type CompletionResult = vscode.CompletionList | readonly vscode.CompletionItem[] | null | undefined;

export interface CompletionMiddlewareCallbacks {
	begin(document: vscode.TextDocument, triggerKind: number): { transactionId?: number };
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
			const { transactionId } = callbacks.begin(document, completionContext.triggerKind);
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
