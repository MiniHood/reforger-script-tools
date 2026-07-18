# server/examples/lsp_corpus_report.rs

## Purpose

Generates a dev-only corpus report for the current LSP document-symbol projection across downloaded or explicitly provided Reforger script data.

## Ownership

This example sits above `server/src/lsp.rs` and exercises the same document-symbol conversion helper used by `textDocument/documentSymbol`. It checks the VS Code-facing projection layer after parser, AST, model, index, query, and display have already produced source-backed symbols.

It is review tooling only. It is not VS Code runtime behavior, not workspace indexing, not cache persistence, and not Workbench validation.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for labeling the generated timing profile.

The report includes file count, byte count, lossy decode count, document-symbol totals, document-symbol kind frequency, LSP kind mapping notes, zero-symbol file classification, parse diagnostics, unknown labels, range sanity failures, top symbol-heavy files, tree-depth summary, projection timing statistics, slowest files, slowest files per symbol, and wall-clock timing.

The companion wrapper is `tools/lsp-corpus-report.mjs`. It accepts `--release` and runs `cargo run --release` while passing `--profile-label release` to the Rust example. Debug mode remains the default.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate LSP helper. It must not dump full source files, full ASTs, full indexes, or full corpus symbol trees. It must not register VS Code commands or package scripts.

Zero-symbol classification uses the lexer for comment/trivia classification. Unknown non-empty zero-symbol files get bounded snippets for review; known empty/comment/docs-only files do not dump source.

## Verification

Run `cargo run --example lsp_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
