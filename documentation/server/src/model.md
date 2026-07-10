# server/src/model.rs

## Purpose

Owns the first file-local declaration model and symbol catalog layer.

## Architecture Role

This file sits above AST extraction and below future workspace indexing, semantic diagnostics, and LSP features. It converts one parsed source file's AST declarations into stable symbol records that editor features can consume later.

## Current Behavior

The model exposes `SymbolCatalog::from_ast()` for one source file. It produces source-backed `SymbolRecord` entries for classes, enums, enum members, typedefs, functions, global fields, class fields, methods, constructors, destructors, and parameters. Each record has a stable file-local `SymbolId`, optional parent ID, symbol kind, name span, declaration span, selection span, detail spans, attribute spans, modifier spans, raw leading doc-comment spans, preprocessor conditional context, and callable declaration form where relevant. Catalogs also carry `SourceFileMetadata` for file provenance: source kind, source category, optional absolute path, optional root path, optional relative path, and source priority. `SymbolCatalog::from_ast()` uses unknown metadata, while `from_ast_with_metadata()` lets callers provide game-data, workspace, or fixture identity.

Typedef symbols use `SymbolDetail::type_text` for the raw aliased type text, such as `string` in `typedef string FactionKey;`. Attribute storage remains span-only, while `SymbolCatalog::attribute_name()` and `record_attribute_names()` provide source-backed name views for reports and future tooling. `SymbolCatalog::type_shape()` and `record_type_shape()` derive source-backed `TypeShape` views with qualifiers, base names, generic arguments, and array suffixes without replacing raw type text or resolving symbols.

Static-array fields rely on AST field extraction for the field name and raw leading type text, while `record_type_shape()` appends suffix spans after the field name. For example, `string TAGS[COUNT];` is stored as name `TAGS`, type text `string`, and type-shape suffix `[COUNT]`.

One parser `FieldDecl` can emit multiple field symbols when the source uses a comma-separated declaration list. Each emitted `Field` or `GlobalField` reuses the declaration's attributes, modifiers, doc comments, and shared type text, but stores its own name and declarator-local span so later fields do not pollute type text or array suffix extraction.

Source categories classify files as workspace, Game, GameCode, GameLib, Core, generated, Workbench, docs/Doxygen, test/autotest, or unknown. The default editor-completion policy includes workspace/runtime categories and excludes docs, tests, Workbench, and unknown source categories; raw index/debug paths still keep every symbol.

Conditional context is descriptive only. It stores the visible `#if`, `#ifdef`, `#ifndef`, `#elif`, and `#else` branch stack around a symbol, including macro/expression text when it can be sliced from source. The model does not evaluate macros or choose active branches.

Callable form is attached to functions, methods, constructors, and destructors. A callable with a body is an implementation, a semicolon form with `proto`, `native`, or `external` is a prototype, and a semicolon form without those markers is a declaration.

Classes own fields and callable members. Enums own enum members. Functions, methods, constructors, and destructors own declaration parameters and local variables discovered from AST statement/header syntax. Local variables are source facts for hover/debug and future local completion; the model does not build lexical scopes, evaluate visibility, or resolve shadowing. Global fields are top-level symbols. Non-declaration callable fragments are counted for review but are not emitted as parameter symbols.

## Dependencies and Boundaries

This file depends on the AST layer, lexer spans, and standard path types for optional source metadata. It must not parse source text directly, resolve symbols, merge declarations across files, infer inheritance, normalize type text, evaluate enum values or defaults, inspect workspace files, call Workbench, or handle LSP requests. Workspace-wide lookup belongs in a later index layer. File identity belongs on `SymbolCatalog`; `SymbolRecord` must stay file-local and must not duplicate paths or source kind.

## Change Notes

- Added the initial source-backed symbol catalog layer.
- Added file-local stable symbol IDs and parent-child relationships.
- Added catalog records for top-level declarations, class members, enum members, and parameters.
- Kept non-declaration callable fragments out of parameter symbols while preserving a review count.
- Added typedef aliased type text to symbol detail records.
- Added catalog helpers for resolving attribute names from stored attribute spans without changing record storage.
- Added source-backed structured type shape helpers for base names, qualifiers, generic arguments, and array suffixes.
- Added file-level catalog metadata for source kind, paths, relative path, and priority without changing symbol records.
- Documented that static-array suffixes remain type-shape facts while field records keep the actual field identifier and leading type text.
- Added source categories, source-backed preprocessor conditional context, and callable declaration form metadata for index/query policy without filtering raw source facts.
- Added comma-separated field-list expansion so each declarator becomes a separate field symbol with shared type text and local span.
- Added `LocalVariable` records under containing callables for local declarations, `foreach` variables, and `for` initializer declarations.

## Future Improvements

- Add a workspace index over many file-local catalogs.
- Add lexical scope modeling separately before using local variables for semantic completion or diagnostics.
- Expand type-shape helpers only when future hover, completion, or indexing work needs more detail.
- Add semantic resolution separately after catalog and index behavior are validated.
