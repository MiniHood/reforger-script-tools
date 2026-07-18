# server/examples/index_cache_composition_report.rs

## Purpose

Explains what the disposable game-data index cache contains and how much of it is likely needed for editor hover/completion.

## Architecture Role

This is dev-only review tooling. It loads the current game-data index through `server/src/index_cache.rs`, inspects public `SymbolIndex` data, and writes a Markdown report under `tools/reports/`.

## Current Behavior

The report summarizes cache status, full-direct-index versus runtime-cache counts, v9 binary structural optimization, source-category composition, symbol-kind composition, presentation metadata counts, and editor-runtime versus debug/review-only slices. It also writes temporary JSON measurement snapshots outside tracked source paths and deletes them immediately. These snapshots are size probes only and are not reusable cache files.

The runtime cache section should show that `LocalVariable` symbols are removed from persisted game-data cache data while `Parameter` and `TypeParameter` symbols are preserved. The v9 structural section should show that lookup maps are omitted, repeated strings are stored through the string table, detail span fields are stripped, copied detail text remains present, compacted per-file symbol ranges are preserved, and map rebuild happens on load. Open-document analysis and dev/debug reports remain full fidelity.

Editor-runtime classification uses `SourceCategory::is_editor_completion_default()`. Runtime categories are compared against docs/Doxygen, test/autotest, Workbench, and unknown categories so cache-format decisions can be made from actual index data.

## Dependencies and Boundaries

Depends on `index_cache`, `SymbolIndex`, `SourceCategory`, and serde JSON serialization for temporary measurement snapshots. It must not change cache format, cache invalidation, runtime language-server behavior, index semantics, source-category policy, or LSP behavior.

## Change Notes

- Added after the cache baseline showed cache hits are faster than rebuilds but the cache is large.
- The report is intended to decide whether a later split or filtered runtime cache is worth designing.
- Updated for the v2 runtime-pruned game-data cache to make removed local variables and preserved parameters explicit.
- Updated for the v8 binary runtime cache to report full-map estimate versus actual cache size, omitted maps, stripped detail spans, copied detail text preservation, and compacted per-file symbol range preservation.
- Updated for the v9 string-table runtime cache to make repeated-string storage part of the structural cache review.

## Future Improvements

- Use this report before designing any binary cache or split runtime/debug cache.
- Add additional slices only after hover/completion/definition clarify which facts are truly needed at startup.
