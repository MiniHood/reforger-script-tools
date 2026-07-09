# server/examples/lexer_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report by running the real Rust lexer across a Reforger scripts folder.

## Architecture Role

This is developer review tooling for lexer coverage. It is not VS Code runtime behavior, not an LSP entrypoint, and not parser/model/index logic.

## Current Behavior

The example scans `.c` files under a scripts folder, lexes each file, and writes `tools/reports/lexer-corpus.report.md` by default. It reports file totals, byte totals, token counts, documentation comment counts, keyword/operator frequency, unknown text frequency, files containing lexer error tokens, and files that required lossy UTF-8 decoding for review.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate lexer. It must not duplicate lexer behavior, parse declarations, inspect semantics, or become runtime extension code.

## Change Notes

- Added corpus-scale lexer validation for real downloaded/manual Reforger script data.
- Added lossy decoding reporting so non-UTF-8 game-data files do not stop the whole scan.

## Future Improvements

- Add targeted sections for newly discovered tokenization edge cases.
- Keep corpus findings as review evidence only; Workbench remains compiler truth.
