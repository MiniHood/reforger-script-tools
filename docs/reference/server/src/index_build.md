# server/src/index_build.rs

## Purpose

Builds in-memory symbol indexes from explicit script source roots.

## Ownership

This module owns deterministic `.c` discovery, read/decode, parse, file-local catalog construction, and index aggregation. Tools, runtime startup, and future LSP indexing reuse this pipeline instead of duplicating it.

## Current Behavior

`IndexSourceRoot` supplies path, `SourceKind`, and priority; `IndexBuildConfig` combines roots. `build_index` validates roots, recursively discovers and sorts `.c` files, creates path-derived metadata/categories, parses each source, builds a `SymbolCatalog`, and adds it to `SymbolIndex`.

`IndexBuildSummary` reports file/byte/decode/diagnostic/catalog counts and bounded review details for lossy decode and parse diagnostics. `IndexBuildTimings` separates discovery, read/decode, parse, catalog construction, aggregation, and total time. Categories are provenance for query/report policy, not compiler truth.

## Dependencies and Boundaries

Depends on parser, AST, model, and index. It does not resolve symbols, merge `modded` classes, watch files, persist caches, discover VS Code workspaces, call Workbench, or handle LSP.

## Verification

Builder tests and corpus/overlay/cache-baseline reports exercise deterministic scanning, diagnostics, categories, summaries, and timings.

## Future Direction

Incremental rebuild inputs belong with a future watcher. Cache validation and persistence remain in `index_cache`.
