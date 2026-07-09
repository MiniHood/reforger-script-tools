# server/examples/index_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report from the first in-memory symbol index.

## Architecture Role

This is developer review tooling for index lookup quality. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic resolution, not Workbench validation, and not compiler truth.

## Current Behavior

The example uses `server/src/index_build.rs` to scan `.c` files under a scripts folder, parse each file, build AST/model catalogs with game-data metadata, aggregate them into `SymbolIndex`, and write `tools/reports/index-corpus.report.md` by default. It reports indexed file/symbol totals, lossy-decoded file details with first replacement locations and bounded ASCII-stable snippets, parse diagnostic snippets, top-level versus child/member symbol breakdowns, wall-clock build timings, map counts, source-kind counts, symbol-kind frequency, duplicate classification buckets, focused suspicious conflict tables, bounded duplicate top-level name groups with symbol details, top-level-only preferred duplicate samples, lookup samples for classes and typedefs, grouped method owner/name samples with overload counts and bounded signature examples, callable details, raw aggregate completion shadows, and preferred-class completion shadows. Shadow review subsections are nested under their owning completion view so repeated summaries remain easy to scan.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate index builder and index modules. It must not duplicate index-build behavior, resolve symbols, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added corpus-scale index reporting for real downloaded/manual Reforger script data.
- Preferred duplicate samples use top-level-only preferred lookup so member or parameter symbols cannot affect declaration conflict review.
- Method owner/name samples render grouped owner-qualified method rows with overload counts, first path, and bounded source-backed signature examples.
- Added report-only visibility for top-level versus child/member symbol counts and rough build timing by phase.
- Method owner/name samples now show indexed source-backed signature examples instead of return-type-only summaries.
- Added completion member shadow-group visibility so humans can review which inherited raw candidates are hidden from the completion-ready member lookup by kind/name/signature de-duplication.
- Lookup sample details now use the general callable signature API when a symbol is a function, method, constructor, or destructor.
- Lossy-decoded files are listed by bounded relative path instead of only counted.
- Duplicate top-level rows now include callable signatures or other available type/base/detail text.
- Completion shadow samples are sorted by hidden candidate count and include shadow counts by member kind.
- Added a preferred-class completion shadow summary for the future editor-facing completion path while keeping the raw aggregate shadow report for debug review.
- Switched report indexing to the shared `index_build` module.
- Added duplicate classification buckets for typedef/function delegate pairs, typedef/class wrapper patterns, generated/non-generated duplicates, workspace overlays, suspicious same-kind duplicates, and mixed-kind leftovers.
- Added lossy decode location/snippet rendering from builder-owned details.
- Added parse diagnostic snippet rendering from builder-owned details.
- Added shadow-review summaries for top shadowed method names, top classes with shadows, source-kind kept/hidden pairs, and expected inherited/base versus suspicious same-owner shadow classifications.
- Added a focused suspicious conflict report that pulls same-kind and mixed-kind top-level duplicates plus preferred-class same-owner completion shadow conflicts out of the broader review tables.
- Lossy decode snippets now render replacement characters as `<U+FFFD>` instead of relying on terminal-specific replacement glyph display.
- Reorganized completion shadow report headings so raw aggregate and preferred-class summaries each own their nested shadow review subsections.

## Future Improvements

- Add workspace-vs-game-data override sections after the index can ingest workspace catalogs.
- Add memory estimates after a real language-server startup path exists.
