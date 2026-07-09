# server/src/index_build.rs

## Purpose

Owns reusable construction of in-memory symbol indexes from explicit script source roots.

## Architecture Role

This file sits above parser, AST, model, and raw index layers. It is the shared pipeline for scanning `.c` files, reading and decoding source, parsing, building file-local symbol catalogs, and adding those catalogs into `SymbolIndex`.

Future dev tools, runtime startup, and LSP indexing code should use this module instead of duplicating scan/read/parse/catalog/index logic.

## Current Behavior

`IndexSourceRoot` describes one source root with an explicit path, `SourceKind`, and priority. `IndexBuildConfig` accepts one or more roots. `build_index` validates each root, recursively discovers `.c` files, sorts them deterministically, creates `SourceFileMetadata`, parses each file, builds a `SymbolCatalog`, and immediately adds it to `SymbolIndex`.

`IndexBuildSummary` records total and per-source-kind counts for files, bytes, lossy UTF-8 files, parse diagnostics, diagnostic files, indexed files, indexed symbols, and non-declaration callable fragments. It also stores bounded human-review details for lossy decoding and parse diagnostics, including location data and short source snippets. Snippets render UTF-8 replacement characters as the ASCII label `<U+FFFD>` so report output is readable across terminals and editors. `IndexBuildTimings` records file discovery, catalog build, index build, and total wall-clock durations for human review.

## Dependencies and Boundaries

This file depends on parser, AST, model, and index modules. It must not resolve symbols semantically, merge `modded` classes, watch files, persist caches, call Workbench, know VS Code workspace state, or handle LSP requests.

The builder is source-root explicit. VS Code workspace discovery and game-data source resolution belong in future TypeScript/LSP integration layers, not here.

## Change Notes

- Added the shared index-building pipeline for corpus report, overlay report, and index debug tooling.
- Removed the need for report/debug examples to keep borrowed source strings alive because `SymbolIndex::add_catalog` copies indexed lookup facts.
- Added per-source summaries and build timings so reports can keep human-review diagnostics without owning indexing logic.
- Added bounded lossy decode details and parse diagnostic details so corpus, overlay, and debug tooling can show actionable source snippets without duplicating parser/report logic.
- Render replacement characters in snippets as `<U+FFFD>` so lossy decode reports remain ASCII-stable and do not display mojibake in PowerShell or other terminals.

## Future Improvements

- Add incremental rebuild inputs after workspace file watching exists.
- Add optional persisted cache only after runtime startup measurements justify it.
- Add memory-size estimates if future language-server startup needs them.
