# server/examples/lexer_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report by running the real Rust lexer across a Reforger scripts folder.

## Ownership

This is developer review tooling for lexer coverage. It is not VS Code runtime behavior, not an LSP entrypoint, and not parser/model/index logic.

## Current Behavior

The example scans `.c` files under a scripts folder, lexes each file, and writes `tools/reports/lexer-corpus.report.md` by default. It reports file totals, byte totals, token counts, documentation comment counts, keyword/operator frequency, unknown text frequency, files containing lexer error tokens, and files that required lossy UTF-8 decoding for review.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate lexer. It must not duplicate lexer behavior, parse declarations, inspect semantics, or become runtime extension code.

## Verification

Run `cargo run --example lexer_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
