# server/examples/index_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report from the first in-memory symbol index.

## Ownership

This is developer review tooling for index lookup quality. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic resolution, not Workbench validation, and not compiler truth.

## Current Behavior

The example uses `server/src/index_build.rs` to scan `.c` files under a scripts folder, parse each file, build AST/model catalogs with game-data metadata, aggregate them into `SymbolIndex`, and write `tools/reports/index-corpus.report.md` by default. It reports indexed file/symbol totals, lossy-decoded file details with first replacement locations and bounded ASCII-stable snippets, parse diagnostic snippets, top-level versus child/member symbol breakdowns, wall-clock build timings, map counts, source-kind counts, source-category counts, editor-completion source-policy counts, symbol-kind frequency, presentation metadata coverage, optional detail coverage by symbol kind, bounded missing optional-detail samples, bounded doc-preview quality samples with raw Doxygen-tag source lines and cleaned previews, duplicate classification buckets, focused suspicious conflict tables, suspicious duplicate provenance, classified same-owner shadow groups, unknown/high-risk same-owner shadows, editor-completion filtering decisions by conflict class, bounded unknown-conflict snippets, bounded duplicate top-level name groups with symbol details, top-level-only preferred duplicate samples, lookup samples for classes and typedefs, grouped method owner/name samples with overload counts and bounded signature examples, callable details, raw aggregate completion shadows, and preferred-class completion shadows. Shadow review subsections are nested under their owning completion view so repeated summaries remain easy to scan.

Conflict rows include callable form and preprocessor conditional context when present. This makes branch-preserved duplicates and prototype/declaration duplicates visible without evaluating macros or removing raw index facts.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate index builder and index modules. It must not duplicate index-build behavior, resolve symbols, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example index_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
