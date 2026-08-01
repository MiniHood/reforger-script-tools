import * as vscode from 'vscode';
import type { WorkbenchNetApiFailureDiagnosis } from './gateway/workbenchGateway';

let scriptsFailureNotificationShown = false;

export function updateWorkbenchFailureNotification(
	diagnosis: WorkbenchNetApiFailureDiagnosis | undefined,
): void {
	if (diagnosis === 'scripts-failing') {
		if (!scriptsFailureNotificationShown) {
			scriptsFailureNotificationShown = true;
			void vscode.window.showWarningMessage('Workbench scripts are failing.');
		}
		return;
	}
	scriptsFailureNotificationShown = false;
}

export function resetWorkbenchFailureNotification(): void {
	scriptsFailureNotificationShown = false;
}
