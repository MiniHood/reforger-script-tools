# server/src/lsp.rs

## Purpose

Owns the first minimal Language Server Protocol implementation for the Rust language engine.

## Architecture Role

This file sits above parser, AST, model, index, query, and display layers. It converts LSP requests into calls against existing language-tooling APIs and returns protocol-shaped responses over stdio.

## Current Behavior

The server handles `initialize`, `initialized`, `shutdown`, `exit`, full text-document sync notifications, `textDocument/documentSymbol`, `textDocument/hover`, `textDocument/definition`, and the custom debug request `reforger/debugHover`. Open documents are stored in memory by URI with the current full source text, optional LSP version, an internal revision counter, and cached file-local analysis. The cached analysis is rebuilt synchronously on `didOpen` and full-sync `didChange`, then reused by document symbols, hover, definition, and debug-hover until the document changes again. Document symbols, hover results, definition locations, and hover debug reports are built from that cached parser -> AST -> model -> file-local index analysis through the resolver, `IndexQuery`, and `SymbolDisplay` where presentation is needed. When the TypeScript client provides game-data paths, the server starts a background load/build of a cached game-data index and uses it as external resolver context after it becomes ready.

Hover converts LSP UTF-16 positions to byte offsets, asks `server/src/resolver.rs` for the hover target, and returns compact Markdown from `SymbolDisplayInfo`. The resolver owns both identifier resolution and syntax-span hover for non-identifier positions inside declaration spans, so LSP does not maintain a second hover selection path. File-local candidates still win over external candidates. Once the background game-data index is ready, unresolved type-like/top-level identifiers can resolve to external game-data symbols such as engine classes and generated API declarations. Receiver/member-call hover uses resolver-owned AST expression views for simple receivers such as locals, parameters, fields, `this`, `super`, static type names, enum static members, known-return call receivers, `Type.Cast(...)`, and simple chains. Named argument labels are suppressed by the resolver so labels such as `level:` and `desc:` do not appear as unresolved hovers. The external game-data cache is runtime-pruned and intentionally omits local variables, but open-document analysis remains full and still supports local-variable hover in the active file. If the external index is missing, building, failed, or unavailable, hover continues with file-local behavior. Hover still does not perform full expression typing, overload resolution, Workbench validation, or workspace indexing.

Definition uses the same resolver selection path as hover for identifier tokens only. File-local targets return the current open-document URI with the selected symbol name range. External targets return a `file://` URI built from indexed absolute path metadata and the selected symbol name range after reading that external source file for line/column conversion. Definition returns `null` for non-identifiers, named argument labels, unresolved identifiers, and external candidates without a readable absolute source path. It returns `Location[]`, not `LocationLink`, in this first slice.

The server writes concise human-review logs to the optional `--log` path. Startup logs record server version, game-data path provenance, cache path, and external-index status. Background index logs record load/build start, cache status, file/symbol counts, parse diagnostics, and elapsed milliseconds. Foreground LSP request logs and background index logs share one mutex-backed logger so concurrent writes do not interleave or corrupt log lines. `didOpen` and `didChange` logs include URI, byte length, LSP version when available, internal revision, analysis build time, document-symbol count, and parse-diagnostic count. `documentSymbol`, `hover`, `definition`, and `debugHover` logs include the cached document revision and mark that cached analysis was used. Hover and definition logs include URI, byte length, hit/miss state, selected source, resolver reason, identifier context, resolver candidate count, external-index status, selected label/kind when available, parse-diagnostic count, and elapsed milliseconds. Runtime logs must stay summary-only; they must not include full source text, full symbol trees, AST dumps, index dumps, or full hover Markdown.

`reforger/debugHover` is intentionally heavier than normal hover logging and should only run from targeted debug commands. It returns a human-readable Markdown/plain-text report for the requested open-document position, including source line context, lexer tokens around the cursor, expected TextMate/theme coloring for nearby tokens, parse diagnostics, resolver resolution, selected hover symbol display facts, all span candidate symbols under the cursor, parent/child context, hover Markdown, and symbol-kind counts. Theme/color entries are derived from the Enforce lexer plus the bundled grammar/theme palette because VS Code does not expose active TextMate color inspection through the normal extension API. This debug request may serialize bounded language-engine details because it is user-triggered and not a hot-path background log.

`server/examples/lsp_report.rs` provides a dev-only fixture report for the current document-symbol path. It scans committed parser fixtures and writes `tools/reports/lsp-fixtures.report.md` with per-file parse diagnostics, symbol counts, max tree depth, unknown labels, range sanity, and bounded symbol trees.

`server/examples/lsp_corpus_report.rs` provides the corpus-scale version for downloaded or explicit game-data scripts. It writes `tools/reports/lsp-corpus.report.md` with aggregate document-symbol counts, kind frequency, zero-symbol files, failure tables, top symbol-heavy files, deepest files, slowest files, and timing.

`server/examples/lsp_hover_report.rs` provides a dev-only fixture report for hover. It writes `tools/reports/lsp-hover-fixtures.report.md` with targeted hover checks for class, field, method, parameter, typedef, enum member, global field, receiver/member access, and whitespace miss behavior.
It also covers local variable, `foreach` variable, and `for` initializer hover checks from a committed local/block-symbol fixture.

`server/examples/lsp_hover_corpus_report.rs` provides the corpus-scale version for hover. It samples identifier-token positions across downloaded or explicit game-data scripts and writes `tools/reports/lsp-hover-corpus.report.md` with hit/miss counts, resolver reason frequency, identifier context frequency, receiver owner/failure frequency, selected kind frequency, bounded samples, and timing.

`server/examples/lsp_definition_report.rs` provides a dev-only fixture report for definition. It writes `tools/reports/lsp-definition-fixtures.report.md` with targeted definition checks for local and external targets.

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
- Added expected token scope/theme color context to hover debug reports for theme troubleshooting.
- Added file logging for startup and request timing.
- Added parse-diagnostic and symbol-count logging for open/change/document-symbol operations.
- Added hover hit/miss, selected label/kind, parse-diagnostic, and timing logs.
- Serialized foreground/background runtime log writes through one shared logger to prevent interleaved log lines.
- Added cached open-document analysis so live document symbols, hover, and debug-hover do not reparse/reindex unchanged open files per request.
- Added the dev-only LSP fixture report for document-symbol review.
- Added the dev-only LSP corpus report for corpus-scale document-symbol projection review.
- Added the dev-only LSP hover fixture report for targeted hover review.
- Added the dev-only LSP hover corpus report for sampled corpus-scale resolver-first hover review.
- Added file-local hover support for local/block symbols while keeping local variables out of document symbols.
- Added resolver metadata to hover logs and debug-hover reports.
- Added type-position resolver context to hover logs, debug-hover reports, and hover fixture/corpus reports.
- Added background game-data index loading/caching and external-index resolver context for hover/debug-hover.
- Added shallow receiver/member-call resolver context to hover logs, debug-hover reports, and hover fixture/corpus reports.
- Moved receiver/member-call hover context to AST expression views and added receiver expression kind to debug/report output.
- Suppressed named argument labels from reference hover resolution.
- Added resolver-backed `textDocument/definition` support with file-local and external `Location[]` targets.
- Added the dev-only LSP definition fixture report for targeted Ctrl+click review.

## Future Improvements

- Add diagnostics, completion, and references in separate verified slices.
- Add semantic hover expansion through workspace/game-data lookup, not through file-local LSP shortcuts.
- Expand definition with workspace index targets after workspace indexing is wired into the LSP runtime.
- Add workspace indexing separately from game-data indexing, with explicit workspace priority and invalidation.
- Add incremental parse/index updates if full-sync analysis on every edit becomes too costly.
- Replace or harden protocol plumbing only if future feature complexity justifies it.
