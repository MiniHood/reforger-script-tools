import * as vscode from 'vscode';
import { diagnostic } from '../diagnostics/diagnostics';
import type { WorkbenchNetApiFailureDiagnosis } from './gateway/workbenchGateway';

type WorkbenchFailureListener = (
	diagnosis: WorkbenchNetApiFailureDiagnosis | undefined,
) => void;

let bridgeInactiveNotificationShown = false;
let currentDiagnosis: WorkbenchNetApiFailureDiagnosis | undefined;
const listeners = new Set<WorkbenchFailureListener>();

export function onDidChangeWorkbenchFailure(
	listener: WorkbenchFailureListener,
): vscode.Disposable {
	listeners.add(listener);
	listener(currentDiagnosis);
	return new vscode.Disposable(() => listeners.delete(listener));
}

function setCurrentDiagnosis(
	diagnosis: WorkbenchNetApiFailureDiagnosis | undefined,
): void {
	if (currentDiagnosis === diagnosis) {
		return;
	}
	currentDiagnosis = diagnosis;
	for (const listener of listeners) {
		listener(diagnosis);
	}
}

export function updateWorkbenchFailureNotification(
	diagnosis: WorkbenchNetApiFailureDiagnosis | undefined,
): void {
	if (diagnosis === 'bridge-inactive') {
		setCurrentDiagnosis(diagnosis);
		if (!bridgeInactiveNotificationShown) {
			bridgeInactiveNotificationShown = true;
			diagnostic('workbenchNetApiFailureNotificationRequested', { diagnosis });
			void vscode.window.showErrorMessage(
				'Workbench NET API bridge inactive. Fix script compilation errors.',
			)
				.then(
					selection => diagnostic('workbenchNetApiFailureNotificationResolved', {
						diagnosis,
						selected: selection !== undefined,
					}),
					() => diagnostic('workbenchNetApiFailureNotificationRejected', { diagnosis }),
				);
		} else {
			diagnostic('workbenchNetApiFailureNotificationSuppressed', { diagnosis });
		}
		return;
	}
	bridgeInactiveNotificationShown = false;
	setCurrentDiagnosis(undefined);
}

export function resetWorkbenchFailureNotification(): void {
	bridgeInactiveNotificationShown = false;
	setCurrentDiagnosis(undefined);
}
