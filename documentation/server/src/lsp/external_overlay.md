# server/src/lsp/external_overlay.rs

## Purpose

Owns the runtime external symbol overlay used by LSP features.

## Architecture Role

This module is part of the Rust LSP layer. It manages the background game-data index load/build, live workspace script indexing, workspace file update/delete handling, overlay recomputation, and status summaries consumed by request dispatch.

## Current Behavior

The module builds a combined external `SymbolIndex` from workspace scripts with priority `200` and game-data scripts with priority `100`. Workspace files are parsed and indexed from configured script roots at startup, then updated from full-text custom LSP notifications. Game data is loaded through the disposable runtime cache path when configured. Debug and release builds both load game data when paths are configured, so hover, definition, completion, and semantic-token resolution see the same external source set during development and packaged use.

Request handlers call `ExternalIndexHandle::with_index` to borrow the current overlay index for hover, definition, completion, semantic tokens, and debug hover. They call `status_summary` for concise logs and debug output. The status summary includes an overlay generation that increments whenever the combined external index changes, allowing cached editor projections such as rich semantic tokens to avoid reusing results computed against stale workspace/game-data facts.

The stdio LSP runtime polls this generation while idle and requests semantic-token refreshes when the overlay becomes ready or changes, so coloring is not dependent on hover or another editor request to notice background indexing completion.

## Dependencies and Boundaries

Depends on parser, AST, model, index, index-cache, and standard-library threading/synchronization. It does not own LSP request dispatch, open-document analysis, feature projection, TypeScript file watching, or user-facing protocol shapes.

The overlay remains best-effort source-backed infrastructure. It does not implement semantic `modded` merging, Workbench validation, diagnostics policy, persisted workspace caches, or file watching by itself.

## Change Notes

Extracted from `server/src/lsp.rs` so request dispatch no longer owns game-data/workspace indexing internals.
Removed the debug-build-only game-data skip after it made development hover/definition unable to resolve external game-data types such as base classes. Performance tuning belongs in cache/index implementation, not in disabling the source set used by language features.
Added an overlay generation counter so semantic-token coloring can distinguish “computed before external index was ready” from “computed with current game-data/workspace symbols.”

## Future Improvements

Measure larger workspace update costs before adding workspace cache or incremental overlay structures. Keep any future optimization behind this module so feature projection code continues to consume a single overlay handle.
