# server/src/lsp.rs

## Purpose

Owns the first minimal Language Server Protocol implementation for the Rust language engine.

## Architecture Role

This file sits above parser, AST, model, index, query, and display layers. It converts LSP requests into calls against existing language-tooling APIs and returns protocol-shaped responses over stdio.

## Current Behavior

The server handles `initialize`, `initialized`, `shutdown`, `exit`, full text-document sync notifications, `textDocument/documentSymbol`, and `textDocument/hover`. Open documents are stored in memory by URI. Document symbols and hover results are built from the current document text through parser, AST, model, file-local index, `IndexQuery`, and `SymbolDisplay`.

Hover is file-local only. It converts LSP UTF-16 positions to byte offsets, selects the smallest indexed symbol whose selection or declaration span contains the position, and returns compact Markdown from `SymbolDisplayInfo`. Parameters are included in hover matching even though document symbols omit them. Hover does not do semantic resolution, game-data lookup, inherited member lookup, Workbench validation, or workspace-wide indexing.

The server writes concise human-review logs to the optional `--log` path. Startup logs record server version and game-data path provenance. `didOpen`, `didChange`, and `documentSymbol` logs include URI, byte length, document-symbol count, parse-diagnostic count, and elapsed milliseconds. Hover logs include URI, byte length, hit/miss state, selected label/kind when available, parse-diagnostic count, and elapsed milliseconds. Runtime logs must stay summary-only; they must not include full source text, full symbol trees, AST dumps, index dumps, or full hover Markdown.

`server/examples/lsp_report.rs` provides a dev-only fixture report for the current document-symbol path. It scans committed parser fixtures and writes `tools/reports/lsp-fixtures.report.md` with per-file parse diagnostics, symbol counts, max tree depth, unknown labels, range sanity, and bounded symbol trees.

`server/examples/lsp_corpus_report.rs` provides the corpus-scale version for downloaded or explicit game-data scripts. It writes `tools/reports/lsp-corpus.report.md` with aggregate document-symbol counts, kind frequency, zero-symbol files, failure tables, top symbol-heavy files, deepest files, slowest files, and timing.

`server/examples/lsp_hover_report.rs` provides a dev-only fixture report for hover. It writes `tools/reports/lsp-hover-fixtures.report.md` with targeted hover checks for class, field, method, parameter, typedef, enum member, global field, and whitespace miss behavior.

## Dependencies and Boundaries

Uses `serde` and `serde_json` for LSP JSON. It must not call VS Code APIs, call Workbench, download game data, persist caches, perform semantic resolution, or implement editor features directly in TypeScript.

This is a minimal protocol scaffold. Do not expand it into a broad framework unless a concrete LSP feature requires that structure.

## Change Notes

- Added stdio `Content-Length` message framing.
- Added JSON-RPC lifecycle handling and full document synchronization.
- Added document-symbol support using existing source-backed declaration layers.
- Added file-local hover support using indexed spans and `SymbolDisplay` Markdown presentation.
- Added file logging for startup and request timing.
- Added parse-diagnostic and symbol-count logging for open/change/document-symbol operations.
- Added hover hit/miss, selected label/kind, parse-diagnostic, and timing logs.
- Added the dev-only LSP fixture report for document-symbol review.
- Added the dev-only LSP corpus report for corpus-scale document-symbol projection review.
- Added the dev-only LSP hover fixture report for targeted hover review.

## Future Improvements

- Add diagnostics, completion, definition, and references in separate verified slices.
- Add semantic hover expansion later through workspace/game-data lookup, not through file-local LSP shortcuts.
- Add workspace/game-data index startup only after the server lifecycle is proven in VS Code.
- Replace or harden protocol plumbing only if future feature complexity justifies it.
