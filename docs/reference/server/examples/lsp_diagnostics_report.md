# server/examples/lsp_diagnostics_report.rs

## Purpose

Generates a small Markdown report for parser diagnostics as projected through the LSP diagnostic format.

## Ownership

This is dev-only review tooling for the Rust LSP layer. It exercises the same parser diagnostic projection used by `textDocument/publishDiagnostics` so diagnostic message, source, code, severity, range, and snippet quality can be reviewed without launching VS Code.

## Current Behavior

The report writes `tools/reports/lsp-diagnostics-fixtures.report.md` by default. It uses small inline valid and malformed Enforce-shaped snippets, runs `parse_source`, converts parse diagnostics with `parser_diagnostics_for_source`, and renders a summary table plus bounded source snippets around each diagnostic range.

## Dependencies and Boundaries

The report depends on the parser and LSP diagnostic projection helpers only. It does not call Workbench, perform semantic validation, read game-data corpora, publish VS Code diagnostics, or register a user-facing extension command.

## Verification

Run `cargo run --example lsp_diagnostics_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
