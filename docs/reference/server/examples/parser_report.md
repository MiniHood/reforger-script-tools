# server/examples/parser_report.rs

## Purpose

Generates a human-readable Markdown report showing parser input vs syntax-tree output for committed parser fixtures.

## Architecture Role

This is developer review tooling for the Rust parser scaffold. It is not VS Code runtime behavior and is not a language server entrypoint.

## Current Behavior

Running `cargo run --manifest-path server/Cargo.toml --example parser_report` writes `tools/reports/parser-fixtures.report.md` by default. The report includes per-fixture summaries, parse diagnostic counts, syntax-kind counts, preserved token counts, and a syntax-tree outline. It includes both focused parser fixtures and larger game-data-derived class excerpts, including editor preview, Workbench formatter, optional-semicolon, and nested field-initializer call examples.

The command also accepts `--out <path>` for an alternate report path.

## Dependencies and Boundaries

The report generator uses only Rust standard library APIs and the crate parser/syntax modules. It must not implement model, index, LSP, Workbench validation, or runtime extension behavior. Generated reports belong under ignored output paths such as `tools/reports/`.

## Change Notes

- Added the first parser fixture report generator for human/Codex review.
- Added larger game-code parser fixtures to the report input set.
- Added editor preview and Workbench formatter fixtures to the report input set.
- Added optional-semicolon fixture coverage to the report input set.
- Added nested field-initializer call fixture coverage to the report input set.

## Future Improvements

- Add source excerpts around diagnostics if parser fixtures intentionally cover recovery cases.
- Add corpus-scale parser summaries after fixture-level parser behavior stabilizes.
