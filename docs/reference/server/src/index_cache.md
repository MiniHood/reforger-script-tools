# server/src/index_cache.rs

## Purpose

Owns the disposable runtime cache for the game-data symbol index.

## Ownership

This module validates cache identity, safely loads a serialized `SymbolIndex`, or rebuilds through `index_build`. It owns cache format, invalidation, safe decoding, and load/build timing; source construction remains with `index_build`.

## Current Behavior

Cache identity combines scripts-root identity with a source fingerprint: downloaded data uses `metadata.json` `commitSha`; manual folders use recursive `.c` count, byte count, and latest timestamp. The v9 binary payload stores metadata, files, symbols, summary counts, and an interned string table. Derived lookup maps are rebuilt after decode.

Persisted game-data indexes omit external `LocalVariable` symbols and source-only detail spans, but retain parameters, callable signatures, docs, attributes, modifiers, declarations, conditional context, provenance, and copied detail text. Any magic/schema/version/index-shape/crate-version/fingerprint/decode mismatch rebuilds and replaces the cache. All length-prefixed decoding is bounded before allocation; corrupt or partial data is a rebuild trigger, never source truth.

Operations expose phase timings and may emit startup-only progress through an LSP callback. Cache reports measure baseline, composition, and duplicate-string evidence.

## Dependencies and Boundaries

Depends on copied index records, `index_build`, `serde`, and JSON only for metadata. It does not parse sources directly, download game data, own VS Code paths, call Workbench, or become authoritative language data.

## Verification

Cache tests cover invalidation, compatibility rejection, corrupt/truncated payload recovery, bounded decoding, lookup-map rebuild, and runtime compaction. Cache reports provide runtime trade-off evidence.

## Future Direction

Split roots or refine manual fingerprints only when reports show a concrete need. The cache may be replaced, split, disabled, or retained based on measured benefit.
