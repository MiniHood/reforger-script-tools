# server/examples/index_cache_baseline.rs

## Purpose

Measures whether the disposable JSON game-data index cache is actually useful compared with rebuilding the index.

## Architecture Role

This is dev-only review tooling. It exercises the runtime cache path in `server/src/index_cache.rs` and compares it with a direct `index_build` rebuild, but it does not change language-server startup behavior or cache policy.

## Current Behavior

The report measures an existing cache path, a temporary cache-miss rebuild/write, and a direct rebuild without cache. Cache measurements use the v4 runtime-pruned game-data cache, while direct rebuild measures the full source index. The report therefore checks that cache symbols equal direct rebuild symbols minus local variables and that parameter symbols remain preserved. It also compares a full-map JSON estimate against the v4 actual cache file, including detail-span stripping and lookup-map rebuild visibility. The Node wrapper runs debug and release profiles and combines them into `tools/reports/index-cache-baseline.report.md`.

Release timing is the only timing used for the cache usefulness decision. Debug timing is informational.

## Dependencies and Boundaries

Depends on `index_cache`, `index_build`, and public `SymbolIndex` data for counts and lower-bound memory estimates. It must not add runtime extension commands, mutate source files, change cache invalidation policy, or treat the cache as source truth.

## Change Notes

- Added to compare the large JSON cache against release rebuild time before expanding runtime indexing.
- Temporary benchmark caches are written outside tracked source paths and removed after the run.
- Updated for v2 runtime cache pruning so local-variable removal and parameter preservation are visible in the count comparison.
- Updated for v4 structural cache optimization so full-map estimates, detail-span stripping, and map rebuild behavior are visible.

## Future Improvements

- Add process RSS measurement only if a dependency-free, platform-appropriate approach becomes necessary.
- Use this report before replacing JSON with a binary cache format or disabling cache entirely.
