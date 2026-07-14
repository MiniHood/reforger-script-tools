# server/examples/lsp_diagnostics_corpus_report.rs

## Purpose

Generates a corpus-style Markdown report for parser diagnostics over committed malformed Enforce fixtures.

## Architecture Role

This is dev-only review tooling for parser diagnostic UX. It exercises the same parser diagnostic projection used by `textDocument/publishDiagnostics`, but against a folder of intentionally malformed fixtures instead of the real game corpus, which should normally parse without diagnostics.

## Current Behavior

The report scans `tools/fixtures/diagnostics/**/*.c` by default and writes `tools/reports/lsp-diagnostics-corpus.report.md`. It reports file counts, diagnostic counts, message frequency, files with no diagnostics, range quality issues, and bounded snippets for the first diagnostic in each file.

## Dependencies and Boundaries

The report depends on the parser and LSP diagnostic projection helpers only. It does not call Workbench, compare compiler diagnostics, publish VS Code diagnostics, or change runtime LSP behavior.

## Change Notes

Added alongside committed malformed fixtures so diagnostic message and range quality can be reviewed even when the real game corpus remains diagnostic-free.

## Future Improvements

Add expected-message/range metadata if the fixture set grows enough that simple human-review tables become too broad. Workbench comparison should remain a separate explicit validation flow.
