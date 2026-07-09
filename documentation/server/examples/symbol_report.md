# server/examples/symbol_report.rs

## Purpose

Generates a fixture-scale Markdown report from `SymbolCatalog` records.

## Architecture Role

This is developer review tooling for symbol tree readability. It is not VS Code runtime behavior, not an LSP entrypoint, not workspace indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example parses committed parser fixtures, builds an `AstSourceFile`, creates a `SymbolCatalog`, and writes `tools/reports/symbol-fixtures.report.md` by default. It renders top-level symbols and nested child symbols as a readable tree with kind, name, symbol ID, parent ID, line/column plus byte spans, detail text including typedef aliased type text, attribute names resolved through the catalog API, modifiers, doc-comment count, and cleaned doc preview.

It accepts `--out <path>`.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, and model modules. It must not duplicate model extraction behavior, resolve symbols, create a workspace index, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added fixture-scale symbol tree reporting for human review.
- Improved report readability with line/column locations, attribute names, `<none>` list markers, and cleaned doc previews.
- Typedef symbols now show their raw aliased type text through existing detail rendering.
- Attribute name rendering now uses `SymbolCatalog::attribute_name()` instead of local parsing.

## Future Improvements

- Add focused fixture sections if future model fields become hard to inspect in the compact tree format.
