# server/examples/lsp_hover_report.rs

## Purpose

Provides a dev-only fixture report for the LSP hover path.

## Architecture Role

This example sits above the Rust LSP helper API. It exercises the same file-local hover projection used by `textDocument/hover` without starting VS Code or a stdio server.

## Current Behavior

The report reads targeted committed parser fixtures plus a small inline hover coverage source for enum/global-field cases. It writes `tools/reports/lsp-hover-fixtures.report.md` with target positions, hit/miss state, selected symbol kind/name, parse diagnostics, elapsed time, and a compact Markdown hover preview.

## Dependencies and Boundaries

The report uses only Rust standard library code and the existing LSP helper functions. It must remain dev-only review tooling. It must not perform semantic lookup, game-data indexing, Workbench validation, runtime logging, VS Code command registration, or source mutation.

## Change Notes

- Added the first hover fixture report alongside the LSP hover feature.
- Inline enum/global source is used because the current committed parser fixture set does not contain those hover shapes.

## Future Improvements

- Add larger game-data-derived hover samples when needed.
- Add a corpus hover report only after there is a useful set of deterministic hover points to sample.
