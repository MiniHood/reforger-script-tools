# server/src/index_cache.rs

## Purpose

Owns disposable runtime caching for the game-data symbol index.

## Architecture Role

This file sits beside `server/src/index_build.rs`. It does not build language facts itself; it validates cache identity, loads a serialized `SymbolIndex` when safe, or rebuilds through the existing index builder when the cache is missing, stale, corrupt, or incompatible.

## Current Behavior

The cache is keyed by scripts-root identity and source fingerprint. Downloaded game data uses `metadata.json` `commitSha` as the primary invalidation key. Manual folders use a recursive `.c` file fingerprint made from file count, byte count, and latest modified timestamp. Cache payloads include a format version, crate version, fingerprint, summary counts, and the copied `SymbolIndex`.

The cache is written as JSON with an explicit snapshot representation for index maps so complex Rust map keys do not become invalid JSON object keys. A cache hit returns the stored index. Any cache mismatch or deserialization failure falls back to rebuilding and replacing the cache.

Cache operations now return timing data for fingerprinting, cache read/deserialization/validation, rebuild, write, and total load-or-build time. These timings are review data only; they do not change cache behavior.

`server/examples/index_cache_baseline.rs` measures whether loading JSON is faster than rebuilding. `server/examples/index_cache_composition_report.rs` measures what the cache contains and how much appears needed for editor runtime features.

## Dependencies and Boundaries

Depends on `serde`, `serde_json`, `server/src/index_build.rs`, and the copied index data model. It must not parse source directly, call Workbench, download game data, own VS Code paths, or become source truth. The cache is always disposable.

## Change Notes

- Added the first runtime game-data index cache for LSP hover external lookup.
- Cache invalidation uses downloaded commit SHA when available and manual-folder file metadata otherwise.
- Corrupt or incompatible cache files rebuild instead of failing the language server.
- Added cache timing fields for `server/examples/index_cache_baseline.rs` so JSON cache usefulness can be compared against release rebuild time.
- Added the cache composition report as the review path for deciding whether a future split or filtered runtime cache is worthwhile.

## Future Improvements

- Split cache files by source root if workspace indexing later needs independent cache invalidation.
- Add a more precise manual-folder fingerprint only if file metadata proves too coarse.
- Replace, split, disable, or keep JSON only after the cache baseline and cache composition reports show a concrete benefit.
