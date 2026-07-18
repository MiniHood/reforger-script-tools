# server/src/syntax.rs

## Purpose

Defines parser syntax-tree data structures for Enfusion Script source.

## Ownership

This module owns `SyntaxKind`, syntax nodes/elements, diagnostics, and parse results. It is the parser output contract consumed by AST and later language-engine layers.

## Current Behavior

Nodes retain byte spans and child elements; tokens remain elements so source stays external. Syntax kinds represent declarations, blocks, initializer expressions, statements, control flow, loops, switch sections, expressions, and recovery `EmptyDecl`/error structure.

Fields, locals, and declaration-form `for` initializers share `TypeRef` plus `DeclaratorList` and `Declarator` boundaries. Foreach variables retain their distinct header form. `ForHeader`, `ForeachHeader`, and switch sections preserve syntax grouping for later consumers without evaluating behavior.

## Dependencies and Boundaries

Depends only on lexer token/span types. It does not import editor APIs, crawl files, analyze semantics, index symbols, call Workbench, or handle LSP.

## Verification

Parser and syntax tests exercise node shapes, spans, declarations, expressions, statement forms, declarators, and recovery.

## Future Direction

Syntax shape expands only with parser needs; typed interpretation remains AST/semantic-layer work.
