# server/examples/lsp_definition_corpus_report.rs

## Purpose

Generates a dev-only corpus report for LSP definition behavior across real game-data identifier positions.

## Ownership

This example sits above `server/src/lsp.rs` and exercises the same resolver-first definition projection used by `textDocument/definition`. It gives corpus-scale evidence for Ctrl+click behavior without starting VS Code.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-definition-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for timing labels.
- `--max-files <n>` for bounded file scans.
- `--max-checks <n>` for bounded definition checks.

It builds a game-data index once, samples identifier tokens evenly across the bounded file set, and reports hit rate, resolver reason frequency, identifier context frequency, selected source/kind frequency, miss classification, review buckets, miss samples, and timing. Miss buckets intentionally separate source-noise positions such as preprocessor directives, preprocessor macro names, named call argument labels, attribute named arguments, and attribute enum/static values from genuinely unresolved value/type/member positions. Attribute named arguments, preprocessor directives, preprocessor macro names, and named argument labels are classified from resolver-owned non-symbol reasons before report-local source-line heuristics are used.

The follow-up review sections split definition targets into file-local versus external game-data targets, group misses into source-noise versus actionable unresolved buckets, and list receiver/member definition misses separately. The corpus report does not include workspace overlay targets; workspace behavior belongs to the runtime overlay tests and reports.

## Dependencies and Boundaries

Uses only Rust standard library APIs, the lexer, reusable index builder, and existing LSP definition helper. It must stay dev-only. It must not register VS Code commands, add runtime logging, perform Workbench validation, or implement a second definition path.

## Verification

Run `cargo run --example lsp_definition_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
