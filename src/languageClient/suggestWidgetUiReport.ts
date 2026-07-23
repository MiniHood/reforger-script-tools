import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const uiAutomationTimeoutMs = 5_000;

interface UiAutomationBounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

interface UiAutomationList {
	name: string;
	automationId: string;
	className: string;
	isOffscreen: boolean;
	hasKeyboardFocus: boolean;
	bounds: UiAutomationBounds;
	verticalScrollPercent: number | null;
	items: UiAutomationItem[];
}

interface UiAutomationItem {
	name: string;
	bounds: UiAutomationBounds;
	isSelected: boolean;
}

interface UiAutomationPayload {
	status: 'ok' | 'no-code-window' | 'no-suggest-widget' | 'ambiguous-suggest-widget';
	focusedElement: string;
	lists: UiAutomationList[];
}

const uiAutomationScript = `
Add-Type -AssemblyName UIAutomationClient

function Convert-Bounds($rectangle) {
	@{ x = $rectangle.X; y = $rectangle.Y; width = $rectangle.Width; height = $rectangle.Height }
}

function Get-CodeWindow($element) {
	$current = $element
	while ($null -ne $current) {
		try {
			$process = Get-Process -Id $current.Current.ProcessId -ErrorAction Stop
			if ($process.ProcessName -in @('Code', 'Code - Insiders', 'VSCodium')) { return $current }
		} catch {}
		$current = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($current)
	}
	return $null
}

$focused = [System.Windows.Automation.AutomationElement]::FocusedElement
$window = Get-CodeWindow $focused
if ($null -eq $window) {
	@{ status = 'no-code-window'; focusedElement = if ($null -eq $focused) { '' } else { $focused.Current.Name }; lists = @() } | ConvertTo-Json -Compress
	exit 0
}

$suggestWidgetCondition = New-Object System.Windows.Automation.PropertyCondition(
	[System.Windows.Automation.AutomationElement]::AutomationIdProperty,
	'editor.widget.suggestWidget'
)
$listCondition = New-Object System.Windows.Automation.PropertyCondition(
	[System.Windows.Automation.AutomationElement]::ControlTypeProperty,
	[System.Windows.Automation.ControlType]::List
)
$itemCondition = New-Object System.Windows.Automation.PropertyCondition(
	[System.Windows.Automation.AutomationElement]::ControlTypeProperty,
	[System.Windows.Automation.ControlType]::ListItem
)
$suggestWidgets = @($window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $suggestWidgetCondition) | Where-Object { !$_.Current.IsOffscreen })
if ($suggestWidgets.Count -eq 0) {
	@{ status = 'no-suggest-widget'; focusedElement = $focused.Current.Name; lists = @() } | ConvertTo-Json -Compress
	exit 0
}
if ($suggestWidgets.Count -ne 1) {
	@{ status = 'ambiguous-suggest-widget'; focusedElement = $focused.Current.Name; lists = @() } | ConvertTo-Json -Compress
	exit 0
}

$lists = @()
foreach ($list in $suggestWidgets[0].FindAll([System.Windows.Automation.TreeScope]::Descendants, $listCondition)) {
	if ($list.Current.IsOffscreen) { continue }
	$items = @()
	foreach ($item in $list.FindAll([System.Windows.Automation.TreeScope]::Children, $itemCondition)) {
		if ($item.Current.IsOffscreen -or !$item.Current.Name) { continue }
		$selected = $false
		try {
			$selected = ([System.Windows.Automation.SelectionItemPattern]$item.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)).Current.IsSelected
		} catch {}
		$items += @{ name = $item.Current.Name; bounds = Convert-Bounds $item.Current.BoundingRectangle; isSelected = $selected }
	}
	if ($items.Count -eq 0) { continue }
	$scrollPercent = $null
	try {
		$scrollPercent = ([System.Windows.Automation.ScrollPattern]$list.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)).Current.VerticalScrollPercent
	} catch {}
	$lists += @{
		name = $list.Current.Name
		automationId = $list.Current.AutomationId
		className = $list.Current.ClassName
		isOffscreen = $list.Current.IsOffscreen
		hasKeyboardFocus = $list.Current.HasKeyboardFocus
		bounds = Convert-Bounds $list.Current.BoundingRectangle
		verticalScrollPercent = $scrollPercent
		items = @($items)
	}
}
@{ status = 'ok'; focusedElement = $focused.Current.Name; lists = @($lists) } | ConvertTo-Json -Depth 6 -Compress
`;

/**
 * Reads the suggestion lists which VS Code has actually rendered through the
 * Windows accessibility tree. The extension API exposes only the completion
 * payload, so this is deliberately a diagnostic boundary rather than a
 * completion implementation dependency.
 */
export async function renderedSuggestWidgetReport(): Promise<string> {
	if (process.platform !== 'win32') {
		return unavailableReport('Windows UI Automation is only available on Windows.');
	}

	try {
		const { stdout } = await execFileAsync('powershell.exe', [
			'-NoLogo', '-NoProfile', '-NonInteractive', '-Command', uiAutomationScript,
		], { timeout: uiAutomationTimeoutMs, windowsHide: true, maxBuffer: 1_000_000 });
		return formatUiAutomationPayload(JSON.parse(stdout) as UiAutomationPayload);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		return unavailableReport(`Windows UI Automation query failed: ${message}`);
	}
}

export function formatUiAutomationPayload(payload: UiAutomationPayload): string {
	const lines = [
		'## Rendered Suggest Widget (Windows UI Automation)',
		'',
		'This section reads accessibility-visible rows after VS Code has filtered, ranked, and rendered them. It is captured before the server debug request.',
		'',
	];
	if (payload.status === 'no-code-window') {
		lines.push('No focused VS Code window was found. Run Ctrl+F2 while the suggestion widget is still open.');
		return lines.join('\n');
	}
	lines.push(`- Focused accessibility element: ${inline(payload.focusedElement) || '<unnamed>'}`);
	if (payload.status === 'no-suggest-widget') {
		lines.push('VS Code did not expose the live `editor.widget.suggestWidget` accessibility node. No rendered rows are reported rather than guessing from another list.');
		return lines.join('\n');
	}
	if (payload.status === 'ambiguous-suggest-widget') {
		lines.push('More than one live `editor.widget.suggestWidget` accessibility node was found. No rendered rows are reported rather than choosing arbitrarily.');
		return lines.join('\n');
	}
	if (payload.lists.length !== 1) {
		lines.push(`The identified suggest widget contains ${payload.lists.length} visible lists. No rendered rows are reported rather than choosing arbitrarily.`);
		return lines.join('\n');
	}
	const [list] = payload.lists;
	if (list.items.length === 0) {
		lines.push('- No visible accessibility list with rendered rows was found. The suggestion widget may have closed before the report ran.');
		return lines.join('\n');
	}
	const visibleItems = [...list.items].sort(compareByScreenPosition);
	lines.push(
		'',
		'### Rendered suggestion list',
		'',
		`- Name: ${inline(list.name) || '<unnamed>'}`,
		`- Automation ID: ${inline(list.automationId) || '<none>'}`,
		`- Class: ${inline(list.className) || '<none>'}`,
		`- Focused: ${list.hasKeyboardFocus}`,
		`- Bounds: ${list.bounds.x},${list.bounds.y} ${list.bounds.width}x${list.bounds.height}`,
		`- Vertical scroll: ${list.verticalScrollPercent === null ? '<unavailable>' : `${list.verticalScrollPercent}%`}`,
		'- Scope: currently visible accessibility rows only; virtualized/offscreen rows are intentionally excluded.',
		'',
		'| Visual order | Selected | Accessible row text |',
		'| ---: | :---: | --- |',
	);
	for (const [itemIndex, item] of visibleItems.entries()) {
		lines.push(`| ${itemIndex + 1} | ${item.isSelected ? 'yes' : ''} | ${escapeCell(item.name)} |`);
	}
	return lines.join('\n');
}

function compareByScreenPosition(left: UiAutomationItem, right: UiAutomationItem): number {
	return left.bounds.y - right.bounds.y || left.bounds.x - right.bounds.x;
}

function unavailableReport(reason: string): string {
	return [
		'## Rendered Suggest Widget (Windows UI Automation)',
		'',
		`Unavailable: ${reason}`,
	].join('\n');
}

function escapeCell(value: string): string {
	return value.replaceAll('|', '\\|').replaceAll('\n', '<br>');
}

function inline(value: string): string {
	return value.replaceAll('`', '\\`');
}
