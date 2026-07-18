# server/examples/lsp_report.rs

## Purpose

Generates a dev-only fixture report for the current LSP document-symbol path.

## Architecture Role

This example sits above `server/src/lsp.rs` and calls the same document-symbol conversion helper used by `textDocument/documentSymbol`. It is human/Codex review tooling for the LSP scaffold, not VS Code runtime behavior, not game-data indexing, and not Workbench validation.

## Current Behavior

The report scans committed parser fixtures under `tools/fixtures/parser` by default and writes `tools/reports/lsp-fixtures.report.md`. It records parse diagnostics, top-level symbol count, nested symbol count, max tree depth, unknown label count, range sanity failures, and a bounded symbol tree per file.

The companion wrapper `tools/lsp-report.mjs` runs the Rust example through Cargo. The example also supports `--fixtures <path>` and `--out <path>` for targeted review.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate LSP helper. It must not register a VS Code command, add package scripts, start the language server process, build workspace/game-data indexes, or dump full source/AST/index data.

## Change Notes

- Added the first LSP fixture report so document-symbol behavior can be reviewed across real parser fixtures before adding hover, diagnostics, completion, or definition.

## Future Improvements

- Add targeted fixture sets for future LSP features as they are implemented.
- Add report sections for diagnostics, hover, completion, or definition only when those LSP features exist.
