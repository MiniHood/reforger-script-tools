# server/src/index_cache.rs

## Purpose

Owns the disposable runtime cache for the game-data symbol index.

## Ownership

This module validates cache identity, safely loads a serialized `SymbolIndex`, or rebuilds through `index_build`. It owns cache format, invalidation, safe decoding, and load/build timing; source construction remains with `index_build`.

## Current Behavior

Cache identity combines scripts-root identity with a source fingerprint: downloaded data uses `metadata.json` `commitSha`; manual folders use recursive `.c` count, byte count, and latest timestamp. The v11 binary payload stores summary counts, an interned string table, and one cache-owned canonical public-fact record per source file. Each file record owns its `SourceFileMetadata`, public declaration graph, copied detail text, modifiers, attributes, docs, conditional facts, callable form, and only the declaration/selection spans consumed by the runtime index. It does not serialize `IndexedFile`/`IndexedSymbol` records, JSON contributions, source-only detail/directive spans, or contribution-only container text. Public IDs are dense and parent edges are remapped when private records are projected out. Decoded records are reconstructed as transient `FileContribution` values, validated, and batch-ingested once to rebuild derived lookup maps. Corrupt or stale public semantic data triggers a source rebuild instead of becoming a legacy index fallback.

The existing-path `RSTIDX10` binary-v3 cache is a one-way compatibility input during the v11 rollout. It is accepted only when its exact v10 schema, shape, crate version, source fingerprint, indexed-file graph, and versioned contribution contract all validate. The loader projects its contributions into canonical public facts, atomically replaces the same file with `RSTIDX11`, then reconstructs the transient index. The still-older `RSTIDX09` binary-v2 input follows the same one-way path through its stricter source-derived-record validation. Legacy bytes are rejected before allocation when their file length exceeds the 128 MiB migration ceiling; wrong identity/version/shape, malformed records, failed projection, or failed replacement rebuild from source. Neither v9 nor v10 is retained as a runtime query path.

Persisted game-data indexes omit external `LocalVariable` symbols and source-only detail spans, but retain parameters, callable signatures, docs, attributes, modifiers, declarations, conditional context, provenance, and copied detail text. Any magic/schema/version/index-shape/crate-version/fingerprint/decode mismatch rebuilds and replaces the cache. All length-prefixed decoding is bounded before allocation; after decoding a legacy cache, its raw bytes are released before validation or v11 replacement. Corrupt or partial data is a rebuild trigger, never source truth.

Operations expose phase timings and may emit startup-only progress through an LSP callback. A successful game-data ready record reports the source-free on-disk `cache_file_bytes` after cold rebuild, warm load, or legacy migration. Cache reports measure baseline, composition, and duplicate-string evidence; the production-corpus release target is at most 30 MiB, with a 32 MiB regression ceiling pending an explicit revised target.

## Dependencies and Boundaries

Depends on copied index records, versioned public contribution records, `index_build`, and `serde`. It does not parse sources directly, download game data, own VS Code paths, call Workbench, or become authoritative language data.

## Verification

Cache tests cover invalidation, v9/v10 compatibility rejection and atomic same-path migration, corrupt/truncated/oversized payload recovery, bounded decoding, lookup-map rebuild, parity, and runtime compaction. Cache reports provide runtime trade-off evidence.

## Future Direction

Split roots or refine manual fingerprints only when reports show a concrete need. The cache may be replaced, split, disabled, or retained based on measured benefit.
