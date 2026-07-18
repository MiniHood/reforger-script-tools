# server/examples/lsp_signature_help_report.rs

## Purpose

Generates a dev-only fixture report for LSP signature help.

## Ownership

This example exercises the same Rust-side signature-help projection used by `textDocument/signatureHelp`, but writes a bounded Markdown report for human review under `tools/reports/`.

## Current Behavior

The report covers fixture checks for regular functions, member methods, constructors, `new` expressions, attributes, named arguments, enum-typed parameters, optional/defaulted parameters, static calls, and non-call misses.

## Dependencies and Boundaries

Depends on the Rust LSP signature-help API and small source fixtures built in the example. It is dev tooling only and must not become a VS Code command or runtime dependency.

## Verification

Run `cargo run --example lsp_signature_help_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
