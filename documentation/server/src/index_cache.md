# server/src/index_cache.rs

## Purpose

Owns disposable runtime caching for the game-data symbol index.

## Architecture Role

This file sits beside `server/src/index_build.rs`. It does not build language facts itself; it validates cache identity, loads a serialized `SymbolIndex` when safe, or rebuilds through the existing index builder when the cache is missing, stale, corrupt, or incompatible.

## Current Behavior

The cache is keyed by scripts-root identity and source fingerprint. Downloaded game data uses `metadata.json` `commitSha` as the primary invalidation key. Manual folders use a recursive `.c` file fingerprint made from file count, byte count, and latest modified timestamp. Cache payloads include a format version, crate version, fingerprint, summary counts, and a compact index snapshot.

The persisted game-data cache is runtime-pruned in format v3: it removes only external `LocalVariable` symbols before serialization. Parameters, callable signatures, docs, attributes, modifiers, classes, fields, methods, constructors, destructors, typedefs, enum values, conditional context, private/protected/static members, global fields, and source provenance remain cached. Open-document analysis and dev corpus/debug builds still use full indexes with locals.

The v3 cache is written as JSON with only metadata, files, symbols, and summary counts. Derived lookup maps are not persisted; they are rebuilt after deserialization from the stored file and symbol records. Source-only detail span fields are stripped from persisted game-data symbols because the external cache does not retain source text, but copied detail text remains available for hover, signatures, and debug display. Any cache mismatch or deserialization failure falls back to rebuilding and replacing the cache.

Cache operations now return timing data for fingerprinting, cache read/deserialization/validation, rebuild, write, and total load-or-build time. These timings are review data only; they do not change cache behavior.

`server/examples/index_cache_baseline.rs` measures whether loading JSON is faster than rebuilding. `server/examples/index_cache_composition_report.rs` measures what the cache contains and how much appears needed for editor runtime features. `server/examples/index_cache_strings_report.rs` measures duplicated copied string values so possible string interning or path-table work can be judged from data.

## Dependencies and Boundaries

Depends on `serde`, `serde_json`, `server/src/index_build.rs`, and the copied index data model. It must not parse source directly, call Workbench, download game data, own VS Code paths, or become source truth. The cache is always disposable.

## Change Notes

- Added the first runtime game-data index cache for LSP hover external lookup.
- Cache invalidation uses downloaded commit SHA when available and manual-folder file metadata otherwise.
- Corrupt or incompatible cache files rebuild instead of failing the language server.
- Added cache timing fields for `server/examples/index_cache_baseline.rs` so JSON cache usefulness can be compared against release rebuild time.
- Added the cache composition report as the review path for deciding whether a future split or filtered runtime cache is worthwhile.
- Bumped the runtime game-data cache to format v2 and pruned external `LocalVariable` symbols from persisted cache writes while preserving parameters.
- Bumped the runtime game-data cache to format v3. The cache now persists files/symbols only, strips detail spans, rebuilds lookup maps on load, and rejects stale v2 cache files.
- Added the cache string duplication report as the review path for deciding whether string interning or path-table cache work is worthwhile.

## Future Improvements

- Split cache files by source root if workspace indexing later needs independent cache invalidation.
- Add a more precise manual-folder fingerprint only if file metadata proves too coarse.
- Replace, split, disable, or keep JSON only after the cache baseline and cache composition reports show a concrete benefit.
