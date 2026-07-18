# server/examples/lexer_report.rs

## Purpose

Generates a human-readable Markdown report showing lexer input vs token output for committed fixtures.

## Ownership

This is developer review tooling for the Rust lexer. It is not VS Code runtime behavior and is not a language server entrypoint.

## Current Behavior

Running `cargo run --manifest-path server/Cargo.toml --example lexer_report` writes `tools/reports/lexer-fixtures.report.md` by default. The report includes per-fixture summaries, token counts, documentation comment counts, error-token counts, and token stream tables with spans, line/column positions, and escaped token text. It includes all committed lexer fixtures from `tools/fixtures/lexer/`, including focused hand-sized examples and larger game-data-derived Core/player-command/game-mode/editor/Workbench fixtures.

The command also accepts `--out <path>` for an alternate report path.

## Dependencies and Boundaries

The report generator uses only Rust standard library APIs and the crate lexer. It must not implement parser, model, index, or LSP behavior. Generated reports belong under ignored output paths such as `tools/reports/`.

## Verification

Run `cargo run --example lexer_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
