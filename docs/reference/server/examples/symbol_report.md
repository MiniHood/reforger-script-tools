# server/examples/symbol_report.rs

## Purpose

Generates a fixture-scale Markdown report from `SymbolCatalog` records.

## Ownership

This is developer review tooling for symbol tree readability. It is not VS Code runtime behavior, not an LSP entrypoint, not workspace indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example parses committed parser fixtures, builds an `AstSourceFile`, creates a `SymbolCatalog` with fixture metadata, and writes `tools/reports/symbol-fixtures.report.md` by default. Each fixture summary shows source kind, relative path, and source priority. It renders top-level symbols and nested child symbols as a readable tree with kind, name, symbol ID, parent ID, line/column plus byte spans, detail text including typedef aliased type text, attribute names resolved through the catalog API, modifiers, doc-comment count, and cleaned doc preview.

It accepts `--out <path>`.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, and model modules. It must not duplicate model extraction behavior, resolve symbols, create a workspace index, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example symbol_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
