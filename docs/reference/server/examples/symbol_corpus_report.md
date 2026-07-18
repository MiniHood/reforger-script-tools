# server/examples/symbol_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report from the file-local declaration model and symbol catalog.

## Architecture Role

This is developer review tooling for model/catalog quality. It is not VS Code runtime behavior, not an LSP entrypoint, not workspace indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds an `AstSourceFile`, creates a `SymbolCatalog` with game-data source metadata, and writes `tools/reports/symbol-corpus.report.md` by default. The summary shows source kind, source root, and source priority for the scanned corpus. It reports corpus totals, parse diagnostics, total symbols, missing symbol names, parent-child coverage, non-declaration callable fragments, attribute/doc-comment coverage, symbol kind frequencies, modifier frequency, attribute name frequency resolved through the catalog API, doc-comment coverage by symbol kind, base/type/return text frequencies, type-shape base/qualifier/generic-arity/array-suffix frequencies, duplicate top-level names with kind and per-declaration path details, regular method overload groups, constructor overload groups, destructor overload groups, sample symbols by kind, and bounded snippets for non-declaration callable fragments.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, and model modules. It must not duplicate model behavior, resolve symbols, create a workspace index, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added corpus-scale symbol catalog reporting for real downloaded/manual Reforger script data.
- Added richer human-review sections for modifiers, attributes, doc coverage, duplicate names, overload groups, sample symbols, and callable-fragment snippets.
- Attribute name frequency now uses `SymbolCatalog::attribute_name()` instead of local parsing.
- Duplicate top-level names now list each declaration as kind, name, path, and line.
- Constructor and destructor overload groups are reported separately from regular methods.
- Added corpus visibility for source-backed type-shape base names, qualifiers, generic arities, and array suffixes.
- Corpus catalogs now carry game-data source metadata with absolute path, scripts root, relative path, and priority.

## Future Improvements

- Add sections for index-specific lookup quality after a real workspace/game-data index exists.
- Keep corpus findings as planning evidence only; Workbench remains compiler truth.
