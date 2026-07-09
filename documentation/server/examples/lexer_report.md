# server/examples/lexer_report.rs

## Purpose

Generates a human-readable Markdown report showing lexer input vs token output for committed fixtures.

## Architecture Role

This is developer review tooling for the Rust lexer. It is not VS Code runtime behavior and is not a language server entrypoint.

## Current Behavior

Running `cargo run --manifest-path server/Cargo.toml --example lexer_report` writes `tools/reports/lexer-fixtures.report.md` by default. The report includes per-fixture summaries, token counts, error-token counts, and token stream tables with spans, line/column positions, and escaped token text. It includes both focused hand-sized fixtures and the larger game-data-derived player commands config fixture.

The command also accepts `--out <path>` for an alternate report path.

## Dependencies and Boundaries

The report generator uses only Rust standard library APIs and the crate lexer. It must not implement parser, model, index, or LSP behavior. Generated reports belong under ignored output paths such as `tools/reports/`.

## Change Notes

- Added the first lexer fixture report generator for human/Codex review.

## Future Improvements

- Add corpus-scale summaries after the lexer has a stable fixture baseline.
- Add focused sections for any Workbench-confirmed tokenization edge cases.
