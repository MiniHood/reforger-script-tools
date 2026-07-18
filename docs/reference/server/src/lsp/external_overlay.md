# server/src/lsp/external_overlay.rs

## Purpose

Owns the runtime external symbol layers used by LSP features.

## Architecture Role

This module is part of the Rust LSP layer. It manages background game-data index load/build, live workspace script indexing, workspace file update/delete handling, workspace aggregate rebuilds, and status summaries consumed by request dispatch.

## Current Behavior

The module keeps workspace and game-data indexes as separate layers. Workspace scripts use priority `200`; game-data scripts use priority `100`. Workspace files are parsed and indexed from configured script roots at startup, then updated from full-text custom LSP notifications. Game data is loaded through the disposable runtime cache path when configured.

Request handlers take an `ExternalIndexSnapshot` of the workspace and game-data layers in priority order. Feature code queries open-document facts first, then workspace, then game data, without constructing a merged workspace/game-data overlay. Request logs include the available layer set: `workspace`, `game-data`, both, or `none`. The stored workspace/game-data indexes are `Arc` owned so worker tasks such as rich semantic-token projection can take cheap snapshots without cloning the full index or holding the external-overlay mutex while they compute.

Workspace updates clone the small map of `Arc`-owned per-file indexes, build the aggregate `SymbolIndex` outside the mutex, then publish only when the captured workspace generation is still current. Startup follows the same snapshot-and-publish pattern while preserving live changes that arrived during the initial scan.

Startup logs keep game-data and workspace timing separate. The `externalIndex gameData phase` lines identify the active cache phase during startup: script-root validation, fingerprinting, cache load, lookup-map rebuild, source rebuild, or cache write. The `externalIndex gameData ready` line includes cache file-read, binary-decode, validation, lookup-map rebuild, and total cache timing. The final `externalIndex layered` line includes game-data, workspace, and total layered readiness timing. These logs are diagnostic only; workspace build ordering is not changed by the timing fields.

Additional external-index startup phase logs surround the handoff points between cache loading, workspace indexing, state publication, and summary recompute. These exist because a bad startup can otherwise go dark after cache map rebuild but before the ready state is published. The background indexing thread catches ordinary Rust panics and logs `externalIndex thread panic` before exiting; allocation aborts and process kills may still terminate before panic logging.

Workspace script roots are canonicalized and deduplicated before startup indexing. This matters on case-insensitive platforms where VS Code may pass both `Scripts` and `scripts` for the same physical folder. Files discovered under those canonical roots are stored directly in the startup map instead of being canonicalized again per file; repeated Windows canonicalization of extended paths is unnecessary and can stall startup. Startup logs record the requested root count, unique root count, discovered file count, and per-file workspace indexing timings so a single pathological workspace file can be identified without dumping source text.

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

Stored external index layers behind `Arc` and use short-lock snapshots for both foreground feature projection and worker-owned tasks.

Added startup cache phase markers so a bad Extension Development Host session that logs only `gameData start` can be narrowed to the precise cache phase before ready/error.

Added post-cache external-index phase markers and background-thread panic logging so startup stalls after `map-rebuild-end` can be separated into cache return, workspace build, state publish, summary recompute, or panic.

Added workspace root normalization and per-file workspace indexing timings after a startup stall was traced past game-data cache load into workspace indexing. Removed redundant per-file canonicalization from the startup workspace indexing loop.

## Future Improvements

Measure larger workspace update costs before adding workspace cache or incremental workspace aggregate structures. Keep any future optimization behind this module so feature projection code continues to consume a single external-index handle with ordered layers.

Feature consumers now take owned `ExternalIndexSnapshot` values, so resolver/rendering work runs without retaining the overlay mutex. Startup workspace data is a baseline merged with recorded live updates and deletion tombstones; caught startup panics now publish `failed` state and an error.
