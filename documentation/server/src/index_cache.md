# server/src/index_cache.rs

## Purpose

Owns disposable runtime caching for the game-data symbol index.

## Architecture Role

This file sits beside `server/src/index_build.rs`. It does not build language facts itself; it validates cache identity, loads a serialized `SymbolIndex` when safe, or rebuilds through the existing index builder when the cache is missing, stale, corrupt, or incompatible.

## Current Behavior

The cache is keyed by scripts-root identity and source fingerprint. Downloaded game data uses `metadata.json` `commitSha` as the primary invalidation key. Manual folders use a recursive `.c` file fingerprint made from file count, byte count, and latest modified timestamp. Cache payloads include a format version, crate version, fingerprint, summary counts, and the copied `SymbolIndex`.

The cache is written as JSON with an explicit snapshot representation for index maps so complex Rust map keys do not become invalid JSON object keys. A cache hit returns the stored index. Any cache mismatch or deserialization failure falls back to rebuilding and replacing the cache.

## Dependencies and Boundaries

Depends on `serde`, `serde_json`, `server/src/index_build.rs`, and the copied index data model. It must not parse source directly, call Workbench, download game data, own VS Code paths, or become source truth. The cache is always disposable.

## Change Notes

- Added the first runtime game-data index cache for LSP hover external lookup.
- Cache invalidation uses downloaded commit SHA when available and manual-folder file metadata otherwise.
- Corrupt or incompatible cache files rebuild instead of failing the language server.

## Future Improvements

- Split cache files by source root if workspace indexing later needs independent cache invalidation.
- Add a more precise manual-folder fingerprint only if file metadata proves too coarse.
- Replace JSON only if measured startup/cache-load performance requires a more compact bundled format.
