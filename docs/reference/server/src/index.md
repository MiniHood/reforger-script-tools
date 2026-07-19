# server/src/index.rs

## Purpose

Owns the in-memory multi-file symbol index built from compiler-produced file
semantic facts.

## Ownership

`SymbolIndex` aggregates validated file contributions into global handles and
raw lookup maps. It owns copied lookup and display facts plus source-priority ordering;
`index_build` owns source-to-semantic construction, `index_cache` owns
persistence, `index_query` owns editor policy, and `symbol_display` owns
presentation. The legacy catalog ingress remains only for differential and
compatibility tests.

Cold builds use `add_file_contributions()`: it validates the complete batch,
preserves each contribution's file-local IDs and metadata, appends all records,
then constructs global lookup maps once. `add_file_contribution()` remains the
per-file incremental-update boundary and rebuilds maps for that single visible
change. This keeps a full game-data rebuild linear in indexed records rather
than rebuilding every global map after every discovered file.

## Current Behavior

Validated contributions receive a `SourceFileId`; symbols use `{ file_id,
symbol_id }` where every contribution's `symbol_id` is dense after private
records are projected out, so snapshot-local declaration identity remains local. Indexed
records retain names, detail text/spans, modifiers, attributes, raw docs,
conditional context, macro names, callable form, provenance, and parent/child
relations without retaining source text.

The index supports raw name/kind/top-level/class/typedef/function/member/child lookup, preferred ordering, callable signatures, conflict review, source counts, and direct or best-effort base-chain member views. Local variables remain available for lookup and display but are excluded from top-level and class-member maps. The current game-data cache reconstructs this index from its canonical per-file contribution records; `from_indexed_parts()` is retained only for validated legacy-cache conversion and compatibility tests. Runtime-cache compaction projects out external locals and source-only spans before canonical persistence while preserving copied editor facts.

Priority ordering is a source-policy aid, not semantic merge or `modded` interpretation. Editor consumers should use `IndexQuery` rather than raw aggregate APIs.

## Dependencies and Boundaries

Depends on lexer spans and model records. It does not parse, extract AST, evaluate types/defaults/macros, resolve semantics, watch files, own cache invalidation, or handle LSP requests.

## Verification

Index unit tests cover lookup ordering, signatures, member queries, semantic
and contribution reconstruction parity, and cache compaction. Builder/cache/report tests exercise pipeline integration.

## Future Direction

Incremental workspace updates and semantic inheritance/override behavior require separate validated layers. Cache policy stays in `index_cache`; type-fact interpretation stays in `type_facts`.
