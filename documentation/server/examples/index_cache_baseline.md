# server/examples/index_cache_baseline.rs

## Purpose

Measures whether the disposable binary game-data index cache is actually useful compared with rebuilding the index.

## Architecture Role

This is dev-only review tooling. It exercises the runtime cache path in `server/src/index_cache.rs` and compares it with a direct `index_build` rebuild, but it does not change language-server startup behavior or cache policy.

## Current Behavior

The report measures an existing cache path, a temporary cache-miss rebuild/write, and a direct rebuild without cache. Cache measurements use the v9 runtime-pruned binary game-data cache, while direct rebuild measures the full source index. The report therefore checks that cache symbols equal direct rebuild symbols minus local variables and that parameter symbols remain preserved. It also compares a full-map JSON estimate against the v9 actual binary cache file, including string-table storage, detail-span stripping, and lookup-map rebuild visibility. Cache hit timing is split into file read, binary decode, validation, and lookup-map rebuild. The Node wrapper runs debug and release profiles and combines them into `tools/reports/index-cache-baseline.report.md`.

The runtime cache structural section also reports lookup-map shape: key counts and symbol-id entry counts for all rebuilt maps. This makes lookup-map rebuild time reviewable without persisting the maps in the cache.

Release timing is the only timing used for the cache usefulness decision. Debug timing is informational.

## Dependencies and Boundaries

Depends on `index_cache`, `index_build`, and public `SymbolIndex` data for counts and lower-bound memory estimates. It must not add runtime extension commands, mutate source files, change cache invalidation policy, or treat the cache as source truth.

## Change Notes

- Added to compare the large cache against release rebuild time before expanding runtime indexing.
- Temporary benchmark caches are written outside tracked source paths and removed after the run.
- Updated for v2 runtime cache pruning so local-variable removal and parameter preservation are visible in the count comparison.
- Updated for v8 binary cache optimization so full-map estimates, detail-span stripping, compacted per-file symbol ranges, and map rebuild behavior are visible.
- Updated for v9 string-table cache storage so repeated string savings are measured without removing editor-visible facts.
- Added split binary cache timing fields for file read, decode, validation, and lookup-map rebuild.
- Added lookup-map shape reporting so cache rebuild-map cost can be tied to concrete key and symbol-id entry counts.

## Future Improvements

- Add process RSS measurement only if a dependency-free, platform-appropriate approach becomes necessary.
- Use this report before changing the binary cache format or disabling cache entirely.
