# server/src/lsp/external_overlay.rs

## Purpose

Owns the runtime external symbol layers used by LSP features.

## Architecture Role

This module is part of the Rust LSP layer. It manages background game-data index load/build, live workspace script indexing, workspace file update/delete handling, workspace aggregate rebuilds, and status summaries consumed by request dispatch.

## Current Behavior

The module keeps workspace and game-data indexes as separate layers. Workspace scripts use priority `200`; game-data scripts use priority `100`. Workspace files are parsed and indexed from configured script roots at startup, then updated from full-text custom LSP notifications. Game data is loaded through the disposable runtime cache path when configured.

Request handlers call `ExternalIndexHandle::with_indexes` to borrow the workspace and game-data layers in priority order. Feature code queries open-document facts first, then workspace, then game data, without constructing a merged workspace/game-data overlay. Request logs include the available layer set: `workspace`, `game-data`, both, or `none`.

Startup logs keep game-data and workspace timing separate. The `externalIndex gameData ready` line includes cache file-read, binary-decode, validation, lookup-map rebuild, and total cache timing. The final `externalIndex layered` line includes game-data, workspace, and total layered readiness timing. These logs are diagnostic only; workspace build ordering is not changed by the timing fields.

The status summary includes an external generation that increments whenever workspace or game-data facts change. Cached projections such as rich semantic tokens use that generation to avoid reusing results computed against stale workspace/game-data facts. The stdio LSP runtime polls this generation while idle and requests semantic-token refreshes when an external layer becomes ready or changes.

## Dependencies and Boundaries

Depends on parser, AST, model, index, index-cache, and standard-library threading/synchronization. It does not own LSP request dispatch, open-document analysis, feature projection, TypeScript file watching, or user-facing protocol shapes.

The layered overlay remains best-effort source-backed infrastructure. It does not implement semantic `modded` merging, Workbench validation, diagnostics policy, persisted workspace caches, or file watching by itself.

## Change Notes

Extracted from `server/src/lsp.rs` so request dispatch no longer owns game-data/workspace indexing internals.

Removed the debug-build-only game-data skip after it made development hover/definition unable to resolve external game-data types such as base classes. Performance tuning belongs in cache/index implementation, not in disabling the source set used by language features.

Added an external generation counter so semantic-token coloring can distinguish "computed before external index was ready" from "computed with current game-data/workspace symbols."

Removed the combined workspace+game-data `SymbolIndex` construction from runtime startup and workspace update paths. The runtime now stores a workspace aggregate layer and a game-data layer separately, and request handlers query them in order. This avoids the post-game-data overlay merge that previously added startup tail latency.

Added split cache timing to game-data startup logs so slow Extension Development Host startup can be attributed to binary read/decode, lookup-map rebuild, workspace indexing, or non-server process startup overhead.

## Future Improvements

Measure larger workspace update costs before adding workspace cache or incremental workspace aggregate structures. Keep any future optimization behind this module so feature projection code continues to consume a single external-index handle with ordered layers.
