import * as vscode from 'vscode';
import { applyVersionedEditorEdits, isCurrentSingleCaret, type VersionedEditResponse } from './versionedEditorEdit';

export function isCurrentSingleVersionedEditorCaret(
	documentVersion: number,
	expectedVersion: number,
	selectionCount: number,
	selectionIsEmpty: boolean,
	selectionActive: vscode.Position,
	expectedPosition: vscode.Position,
): boolean {
	return documentVersion === expectedVersion
		&& selectionCount === 1
		&& selectionIsEmpty
		&& selectionActive.isEqual(expectedPosition);
}

export class VersionedEditorTransaction<Response extends VersionedEditResponse> {
	public response: Response | undefined;
	public caretReady: boolean;
	private terminal = false;

	public constructor(
		public readonly document: vscode.TextDocument,
		public readonly version: number,
		public readonly expectedPosition: vscode.Position,
		public readonly preTriggerPosition: vscode.Position,
	) {
		this.caretReady = isCurrentSingleCaret(document, version, expectedPosition);
	}

	public isCurrent(): boolean {
		return isCurrentSingleCaret(this.document, this.version, this.expectedPosition);
	}

	public isAtPreTriggerCaret(): boolean {
		return isCurrentSingleCaret(this.document, this.version, this.preTriggerPosition);
	}

	public accept(response: Response): boolean {
		if (this.terminal || this.response || this.document.version !== this.version || response.edits.length === 0) {
			return false;
		}
		this.response = response;
		return true;
	}

	public reject(): boolean {
		if (this.terminal) {
			return false;
		}
		this.terminal = true;
		return true;
	}

	public observeSelection(): 'ready' | 'moved' | 'pending' {
		if (this.terminal) {
			return 'moved';
		}
		if (this.document.version !== this.version) {
			return 'moved';
		}
		if (this.isCurrent()) {
			this.caretReady = true;
			return 'ready';
		}
		return this.isAtPreTriggerCaret() ? 'pending' : 'moved';
	}

	public async apply(): Promise<'pending' | 'stale' | 'applied' | 'editRejected'> {
		if (this.terminal || !this.response || !this.caretReady) {
			return 'pending';
		}
		this.terminal = true;
		if (!this.isCurrent()) {
			return 'stale';
		}
		const editor = vscode.window.activeTextEditor;
		if (!editor) {
			return 'stale';
		}
		return (await applyVersionedEditorEdits(editor, this.response)) ? 'applied' : 'editRejected';
	}
}
