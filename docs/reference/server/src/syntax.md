# server/src/syntax.rs

## Purpose

Owns parser syntax-tree data structures for Enfusion Script source text.

## Architecture Role

This file is part of the Rust language-engine layer. It defines the full-fidelity parser output shape used by the parser and future AST, formatting, diagnostics, and LSP layers.

## Current Behavior

The syntax layer exposes syntax kinds, syntax nodes, syntax elements, parse diagnostics, and parse results. Nodes store byte spans and child elements. Tokens are preserved as syntax elements so source text remains external and can be sliced by span. Brace initializer defaults are represented as structured `InitializerExpression` nodes, which keeps them distinct from class and method `Block` nodes while still exposing nested expressions. `EmptyDecl` preserves standalone semicolons in declaration context without turning them into parser errors.

Callable body `Block` nodes can contain statement and expression syntax nodes. Statement kinds cover control flow, loops, switch sections and labels, flow statements, local declarations, and expression statements. `ForHeader` contains `ForInitializer`, `ForCondition`, and `ForIncrement`; declaration-shaped `ForInitializer` nodes own a nested `LocalDeclStatement`. `ForeachHeader` contains `ForeachVariableList`, `ForeachVariable`, and `ForeachIterable` nodes. `SwitchStatement` owns `SwitchSection` groups, each containing `CaseClause` / `DefaultClause` labels and the following statements until the next section. Expression kinds cover names, literals, calls, arguments, named arguments, member access, indexing, casts, unary/binary/assignment/ternary expressions, postfix operations, `new`, and initializer expressions.

Fields, local declaration statements, and declaration-form `for` initializers use one parser-owned declaration shape: optional existing modifier syntax, `TypeRef`, then `DeclaratorList` containing `Declarator` nodes. A declarator owns its name/array suffix and optional equals/default expression; the list owns comma tokens and the declaration owns its terminator/trivia. `ForeachVariable` is a distinct header form with direct `TypeRef` and single `Declarator` children because it has no default initializer.

## Dependencies and Boundaries

This file depends only on lexer token/span types. It must not import VS Code APIs, Workbench behavior, file-system crawling, semantic analysis, indexing, or LSP request handling.

## Change Notes

- Added initial full-fidelity syntax tree structures for parser scaffolding.
- Added structured brace initializer syntax for field and body expressions.
- Added `EmptyDecl` for standalone semicolon declarations.
- Added statement and expression syntax kinds for callable body parsing.
- Added structured `for` initializer declarations, `foreach` header parts, and switch sections.
- Added parser-owned `TypeRef` / `DeclaratorList` / `Declarator` boundaries for field and local declaration consumers.

## Future Improvements

- Add typed AST wrappers in a separate AST layer when parser behavior is stable.
- Refine statement/expression syntax kinds as resolver, diagnostics, and formatting start consuming body syntax.
