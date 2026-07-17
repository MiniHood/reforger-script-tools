# tools/lsp-signature-help-report.mjs

## Purpose

Runs the Rust LSP signature-help fixture report from Node and writes `tools/reports/lsp-signature-help-fixtures.report.md` by default.

## Architecture Role

This is dev-only report glue. It exists so contributors can invoke the signature-help proof path the same way other LSP reports are invoked from `tools/`.

## Current Behavior

The wrapper runs `cargo run --manifest-path server/Cargo.toml --example lsp_signature_help_report` and forwards any additional command-line arguments to the Rust example.

## Dependencies and Boundaries

Depends on local development Rust/Cargo tooling. It is not a packaged extension runtime dependency, not a VS Code command, and not language logic.

## Change Notes

Added with the source-backed LSP signature-help slice.

## Future Improvements

Keep this wrapper thin. Add broader corpus reporting in a separate Rust example if signature-help sampling needs game-data scale.
