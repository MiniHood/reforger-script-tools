# server/examples/parser_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report by running the real Rust declaration parser across a Reforger scripts folder.

## Ownership

This is developer review tooling for parser coverage. It is not VS Code runtime behavior, not an LSP entrypoint, not Workbench validation, and not parser truth by itself.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, and writes `tools/reports/parser-corpus.report.md` by default. It reports file totals, byte totals, preserved-token totals, parse diagnostics, syntax-kind frequency, diagnostic message frequency, top files with diagnostics, bounded diagnostic snippets, expected recovery nodes, and files that required lossy UTF-8 decoding for review.

Expected recovery currently labels the known `Game\game.c` `#ifdef BREAK_COMPILATION` invalid branch as preprocessor-test text when that source pattern is present. This classification is review evidence only; the parser still preserves the source and does not evaluate preprocessor branches.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate parser/syntax modules. It must not duplicate parser behavior, add syntax rules, inspect semantics, index declarations, call Workbench, or become runtime extension code.

## Verification

Run `cargo run --example parser_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
