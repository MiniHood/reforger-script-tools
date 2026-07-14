# server/src/lsp.rs

## Purpose

Owns the first minimal Language Server Protocol implementation for the Rust language engine.

## Architecture Role

This file sits above parser, AST, model, index, query, and display layers. It converts LSP requests into calls against existing language-tooling APIs and returns protocol-shaped responses over stdio.

Feature projection code is split into focused child modules:

- `server/src/lsp/completion.rs` owns completion reports, candidate combination, item rendering, sorting, and timing.
- `server/src/lsp/debug_hover.rs` owns the custom `reforger/debugHover` Markdown report.
- `server/src/lsp/definition.rs` owns definition reports and `Location[]` URI/range projection.
- `server/src/lsp/diagnostics.rs` owns parser diagnostic conversion and publish/clear messages.
- `server/src/lsp/external_overlay.rs` owns runtime game-data/workspace overlay indexing, update/delete handling, recompute, and status summaries.
- `server/src/lsp/hover.rs` owns normal hover selection and hover report construction.
- `server/src/lsp/hover_render.rs` owns normal hover Markdown presentation.
- `server/src/lsp/open_documents.rs` owns open-document source text, revision, and cached file-local analysis.
- `server/src/lsp/semantic_tokens.rs` owns semantic-token legend, projection, and debug rows.

`lsp.rs` should keep request dispatch, runtime logging, and shared protocol helpers. Do not put substantial feature projection logic, open-document analysis ownership, or external overlay indexing internals back into this file when a child module owns that feature.

## Current Behavior

The server handles `initialize`, `initialized`, `shutdown`, `exit`, full text-document sync notifications, `textDocument/documentSymbol`, `textDocument/hover`, `textDocument/definition`, `textDocument/completion`, `textDocument/semanticTokens/full`, live workspace overlay notifications, parser-diagnostic publishing, and the custom debug request `reforger/debugHover`. Open documents are stored in memory by URI with the current full source text, optional LSP version, an internal revision counter, cached file-local analysis, and cached document-symbol projection. The cached analysis and document-symbol projection are rebuilt synchronously on `didOpen` and full-sync `didChange`, then reused by document symbols, hover, definition, completion, semantic tokens, parser diagnostics, and debug-hover until the document changes again. Cached analysis includes parse, file-local index, lexical scope model, parser diagnostic count, and parser diagnostics. Document symbols are projected once per open-document revision, while hover results, definition locations, completion items, semantic tokens, parser diagnostics, and hover debug reports are built from cached parser -> AST -> model -> file-local index -> lexical scope analysis through the resolver, `IndexQuery`, and `SymbolDisplay` where presentation is needed. Semantic tokens additionally keep a rich-token cache keyed by open-document revision and external overlay generation. When the TypeScript client provides game-data paths, the server starts a background load/build of a cached game-data index. Debug and release builds both load game data when paths are configured, because development hover/definition must see the same external source set as packaged builds. When the TypeScript client provides workspace script roots, the server also builds a live workspace index. Resolver consumers see one external overlay with workspace priority `200` before game-data priority `100` when those indexes are available.

Hover converts LSP UTF-16 positions to byte offsets, asks `server/src/resolver.rs` for the hover target, and returns compact Markdown from `SymbolDisplayInfo`. The resolver owns both identifier resolution and syntax-span hover for non-identifier positions inside declaration spans, so LSP does not maintain a second hover selection path. File-local candidates still win over external candidates. Once the external overlay is ready, unresolved type-like/top-level identifiers can resolve to workspace or game-data symbols such as mod classes, engine classes, and generated API declarations. Receiver/member-call hover uses resolver-owned AST expression views for simple receivers such as locals, parameters, fields, `this`, `super`, static type names, enum static members, known-return call receivers, `Type.Cast(...)`, and simple chains. Named argument labels are suppressed by the resolver so labels such as `level:` and `desc:` do not appear as unresolved hovers. The external game-data cache is runtime-pruned and intentionally omits local variables, but open-document analysis remains full and still supports local-variable hover in the active file. Workspace external indexes are live and uncached. If the external index is missing, building, failed, or unavailable, hover continues with file-local behavior. Hover still does not perform full expression typing, overload resolution, Workbench validation, or semantic `modded` merging.

Definition uses the same resolver selection path as hover for identifier tokens only. File-local targets return the current open-document URI with origin selection range, full target declaration range, and selected symbol name range. External targets return a `file://` URI built from indexed absolute path metadata with target ranges computed after reading that external source file. Workspace targets are preferred over game-data targets when the resolver selects a workspace overlay symbol. Definition returns `null` for non-identifiers, named argument labels, unresolved identifiers, and external candidates without a readable absolute source path. Live LSP definition responses use `LocationLink[]`; definition reports retain compatibility `Location[]` rows derived from target selection ranges.

Completion currently covers member access plus prefix-based type/top-level identifiers. The server advertises `"."` as the trigger character and also responds to manual completion requests on identifier prefixes such as `SCR_`, `array<SCR_`, and `GetG`. The resolver owns member-completion context detection, receiver type inference, static-owner detection, and top-level/type prefix context detection. Member completion uses the full receiver expression ending at the dot, so chained receivers such as `wrapper.m_Value.` complete against the final inferred owner rather than the last field name alone. Completion queries the open-document file-local index and the external workspace/game-data overlay index separately, then combines the returned candidates with open-document candidates first. It must not build a full merged `SymbolIndex` per completion request. Instance member completion maps `IndexQuery::completion_members_for_class` candidates into completion items. Static-owner completion maps `IndexQuery::completion_static_members_for_type` candidates into completion items, so enum owners such as `LogLevel.` expose enum members and class owners expose copied static fields/methods plus the source-backed engine `Class.Cast` method when indexed. Direct `new Type(...)` receivers infer `Type` for member completion and hover. Typedef owner completion expands source-backed typedef targets such as `typedef array<int> TIntArray;` to the target owner where practical. Type completion returns class, enum, and typedef candidates. Top-level value/callable completion returns source-backed classes, enums, typedefs, functions, global fields, and enum members. Completion items use `SymbolDisplay` labels, details/signatures, documentation previews, label details for callable parameter/return shapes, source-aware sort text, and text edits that replace only the typed prefix. Callable completions insert simple call text such as `Run()` or `SetVisible(visible)`. It does not provide placeholder snippets, `completionItem/resolve`, full overload UI, diagnostics, fuzzy matching, semantic typedef/generic instantiation, or semantic `modded` merge behavior in this slice.

Semantic tokens are the only Enforce coloring path. The server advertises a fixed semantic-token legend and computes full-document tokens from lexer facts plus cached index symbols. Lexer facts color comments, strings, numbers, keywords, operators, punctuation, preprocessor lines, and source-backed attribute syntax. Indexed symbol facts color declarations such as classes, enums, typedefs, functions, methods, fields, parameters, local variables, enum members, and type parameters, with modifiers such as declaration, static, readonly, and modification where available. To keep first paint responsive on large files, live semantic-token requests use a two-tier path: the first request for a document revision returns fast lexer/declaration/source-shape tokens, then the server computes one resolver-backed rich projection for that revision, caches it, and requests `workspace/semanticTokens/refresh`. The follow-up request returns the cached rich projection. The stdio runtime polls for external overlay generation changes while idle, so game-data/workspace index readiness can request semantic-token refreshes without waiting for a hover or other editor request. The old TextMate grammar path is intentionally removed so token coloring has one source of truth: Rust LSP semantic tokens consumed by the bundled semantic-token theme.

Parser diagnostics are published through standard `textDocument/publishDiagnostics` notifications on `didOpen` and full-sync `didChange`, then cleared on `didClose`. These diagnostics come only from the extension parser and lexer and use source `Reforger Script Tools parser` with stable code `reforger.parser.syntax`. They are quick editor feedback, not Workbench/compiler validation, and they must not be expanded into semantic diagnostics without a separate verified slice. Parser diagnostic projection expands zero-width parser spans to a nearby visible editor range where possible so recovery diagnostics are easier to find in VS Code.

The custom notifications `reforger/workspaceFileChanged` and `reforger/workspaceFileDeleted` update the live workspace side of the external overlay. Changed files are parsed from full text sent by the TypeScript watcher; deleted files remove stale workspace symbols. Workspace indexing is intentionally not persisted in this slice.

The server writes concise human-review logs to the optional `--log` path. Startup logs record server version, game-data path provenance, cache path, and external-index status. Background index logs record load/build start, cache status, file/symbol counts, parse diagnostics, and elapsed milliseconds. Foreground LSP request logs and background index logs share one mutex-backed logger so concurrent writes do not interleave or corrupt log lines. `didOpen` and `didChange` logs include URI, byte length, LSP version when available, internal revision, analysis build time, document-symbol count, document-symbol cache state, and parse-diagnostic count. `documentSymbol`, `hover`, `definition`, and `debugHover` logs include the cached document revision and mark that cached analysis was used; `documentSymbol` logs also mark that cached document-symbol projection was used. Hover and definition logs include URI, byte length, hit/miss state, selected source, resolver reason, identifier context, resolver candidate count, external-index status, selected label/kind when available, parse-diagnostic count, and elapsed milliseconds. Semantic-token logs include projection mode (`fast-compute` or `rich-cache`), token counts, external-index status, phase timings, resolver call count, and elapsed time; rich projection preparation logs separately when the cached projection is ready and a refresh is requested. Runtime logs must stay summary-only; they must not include full source text, full symbol trees, AST dumps, index dumps, or full hover Markdown.

`reforger/debugHover` is intentionally heavier than normal hover logging and should only run from targeted debug commands. It returns a human-readable Markdown/plain-text report for the requested open-document position, including source line context, lexer tokens around the cursor, semantic-token coloring for nearby tokens, parse diagnostics, resolver resolution, selected hover symbol display facts, all span candidate symbols under the cursor, parent/child context, hover Markdown, and symbol-kind counts. Theme/color entries are derived from the same Rust semantic-token builder used by `textDocument/semanticTokens/full`, so hover debug reflects the server-side classification rather than a separate grammar approximation. This debug request may serialize bounded language-engine details because it is user-triggered and not a hot-path background log.

`server/examples/lsp_report.rs` provides a dev-only fixture report for the current document-symbol path. It scans committed parser fixtures and writes `tools/reports/lsp-fixtures.report.md` with per-file parse diagnostics, symbol counts, max tree depth, unknown labels, range sanity, and bounded symbol trees.

`server/examples/lsp_corpus_report.rs` provides the corpus-scale version for downloaded or explicit game-data scripts. It writes `tools/reports/lsp-corpus.report.md` with aggregate document-symbol counts, kind frequency, zero-symbol files, failure tables, top symbol-heavy files, deepest files, slowest files, and timing.

`server/examples/lsp_hover_report.rs` provides a dev-only fixture report for hover. It writes `tools/reports/lsp-hover-fixtures.report.md` with targeted hover checks for class, field, method, parameter, typedef, enum member, global field, receiver/member access, and whitespace miss behavior.
It also covers local variable, `foreach` variable, and `for` initializer hover checks from a committed local/block-symbol fixture.

`server/examples/lsp_hover_corpus_report.rs` provides the corpus-scale version for hover. It samples identifier-token positions across downloaded or explicit game-data scripts and writes `tools/reports/lsp-hover-corpus.report.md` with hit/miss counts, resolver reason frequency, identifier context frequency, receiver owner/failure frequency, selected kind frequency, bounded samples, and timing.

`server/examples/lsp_definition_report.rs` provides a dev-only fixture report for definition. It writes `tools/reports/lsp-definition-fixtures.report.md` with targeted definition checks for local and external targets.

`server/examples/lsp_definition_corpus_report.rs` provides a corpus-scale definition report. It writes `tools/reports/lsp-definition-corpus.report.md` with evenly sampled real identifier positions, hit rate, resolver reason frequency, identifier context frequency, selected source/kind frequency, miss classification, miss samples, and timing.

`server/examples/lsp_workspace_overlay_report.rs` provides a dev-only report for workspace overlay behavior. It writes `tools/reports/lsp-workspace-overlay.report.md` with checks for workspace-over-game-data hover, update behavior, delete behavior, and definition targets.

`server/examples/lsp_completion_report.rs` provides a dev-only fixture report for completion. It writes `tools/reports/lsp-completion-fixtures.report.md` with completion context, receiver inference, owner type, prefix, candidate count, and sample items for member, type-prefix, top-level-prefix, workspace overlay, delete, and miss cases.

`server/examples/lsp_completion_corpus_report.rs` provides a corpus-scale completion report. It writes `tools/reports/lsp-completion-corpus.report.md` with evenly sampled real member-access positions, cached per-file analysis reuse, completion context frequency, failure reason frequency, empty-result classification, candidate-count buckets, top inferred owner types, empty/failure samples, large-candidate samples, and timing. The timing section splits completion projection into context/receiver detection, candidate lookup, item rendering, and total reported completion time.

`server/examples/lsp_diagnostics_report.rs` provides a dev-only fixture report for parser diagnostic UX. It writes `tools/reports/lsp-diagnostics-fixtures.report.md` with focused malformed-source cases, LSP-projected diagnostic messages, sources, codes, severities, ranges, and bounded snippets.

`server/examples/lsp_diagnostics_corpus_report.rs` provides the malformed-fixture corpus version for parser diagnostic UX. It scans committed fixtures under `tools/fixtures/diagnostics/` and writes `tools/reports/lsp-diagnostics-corpus.report.md` with diagnostic message frequency, zero-diagnostic malformed files, range quality issues, and bounded snippets.

`server/examples/lsp_semantic_tokens_report.rs` provides a dev-only fixture report for semantic-token coloring. It writes `tools/reports/lsp-semantic-tokens-fixtures.report.md` with decoded semantic token text, ranges, token types, modifiers, and palette colors.

`server/examples/lsp_semantic_tokens_corpus_report.rs` provides a corpus-scale semantic-token report. It writes `tools/reports/lsp-semantic-tokens-corpus.report.md` with token type frequency, modifier frequency, identifier coloring coverage, declaration/reference/lexical token split, weakest coverage files, uncolored identifier classification, uncolored identifier samples, and timing.

## Dependencies and Boundaries

Uses `serde` and `serde_json` for LSP JSON and delegates disposable game-data index caching to `server/src/index_cache.rs`. It must not call VS Code APIs, call Workbench, download game data, perform semantic resolution, or implement editor features directly in TypeScript.

This is a minimal protocol scaffold. Do not expand it into a broad framework unless a concrete LSP feature requires that structure.

## Change Notes

- Added stdio `Content-Length` message framing.
- Added JSON-RPC lifecycle handling and full document synchronization.
- Added document-symbol support using existing source-backed declaration layers.
- Added file-local hover support using indexed spans and `SymbolDisplay` Markdown presentation.
- Updated hover so resolver owns both identifier and non-identifier syntax-span hover selection.
- Added the custom `reforger/debugHover` request for cursor-position hover debugging from the VS Code command layer.
- Added semantic-token theme color context to hover debug reports for coloring troubleshooting.
- Added fast-first semantic-token projection plus revision/external-generation-bound rich semantic-token caching and refresh for large-file first-paint responsiveness.
- Added idle stdio polling for external overlay generation changes so semantic-token refreshes are requested when game-data/workspace symbols become ready, rather than waiting for hover or another editor request.
- Added file logging for startup and request timing.
- Added parse-diagnostic and symbol-count logging for open/change/document-symbol operations.
- Added hover hit/miss, selected label/kind, parse-diagnostic, and timing logs.
- Serialized foreground/background runtime log writes through one shared logger to prevent interleaved log lines.
- Added cached open-document analysis so live document symbols, hover, and debug-hover do not reparse/reindex unchanged open files per request.
- Cached lexical scope in open-document analysis and routed resolver local/parameter lookup through that scope model.
- Added the dev-only LSP fixture report for document-symbol review.
- Added the dev-only LSP corpus report for corpus-scale document-symbol projection review.
- Added the dev-only LSP hover fixture report for targeted hover review.
- Added the dev-only LSP hover corpus report for sampled corpus-scale resolver-first hover review.
- Added file-local hover support for local/block symbols while keeping local variables out of document symbols.
- Added resolver metadata to hover logs and debug-hover reports.
- Added type-position resolver context to hover logs, debug-hover reports, and hover fixture/corpus reports.
- Added background game-data index loading/caching and external-index resolver context for hover/debug-hover.
- Added live workspace index overlay support with workspace priority over game-data priority.
- Removed the debug-build-only game-data cache-load skip so development hover/definition can resolve external game-data types consistently with release/package builds.
- Added shallow receiver/member-call resolver context to hover logs, debug-hover reports, and hover fixture/corpus reports.
- Moved receiver/member-call hover context to AST expression views and added receiver expression kind to debug/report output.
- Suppressed named argument labels from reference hover resolution.
- Added resolver-backed `textDocument/definition` support with file-local and external `Location[]` targets.
- Moved live definition responses to `LocationLink[]` while preserving report compatibility `Location[]` data.
- Added the dev-only LSP definition fixture report for targeted Ctrl+click review.
- Added the dev-only LSP definition corpus report for sampled game-data Ctrl+click review.
- Added the dev-only LSP workspace overlay report.
- Added member, type-prefix, and top-level-prefix `textDocument/completion` support and the dev-only LSP completion fixture report.
- Added the dev-only LSP completion corpus report for sampled game-data member completion review.
- Updated completion corpus reporting to reuse cached per-file analysis and classify empty member-completion results.
- Removed per-request full-index merging from completion projection; completion now combines file-local and external query results without rebuilding a merged index.
- Added static-owner completion for enum members/static class members, source-backed engine `Class.Cast`, and typedef-owner expansion for member completion.
- Added full receiver-expression member completion and direct `new Type(...)` receiver inference.
- Added `textDocument/semanticTokens/full` support and removed TextMate as an Enforce coloring path.
- Added parser-only `textDocument/publishDiagnostics` notifications from cached open-document analysis.
- Added stable parser diagnostic codes, visible-range projection for zero-width parser diagnostics, and the dev-only diagnostics fixture report.
- Added committed malformed diagnostic fixtures and the dev-only diagnostics corpus report for parser diagnostic message/range quality review.
- Added the dev-only LSP semantic-token fixture report.
- Added the dev-only LSP semantic-token corpus report for game-data coloring coverage review.
- Updated definition and semantic-token corpus reports with less biased sampling/classification for easier high-signal review.
- Split diagnostics, semantic-token projection, completion projection, definition projection, hover rendering, and debug-hover rendering into child modules while keeping request dispatch in `lsp.rs`.
- Split open-document analysis/cache ownership into `lsp/open_documents.rs` while keeping request dispatch in `lsp.rs`.
- Split runtime external overlay ownership into `lsp/external_overlay.rs` while keeping request dispatch in `lsp.rs`.
- Cached document-symbol projection per open-document revision so repeated Outline/document-symbol requests do not rebuild the LSP symbol tree from the index.

## Future Improvements

- Add semantic diagnostics and references in separate verified slices.
- Add completion snippets, overload UI polish, and broader completion ranking in separate verified slices.
- Add semantic-token range/delta support only if full-document semantic token refresh becomes too costly.
- Add semantic hover expansion through workspace/game-data lookup, not through file-local LSP shortcuts.
- Add semantic `modded` merge behavior only after source-backed overlay behavior is reviewed.
- Add incremental parse/index updates if full-sync analysis on every edit becomes too costly.
- Replace or harden protocol plumbing only if future feature complexity justifies it.
