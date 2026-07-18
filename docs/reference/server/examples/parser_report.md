# server/examples/parser_report.rs

## Purpose

Generates a human-readable Markdown report showing parser input vs syntax-tree output for committed parser fixtures.

## Ownership

This is developer review tooling for the Rust parser scaffold. It is not VS Code runtime behavior and is not a language server entrypoint.

## Current Behavior

Running `cargo run --manifest-path server/Cargo.toml --example parser_report` writes `tools/reports/parser-fixtures.report.md` by default. The report includes per-fixture summaries, parse diagnostic counts, syntax-kind counts, preserved token counts, and a syntax-tree outline. It includes both focused parser fixtures and larger game-data-derived class excerpts, including editor preview, Workbench formatter, optional-semicolon, nested field-initializer call, and local-block-symbol examples.

The command also accepts `--out <path>` for an alternate report path.

## Dependencies and Boundaries

The report generator uses only Rust standard library APIs and the crate parser/syntax modules. It must not implement model, index, LSP, Workbench validation, or runtime extension behavior. Generated reports belong under ignored output paths such as `tools/reports/`.

## Verification

Run `cargo run --example parser_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
