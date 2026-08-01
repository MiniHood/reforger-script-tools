import * as vscode from 'vscode';
import { diagnostic } from '../diagnostics/diagnostics';
import type { WorkbenchNetApiFailureDiagnosis } from './gateway/workbenchGateway';

let scriptsFailureNotificationShown = false;

export function updateWorkbenchFailureNotification(
	diagnosis: WorkbenchNetApiFailureDiagnosis | undefined,
): void {
	if (diagnosis === 'scripts-failing') {
		if (!scriptsFailureNotificationShown) {
			scriptsFailureNotificationShown = true;
			diagnostic('workbenchNetApiFailureNotificationRequested', { diagnosis });
			void vscode.window.showWarningMessage('Workbench scripts are failing.')
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
	scriptsFailureNotificationShown = false;
}

export function resetWorkbenchFailureNotification(): void {
	scriptsFailureNotificationShown = false;
}
