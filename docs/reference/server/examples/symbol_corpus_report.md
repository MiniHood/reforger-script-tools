# server/examples/symbol_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report from the file-local declaration model and symbol catalog.

## Ownership

This is developer review tooling for model/catalog quality. It is not VS Code runtime behavior, not an LSP entrypoint, not workspace indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds an `AstSourceFile`, creates a `SymbolCatalog` with game-data source metadata, and writes `tools/reports/symbol-corpus.report.md` by default. The summary shows source kind, source root, and source priority for the scanned corpus. It reports corpus totals, parse diagnostics, total symbols, missing symbol names, parent-child coverage, non-declaration callable fragments, attribute/doc-comment coverage, symbol kind frequencies, modifier frequency, attribute name frequency resolved through the catalog API, doc-comment coverage by symbol kind, base/type/return text frequencies, type-shape base/qualifier/generic-arity/array-suffix frequencies, duplicate top-level names with kind and per-declaration path details, regular method overload groups, constructor overload groups, destructor overload groups, sample symbols by kind, and bounded snippets for non-declaration callable fragments.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, and model modules. It must not duplicate model behavior, resolve symbols, create a workspace index, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example symbol_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
