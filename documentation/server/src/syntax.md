# server/src/syntax.rs

## Purpose

Owns parser syntax-tree data structures for Enfusion Script source text.

## Architecture Role

This file is part of the Rust language-engine layer. It defines the full-fidelity parser output shape used by the parser and future AST, formatting, diagnostics, and LSP layers.

## Current Behavior

The syntax layer exposes syntax kinds, syntax nodes, syntax elements, parse diagnostics, and parse results. Nodes store byte spans and child elements. Tokens are preserved as syntax elements so source text remains external and can be sliced by span. `InitializerList` distinguishes field initializer braces from class and method `Block` nodes. `EmptyDecl` preserves standalone semicolons in declaration context without turning them into parser errors.

Callable body `Block` nodes can contain statement and expression syntax nodes. Statement kinds cover control flow, loops, switch labels, flow statements, local declarations, and expression statements. `ForHeader` contains `ForInitializer`, `ForCondition`, and `ForIncrement` section nodes so loop headers can be reviewed without losing full-fidelity token preservation. Expression kinds cover names, literals, calls, arguments, named arguments, member access, indexing, casts, unary/binary/assignment/ternary expressions, postfix operations, `new`, and initializer expressions.

## Dependencies and Boundaries

This file depends only on lexer token/span types. It must not import VS Code APIs, Workbench behavior, file-system crawling, semantic analysis, indexing, or LSP request handling.

## Change Notes

- Added initial full-fidelity syntax tree structures for parser scaffolding.
- Added `InitializerList` for brace-delimited field initializer syntax.
- Added `EmptyDecl` for standalone semicolon declarations.
- Added statement and expression syntax kinds for callable body parsing.

## Future Improvements

- Add typed AST wrappers in a separate AST layer when parser behavior is stable.
- Refine statement/expression syntax kinds as resolver, diagnostics, and formatting start consuming body syntax.
