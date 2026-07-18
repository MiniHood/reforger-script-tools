# server/src/lsp/open_documents.rs

## Purpose

Owns open-document state and cached file-local language analysis for the Rust LSP server.

## Architecture Role

This child module sits under `server/src/lsp.rs`. It keeps document text, optional LSP version, internal revision, and cached parser/AST/model/index/scope analysis together so request dispatch can reuse one authoritative open-file analysis across document symbols, hover, definition, completion, diagnostics, semantic tokens, and debug-hover.

## Current Behavior

`OpenDocument::new` builds `FileIndexAnalysis` from full source text and records split analysis timings for parse, catalog, index, and scope construction. `OpenDocument::replace` updates the full text, optional version, increments the internal revision, rebuilds analysis synchronously, clears the document-symbol projection, and clears revision-bound semantic-token cache data. `file_index_for_source` is still re-exported from `lsp.rs` for reports and tests. `FileIndexAnalysis` contains the parse tree, cached lexer tokens, file-local index, lexical scope model, parse diagnostic count, and cloned diagnostics. The tokens are created in the same full-sync replacement transaction and reused by live completion and semantic-token projection.

Each open document stores the projected `LspDocumentSymbol` tree for the current revision. `lsp.rs` refreshes that projection immediately after `didOpen`, but `didChange` only invalidates it. The next live `textDocument/documentSymbol` request rebuilds the projection lazily for that revision, then later document-symbol requests serialize the cached tree. This keeps Outline projection off the edit/autocomplete hot path for large files while preserving one source-backed projection path.

Each open document also owns a `SemanticTokenCache` for rich semantic tokens computed for the current revision and external overlay generation. Live semantic-token requests may return a fast lexical/declaration projection first, then mark a resolver-backed rich projection pending. A single bounded scheduler waits briefly for editor idle before projection. It keeps only the latest pending job for each document, caps distinct pending documents, and cancels evicted or superseded work. The worker result is cached only when it still matches the open document revision and external generation, then the server requests a semantic-token refresh. The cache is invalidated by `OpenDocument::replace` and naturally bypassed when the workspace/game-data overlay generation changes.

## Dependencies and Boundaries

This module depends on parser, AST, model, index, and scope layers. It must not handle JSON-RPC dispatch, LSP wire formatting, external workspace/game-data overlay state, runtime logging, or editor feature projection. It must not add incremental parsing; full-sync rebuild remains the current behavior.

## Change Notes

- Extracted open-document cache ownership from `lsp.rs` without changing request behavior.
- Added current-revision document-symbol projection storage so repeated Outline/document-symbol requests reuse one projected symbol tree.
- Made document-symbol projection lazy after `didChange` so large-file edits rebuild parser/index/scope immediately but do not also rebuild Outline data before completion, hover, definition, or diagnostics can run.
- Added split open-document analysis timings for parse, model catalog, index aggregation, and lexical scope construction.
- Added revision-bound rich semantic-token cache storage so large-file coloring can return a fast first projection and reuse one resolver-backed projection for follow-up refreshes.
- Rich semantic-token cache entries are also keyed by external overlay generation so type coloring computed before game-data/workspace indexes are ready cannot survive after those indexes become available.
- Live stdio rich semantic-token projection is worker-scheduled so large-file rich coloring does not block hover, completion, definition, or later protocol messages on the main request loop.
- Added a pending rich-projection marker so repeated semantic-token requests cannot spawn duplicate rich workers for the same open-document revision and external generation.
- Added an idle-delay gate before rich semantic-token workers start, so rapid large-file edits skip obsolete rich projections before paying resolver cost.

## Future Improvements

- Add incremental analysis only if full-sync rebuild on `didChange` becomes a measured hotspot.
- Keep any future cache invalidation rules local to this module so feature handlers continue to consume one cached analysis value.
