import * as vscode from 'vscode';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { ExtensionLogger } from '../../core/logger';
import { tracePerformanceAsync } from '../../core/performanceTrace';
import { parseEnforceDeclarations } from '../parser/declarationParser';

export type EnforceSymbolType = 'class' | 'enum' | 'enumValue' | 'function' | 'memberFunction' | 'property' | 'macro';
export type EnforceDeclarationKind = 'class' | 'moddedClass' | 'typedef' | 'enum' | 'enumMember' | 'function' | 'memberFunction' | 'constructor' | 'destructor' | 'property' | 'macro';
export type EnforceSymbolOrigin = 'gameData' | 'workspace';

export interface EnforceDecorator {
	name: string;
	arguments?: string;
}

export interface EnforceSymbol {
	name: string;
	type: EnforceSymbolType;
	uri: vscode.Uri;
	range: vscode.Range;
	selectionRange: vscode.Range;
	containerName?: string;
	signature?: string;
	detail?: string;
	documentation?: string;
	baseClassName?: string;
	decorators?: string[];
	decoratorDetails?: EnforceDecorator[];
	declarationKind?: EnforceDeclarationKind;
	declarationRange?: vscode.Range;
	bodyRange?: vscode.Range;
	modifiers?: string[];
	parserBacked?: boolean;
	functions?: string[];
	properties?: string[];
	enumMembers?: string[];
	origin?: EnforceSymbolOrigin;
	id?: string;
}

export interface EnforceContainerMemberSymbol extends EnforceSymbol {
	type: 'memberFunction' | 'property';
	containerName: string;
}

export interface EnforceIndexStats {
	files: number;
	symbols: number;
	classes: number;
	enums: number;
	functions: number;
	properties: number;
}

export interface EnforceIndexState {
	ready: boolean;
	refreshing: boolean;
	cacheLoaded: boolean;
	cacheStale: boolean;
	snapshotVersion: number;
	lastRefreshReason?: string;
}

export interface EnforcePrefixSearchDebugCandidate {
	name: string;
	key: string;
	score?: number;
	reason: string;
}

export interface EnforcePrefixSearchDebug {
	prefix: string;
	normalizedPrefix: string;
	limit: number;
	normalMatches: number;
	typoRecoveryRan: boolean;
	typoRecoveryReason: string;
	typoMatches: number;
	results: EnforcePrefixSearchDebugCandidate[];
	normalAccepted: EnforcePrefixSearchDebugCandidate[];
	typoAccepted: EnforcePrefixSearchDebugCandidate[];
	rejected: EnforcePrefixSearchDebugCandidate[];
}

interface IndexSnapshot {
	version: number;
	gameDataSymbols: readonly EnforceSymbol[];
	workspaceSymbols: readonly EnforceSymbol[];
	allSymbols: readonly EnforceSymbol[];
	byName: ReadonlyMap<string, readonly EnforceSymbol[]>;
	byUri: ReadonlyMap<string, readonly EnforceSymbol[]>;
	byType: ReadonlyMap<EnforceSymbolType, readonly EnforceSymbol[]>;
	classesByName: ReadonlyMap<string, readonly EnforceSymbol[]>;
	childrenByBase: ReadonlyMap<string, readonly string[]>;
	members: readonly EnforceContainerMemberSymbol[];
	membersByContainer: ReadonlyMap<string, readonly EnforceContainerMemberSymbol[]>;
	membersByName: ReadonlyMap<string, readonly EnforceContainerMemberSymbol[]>;
	membersByContainerAndName: ReadonlyMap<string, readonly EnforceContainerMemberSymbol[]>;
	enumValuesByContainer: ReadonlyMap<string, readonly EnforceSymbol[]>;
	decoratorNames: ReadonlySet<string>;
	typeSymbols: readonly EnforceSymbol[];
	classPrefixEntries: readonly PrefixEntry<EnforceSymbol>[];
	typePrefixEntries: readonly PrefixEntry<EnforceSymbol>[];
	functionPrefixEntries: readonly PrefixEntry<EnforceSymbol>[];
	decoratorPrefixEntries: readonly PrefixEntry<string>[];
	stats: EnforceIndexStats;
}

interface PrefixEntry<T> {
	key: string;
	value: T;
}

interface RefreshOptions {
	reason: 'startup' | 'manual' | 'configuration' | 'export';
	forceGameDataRebuild?: boolean;
	progress?: (message: string) => void;
	testingAllowSmallGameData?: boolean;
}

interface SerializedIndexCache {
	schemaVersion: number;
	exportedRoots: string[];
	files: SerializedFileManifest[];
	symbolKeys: SerializedSymbolKey[];
	symbols: SerializedSymbol[];
	testingAllowSmallGameData?: boolean;
}

type SerializedFileManifest = string;

interface SerializedSymbolRecord {
	name: string;
	type: EnforceSymbolType;
	file: number;
	range: SerializedRange;
	selectionRange: SerializedRange;
	containerName?: string;
	signature?: string;
	documentation?: string;
	baseClassName?: string;
	decorators?: string[];
	decoratorDetails?: EnforceDecorator[];
	declarationKind?: EnforceDeclarationKind;
	modifiers?: string[];
}

type SerializedSymbolKey = keyof SerializedSymbolRecord;
type SerializedSymbolValue = SerializedSymbolRecord[SerializedSymbolKey] | null;
type SerializedSymbol = SerializedSymbolValue[];

type SerializedRange = [startLine: number, startCharacter: number, endLine: number, endCharacter: number];

interface IndexProfile {
	startedAt: number;
	parsedFiles: number;
	failedFiles: number;
	parseTotalMs: number;
	parseMaxMs: number;
	cacheReadMs: number;
	cacheWriteMs: number;
	viewBuildMs: number;
}

const cacheSchemaVersion = 5;
const slowParseMs = 20;
const minimumExpectedGameScriptFiles = 5000;
const gameDataIndexYieldBatchSize = 200;
const prefixLimit = 200;
const legacyCallableMemberType: string = `meth${'od'}`;
const serializedSymbolKeys: readonly SerializedSymbolKey[] = [
	'name',
	'type',
	'file',
	'range',
	'selectionRange',
	'containerName',
	'signature',
	'documentation',
	'baseClassName',
	'decorators',
	'decoratorDetails',
	'declarationKind',
	'modifiers',
];

export class EnforceSymbolIndex {
	private snapshot = buildSnapshot([], [], 0);
	private readonly indexedDocumentVersions = new Map<string, number>();
	private readonly pendingDocumentIndexes = new Map<string, ReturnType<typeof setTimeout>>();
	private readonly indexLogPath: string;
	private refreshInProgress = false;
	private ready = false;
	private cacheLoaded = false;
	private cacheStale = false;
	private manualRefreshPromptShown = false;
	private lastRefreshReason: string | undefined;
	private initialized?: Promise<void>;
	private activeRefreshWait?: Promise<void>;
	private statusBarItem?: vscode.StatusBarItem;

	constructor(private readonly context: vscode.ExtensionContext, private readonly logger: ExtensionLogger, private readonly output?: vscode.OutputChannel) {
		this.indexLogPath = path.join(context.globalStorageUri.fsPath, 'logs', 'symbol-index.log');
	}

	register(context: vscode.ExtensionContext): void {
		this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 90);
		context.subscriptions.push(this.statusBarItem);
		context.subscriptions.push(
			vscode.workspace.onDidSaveTextDocument(async document => {
				if (isEnforceDocument(document)) {
					this.clearPendingDocumentIndex(document.uri);
					await this.indexDocument(document);
				}
			}),
			vscode.workspace.onDidChangeTextDocument(event => {
				if (isEnforceDocument(event.document)) {
					this.scheduleIndexDocument(event.document);
				}
			}),
			vscode.workspace.onDidOpenTextDocument(async document => {
				if (isEnforceDocument(document)) {
					await this.indexDocument(document);
				}
			}),
			vscode.workspace.onDidCloseTextDocument(async document => {
				if (isEnforceDocument(document)) {
					this.clearPendingDocumentIndex(document.uri);
					this.removeWorkspaceUri(document.uri);
				}
			}),
			vscode.workspace.onDidDeleteFiles(event => {
				for (const uri of event.files) {
					this.clearPendingDocumentIndex(uri);
					this.removeWorkspaceUri(uri);
				}
			}),
			vscode.workspace.onDidRenameFiles(async event => {
				for (const file of event.files) {
					this.removeWorkspaceUri(file.oldUri);
					if (file.newUri.fsPath.toLowerCase().endsWith('.c')) {
						await this.indexUri(file.newUri, 'workspace');
					}
				}
			}),
		);
		this.initialized = this.initialize();
	}

	isRefreshing(): boolean {
		return this.refreshInProgress;
	}

	getState(): EnforceIndexState {
		return {
			ready: this.ready,
			refreshing: this.refreshInProgress,
			cacheLoaded: this.cacheLoaded,
			cacheStale: this.cacheStale,
			snapshotVersion: this.snapshot.version,
			lastRefreshReason: this.lastRefreshReason,
		};
	}

	async refresh(showMessages: boolean, options: RefreshOptions = { reason: 'manual', forceGameDataRebuild: true }): Promise<EnforceIndexStats | undefined> {
		if (this.refreshInProgress) {
			const message = 'Reforger symbol index build is already running; waiting for it to finish.';
			this.logIndexProgress(message);
			if (showMessages) {
				vscode.window.showInformationMessage('Reforger symbol index build is already running. Waiting for it to finish.');
			}
			await (this.activeRefreshWait ?? waitFor(() => !this.refreshInProgress));
			return this.getStats();
		}
		this.refreshInProgress = true;
		this.activeRefreshWait = waitFor(() => !this.refreshInProgress);
		this.lastRefreshReason = options.reason;
		this.statusBarItem?.show();
		if (this.statusBarItem) {
			this.statusBarItem.text = '$(sync~spin) Reforger compiling script index';
			this.statusBarItem.tooltip = 'Reforger Script Tools is compiling the game-data symbol index.';
		}
		await this.clearIndexLog();
		const profile = newProfile();
		try {
			const startMessage = 'Compiling Reforger symbol index from script data...';
			options.progress?.(startMessage);
			this.logIndexProgress(startMessage);
			this.logger.info(`[Index] Refresh options. reason=${options.reason} forceGameData=${options.forceGameDataRebuild === true}`);
			if (options.forceGameDataRebuild !== false && !(await this.hasGameDataFiles())) {
				this.cacheLoaded = false;
				this.cacheStale = true;
				const message = 'Cannot build Reforger symbol index because downloaded BI script data is missing. Run Reforger: Refresh Game Data first.';
				this.logger.warn(message);
				await this.writeIndexLog(message);
				if (showMessages) {
					vscode.window.showWarningMessage(message);
				}
				this.statusBarItem?.hide();
				return undefined;
			}
			const gameDataSymbols = options.forceGameDataRebuild === false && this.cacheLoaded
				? [...this.snapshot.gameDataSymbols]
				: await this.rebuildGameDataIndex(profile, options.testingAllowSmallGameData === true);
			options.progress?.('Building symbol lookup tables...');
			this.logIndexProgress('Building symbol lookup tables...');
			this.swapSnapshot(gameDataSymbols, [...this.snapshot.workspaceSymbols], profile);
			this.cacheLoaded = true;
			this.cacheStale = false;
			const totalMs = performance.now() - profile.startedAt;
			const stats = this.getStats();
			await this.writeIndexLog([
				`Full index rebuild complete in ${fmt(totalMs)}.`,
				`files=${stats.files} symbols=${stats.symbols} classes=${stats.classes} enums=${stats.enums} functions=${stats.functions} properties=${stats.properties}`,
				`parsedFiles=${profile.parsedFiles} failedFiles=${profile.failedFiles} parseTotal=${fmt(profile.parseTotalMs)} parseAvg=${fmt(profile.parseTotalMs / Math.max(1, profile.parsedFiles))} parseMax=${fmt(profile.parseMaxMs)}`,
				`cacheRead=${fmt(profile.cacheReadMs)} cacheWrite=${fmt(profile.cacheWriteMs)} viewBuild=${fmt(profile.viewBuildMs)}`,
			].join('\n'));
			const message = `Indexed ${stats.symbols} symbols from ${stats.files} files.`;
			this.logIndexProgress(`${message} classes=${stats.classes} enums=${stats.enums} functions=${stats.functions} properties=${stats.properties}.`);
			if (this.statusBarItem) {
				this.statusBarItem.text = `$(check) ${message}`;
				setTimeout(() => this.statusBarItem?.hide(), 3000);
			}
			if (showMessages) {
				vscode.window.showInformationMessage(message);
			}
			return stats;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			this.logger.error(`Symbol indexing failed: ${message}`);
			await this.writeIndexLog(`Full index rebuild failed: ${message}`);
			this.statusBarItem?.hide();
			if (showMessages) {
				vscode.window.showWarningMessage('Reforger symbol indexing failed. Check the symbol index log.');
			}
			return undefined;
		} finally {
			this.refreshInProgress = false;
			this.activeRefreshWait = undefined;
		}
	}

	async indexDocument(document: vscode.TextDocument): Promise<void> {
		return tracePerformanceAsync('index.indexDocument', `${shortName(document.uri)} | lines=${document.lineCount} | version=${document.version}`, async () => {
			const symbols = parseSymbols(document.getText(), document.uri).map(symbol => withOrigin(symbol, 'workspace'));
			this.replaceWorkspaceSymbols(document.uri, symbols);
			this.indexedDocumentVersions.set(document.uri.toString(), document.version);
		});
	}

	async flushPendingDocumentIndex(document: vscode.TextDocument): Promise<void> {
		const key = document.uri.toString();
		const existing = this.pendingDocumentIndexes.get(key);
		if (existing) {
			clearTimeout(existing);
			this.pendingDocumentIndexes.delete(key);
			await this.indexDocument(document);
		}
	}

	async ensureDocumentIndexCurrent(document: vscode.TextDocument): Promise<void> {
		await this.flushPendingDocumentIndex(document);
		if (this.indexedDocumentVersions.get(document.uri.toString()) !== document.version) {
			await this.indexDocument(document);
		}
	}

	async indexUri(uri: vscode.Uri, origin: EnforceSymbolOrigin = 'workspace'): Promise<void> {
		try {
			const openDocument = vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
			if (openDocument && origin === 'workspace') {
				await this.indexDocument(openDocument);
				return;
			}
			const text = await fs.readFile(uri.fsPath, 'utf8');
			const symbols = parseSymbols(text, uri).map(symbol => withOrigin(symbol, origin));
			if (origin === 'workspace') {
				this.replaceWorkspaceSymbols(uri, symbols);
			}
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			this.logger.warn(`Could not index ${uri.fsPath}: ${message}`);
			await this.writeIndexLog(`Could not index ${uri.fsPath}: ${message}`);
			if (origin === 'workspace') {
				this.removeWorkspaceUri(uri);
			}
		}
	}

	find(name: string): EnforceSymbol[] { return [...(this.snapshot.byName.get(name) ?? [])]; }
	getDocumentSymbols(uri: vscode.Uri): EnforceSymbol[] { return [...(this.snapshot.byUri.get(uri.toString()) ?? [])]; }
	getDocumentIndexVersion(uri: vscode.Uri): number | undefined { return this.indexedDocumentVersions.get(uri.toString()); }
	hasPendingDocumentIndex(uri: vscode.Uri): boolean { return this.pendingDocumentIndexes.has(uri.toString()); }
	getAllSymbols(): EnforceSymbol[] { return [...this.snapshot.allSymbols]; }
	getSymbolsByType(type: EnforceSymbolType): readonly EnforceSymbol[] { return this.snapshot.byType.get(type) ?? []; }
	getClassSymbols(): readonly EnforceSymbol[] { return this.getSymbolsByType('class'); }
	getClassSymbolsByName(name: string): readonly EnforceSymbol[] { return this.snapshot.classesByName.get(name) ?? []; }
	getClassSymbol(name: string): EnforceSymbol | undefined { return this.getClassSymbolsByName(name)[0]; }
	getBaseClassName(name: string): string | undefined { return this.getClassSymbol(name)?.baseClassName; }
	getEnumSymbols(): readonly EnforceSymbol[] { return this.getSymbolsByType('enum'); }
	getFunctionSymbols(): readonly EnforceSymbol[] { return this.getSymbolsByType('function'); }
	getMacroSymbols(): readonly EnforceSymbol[] { return this.getSymbolsByType('macro'); }
	getTypeSymbols(): readonly EnforceSymbol[] { return this.snapshot.typeSymbols; }
	findClassesByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): readonly EnforceSymbol[] { return findClassPrefix(this.snapshot, prefix, limit, include).results; }
	findTypesByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): readonly EnforceSymbol[] { return findTypePrefix(this.snapshot, prefix, limit, include).results; }
	findFunctionsByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): readonly EnforceSymbol[] { return findFunctionPrefix(this.snapshot, prefix, limit, include).results; }
	debugFindClassesByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): EnforcePrefixSearchDebug { return findClassPrefix(this.snapshot, prefix, limit, include).debug; }
	debugFindTypesByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): EnforcePrefixSearchDebug { return findTypePrefix(this.snapshot, prefix, limit, include).debug; }
	debugFindFunctionsByPrefix(prefix: string, limit = prefixLimit, include?: (value: EnforceSymbol) => boolean): EnforcePrefixSearchDebug { return findFunctionPrefix(this.snapshot, prefix, limit, include).debug; }
	getContainerMemberSymbols(): readonly EnforceContainerMemberSymbol[] { return this.snapshot.members; }
	getContainerMemberSymbolsForContainers(containerNames: readonly string[]): readonly EnforceContainerMemberSymbol[] { return containerNames.flatMap(name => this.snapshot.membersByContainer.get(name) ?? []); }
	getContainerMemberSymbolsByName(name: string): readonly EnforceContainerMemberSymbol[] { return this.snapshot.membersByName.get(name) ?? []; }
	getContainerMemberSymbolsForContainerAndName(containerName: string, name: string): readonly EnforceContainerMemberSymbol[] { return this.snapshot.membersByContainerAndName.get(memberKey(containerName, name)) ?? []; }
	getContainerMemberSymbolsForContainersAndName(containerNames: readonly string[], name: string): readonly EnforceContainerMemberSymbol[] { return containerNames.flatMap(container => this.getContainerMemberSymbolsForContainerAndName(container, name)); }
	findMembers(containerNames: readonly string[], options?: { name?: string; prefix?: string; limit?: number }): readonly EnforceContainerMemberSymbol[] {
		const members = options?.name ? this.getContainerMemberSymbolsForContainersAndName(containerNames, options.name) : this.getContainerMemberSymbolsForContainers(containerNames);
		const prefix = options?.prefix?.toLowerCase();
		return (prefix ? members.filter(member => member.name.toLowerCase().startsWith(prefix)) : members).slice(0, options?.limit ?? prefixLimit);
	}
	getClassChildNames(name: string): readonly string[] { return this.snapshot.childrenByBase.get(name) ?? []; }
	getClassDescendantNames(name: string): readonly string[] {
		const descendants: string[] = [];
		const seen = new Set<string>();
		const pending = [...this.getClassChildNames(name)];
		while (pending.length > 0) {
			const child = pending.shift()!;
			if (!seen.has(child)) {
				seen.add(child);
				descendants.push(child);
				pending.push(...this.getClassChildNames(child));
			}
		}
		return descendants;
	}
	getClassAncestorNames(name: string, includeSelf = false): readonly string[] {
		const ancestors: string[] = [];
		const seen = new Set<string>();
		let current = includeSelf ? name : this.getBaseClassName(name);
		while (current && !seen.has(current)) {
			seen.add(current);
			ancestors.push(current);
			current = this.getBaseClassName(current);
		}
		return ancestors;
	}
	getClassRelationshipAliasNames(name: string): readonly string[] { return dedupe([...this.getClassAncestorNames(name), ...this.getClassDescendantNames(name)]); }
	getEnumValueSymbols(containerName: string): readonly EnforceSymbol[] { return this.snapshot.enumValuesByContainer.get(containerName) ?? []; }
	findEnumValues(enumName: string): readonly EnforceSymbol[] { return this.getEnumValueSymbols(enumName); }
	getDecoratorNames(): ReadonlySet<string> { return this.snapshot.decoratorNames; }
	findDecoratorsByPrefix(prefix: string, limit = prefixLimit): readonly string[] { return findPrefix(this.snapshot.decoratorPrefixEntries, prefix, limit); }
	getIndexedUris(): vscode.Uri[] { return [...this.snapshot.byUri.keys()].map(uri => vscode.Uri.parse(uri)); }
	getStats(): EnforceIndexStats { return this.snapshot.stats; }

	isGameDataCacheLoaded(): boolean {
		return this.cacheLoaded && !this.cacheStale;
	}

	async ensureGameDataIndex(): Promise<boolean> {
		await (this.initialized ?? Promise.resolve());
		if (this.isGameDataCacheLoaded()) {
			return true;
		}
		if (!(await this.hasGameDataFiles())) {
			this.cacheLoaded = false;
			this.cacheStale = true;
			await this.writeIndexLog('Game-data index skipped: downloaded BI script data is missing.');
			return false;
		}
		await this.writeIndexLog('Game-data index unavailable: cache is missing or stale. Run Reforger: Refresh Game Data to import and rebuild it.');
		this.showManualRefreshPrompt();
		return false;
	}

	async findReferences(name: string): Promise<vscode.Location[]> {
		const locations: vscode.Location[] = [];
		const pattern = new RegExp(`\\b${escapeRegExp(name)}\\b`, 'g');
		for (const uri of this.getIndexedUris()) {
			const text = await this.getText(uri);
			if (!text) { continue; }
			const lineStarts = getLineStarts(text);
			let match: RegExpExecArray | null;
			while ((match = pattern.exec(text)) !== null) {
				if (isInsideCommentOrString(text, match.index)) { continue; }
				const position = positionAtOffset(lineStarts, match.index);
				locations.push(new vscode.Location(uri, new vscode.Range(position.line, position.character, position.line, position.character + name.length)));
			}
		}
		return locations;
	}

	private async initialize(): Promise<void> {
		await this.ensureIndexLogDir();
		await this.writeIndexLog('Index initialization started.');
		await this.removeLegacyIndexCaches();
		await this.removeLegacyGeneratedFiles();
		await this.loadGameDataCache();
		for (const document of vscode.workspace.textDocuments) {
			if (isEnforceDocument(document)) { void this.indexDocument(document); }
		}
		this.ready = true;
	}

	private scheduleIndexDocument(document: vscode.TextDocument): void {
		const key = document.uri.toString();
		const existing = this.pendingDocumentIndexes.get(key);
		if (existing) { clearTimeout(existing); }
		this.pendingDocumentIndexes.set(key, setTimeout(() => {
			this.pendingDocumentIndexes.delete(key);
			void this.indexDocument(document);
		}, 1000));
	}
	private clearPendingDocumentIndex(uri: vscode.Uri): void {
		const existing = this.pendingDocumentIndexes.get(uri.toString());
		if (existing) { clearTimeout(existing); this.pendingDocumentIndexes.delete(uri.toString()); }
	}
	private replaceWorkspaceSymbols(uri: vscode.Uri, symbols: readonly EnforceSymbol[]): void {
		const key = uri.toString();
		this.swapSnapshot([...this.snapshot.gameDataSymbols], [...this.snapshot.workspaceSymbols.filter(symbol => symbol.uri.toString() !== key), ...symbols]);
	}
	private removeWorkspaceUri(uri: vscode.Uri): void {
		const key = uri.toString();
		this.swapSnapshot([...this.snapshot.gameDataSymbols], this.snapshot.workspaceSymbols.filter(symbol => symbol.uri.toString() !== key));
		this.indexedDocumentVersions.delete(key);
	}
	private swapSnapshot(gameDataSymbols: readonly EnforceSymbol[], workspaceSymbols: readonly EnforceSymbol[], profile?: IndexProfile): void {
		const started = performance.now();
		this.snapshot = buildSnapshot(gameDataSymbols, workspaceSymbols, this.snapshot.version + 1);
		const elapsed = performance.now() - started;
		if (profile) { profile.viewBuildMs += elapsed; }
		void this.writeIndexLog(`Snapshot swapped. version=${this.snapshot.version} symbols=${this.snapshot.allSymbols.length} viewBuild=${fmt(elapsed)}`);
	}
	private async loadGameDataCache(): Promise<void> {
		const started = performance.now();
		try {
			await this.writeIndexLog('Cache load started.');
			const cache = JSON.parse(await fs.readFile(this.getCachePath(), 'utf8')) as SerializedIndexCache;
			const validation = await this.validateCache(cache);
			if (!validation.valid) {
				this.cacheLoaded = false; this.cacheStale = true;
				await this.writeIndexLog(`Cache stale: ${validation.reason}`);
				return;
			}
			this.swapSnapshot(restoreDerivedSymbolFields(cache.symbols.map(symbol => deserializeSymbol(symbol, cache.symbolKeys, cache.files))).map(symbol => withOrigin(symbol, 'gameData')), [...this.snapshot.workspaceSymbols]);
			this.cacheLoaded = true; this.cacheStale = false;
			await this.writeIndexLog(`Cache load complete. symbols=${cache.symbols.length} files=${cache.files.length} read=${fmt(performance.now() - started)}`);
		} catch (error) {
			this.cacheLoaded = false; this.cacheStale = true;
			await this.writeIndexLog(`Cache load skipped: ${error instanceof Error ? error.message : String(error)}`);
		}
	}
	private async validateCache(cache: SerializedIndexCache): Promise<{ valid: boolean; reason: string }> {
		if (cache.schemaVersion !== cacheSchemaVersion) { return { valid: false, reason: 'schema changed' }; }
		const roots = getExportedGameDataIndexPaths(this.context).map(normalizePath);
		if (cache.exportedRoots.join('|') !== roots.join('|')) { return { valid: false, reason: 'exported roots changed' }; }
		if (!cache.testingAllowSmallGameData && cache.files.length < minimumExpectedGameScriptFiles) { return { valid: false, reason: gameScriptDataNotExpectedMessage(cache.files.length) }; }
		return { valid: true, reason: 'ok' };
	}
	private async rebuildGameDataIndex(profile: IndexProfile, allowSmallGameData = false): Promise<EnforceSymbol[]> {
		const roots = getExportedGameDataIndexPaths(this.context);
		const files = await this.findGameDataFiles(roots);
		if (!allowSmallGameData && files.length < minimumExpectedGameScriptFiles) {
			throw new Error(gameScriptDataNotExpectedMessage(files.length));
		}
		const symbols: EnforceSymbol[] = [];
		const manifest: SerializedFileManifest[] = [];
		for (let index = 0; index < files.length; index++) {
			const file = files[index];
			const started = performance.now();
			try {
				const [stat, text] = await Promise.all([fs.stat(file), fs.readFile(file, 'utf8')]);
				const parsed = parseEnforceDeclarations(text, vscode.Uri.file(file)).map(symbol => withOrigin(symbol, 'gameData'));
				const elapsed = performance.now() - started;
				profile.parsedFiles++; profile.parseTotalMs += elapsed; profile.parseMaxMs = Math.max(profile.parseMaxMs, elapsed);
				if (elapsed >= slowParseMs) { await this.writeIndexLog(`Slow parse ${fmt(elapsed)} symbols=${parsed.length} size=${stat.size} path=${file}`); }
				symbols.push(...parsed);
				manifest.push(normalizePath(file));
			} catch (error) {
				profile.failedFiles++;
				await this.writeIndexLog(`Failed to index ${file}: ${error instanceof Error ? error.message : String(error)}`);
			}
			const processedFiles = index + 1;
			if (processedFiles % gameDataIndexYieldBatchSize === 0 || processedFiles === files.length) {
				await yieldToExtensionHost();
			}
		}
		await this.writeGameDataCache(roots, manifest, symbols, profile);
		return symbols;
	}
	private async writeGameDataCache(roots: readonly string[], files: readonly SerializedFileManifest[], symbols: readonly EnforceSymbol[], profile: IndexProfile): Promise<void> {
		const started = performance.now();
		await fs.mkdir(path.dirname(this.getCachePath()), { recursive: true });
		const sortedFiles = [...files].sort((a, b) => a.localeCompare(b));
		const fileIndexes = new Map(sortedFiles.map((file, index) => [file, index]));
		const cache: SerializedIndexCache = {
			schemaVersion: cacheSchemaVersion,
			exportedRoots: roots.map(normalizePath),
			files: sortedFiles,
			symbolKeys: [...serializedSymbolKeys],
			symbols: symbols.map(symbol => serializeSymbol(symbol, fileIndexes)),
		};
		const tempPath = `${this.getCachePath()}.tmp`;
		await fs.writeFile(tempPath, JSON.stringify(cache), 'utf8');
		await fs.rename(tempPath, this.getCachePath());
		profile.cacheWriteMs = performance.now() - started;
	}
	private async findGameDataFiles(roots: readonly string[]): Promise<string[]> {
		const byPath = new Map<string, string>();
		for (const root of roots) {
			for (const file of await findScriptFiles(root)) { byPath.set(await normalizeRealPath(file), file); }
		}
		return [...byPath.values()];
	}
	private async hasGameDataFiles(): Promise<boolean> { return (await this.findGameDataFiles(getExportedGameDataIndexPaths(this.context))).length > 0; }
	private getCachePath(): string { return path.join(this.context.globalStorageUri.fsPath, 'index', 'game-data-index.json'); }
	private getLegacyCachePaths(): string[] {
		return [
			path.join(this.context.globalStorageUri.fsPath, 'index', 'game-data-index-v1.json'),
			path.join(this.context.globalStorageUri.fsPath, 'index', 'game-data-index-v2.json'),
		];
	}
	private getLegacyGeneratedPaths(): string[] { return [path.join(this.context.globalStorageUri.fsPath, 'generated', 'compiler-defines.c')]; }
	private async removeLegacyIndexCaches(): Promise<void> {
		for (const cachePath of this.getLegacyCachePaths()) {
			try {
				await fs.rm(cachePath, { force: true });
			} catch {
				// Best-effort cleanup; a stale cache should not block indexing.
			}
		}
	}
	private async removeLegacyGeneratedFiles(): Promise<void> {
		for (const filePath of this.getLegacyGeneratedPaths()) {
			try {
				await fs.rm(filePath, { force: true });
			} catch {
				// Best-effort cleanup; a stale generated file should not block indexing.
			}
		}
	}
	private showIndexSummary(): void {
		const stats = this.getStats();
		const state = this.getState();
		const message = `Indexed ${stats.symbols} symbols from ${stats.files} files. Classes: ${stats.classes}, functions: ${stats.functions}, properties: ${stats.properties}, enums: ${stats.enums}. Ready: ${state.ready}, refreshing: ${state.refreshing}, cacheLoaded: ${state.cacheLoaded}, cacheStale: ${state.cacheStale}, snapshot: ${state.snapshotVersion}.`;
		this.logger.info(message);
		void this.writeIndexLog(`Inspect index: ${message}`);
		vscode.window.showInformationMessage(message);
	}
	private async ensureIndexLogDir(): Promise<void> { await fs.mkdir(path.dirname(this.indexLogPath), { recursive: true }); }
	private async clearIndexLog(): Promise<void> { await this.ensureIndexLogDir(); await fs.writeFile(this.indexLogPath, '', 'utf8'); }
	private async writeIndexLog(message: string): Promise<void> { await this.ensureIndexLogDir(); await fs.appendFile(this.indexLogPath, `${new Date().toISOString()} ${message}\n`, 'utf8'); }
	private logIndexProgress(message: string): void {
		const formatted = `[Index] ${message}`;
		this.output?.appendLine(message);
		this.logger.info(formatted);
		void this.writeIndexLog(message);
	}
	private showManualRefreshPrompt(): void {
		if (this.manualRefreshPromptShown) {
			return;
		}
		this.manualRefreshPromptShown = true;
		void vscode.window.showWarningMessage(
			'Reforger game-data symbol cache is missing or stale. Run Reforger: Refresh Game Data to import and rebuild it.',
			'Reforger: Refresh Game Data'
		).then(selection => {
			if (selection === 'Reforger: Refresh Game Data') {
				void vscode.commands.executeCommand('reforger-script-tools.refreshGameData');
			}
		});
	}
	private async getText(uri: vscode.Uri): Promise<string | undefined> {
		const openDocument = vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
		if (openDocument) { return openDocument.getText(); }
		try { return await fs.readFile(uri.fsPath, 'utf8'); } catch { return undefined; }
	}
}

function buildSnapshot(gameDataSymbols: readonly EnforceSymbol[], workspaceSymbols: readonly EnforceSymbol[], version: number): IndexSnapshot {
	const allSymbols = dedupeSymbolsByCanonicalLocation([...gameDataSymbols, ...workspaceSymbols].map(assignSymbolId));
	const classSymbols = allSymbols.filter(symbol => symbol.type === 'class');
	const functionSymbols = allSymbols.filter(symbol => symbol.type === 'function');
	const members = allSymbols.filter(isContainerMemberSymbol);
	const typeSymbols = allSymbols.filter(symbol => symbol.type === 'class' || symbol.type === 'enum');
	const childrenByBase = new Map<string, string[]>();
	for (const symbol of classSymbols) {
		if (symbol.baseClassName) { const values = childrenByBase.get(symbol.baseClassName) ?? []; values.push(symbol.name); childrenByBase.set(symbol.baseClassName, values); }
	}
	const classesByName = groupBy(classSymbols, symbol => symbol.name);
	const decoratorNames = new Set([
		...allSymbols.flatMap(symbol => symbol.decorators ?? []),
		...classSymbols.filter(isAttributeStyleClassSymbol).map(symbol => symbol.name),
	]);
	return {
		version, gameDataSymbols, workspaceSymbols, allSymbols,
		byName: groupBy(allSymbols, symbol => symbol.name),
		byUri: groupBy(allSymbols, symbol => symbol.uri.toString()),
		byType: groupBy(allSymbols, symbol => symbol.type) as Map<EnforceSymbolType, EnforceSymbol[]>,
		classesByName,
		childrenByBase,
		members,
		membersByContainer: groupBy(members, symbol => symbol.containerName),
		membersByName: groupBy(members, symbol => symbol.name),
		membersByContainerAndName: groupBy(members, symbol => memberKey(symbol.containerName, symbol.name)),
		enumValuesByContainer: groupBy(allSymbols.filter(symbol => symbol.type === 'enumValue' && symbol.containerName), symbol => symbol.containerName ?? ''),
		decoratorNames,
		typeSymbols,
		classPrefixEntries: buildSymbolPrefixEntries(classSymbols),
		typePrefixEntries: buildSymbolPrefixEntries(typeSymbols),
		functionPrefixEntries: buildSymbolPrefixEntries(functionSymbols),
		decoratorPrefixEntries: buildDecoratorPrefixEntries(decoratorNames),
		stats: calculateStats(allSymbols, new Set(allSymbols.map(symbol => canonicalUriKey(symbol.uri))).size),
	};
}

function groupBy<T, K extends string>(items: readonly T[], keyOf: (item: T) => K): Map<K, T[]> {
	const map = new Map<K, T[]>();
	for (const item of items) { const key = keyOf(item); const values = map.get(key) ?? []; values.push(item); map.set(key, values); }
	return map;
}
function isAttributeStyleClassSymbol(symbol: EnforceSymbol): boolean {
	return symbol.name === 'Attribute' || symbol.name.endsWith('Attribute');
}
function findPrefix<T>(entries: readonly PrefixEntry<T>[], prefix: string, limit: number, include?: (value: T) => boolean): T[] {
	return findPrefixDetailed(entries, prefix, limit, include).results;
}

function findPrefixDetailed<T>(entries: readonly PrefixEntry<T>[], prefix: string, limit: number, include?: (value: T) => boolean): { results: T[]; debug: EnforcePrefixSearchDebug } {
	const normalized = prefix.toLowerCase();
	const bestMatches = new Map<T, { value: T; score: number; key: string }>();
	const normalAccepted: EnforcePrefixSearchDebugCandidate[] = [];
	const typoAccepted: EnforcePrefixSearchDebugCandidate[] = [];
	const rejected: EnforcePrefixSearchDebugCandidate[] = [];
	for (const entry of entries) {
		if (entry.key.startsWith(normalized)) {
			if (include && !include(entry.value)) {
				pushDebugCandidate(rejected, entry, undefined, 'normal rejected by include filter');
				continue;
			}
			const score = prefixEntryScore(entry, normalized);
			pushDebugCandidate(normalAccepted, entry, score, 'normal prefix/key match');
			const existing = bestMatches.get(entry.value);
			if (!existing || score < existing.score || (score === existing.score && entry.key.length < existing.key.length)) {
				bestMatches.set(entry.value, { value: entry.value, score, key: entry.key });
			}
		}
	}
	const typoRecoveryRan = normalized.length >= 8;
	const typoRecoveryReason = typoRecoveryRan ? 'prefix length is sufficient' : 'prefix shorter than 8';
	if (typoRecoveryRan) {
		for (const entry of entries) {
			const score = approximatePrefixEntryScore(entry, normalized);
			if (score !== undefined) {
				if (include && !include(entry.value)) {
					pushDebugCandidate(rejected, entry, score, 'typo rejected by include filter');
					continue;
				}
				pushDebugCandidate(typoAccepted, entry, score, 'typo recovery key match');
				const existing = bestMatches.get(entry.value);
				if (!existing || score < existing.score || (score === existing.score && entry.key.length < existing.key.length)) {
					bestMatches.set(entry.value, { value: entry.value, score, key: entry.key });
				}
			}
		}
	}
	const matches = [...bestMatches.values()]
		.sort((left, right) => left.score - right.score || valueName(left.value).localeCompare(valueName(right.value)));
	const limitedMatches = matches.slice(0, limit);
	return {
		results: limitedMatches.map(match => match.value),
		debug: {
			prefix,
			normalizedPrefix: normalized,
			limit,
			normalMatches: normalAccepted.length,
			typoRecoveryRan,
			typoRecoveryReason,
			typoMatches: typoAccepted.length,
			results: limitedMatches.slice(0, 30).map(match => debugCandidate(match, 'final result')),
			normalAccepted: normalAccepted.slice(0, 30),
			typoAccepted: typoAccepted.slice(0, 30),
			rejected: rejected.slice(0, 30),
		},
	};
}
function findClassPrefix(snapshot: IndexSnapshot, prefix: string, limit: number, include?: (value: EnforceSymbol) => boolean): { results: EnforceSymbol[]; debug: EnforcePrefixSearchDebug } {
	const direct = findPrefixDetailed(snapshot.classPrefixEntries, prefix, limit, include);
	return expandClassRelationResults(snapshot, direct, limit, include);
}
function findTypePrefix(snapshot: IndexSnapshot, prefix: string, limit: number, include?: (value: EnforceSymbol) => boolean): { results: EnforceSymbol[]; debug: EnforcePrefixSearchDebug } {
	const direct = findPrefixDetailed(snapshot.typePrefixEntries, prefix, limit, include);
	return expandClassRelationResults(snapshot, direct, limit, include);
}
function findFunctionPrefix(snapshot: IndexSnapshot, prefix: string, limit: number, include?: (value: EnforceSymbol) => boolean): { results: EnforceSymbol[]; debug: EnforcePrefixSearchDebug } {
	return findPrefixDetailed(snapshot.functionPrefixEntries, prefix, limit, include);
}
function expandClassRelationResults(snapshot: IndexSnapshot, direct: { results: EnforceSymbol[]; debug: EnforcePrefixSearchDebug }, limit: number, include?: (value: EnforceSymbol) => boolean): { results: EnforceSymbol[]; debug: EnforcePrefixSearchDebug } {
	const results: EnforceSymbol[] = [];
	const seen = new Set<string>();
	for (const symbol of direct.results) {
		pushRelationResult(symbol);
	}
	for (const symbol of direct.results) {
		if (symbol.type !== 'class') {
			continue;
		}
		for (const relatedName of classRelationNames(snapshot, symbol)) {
			for (const related of snapshot.classesByName.get(relatedName) ?? []) {
				pushRelationResult(related);
			}
		}
	}
	return { results: results.slice(0, limit), debug: direct.debug };

	function pushRelationResult(symbol: EnforceSymbol): void {
		if (results.length >= limit || (include && !include(symbol))) {
			return;
		}
		const key = symbol.id ?? `${symbol.uri.toString()}:${symbol.range.start.line}:${symbol.range.start.character}:${symbol.name}`;
		if (!seen.has(key)) {
			seen.add(key);
			results.push(symbol);
		}
	}
}
function classRelationNames(snapshot: IndexSnapshot, symbol: EnforceSymbol): string[] {
	const names: string[] = [];
	const seen = new Set<string>();
	let current = symbol.baseClassName;
	while (current && !seen.has(current)) {
		seen.add(current);
		names.push(current);
		current = snapshot.classesByName.get(current)?.[0]?.baseClassName;
	}
	for (const descendant of classDescendantNames(symbol.name, snapshot.childrenByBase)) {
		if (!seen.has(descendant)) {
			seen.add(descendant);
			names.push(descendant);
		}
	}
	return names;
}
function buildSymbolPrefixEntries<T extends EnforceSymbol>(symbols: readonly T[]): PrefixEntry<T>[] {
	return symbols
		.flatMap(symbol => symbolSearchKeys(symbol.name).map(key => ({ key, value: symbol })))
		.sort(comparePrefixEntry);
}
function buildDecoratorPrefixEntries(names: ReadonlySet<string>): PrefixEntry<string>[] {
	return [...names]
		.flatMap(name => decoratorSearchKeys(name).map(key => ({ key, value: name })))
		.sort(comparePrefixEntry);
}
function decoratorSearchKeys(name: string): string[] {
	const keys = new Set(symbolSearchKeys(name));
	keys.add('attribute');
	keys.add('attributes');
	keys.add('decorator');
	keys.add('decorators');
	return [...keys];
}
function classDescendantNames(name: string, childrenByBase: ReadonlyMap<string, readonly string[]>): string[] {
	const descendants: string[] = [];
	const seen = new Set<string>();
	const pending = [...childrenByBase.get(name) ?? []];
	while (pending.length > 0) {
		const child = pending.shift()!;
		if (!seen.has(child)) {
			seen.add(child);
			descendants.push(child);
			pending.push(...childrenByBase.get(child) ?? []);
		}
	}
	return descendants;
}
function symbolSearchKeys(name: string): string[] {
	const keys = new Set<string>();
	const normalizedName = name.toLowerCase();
	keys.add(normalizedName);
	const words = name
		.split(/[^A-Za-z0-9]+/)
		.flatMap(part => part.match(/[A-Z]+(?=[A-Z][a-z]|\d|$)|[A-Z]?[a-z]+|\d+/g) ?? [])
		.map(part => part.toLowerCase())
		.filter(Boolean);
	for (let index = 0; index < words.length; index++) {
		keys.add(words.slice(index).join(''));
	}
	return [...keys];
}
function prefixEntryScore<T>(entry: PrefixEntry<T>, normalizedPrefix: string): number {
	const name = valueName(entry.value);
	if (name === normalizedPrefix) {
		return 0;
	}
	if (entry.key === normalizedPrefix) {
		return 5 + name.length - normalizedPrefix.length;
	}
	if (name.startsWith(normalizedPrefix)) {
		return 20 + name.length - normalizedPrefix.length;
	}
	return 40 + entry.key.length - normalizedPrefix.length;
}
function approximatePrefixEntryScore<T>(entry: PrefixEntry<T>, normalizedPrefix: string): number | undefined {
	if (!entry.key || entry.key[0] !== normalizedPrefix[0]) {
		return undefined;
	}
	const name = valueName(entry.value);
	if (editDistanceAtMostOneOrAdjacentTransposition(normalizedPrefix, entry.key)) {
		return 15 + Math.abs(name.length - normalizedPrefix.length);
	}
	const closePrefixLength = closeApproximatePrefixLength(entry.key, normalizedPrefix);
	if (closePrefixLength === undefined) {
		return undefined;
	}
	const keyPenalty = Math.abs(entry.key.length - normalizedPrefix.length);
	const namePenalty = Math.abs(name.length - normalizedPrefix.length);
	const continuationPenalty = Math.max(0, entry.key.length - closePrefixLength);
	return 100 + keyPenalty + namePenalty + continuationPenalty;
}
function closeApproximatePrefixLength(key: string, normalizedPrefix: string): number | undefined {
	const lengths = [
		normalizedPrefix.length,
		normalizedPrefix.length - 1,
		normalizedPrefix.length + 1,
	]
		.filter(length => length > 0 && length <= key.length)
		.sort((left, right) => Math.abs(left - normalizedPrefix.length) - Math.abs(right - normalizedPrefix.length));
	for (const length of lengths) {
		const keyPrefix = key.slice(0, length);
		if (editDistanceAtMostOneOrAdjacentTransposition(normalizedPrefix, keyPrefix)) {
			return length;
		}
	}
	return undefined;
}
function pushDebugCandidate<T>(target: EnforcePrefixSearchDebugCandidate[], entry: PrefixEntry<T>, score: number | undefined, reason: string): void {
	if (target.length >= 50) {
		return;
	}
	target.push(debugCandidate({ value: entry.value, key: entry.key, score: score ?? Number.NaN }, reason));
}
function debugCandidate<T>(match: { value: T; key: string; score: number }, reason: string): EnforcePrefixSearchDebugCandidate {
	return {
		name: displayValueName(match.value),
		key: match.key,
		score: Number.isNaN(match.score) ? undefined : match.score,
		reason,
	};
}
function editDistanceAtMostOneOrAdjacentTransposition(left: string, right: string): boolean {
	if (editDistanceAtMostOne(left, right)) {
		return true;
	}
	if (left.length !== right.length) {
		return false;
	}
	let firstMismatch = -1;
	for (let index = 0; index < left.length; index++) {
		if (left[index] !== right[index]) {
			if (firstMismatch >= 0) {
				return index === firstMismatch + 1
					&& left[firstMismatch] === right[index]
					&& left[index] === right[firstMismatch]
					&& left.slice(index + 1) === right.slice(index + 1);
			}
			firstMismatch = index;
		}
	}
	return false;
}
function editDistanceAtMostOne(left: string, right: string): boolean {
	if (left === right) {
		return true;
	}
	if (Math.abs(left.length - right.length) > 1) {
		return false;
	}
	let edits = 0;
	let leftIndex = 0;
	let rightIndex = 0;
	while (leftIndex < left.length && rightIndex < right.length) {
		if (left[leftIndex] === right[rightIndex]) {
			leftIndex++;
			rightIndex++;
			continue;
		}
		edits++;
		if (edits > 1) {
			return false;
		}
		if (left.length > right.length) {
			leftIndex++;
		} else if (right.length > left.length) {
			rightIndex++;
		} else {
			leftIndex++;
			rightIndex++;
		}
	}
	if (leftIndex < left.length || rightIndex < right.length) {
		edits++;
	}
	return edits <= 1;
}
function valueName(value: unknown): string {
	return typeof value === 'object' && value && 'name' in value
		? String((value as { name: unknown }).name).toLowerCase()
		: String(value).toLowerCase();
}
function displayValueName(value: unknown): string {
	return typeof value === 'object' && value && 'name' in value
		? String((value as { name: unknown }).name)
		: String(value);
}
function comparePrefixEntry<T>(a: PrefixEntry<T>, b: PrefixEntry<T>): number {
	const byKey = a.key.localeCompare(b.key);
	if (byKey !== 0) {
		return byKey;
	}
	return valueName(a.value).localeCompare(valueName(b.value));
}
function calculateStats(symbols: readonly EnforceSymbol[], files: number): EnforceIndexStats {
	return { files, symbols: symbols.length, classes: symbols.filter(s => s.type === 'class').length, enums: symbols.filter(s => s.type === 'enum').length, functions: symbols.filter(s => s.type === 'function' || s.type === 'memberFunction').length, properties: symbols.filter(s => s.type === 'property').length };
}
function isContainerMemberSymbol(symbol: EnforceSymbol): symbol is EnforceContainerMemberSymbol { return symbol.containerName !== undefined && (symbol.type === 'memberFunction' || symbol.type === 'property'); }
function memberKey(containerName: string, name: string): string { return `${containerName}\0${name}`; }
function assignSymbolId(symbol: EnforceSymbol): EnforceSymbol { return symbol.id ? symbol : { ...symbol, id: [symbol.origin ?? 'workspace', symbol.uri.toString(), symbol.type, symbol.containerName ?? '', symbol.name, symbol.selectionRange.start.line, symbol.selectionRange.start.character].join('|') }; }
function dedupeSymbolsByCanonicalLocation<T extends EnforceSymbol>(symbols: readonly T[]): T[] {
	const seen = new Set<string>();
	return symbols.filter(symbol => {
		const key = canonicalSymbolKey(symbol);
		if (seen.has(key)) {
			return false;
		}
		seen.add(key);
		return true;
	});
}
function canonicalSymbolKey(symbol: EnforceSymbol): string {
	return [
		canonicalUriKey(symbol.uri),
		symbol.type,
		symbol.containerName ?? '',
		symbol.name,
		symbol.selectionRange.start.line,
		symbol.selectionRange.start.character,
		symbol.selectionRange.end.line,
		symbol.selectionRange.end.character,
	].join('|');
}
function canonicalUriKey(uri: vscode.Uri): string {
	return uri.scheme === 'file' && uri.fsPath
		? uri.fsPath.replace(/\\/g, '/').toLowerCase()
		: uri.toString().toLowerCase();
}
function withOrigin(symbol: EnforceSymbol, origin: EnforceSymbolOrigin): EnforceSymbol { return assignSymbolId({ ...symbol, origin }); }
function serializeSymbol(symbol: EnforceSymbol, fileIndexes: ReadonlyMap<string, number>): SerializedSymbol {
	const file = fileIndexes.get(normalizePath(symbol.uri.fsPath));
	if (file === undefined) {
		throw new Error(`Could not map symbol file into index manifest: ${symbol.uri.fsPath}`);
	}
	const record: SerializedSymbolRecord = {
		name: symbol.name,
		type: symbol.type,
		file,
		range: serializeRange(symbol.range),
		selectionRange: serializeRange(symbol.selectionRange),
	};
	if (symbol.containerName !== undefined) { record.containerName = symbol.containerName; }
	if (symbol.signature !== undefined) { record.signature = symbol.signature; }
	if (symbol.documentation !== undefined) { record.documentation = symbol.documentation; }
	if (symbol.baseClassName !== undefined) { record.baseClassName = symbol.baseClassName; }
	if (symbol.decorators !== undefined) { record.decorators = symbol.decorators; }
	if (symbol.decoratorDetails !== undefined) { record.decoratorDetails = symbol.decoratorDetails; }
	if (symbol.declarationKind !== undefined) { record.declarationKind = symbol.declarationKind; }
	if (symbol.modifiers !== undefined) { record.modifiers = symbol.modifiers; }
	return serializedSymbolKeys.map(key => record[key] ?? null);
}
function deserializeSymbol(symbol: SerializedSymbol, keys: readonly SerializedSymbolKey[], files: readonly SerializedFileManifest[]): EnforceSymbol {
	const record = deserializeSymbolRecord(symbol, keys);
	const file = files[record.file];
	const range = deserializeRange(record.range);
	const selectionRange = deserializeRange(record.selectionRange);
	return {
		...record,
		uri: vscode.Uri.file(file ?? ''),
		range,
		selectionRange,
		detail: getDefaultSymbolDetail(record),
		declarationRange: range,
		parserBacked: true,
	};
}
function deserializeSymbolRecord(symbol: SerializedSymbol, keys: readonly SerializedSymbolKey[]): SerializedSymbolRecord {
	const record: Partial<SerializedSymbolRecord> = {};
	for (let index = 0; index < keys.length; index++) {
		const value = symbol[index];
		if (value !== null && value !== undefined) {
			(record as Record<SerializedSymbolKey, SerializedSymbolValue>)[keys[index]] = value;
		}
	}
	normalizeSerializedSymbolRecord(record);
	if (!record.name || !record.type || record.file === undefined || !record.range || !record.selectionRange) {
		throw new Error('Serialized symbol is missing required fields.');
	}
	return record as SerializedSymbolRecord;
}
function normalizeSerializedSymbolRecord(record: Partial<SerializedSymbolRecord>): void {
	const mutableRecord = record as Partial<SerializedSymbolRecord> & { type?: string; declarationKind?: string };
	if (mutableRecord.type === legacyCallableMemberType) {
		mutableRecord.type = 'memberFunction';
	}
	if (mutableRecord.declarationKind === legacyCallableMemberType) {
		mutableRecord.declarationKind = 'memberFunction';
	}
}
function restoreDerivedSymbolFields(symbols: EnforceSymbol[]): EnforceSymbol[] {
	const membersByContainer = groupBy(symbols.filter(symbol => symbol.containerName), symbol => symbol.containerName ?? '');
	for (const symbol of symbols) {
		if (symbol.type === 'class') {
			const members = membersByContainer.get(symbol.name) ?? [];
			symbol.functions = members
				.filter(member => member.type === 'memberFunction')
				.map(member => member.signature ?? member.name);
			symbol.properties = members
				.filter(member => member.type === 'property')
				.map(member => member.signature ?? member.name);
		} else if (symbol.type === 'enum') {
			symbol.enumMembers = (membersByContainer.get(symbol.name) ?? [])
				.filter(member => member.type === 'enumValue')
				.map(member => member.signature ?? member.name);
		}
	}
	return symbols;
}
function getDefaultSymbolDetail(symbol: SerializedSymbolRecord): string {
	if (symbol.containerName && (symbol.type === 'memberFunction' || symbol.type === 'property' || symbol.type === 'enumValue')) {
		return `${symbol.containerName}.${symbol.name}`;
	}
	return symbol.signature ?? symbol.name;
}
function serializeRange(range: vscode.Range): SerializedRange { return [range.start.line, range.start.character, range.end.line, range.end.character]; }
function deserializeRange(range: SerializedRange): vscode.Range { return new vscode.Range(range[0], range[1], range[2], range[3]); }
function newProfile(): IndexProfile { return { startedAt: performance.now(), parsedFiles: 0, failedFiles: 0, parseTotalMs: 0, parseMaxMs: 0, cacheReadMs: 0, cacheWriteMs: 0, viewBuildMs: 0 }; }
function dedupe(values: readonly string[]): string[] { return [...new Set(values.filter(Boolean))]; }
function yieldToExtensionHost(): Promise<void> { return new Promise(resolve => setTimeout(resolve, 0)); }
function waitFor(predicate: () => boolean): Promise<void> {
	return new Promise(resolve => {
		const check = () => {
			if (predicate()) {
				resolve();
				return;
			}
			setTimeout(check, 100);
		};
		check();
	});
}
export function parseSymbols(text: string, uri: vscode.Uri): EnforceSymbol[] { return parseEnforceDeclarations(text, uri); }
function getExportedGameDataIndexPaths(context: vscode.ExtensionContext): string[] {
	return [path.join(context.globalStorageUri.fsPath, 'exported-game-data')];
}
function gameScriptDataNotExpectedMessage(fileCount: number): string {
	return `Game script data not as expected: found ${fileCount} .c script file(s), expected at least ${minimumExpectedGameScriptFiles}. Refresh from GitHub or select the Reforger scripts folder.`;
}
async function findScriptFiles(rootPath: string): Promise<string[]> {
	const results: string[] = [];
	try { const stat = await fs.stat(rootPath); if (stat.isFile()) { return rootPath.toLowerCase().endsWith('.c') ? [rootPath] : []; } await walk(rootPath, results); } catch { return []; }
	return results;
}
async function walk(directory: string, results: string[]): Promise<void> {
	for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
		const fullPath = path.join(directory, entry.name);
		if (entry.isDirectory()) { if (!['.git', 'node_modules', 'out'].includes(entry.name)) { await walk(fullPath, results); } }
		else if (entry.isFile() && entry.name.toLowerCase().endsWith('.c')) { results.push(fullPath); }
	}
}
function isEnforceDocument(document: vscode.TextDocument): boolean { return document.uri.scheme === 'file' && document.fileName.toLowerCase().endsWith('.c'); }
function normalizePath(filePath: string): string { return path.resolve(filePath).replace(/\\/g, '/').toLowerCase(); }
async function normalizeRealPath(filePath: string): Promise<string> { try { return normalizePath(await fs.realpath(filePath)); } catch { return normalizePath(filePath); } }
function shortName(uri: vscode.Uri): string { return uri.fsPath.split(/[\\/]/).pop() ?? uri.toString(); }
function escapeRegExp(value: string): string { return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }
function getLineStarts(text: string): number[] { const starts = [0]; for (let i = 0; i < text.length; i++) { if (text[i] === '\n') { starts.push(i + 1); } } return starts; }
function positionAtOffset(lineStarts: number[], offset: number): vscode.Position {
	let low = 0; let high = lineStarts.length - 1;
	while (low <= high) { const mid = Math.floor((low + high) / 2); if (lineStarts[mid] <= offset) { low = mid + 1; } else { high = mid - 1; } }
	const line = Math.max(0, low - 1); return new vscode.Position(line, offset - lineStarts[line]);
}
function isInsideCommentOrString(text: string, offset: number): boolean {
	let line = false; let block = false; let string: '"' | "'" | undefined;
	for (let i = 0; i < offset; i++) { const c = text[i]; const n = text[i + 1]; if (line) { if (c === '\n' || c === '\r') { line = false; } continue; } if (block) { if (c === '*' && n === '/') { block = false; i++; } continue; } if (string) { if (c === '\\') { i++; } else if (c === string) { string = undefined; } continue; } if (c === '/' && n === '/') { line = true; i++; } else if (c === '/' && n === '*') { block = true; i++; } else if (c === '"' || c === "'") { string = c; } }
	return line || block || string !== undefined;
}
function fmt(value: number): string { return `${value.toFixed(2)}ms`; }
