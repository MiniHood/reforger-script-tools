# server/examples/lsp_semantic_tokens_report.rs

## Purpose

Provides a dev-only fixture report for the LSP semantic-token path that drives Enforce coloring.

## Architecture Role

This example sits above `server/src/lsp.rs` and exercises the same semantic-token builder used by `textDocument/semanticTokens/full` and hover debug. It exists so color classification can be reviewed without relying only on VS Code's token inspector.

## Current Behavior

The report uses a small game-data-shaped inline source containing comments, attributes, modifiers, a class, fields, a method, parameters, a local variable, strings, numbers, punctuation, and preprocessor lines. It writes `tools/reports/lsp-semantic-tokens-fixtures.report.md` with parse diagnostics, encoded-token size, decoded token count, token text, LSP range, semantic token type, modifiers, and palette color.

TextMate scopes are intentionally not involved. The report verifies the single coloring path: Rust LSP semantic tokens consumed by the bundled semantic-token theme.

## Dependencies and Boundaries

The report uses only Rust standard library code and the existing LSP helper function. It accepts `--out <path>` for custom output. It must remain dev-only review tooling and must not register a VS Code command, mutate source, or add a second coloring implementation.

## Change Notes

- Added with the removal of the Enforce TextMate grammar so semantic-token classification has a stable review artifact.

## Future Improvements

- Add corpus-scale semantic-token sampling only if fixture-level reports and hover debug are not enough to diagnose coloring issues.
