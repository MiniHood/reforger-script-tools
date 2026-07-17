# server/examples/lsp_signature_help_report.rs

## Purpose

Generates a dev-only fixture report for LSP signature help.

## Architecture Role

This example exercises the same Rust-side signature-help projection used by `textDocument/signatureHelp`, but writes a bounded Markdown report for human review under `tools/reports/`.

## Current Behavior

The report covers fixture checks for regular functions, member methods, constructors, `new` expressions, attributes, named arguments, enum-typed parameters, optional/defaulted parameters, static calls, and non-call misses.

## Dependencies and Boundaries

Depends on the Rust LSP signature-help API and small source fixtures built in the example. It is dev tooling only and must not become a VS Code command or runtime dependency.

## Change Notes

Added with the first source-backed signature-help slice.

## Future Improvements

Add corpus-scale sampling if fixture checks stop being enough to review real game-data callable shapes.
