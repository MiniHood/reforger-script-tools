import * as vscode from 'vscode';
import { diagnostic, diagnosticsEnabled } from '../diagnostics/diagnostics';
import { languageClientLanguage } from '../extensionConfig/languageClient';

interface EditTiming {
	version: number;
	observedAt: number;
	eventLoopTurnMs: number | undefined;
}

export interface SemanticTokenRequestTiming {
	request: number;
	version: number;
	editObserved: boolean;
	editAgeMs: number | undefined;
	eventLoopTurnMs: number | undefined;
	startedAt: number;
}

export interface SemanticTokenTimingBridge extends vscode.Disposable {
	start(document: vscode.TextDocument): SemanticTokenRequestTiming;
	complete(
		timing: SemanticTokenRequestTiming,
		status: 'ok' | 'error',
		cancelled: boolean,
	): void;
}

export function registerSemanticTokenTimingBridge(): SemanticTokenTimingBridge {
	const edits = new Map<string, EditTiming>();
	let nextRequest = 1;
	const documentChanges = diagnosticsEnabled()
		? vscode.workspace.onDidChangeTextDocument(event => {
			if (event.document.languageId !== languageClientLanguage.id) {
				return;
			}
			const key = event.document.uri.toString();
			const timing: EditTiming = {
				version: event.document.version,
				observedAt: Date.now(),
				eventLoopTurnMs: undefined,
			};
			edits.set(key, timing);
			setTimeout(() => {
				if (edits.get(key) === timing) {
					timing.eventLoopTurnMs = Date.now() - timing.observedAt;
				}
			}, 0);
		})
		: undefined;

	return {
		start(document) {
			const startedAt = Date.now();
			const edit = edits.get(document.uri.toString());
			const matchingEdit = edit?.version === document.version ? edit : undefined;
			const timing: SemanticTokenRequestTiming = {
				request: nextRequest++,
				version: document.version,
				editObserved: Boolean(matchingEdit),
				editAgeMs: matchingEdit ? startedAt - matchingEdit.observedAt : undefined,
				eventLoopTurnMs: matchingEdit?.eventLoopTurnMs,
				startedAt,
			};
			diagnostic('semanticTokens.middleware.start', {
				request: timing.request,
				version: timing.version,
				editObserved: timing.editObserved,
				editAgeMs: timing.editAgeMs,
				eventLoopTurnMs: timing.eventLoopTurnMs,
				lineCount: document.lineCount,
			});
			return timing;
		},
		complete(timing, status, cancelled) {
			diagnostic('semanticTokens.middleware.complete', {
				request: timing.request,
				version: timing.version,
				status,
				cancelled,
				elapsedMs: Date.now() - timing.startedAt,
			});
		},
		dispose() {
			documentChanges?.dispose();
			edits.clear();
		},
	};
}
