# server/src/lsp.rs

## Purpose

Owns the first minimal Language Server Protocol implementation for the Rust language engine.

## Architecture Role

This file sits above parser, AST, model, index, query, and display layers. It converts LSP requests into calls against existing language-tooling APIs and returns protocol-shaped responses over stdio.

## Current Behavior

The server handles `initialize`, `initialized`, `shutdown`, `exit`, full text-document sync notifications, and `textDocument/documentSymbol`. Open documents are stored in memory by URI. Document symbols are built from the current document text through parser, AST, model, file-local index, `IndexQuery`, and `SymbolDisplay`.

The server writes concise human-review logs to the optional `--log` path. Startup logs record server version and game-data path provenance. `didOpen`, `didChange`, and `documentSymbol` logs include URI, byte length, document-symbol count, parse-diagnostic count, and elapsed milliseconds. Runtime logs must stay summary-only; they must not include full source text, full symbol trees, AST dumps, or index dumps.

`server/examples/lsp_report.rs` provides a dev-only fixture report for the current document-symbol path. It scans committed parser fixtures and writes `tools/reports/lsp-fixtures.report.md` with per-file parse diagnostics, symbol counts, max tree depth, unknown labels, range sanity, and bounded symbol trees.

`server/examples/lsp_corpus_report.rs` provides the corpus-scale version for downloaded or explicit game-data scripts. It writes `tools/reports/lsp-corpus.report.md` with aggregate document-symbol counts, kind frequency, zero-symbol files, failure tables, top symbol-heavy files, deepest files, slowest files, and timing.

## Dependencies and Boundaries

Uses `serde` and `serde_json` for LSP JSON. It must not call VS Code APIs, call Workbench, download game data, persist caches, perform semantic resolution, or implement editor features directly in TypeScript.

This is a minimal protocol scaffold. Do not expand it into a broad framework unless a concrete LSP feature requires that structure.

## Change Notes

- Added stdio `Content-Length` message framing.
- Added JSON-RPC lifecycle handling and full document synchronization.
- Added document-symbol support using existing source-backed declaration layers.
- Added file logging for startup and request timing.
- Added parse-diagnostic and symbol-count logging for open/change/document-symbol operations.
- Added the dev-only LSP fixture report for document-symbol review.
- Added the dev-only LSP corpus report for corpus-scale document-symbol projection review.

## Future Improvements

- Add diagnostics, hover, completion, definition, and references in separate verified slices.
- Add workspace/game-data index startup only after the server lifecycle is proven in VS Code.
- Replace or harden protocol plumbing only if future feature complexity justifies it.
