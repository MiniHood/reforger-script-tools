# server/examples/lsp_diagnostics_report.rs

## Purpose

Generates a small Markdown report for parser diagnostics as projected through the LSP diagnostic format.

## Architecture Role

This is dev-only review tooling for the Rust LSP layer. It exercises the same parser diagnostic projection used by `textDocument/publishDiagnostics` so diagnostic message, source, code, severity, range, and snippet quality can be reviewed without launching VS Code.

## Current Behavior

The report writes `tools/reports/lsp-diagnostics-fixtures.report.md` by default. It uses small inline valid and malformed Enforce-shaped snippets, runs `parse_source`, converts parse diagnostics with `parser_diagnostics_for_source`, and renders a summary table plus bounded source snippets around each diagnostic range.

## Dependencies and Boundaries

The report depends on the parser and LSP diagnostic projection helpers only. It does not call Workbench, perform semantic validation, read game-data corpora, publish VS Code diagnostics, or register a user-facing extension command.

## Change Notes

- Added as the first focused parser-diagnostics UX report.
- The report is intended to catch bad diagnostic ranges such as zero-width locations that are hard to see in the editor.

## Future Improvements

- Add corpus-scale diagnostic samples once parser diagnostics are expected in real workspace scripts.
- Add Workbench comparison only if a future validation flow can run Workbench diagnostics safely and explicitly.
