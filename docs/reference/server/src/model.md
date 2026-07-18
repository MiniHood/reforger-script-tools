# server/src/model.rs

## Purpose

Builds the file-local declaration model and symbol catalog from AST facts.

## Ownership

`SymbolCatalog` owns stable file-local symbol records, parent relationships, source metadata, and source-backed declaration facts. Indexing aggregates catalogs; scope owns visibility; resolver owns selection and semantic-like inference.

## Current Behavior

`SymbolCatalog::from_ast()` and `from_ast_with_metadata()` produce records for classes, enums/members, typedefs, functions, global/class fields, methods, constructors, destructors, parameters, locals, and `#define` names. Records preserve file-local IDs, declaration/name/selection/detail/attribute/modifier/doc spans, conditional context, and callable form. Catalog metadata identifies source kind, category, paths, and priority.

Raw type text remains source faithful. `TypeShape` provides optional qualifiers, base names, generic arguments, and array suffixes without resolving symbols; this supports typedefs, static arrays, and comma-separated field declarations. Conditional context describes visible preprocessor branches but does not evaluate them. Callable form distinguishes body implementations, qualified prototypes, and semicolon declarations. Locals are catalog facts only; visibility and shadowing remain scope concerns.

## Dependencies and Boundaries

Depends on AST, lexer spans, and path types. It does not parse directly, resolve symbols, merge files/inheritance, evaluate expressions/macros, crawl workspaces, call Workbench, or handle LSP. Paths belong to catalog metadata, not each record.

## Verification

Model tests cover symbol kinds, parentage, spans, type shapes, metadata/categories, conditionals, callable forms, declarator expansion, locals, and macros.

## Future Direction

Workspace aggregation, lexical scope, and semantic resolution remain separate layers. Type-shape expansion follows demonstrated consumer needs.
