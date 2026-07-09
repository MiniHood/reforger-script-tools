# server/src/model.rs

## Purpose

Owns the first file-local declaration model and symbol catalog layer.

## Architecture Role

This file sits above AST extraction and below future workspace indexing, semantic diagnostics, and LSP features. It converts one parsed source file's AST declarations into stable symbol records that editor features can consume later.

## Current Behavior

The model exposes `SymbolCatalog::from_ast()` for one source file. It produces source-backed `SymbolRecord` entries for classes, enums, enum members, typedefs, functions, global fields, class fields, methods, constructors, destructors, and parameters. Each record has a stable file-local `SymbolId`, optional parent ID, symbol kind, name span, declaration span, selection span, detail spans, attribute spans, modifier spans, and raw leading doc-comment spans. Typedef symbols use `SymbolDetail::type_text` for the raw aliased type text, such as `string` in `typedef string FactionKey;`. Attribute storage remains span-only, while `SymbolCatalog::attribute_name()` and `record_attribute_names()` provide source-backed name views for reports and future tooling.

Classes own fields and callable members. Enums own enum members. Functions, methods, constructors, and destructors own declaration parameters. Global fields are top-level symbols. Non-declaration callable fragments are counted for review but are not emitted as parameter symbols.

## Dependencies and Boundaries

This file depends on the AST layer and lexer spans. It must not parse source text directly, resolve symbols, merge declarations across files, infer inheritance, normalize type text, evaluate enum values or defaults, inspect workspace files, call Workbench, or handle LSP requests. Workspace-wide lookup belongs in a later index layer.

## Change Notes

- Added the initial source-backed symbol catalog layer.
- Added file-local stable symbol IDs and parent-child relationships.
- Added catalog records for top-level declarations, class members, enum members, and parameters.
- Kept non-declaration callable fragments out of parameter symbols while preserving a review count.
- Added typedef aliased type text to symbol detail records.
- Added catalog helpers for resolving attribute names from stored attribute spans without changing record storage.

## Future Improvements

- Add a workspace index over many file-local catalogs.
- Add a separate type-shape layer for structured type facts without replacing raw source-backed type text.
- Add semantic resolution separately after catalog and index behavior are validated.
