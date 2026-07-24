import * as vscode from 'vscode';

const semanticTokenMarker = /<span data-semantic-token="([A-Za-z][A-Za-z0-9]*)">/g;
const vscodeColor = /^#[0-9a-fA-F]{3,4}(?:[0-9a-fA-F]{3,4})?$/;
const hoverSemanticRoles = [
	'class',
	'enum',
	'type',
	'typeParameter',
	'function',
	'reforgerField',
	'variable',
	'parameter',
	'enumMember',
	'keyword',
	'comment',
	'string',
	'number',
	'operator',
	'reforgerPunctuation',
	'reforgerPreprocessor',
] as const;

export type HoverSemanticForegrounds = Readonly<Record<string, string>>;

export interface HoverSemanticPalette {
	enabled: boolean;
	activeTheme: string;
	foregrounds: HoverSemanticForegrounds;
}

/** Applies resolved foregrounds to Rust-authored semantic-role markers. */
export function applyHoverSemanticPalette(
	markdown: string,
	foregrounds: HoverSemanticForegrounds,
	enabled = true,
): string {
	return markdown.replace(semanticTokenMarker, (_marker, tokenType: string) => {
		const foreground = enabled ? foregrounds[tokenType] : undefined;
		return foreground
			? `<span style="color:${foreground};">`
			: '<span>';
	});
}

/** Resolves the exact language and active-theme rules used by hover markup. */
export function hoverSemanticForegrounds(
	customizations: unknown,
	activeTheme: string,
	languageId: string,
): Record<string, string> {
	const root = recordValue(customizations);
	const baseRules = recordValue(root?.rules);
	const themeRules = recordValue(recordValue(root?.[`[${activeTheme}]`])?.rules);
	const rules = { ...baseRules, ...themeRules };
	const foregrounds: Record<string, string> = {};

	for (const role of hoverSemanticRoles) {
		const unqualified = semanticForeground(rules[role]);
		const languageQualified = semanticForeground(rules[`${role}:${languageId}`]);
		const foreground = languageQualified ?? unqualified;
		if (foreground) {
			foregrounds[role] = foreground;
		}
	}

	return foregrounds;
}

export function hoverSemanticPaletteForDocument(
	document: vscode.TextDocument,
): HoverSemanticPalette {
	const editor = vscode.workspace.getConfiguration('editor', document);
	const activeTheme = vscode.workspace
		.getConfiguration('workbench', document)
		.get<string>('colorTheme', '');
	const semanticHighlighting = editor.get<boolean | string>(
		'semanticHighlighting.enabled',
		true,
	);
	const customizations = editor.get<unknown>('semanticTokenColorCustomizations');

	return {
		enabled: semanticHighlighting !== false,
		activeTheme,
		foregrounds: hoverSemanticForegrounds(
			customizations,
			activeTheme,
			document.languageId,
		),
	};
}

export function hoverSemanticPaletteReport(document: vscode.TextDocument): string {
	const palette = hoverSemanticPaletteForDocument(document);
	const lines = [
		'## VS Code Hover Presentation',
		'',
		`- Active color theme: \`${escapeMarkdownCode(palette.activeTheme || '<unnamed>')}\``,
		`- Semantic highlighting: \`${palette.enabled ? 'enabled' : 'disabled'}\``,
		'- Palette source: effective `editor.semanticTokenColorCustomizations` for this Enforce document',
		'- Link behavior: semantic-role foregrounds are applied inside command links, overriding the normal hover-link blue',
		'',
		'| Hover role | Selector | Resolved foreground |',
		'| --- | --- | --- |',
	];
	for (const role of hoverSemanticRoles) {
		lines.push(
			`| \`${role}\` | \`${role}:${document.languageId}\` | \`${palette.foregrounds[role] ?? '<theme-owned>'}\` |`,
		);
	}
	return lines.join('\n');
}

function semanticForeground(value: unknown): string | undefined {
	if (typeof value === 'string') {
		return vscodeColor.test(value) ? value : undefined;
	}
	const foreground = recordValue(value)?.foreground;
	return typeof foreground === 'string' && vscodeColor.test(foreground)
		? foreground
		: undefined;
}

function recordValue(value: unknown): Record<string, unknown> | undefined {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
		? value as Record<string, unknown>
		: undefined;
}

function escapeMarkdownCode(value: string): string {
	return value.replaceAll('`', '\\`');
}
