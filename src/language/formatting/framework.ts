import type { ParsedEnforceSource } from '../parser/ast';

export type EnforceFormatFeature =
	| 'semicolons'
	| 'autoBrackets'
	| 'classInheritanceColon'
	| 'comments'
	| 'equalSigns';

export type EnforceFormatActivation = 'disabled' | 'manual' | 'formatOnSave' | 'type';

export interface EnforceFormatFeatureSpec {
	id: EnforceFormatFeature;
	setting: string;
	activation: EnforceFormatActivation;
	description: string;
	parserFactsRequired: readonly string[];
	gameStyleNotes: readonly string[];
	movesCode: boolean;
}

export interface EnforceFormatOptions {
	enabledFeatures?: readonly EnforceFormatFeature[];
	activation?: EnforceFormatActivation;
}

export interface EnforceFormatTextEdit {
	range: {
		start: { line: number; character: number };
		end: { line: number; character: number };
	};
	newText: string;
	feature: EnforceFormatFeature;
}

export interface EnforceFormatPlan {
	edits: EnforceFormatTextEdit[];
	skippedFeatures: EnforceFormatFeatureSpec[];
}

export const enforceFormatFeatureCatalog: readonly EnforceFormatFeatureSpec[] = [
	{
		id: 'semicolons',
		setting: 'reforgerScriptTools.formatting.semicolons.enabled',
		activation: 'manual',
		description: 'Repair declaration terminators only where parser declarations prove a missing or duplicate semicolon.',
		parserFactsRequired: ['complete declaration node', 'declaration terminator token', 'ignored token ranges'],
		gameStyleNotes: ['Properties and proto declarations end with semicolons.', 'Class bodies may appear with or without a trailing semicolon in samples.'],
		movesCode: false,
	},
	{
		id: 'autoBrackets',
		setting: 'reforgerScriptTools.formatting.autoBrackets.enabled',
		activation: 'type',
		description: 'When an opening bracket or block-header newline is typed in a safe code position, insert or enter the matching bracket/body shape and keep the cursor inside.',
		parserFactsRequired: ['current line ignored-token guard', 'typed bracket/newline character', 'next character safety check', 'block header shape'],
		gameStyleNotes: ['Enforce uses parentheses for calls/control headers, square brackets for attributes and indexing, and braces for class/function/control/value initializer bodies.', 'Game code commonly places class, function, and control braces on their own lines.', 'Existing empty bracket bodies must be entered rather than duplicated.', 'Generic angle brackets are intentionally excluded from typed pairing because < and > are also comparison and shift operators.'],
		movesCode: false,
	},
	{
		id: 'classInheritanceColon',
		setting: 'reforgerScriptTools.formatting.classInheritanceColon.enabled',
		activation: 'type',
		description: 'When a space completes a new class declaration name, insert the inheritance colon and trigger base-class completion.',
		parserFactsRequired: ['current line lexical class-declaration prefix', 'ignored token guard'],
		gameStyleNotes: ['Enforce supports class Child : Parent and class Child extends Parent.', 'Raw game code contains both spaced and compact colon inheritance declarations.', 'Modded class declarations can target existing classes and should not be forced into inheritance syntax.'],
		movesCode: false,
	},
	{
		id: 'comments',
		setting: 'reforgerScriptTools.formatting.comments.enabled',
		activation: 'type',
		description: 'When a standalone block-comment opener is typed or entered, insert the closing comment shape and keep the cursor in the body.',
		parserFactsRequired: ['current line ignored-token guard', 'typed comment opener/newline character'],
		gameStyleNotes: ['Samples use //! member docs, // divider lines, and /*! ... */ docs with \\code blocks.', 'Typed assists must not rewrite existing documentation comments.'],
		movesCode: false,
	},
	{
		id: 'equalSigns',
		setting: 'reforgerScriptTools.formatting.equalSigns.enabled',
		activation: 'manual',
		description: 'Insert assignment spacing for parser-shaped declaration initializers and model-proven visible assignment targets.',
		parserFactsRequired: ['operator tokens', 'declaration initializer ranges', 'parameter default ranges', 'visible local/property target facts'],
		gameStyleNotes: ['Docs show value declarations as type identifier = value.', 'Alignment columns are not stable enough for automatic formatting.'],
		movesCode: false,
	},
];

export function createEnforceFormattingPlan(parsed: ParsedEnforceSource, options: EnforceFormatOptions = {}): EnforceFormatPlan {
	const enabled = new Set(options.enabledFeatures ?? []);
	const activation = options.activation ?? 'manual';
	const skippedFeatures = enforceFormatFeatureCatalog.filter(feature =>
		!enabled.has(feature.id)
		|| feature.activation === 'disabled'
		|| !activationAllowsFeature(activation, feature.activation)
		|| !parsedHasSafeFormattingShape(parsed)
	);

	return {
		edits: [],
		skippedFeatures,
	};
}

function activationAllowsFeature(requested: EnforceFormatActivation, feature: EnforceFormatActivation): boolean {
	if (feature === 'manual') {
		return requested === 'manual' || requested === 'formatOnSave';
	}
	return requested === feature;
}

function parsedHasSafeFormattingShape(parsed: ParsedEnforceSource): boolean {
	return parsed.diagnostics.every(diagnostic => diagnostic.severity !== 'error');
}
