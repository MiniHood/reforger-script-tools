import * as vscode from 'vscode';
import { ExtensionLogger } from '../../core/logger';
import { BasicCompletionProvider } from '../completion/provider';
import { formatCompletionTrace } from './completionTrace';
import { EnforcePrefixSearchDebug, EnforceSymbolIndex } from '../index/symbolIndex';
import { buildCodeIntelligenceModel } from '../model/codeIntelligence';
import { buildLanguageModel } from '../model/languageModel';
import { formatModelInspection, ModelDefinitionProvider, ModelHoverProvider } from '../providers/modelProviders';
import type { EnforceParserPosition, EnforceParserRange } from '../parser/ast';

export function registerCompletionDebugTools(context: vscode.ExtensionContext, logger: ExtensionLogger, symbolIndex: EnforceSymbolIndex): void {
	context.subscriptions.push(
		vscode.commands.registerCommand('reforger-script-tools.captureCompletions', async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== 'enforce') {
				vscode.window.showWarningMessage('Open an Enforce script file first.');
				return;
			}

			const position = editor.selection.active;
			const linePrefix = editor.document.lineAt(position.line).text.slice(0, position.character);
			const currentPrefix = getPrefix(linePrefix);
			const stats = symbolIndex.getStats();
			const debugLogDirUri = vscode.Uri.joinPath(context.globalStorageUri, 'logs');
			const debugLogUri = vscode.Uri.joinPath(debugLogDirUri, 'ac-debug.log');
			const direct = new BasicCompletionProvider(symbolIndex).provideCompletionItems(
				editor.document,
				position,
				new vscode.CancellationTokenSource().token
			);
			const vscodeList = await vscode.commands.executeCommand<vscode.CompletionList>(
				'vscode.executeCompletionItemProvider',
				editor.document.uri,
				position
			);

			const report = [
				'# Reforger AC Debug Snapshot',
				`timestamp=${new Date().toISOString()}`,
				`command=reforger-script-tools.captureCompletions`,
				`document=${editor.document.uri.toString()}`,
				`path=${editor.document.uri.fsPath}`,
				`language=${editor.document.languageId}`,
				`version=${editor.document.version}`,
				`position=${position.line + 1}:${position.character + 1}`,
				`selection=${formatSingleRange(editor.selection)}`,
				`indexStats=files:${stats.files} symbols:${stats.symbols} classes:${stats.classes} enums:${stats.enums} functions:${stats.functions} properties:${stats.properties}`,
				`indexState=${JSON.stringify(typeof symbolIndex.getState === 'function' ? symbolIndex.getState() : { refreshing: symbolIndex.isRefreshing() })}`,
				`line=${JSON.stringify(editor.document.lineAt(position.line).text)}`,
				`linePrefix=${JSON.stringify(linePrefix)}`,
				`lineSuffix=${JSON.stringify(editor.document.lineAt(position.line).text.slice(position.character))}`,
				'',
				'## Nearby Text',
				formatNearbyText(editor.document, position),
				'',
				'## Indexed Prefix Search Debug',
				formatPrefixSearchDebug('classes', symbolIndex.debugFindClassesByPrefix(currentPrefix, 30)),
				'',
				formatPrefixSearchDebug('types', symbolIndex.debugFindTypesByPrefix(currentPrefix, 30)),
				'',
				formatPrefixSearchDebug('functions', symbolIndex.debugFindFunctionsByPrefix(currentPrefix, 30)),
				'',
				'## Model Completion Context',
				formatModelCompletionContext(editor.document, position, symbolIndex),
				'',
				'## Navigation And Hover Debug',
				await formatNavigationDebug(editor.document, position, symbolIndex),
				'',
				formatCompletionList('direct BasicCompletionProvider', direct),
				'',
				formatCompletionList('VS Code executeCompletionItemProvider', vscodeList),
				'',
				'## Live AC Trace',
				formatCompletionTrace(),
				'',
				'## Suggest Widget Visibility',
				'liveSuggestWidgetItems=unavailable (VS Code extension API does not expose the currently open suggest-widget item list)',
				'detailsPane=unavailable (VS Code extension API does not expose suggest details pane visibility; use VS Code suggest-widget controls if details are hidden)',
				`currentPrefix=${JSON.stringify(currentPrefix)}`,
				formatCurrentPrefixVisibleList('direct provider current-prefix visible approximation', direct, currentPrefix),
				'',
				formatCurrentPrefixVisibleList('VS Code current-prefix visible approximation', vscodeList, currentPrefix),
			].join('\n');

			await vscode.workspace.fs.createDirectory(debugLogDirUri);
			await vscode.workspace.fs.writeFile(debugLogUri, Buffer.from(report, 'utf8'));
			const debugDocument = await vscode.workspace.openTextDocument(debugLogUri);
			await vscode.window.showTextDocument(debugDocument, { preview: false });
			logger.info(`AC debug snapshot written to ${debugLogUri.fsPath}`);
			vscode.window.showInformationMessage(`Captured AC debug: ${completionCount(vscodeList)} VS Code item(s). Opened ${debugLogUri.fsPath}`);
		})
	);
}

async function formatNavigationDebug(document: vscode.TextDocument, position: vscode.Position, symbolIndex: EnforceSymbolIndex): Promise<string> {
	const code = buildCodeIntelligenceModel(document, symbolIndex);
	const identity = code.resolveIdentityAt(position);
	const directDefinition = identity ? code.resolveDefinition(identity) : [];
	const directHover = identity ? code.formatHover(identity) : [];
	const providerHover = await new ModelHoverProvider(symbolIndex).provideHover(document, position);
	const providerDefinition = await new ModelDefinitionProvider(symbolIndex).provideDefinition(document, position);
	const vscodeHover = await vscode.commands.executeCommand<vscode.Hover[]>(
		'vscode.executeHoverProvider',
		document.uri,
		position
	);
	const vscodeDefinition = await vscode.commands.executeCommand<Array<vscode.Location | vscode.LocationLink>>(
		'vscode.executeDefinitionProvider',
		document.uri,
		position
	);
	return [
		`selectorEligible=${document.languageId === 'enforce' && document.uri.scheme === 'file'}`,
		`language=${document.languageId}`,
		`scheme=${document.uri.scheme}`,
		'',
		'### Model Inspect',
		formatModelInspection(document, position, symbolIndex),
		'',
		'### Nearby Parser Relations',
		formatNearbyParserRelations(document, position, symbolIndex),
		'',
		'### Nearby Identifier Identity Matrix',
		formatNearbyIdentifierIdentityMatrix(document, position, symbolIndex),
		'',
		'### Direct Identity',
		formatIdentity(identity),
		'',
		formatLocations('direct definition', directDefinition),
		'',
		formatMarkdownList('direct hover', directHover),
		'',
		formatHover('provider hover', providerHover),
		'',
		formatProviderDefinition('provider definition', providerDefinition),
		'',
		formatHoverList('VS Code executeHoverProvider', vscodeHover),
		'',
		formatDefinitionList('VS Code executeDefinitionProvider', vscodeDefinition),
	].join('\n');
}

function formatNearbyIdentifierIdentityMatrix(document: vscode.TextDocument, position: vscode.Position, symbolIndex: EnforceSymbolIndex): string {
	const code = buildCodeIntelligenceModel(document, symbolIndex);
	const model = buildLanguageModel(document, symbolIndex);
	const rows = model.parsed.tokens
		.filter(token => token.kind === 'identifier' && Math.abs(token.line - position.line) <= 6)
		.map(token => {
			const tokenPosition = new vscode.Position(token.line, token.character);
			const identity = code.resolveIdentityAt(tokenPosition);
			const definitions = identity ? code.resolveDefinition(identity) : [];
			const localAtName = model.visibleLocals(tokenPosition).find(local => local.name === token.text);
			const symbols = symbolIndex.find(token.text).slice(0, 8);
			return [
				`identifier=${JSON.stringify(token.text)} token=${formatParserRange({ start: { line: token.line, character: token.character }, end: { line: token.endLine, character: token.endCharacter } })}`,
				`  identity=${identity ? `${identity.kind}:${identity.name}:confidence=${identity.confidence}:range=${formatLanguageRange(identity.range)}` : 'undefined'}`,
				`  definitions=${definitions.length}${definitions.length ? ` ${definitions.map(formatLocation).join(' | ')}` : ''}`,
				`  visibleLocalSameName=${localAtName ? `${localAtName.kind}:${localAtName.valueType ?? 'var'}:${localAtName.name}:range=${formatParserRange(localAtName.range)}:selection=${localAtName.selectionRange ? formatParserRange(localAtName.selectionRange) : ''}` : ''}`,
				`  indexMatches=${symbols.length}${symbols.length ? ` ${symbols.map(symbol => `${symbol.type}:${symbol.name}:kind=${symbol.declarationKind ?? ''}:origin=${symbol.origin ?? ''}:${symbol.uri.fsPath || symbol.uri.toString()}:${symbol.selectionRange.start.line + 1}:${symbol.selectionRange.start.character + 1}`).join(' | ')}` : ''}`,
			].join('\n');
		});
	if (rows.length === 0) {
		return 'nearbyIdentifiers=0';
	}
	return [
		`nearbyIdentifiers=${rows.length}`,
		...rows.map((row, index) => `-- ${index + 1} --\n${row}`),
	].join('\n');
}

function formatNearbyParserRelations(document: vscode.TextDocument, position: vscode.Position, symbolIndex: EnforceSymbolIndex): string {
	const model = buildLanguageModel(document, symbolIndex);
	const parserPosition = { line: position.line, character: position.character };
	const containingNodes = model.parsed.nodes
		.filter(node => rangeContainsPosition(node.range, parserPosition))
		.sort((left, right) => rangeSize(left.range) - rangeSize(right.range))
		.slice(0, 20);
	const visibleLocals = model.visibleLocals(position);
	const nearbyTokens = model.parsed.tokens
		.filter(token => Math.abs(token.line - position.line) <= 3 && token.kind !== 'whitespace' && token.kind !== 'newline')
		.map(token => `${token.kind}:${JSON.stringify(token.text)}@${formatParserRange({ start: { line: token.line, character: token.character }, end: { line: token.endLine, character: token.endCharacter } })}`)
		.slice(0, 80);
	return [
		`parserPosition=${position.line + 1}:${position.character + 1}`,
		`containingNodes=${containingNodes.length}`,
		...containingNodes.map((node, index) => `  node ${index + 1}. ${node.kind} name=${JSON.stringify(node.name ?? '')} member=${JSON.stringify(node.memberName ?? '')} valueType=${JSON.stringify(node.valueType ?? '')} range=${formatParserRange(node.range)} selection=${node.selectionRange ? formatParserRange(node.selectionRange) : ''} complete=${node.complete ?? ''} confidence=${node.confidence ?? ''}`),
		`visibleLocals=${visibleLocals.length}`,
		...visibleLocals.map((local, index) => `  local ${index + 1}. ${local.kind} ${local.valueType ?? 'var'} ${local.name ?? ''} range=${formatParserRange(local.range)} selection=${local.selectionRange ? formatParserRange(local.selectionRange) : ''} depth=${local.depth ?? ''}`),
		`nearbyTokens=${nearbyTokens.length}`,
		...nearbyTokens.map((token, index) => `  token ${index + 1}. ${token}`),
	].join('\n');
}

function formatParserRange(range: EnforceParserRange): string {
	return `${range.start.line + 1}:${range.start.character + 1}-${range.end.line + 1}:${range.end.character + 1}`;
}

function rangeContainsPosition(range: EnforceParserRange, position: EnforceParserPosition): boolean {
	return compareParserPositions(range.start, position) <= 0 && compareParserPositions(position, range.end) <= 0;
}

function rangeSize(range: EnforceParserRange): number {
	return (range.end.line - range.start.line) * 100000 + (range.end.character - range.start.character);
}

function compareParserPositions(left: EnforceParserPosition, right: EnforceParserPosition): number {
	return left.line !== right.line ? left.line - right.line : left.character - right.character;
}

function formatIdentity(identity: ReturnType<ReturnType<typeof buildCodeIntelligenceModel>['resolveIdentityAt']>): string {
	if (!identity) {
		return 'identity=undefined';
	}
	return [
		`identity=${identity.kind}:${identity.name}`,
		`confidence=${identity.confidence}`,
		`range=${formatLanguageRange(identity.range)}`,
		`container=${identity.containerName ?? ''}`,
		`detail=${JSON.stringify(identity.detail ?? '')}`,
		`signature=${JSON.stringify(identity.signature ?? '')}`,
		`symbol=${identity.symbol ? `${identity.symbol.type}:${identity.symbol.name}:kind=${identity.symbol.declarationKind ?? ''}:origin=${identity.symbol.origin ?? ''}` : ''}`,
		`targetLocations=${identity.targetLocations.length}`,
		...identity.targetLocations.map((location, index) => `  ${index + 1}. ${formatLocation(location)}`),
	].join('\n');
}

function formatLanguageRange(range: { start: { line: number; character: number }; end: { line: number; character: number } }): string {
	return `${range.start.line + 1}:${range.start.character + 1}-${range.end.line + 1}:${range.end.character + 1}`;
}

function formatLocations(label: string, locations: readonly vscode.Location[]): string {
	if (locations.length === 0) {
		return `${label}=none`;
	}
	return [
		`${label}=${locations.length}`,
		...locations.map((location, index) => `  ${index + 1}. ${formatLocation(location)}`),
	].join('\n');
}

function formatLocation(location: vscode.Location): string {
	return `${location.uri.toString()} ${location.range.start.line + 1}:${location.range.start.character + 1}-${location.range.end.line + 1}:${location.range.end.character + 1}`;
}

function formatMarkdownList(label: string, values: readonly vscode.MarkdownString[]): string {
	return [
		`${label}=${values.length}`,
		...values.map((value, index) => `  ${index + 1}. ${JSON.stringify(value.value)}`),
	].join('\n');
}

function formatHover(label: string, hover: vscode.Hover | undefined): string {
	if (!hover) {
		return `${label}=undefined`;
	}
	return [
		`${label}=1`,
		...hover.contents.map((content, index) => `  ${index + 1}. ${JSON.stringify(typeof content === 'string' ? content : content.value)}`),
	].join('\n');
}

function formatHoverList(label: string, hovers: readonly vscode.Hover[] | undefined): string {
	if (!hovers) {
		return `${label}=undefined`;
	}
	return [
		`${label}=${hovers.length}`,
		...hovers.flatMap((hover, hoverIndex) => hover.contents.map((content, contentIndex) => `  ${hoverIndex + 1}.${contentIndex + 1}. ${JSON.stringify(typeof content === 'string' ? content : content.value)}`)),
	].join('\n');
}

function formatProviderDefinition(label: string, definition: vscode.Definition | undefined): string {
	if (!definition) {
		return `${label}=undefined`;
	}
	const locations = Array.isArray(definition) ? definition : [definition];
	return formatDefinitionList(label, locations);
}

function formatDefinitionList(label: string, definitions: readonly (vscode.Location | vscode.LocationLink)[] | undefined): string {
	if (!definitions) {
		return `${label}=undefined`;
	}
	if (definitions.length === 0) {
		return `${label}=none`;
	}
	return [
		`${label}=${definitions.length}`,
		...definitions.map((definition, index) => `  ${index + 1}. ${formatDefinition(definition)}`),
	].join('\n');
}

function formatDefinition(definition: vscode.Location | vscode.LocationLink): string {
	if (definition instanceof vscode.Location) {
		return formatLocation(definition);
	}
	return `${definition.targetUri.toString()} ${definition.targetRange.start.line + 1}:${definition.targetRange.start.character + 1}-${definition.targetRange.end.line + 1}:${definition.targetRange.end.character + 1}`;
}

function formatModelCompletionContext(document: vscode.TextDocument, position: vscode.Position, symbolIndex: EnforceSymbolIndex): string {
	const model = buildLanguageModel(document, symbolIndex);
	const context = model.contextAt(position);
	const currentClass = model.currentClass(position)?.name ?? '';
	const ancestors = currentClass ? model.classAncestorNames(currentClass, true) : [];
	const members = currentClass ? model.members('this', position) : [];
	const overridePrefix = context.kind === 'override' ? context.prefix : '';
	const inheritedOverrideMembers = members.filter(member =>
		member.containerName !== currentClass
		&& member.type === 'memberFunction'
		&& member.signature
		&& member.declarationKind !== 'constructor'
		&& member.declarationKind !== 'destructor'
		&& !member.modifiers?.includes('static')
	);
	const indexedPrefixMembers = typeof symbolIndex.getContainerMemberSymbols === 'function'
		? symbolIndex.getContainerMemberSymbols()
			.filter(member => member.type === 'memberFunction' && member.name.toLowerCase().startsWith(overridePrefix.toLowerCase()))
			.slice(0, 30)
		: [];
	return [
		`contextKind=${context.kind}`,
		`contextPrefix=${JSON.stringify(context.prefix)}`,
		`contextRange=${context.range.start.line + 1}:${context.range.start.character + 1}-${context.range.end.line + 1}:${context.range.end.character + 1}`,
		`currentClass=${JSON.stringify(currentClass)}`,
		`ancestors=${JSON.stringify(ancestors)}`,
		`membersThisCount=${members.length}`,
		`inheritedOverrideMembersCount=${inheritedOverrideMembers.length}`,
		...inheritedOverrideMembers.slice(0, 30).map((member, index) => `  inherited ${index + 1}. ${member.containerName}.${member.name} signature=${JSON.stringify(member.signature ?? '')} modifiers=${JSON.stringify(member.modifiers ?? [])}`),
		`indexedMemberPrefixFallbackCount=${indexedPrefixMembers.length}`,
		...indexedPrefixMembers.map((member, index) => `  indexed ${index + 1}. ${member.containerName}.${member.name} signature=${JSON.stringify(member.signature ?? '')} modifiers=${JSON.stringify(member.modifiers ?? [])}`),
	].join('\n');
}

function formatCompletionList(label: string, list: vscode.CompletionList | undefined): string {
	if (!list) {
		return `${label}: unavailable`;
	}
	const lines = [
		`${label}: total=${list.items.length} isIncomplete=${list.isIncomplete}`,
	];
	list.items.slice(0, 100).forEach((item, index) => {
		lines.push([
			`  ${index + 1}.`,
			`label=${JSON.stringify(formatLabel(item.label))}`,
			`kind=${formatKind(item.kind)}`,
			`detail=${JSON.stringify(item.detail ?? '')}`,
			`filterText=${JSON.stringify(item.filterText ?? '')}`,
			`insertText=${JSON.stringify(formatInsertText(item.insertText))}`,
			`sortText=${JSON.stringify(item.sortText ?? '')}`,
			`range=${formatRange(item.range)}`,
			`documentation=${JSON.stringify(formatDocumentation(item.documentation))}`,
			formatRankingData((item as vscode.CompletionItem & { data?: unknown }).data),
		].join(' '));
	});
	if (list.items.length > 100) {
		lines.push(`  ...and ${list.items.length - 100} more`);
	}
	return lines.join('\n');
}

function completionCount(list: vscode.CompletionList | undefined): number {
	return list?.items.length ?? 0;
}

function formatPrefixSearchDebug(label: string, debug: EnforcePrefixSearchDebug): string {
	return [
		`${label}: prefix=${JSON.stringify(debug.prefix)} normalized=${JSON.stringify(debug.normalizedPrefix)} limit=${debug.limit}`,
		`normalMatches=${debug.normalMatches} typoRecoveryRan=${debug.typoRecoveryRan} typoRecoveryReason=${JSON.stringify(debug.typoRecoveryReason)} typoMatches=${debug.typoMatches}`,
		formatDebugCandidates('results', debug.results),
		formatDebugCandidates('normalAccepted', debug.normalAccepted),
		formatDebugCandidates('typoAccepted', debug.typoAccepted),
		formatDebugCandidates('rejected', debug.rejected),
	].join('\n');
}

function formatDebugCandidates(label: string, candidates: readonly { name: string; key: string; score?: number; reason: string }[]): string {
	if (candidates.length === 0) {
		return `${label}: none`;
	}
	return [
		`${label}:`,
		...candidates.slice(0, 30).map((candidate, index) => `  ${index + 1}. name=${JSON.stringify(candidate.name)} key=${JSON.stringify(candidate.key)} score=${candidate.score ?? ''} reason=${JSON.stringify(candidate.reason)}`),
	].join('\n');
}

function formatCurrentPrefixVisibleList(label: string, list: vscode.CompletionList | undefined, prefix: string): string {
	if (!list) {
		return `${label}: unavailable`;
	}
	const visibleItems = list.items.filter(item => completionItemMatchesPrefix(item, prefix));
	const lines = [
		`${label}: total=${visibleItems.length} from=${list.items.length} prefix=${JSON.stringify(prefix)}`,
	];
	visibleItems.slice(0, 100).forEach((item, index) => {
		lines.push([
			`  ${index + 1}.`,
			`label=${JSON.stringify(formatLabel(item.label))}`,
			`filterText=${JSON.stringify(item.filterText ?? '')}`,
			`sortText=${JSON.stringify(item.sortText ?? '')}`,
			formatRankingData((item as vscode.CompletionItem & { data?: unknown }).data),
		].join(' '));
	});
	if (visibleItems.length > 100) {
		lines.push(`  ...and ${visibleItems.length - 100} more`);
	}
	return lines.join('\n');
}

function completionItemMatchesPrefix(item: vscode.CompletionItem, prefix: string): boolean {
	if (!prefix) {
		return true;
	}
	const filterText = (item.filterText || formatLabel(item.label)).toLowerCase();
	return filterText.startsWith(prefix.toLowerCase());
}

function formatLabel(label: string | vscode.CompletionItemLabel): string {
	return typeof label === 'string' ? label : `${label.label}${label.description ? ` ${label.description}` : ''}${label.detail ? ` ${label.detail}` : ''}`;
}

function formatKind(kind: vscode.CompletionItemKind | undefined): string {
	if (kind === undefined) {
		return '';
	}
	return `${kind}(${vscode.CompletionItemKind[kind] ?? 'Unknown'})`;
}

function formatInsertText(insertText: string | vscode.SnippetString | undefined): string {
	if (insertText === undefined) {
		return '';
	}
	return typeof insertText === 'string' ? insertText : insertText.value;
}

function formatDocumentation(documentation: string | vscode.MarkdownString | undefined): string {
	if (documentation === undefined) {
		return '';
	}
	return typeof documentation === 'string' ? documentation : documentation.value;
}

function formatRange(range: vscode.Range | { inserting: vscode.Range; replacing: vscode.Range } | undefined): string {
	if (!range) {
		return '';
	}
	if (range instanceof vscode.Range) {
		return formatSingleRange(range);
	}
	return `insert=${formatSingleRange(range.inserting)} replace=${formatSingleRange(range.replacing)}`;
}

function formatSingleRange(range: vscode.Range): string {
	return `${range.start.line + 1}:${range.start.character + 1}-${range.end.line + 1}:${range.end.character + 1}`;
}

function getPrefix(linePrefix: string): string {
	const directiveMatch = /^\s*#\s*([A-Za-z_]*)$/.exec(linePrefix);
	if (directiveMatch) {
		return directiveMatch[1] ? `#${directiveMatch[1]}` : '#';
	}
	const operatorMatch = /[=!<>|&+\-*\/%^~]+$/.exec(linePrefix);
	if (operatorMatch) {
		return operatorMatch[0];
	}
	return /[A-Za-z_][A-Za-z0-9_]*$/.exec(linePrefix)?.[0] ?? '';
}

function formatRankingData(data: unknown): string {
	if (!isRankingData(data)) {
		return 'ranking=unavailable';
	}
	const parts = [
		`tier=${data.tier}:${data.tierName}`,
		`source=${data.source}`,
		`reason=${data.reason}`,
		`finalRank=${data.finalRank}`,
	];
	if (typeof data.matchScore === 'number') {
		parts.push(`matchScore=${data.matchScore}`);
	}
	if (typeof data.valueType === 'string' && data.valueType) {
		parts.push(`valueType=${data.valueType}`);
	}
	if (typeof data.expectedType === 'string' && data.expectedType) {
		parts.push(`expectedType=${data.expectedType}`);
	}
	if (typeof data.typeContext === 'string' && data.typeContext) {
		parts.push(`typeContext=${data.typeContext}`);
	}
	return `ranking=${JSON.stringify(parts.join(' | '))}`;
}

function isRankingData(data: unknown): data is {
	tier: number;
	tierName: string;
	source: string;
	reason: string;
	finalRank?: number;
	matchScore?: number;
	valueType?: string;
	expectedType?: string;
	typeContext?: string;
} {
	return typeof data === 'object'
		&& data !== null
		&& typeof (data as { tier?: unknown }).tier === 'number'
		&& typeof (data as { tierName?: unknown }).tierName === 'string'
		&& typeof (data as { source?: unknown }).source === 'string'
		&& typeof (data as { reason?: unknown }).reason === 'string';
}

function formatNearbyText(document: vscode.TextDocument, position: vscode.Position): string {
	const startLine = Math.max(0, position.line - 5);
	const endLine = Math.min(document.lineCount - 1, position.line + 5);
	const lines: string[] = [];
	for (let line = startLine; line <= endLine; line++) {
		const marker = line === position.line ? '>' : ' ';
		lines.push(`${marker} ${line + 1}: ${document.lineAt(line).text}`);
		if (line === position.line) {
			lines.push(`  ${' '.repeat(String(line + 1).length + position.character + 2)}^`);
		}
	}
	return lines.join('\n');
}
