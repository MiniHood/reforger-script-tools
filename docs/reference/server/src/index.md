# server/src/index.rs

## Purpose

Owns the in-memory multi-file symbol index built from file-local model catalogs.

## Ownership

`SymbolIndex` aggregates catalogs into global handles and raw lookup maps. It owns copied lookup and display facts plus source-priority ordering; `index_build` owns source-to-catalog construction, `index_cache` owns persistence, `index_query` owns editor policy, and `symbol_display` owns presentation.

## Current Behavior

Catalogs receive a `SourceFileId`; symbols use `{ file_id, symbol_id }` so file-local `SymbolId` remains local. Indexed records retain names, detail text/spans, modifiers, attributes, raw docs, conditional context, macro names, callable form, provenance, and parent/child relations without retaining source text.

The index supports raw name/kind/top-level/class/typedef/function/member/child lookup, preferred ordering, callable signatures, conflict review, source counts, and direct or best-effort base-chain member views. Local variables remain available for lookup and display but are excluded from top-level and class-member maps. `from_indexed_parts()` reconstructs derived maps from trusted cached records. Runtime-cache compaction removes external locals and source-only spans while preserving copied editor facts.

Priority ordering is a source-policy aid, not semantic merge or `modded` interpretation. Editor consumers should use `IndexQuery` rather than raw aggregate APIs.

## Dependencies and Boundaries

Depends on lexer spans and model records. It does not parse, extract AST, evaluate types/defaults/macros, resolve semantics, watch files, own cache invalidation, or handle LSP requests.

## Verification

Index unit tests cover lookup ordering, signatures, member queries, catalog reconstruction, and cache compaction. Builder/cache/report tests exercise pipeline integration.

## Future Direction

Incremental workspace updates and semantic inheritance/override behavior require separate validated layers. Cache policy stays in `index_cache`; type-fact interpretation stays in `type_facts`.
