# server/examples/scope_corpus_report.rs

## Purpose

Generates a dev-only Markdown report for lexical scope model quality across a Reforger scripts corpus.

## Ownership

This report sits beside the parser, AST, expression, resolver, and LSP reports. It proves that `server/src/scope.rs` is building callable/block scopes and attaching parameter/local symbols correctly before later rename, references, diagnostics, and semantic-type work depend on it.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds AST/model/index data, builds `LexicalScopeModel`, and writes `tools/reports/scope-corpus.report.md` by default. It reports root, callable, block, `for`, and `foreach` scope counts; scoped versus unscoped parameters and locals; local declaration kind counts; scope-depth frequency; symbols-per-scope frequency; shadow classification; declaration-before-use quality; top files by block scopes/depth/locals; and bounded snippets for shadows or visibility anomalies.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is supplied, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate parser, AST, model, index, and scope modules. It must not change resolver behavior, LSP behavior, parser behavior, Workbench validation, or runtime extension state. Scope remains lexical and source-backed; this report must not turn scope into semantic type resolution.

## Verification

Run `cargo run --example scope_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
