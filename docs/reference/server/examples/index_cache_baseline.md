# server/examples/index_cache_baseline.rs

## Purpose

Measures whether the disposable binary game-data index cache is actually useful compared with rebuilding the index.

## Ownership

This is dev-only review tooling. It exercises the runtime cache path in `server/src/index_cache.rs` and compares it with a direct `index_build` rebuild, but it does not change language-server startup behavior or cache policy.

## Current Behavior

The report measures an existing cache path, a temporary cache-miss rebuild/write, and a direct rebuild without cache. Cache measurements use the v9 runtime-pruned binary game-data cache, while direct rebuild measures the full source index. The report therefore checks that cache symbols equal direct rebuild symbols minus local variables and that parameter symbols remain preserved. It also compares a full-map JSON estimate against the v9 actual binary cache file, including string-table storage, detail-span stripping, and lookup-map rebuild visibility. Cache hit timing is split into file read, binary decode, validation, and lookup-map rebuild. The Node wrapper runs debug and release profiles and combines them into `tools/reports/index-cache-baseline.report.md`.

The runtime cache structural section also reports lookup-map shape: key counts and symbol-id entry counts for all rebuilt maps. This makes lookup-map rebuild time reviewable without persisting the maps in the cache.

Release timing is the only timing used for the cache usefulness decision. Debug timing is informational.

## Dependencies and Boundaries

Depends on `index_cache`, `index_build`, and public `SymbolIndex` data for counts and lower-bound memory estimates. It must not add runtime extension commands, mutate source files, change cache invalidation policy, or treat the cache as source truth.

## Verification

Run `cargo run --example index_cache_baseline` from `server/` against a known script corpus. Confirm temporary benchmark artifacts are outside tracked paths and removed after the run.
