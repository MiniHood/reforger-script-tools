# server/src/ast.rs

## Purpose

Provides typed, source-backed AST views over the parser's syntax tree.

## Ownership

`ast.rs` is the file-local bridge between CST structure and consumers such as the model, resolver, index, and formatter. It owns ergonomic declaration and expression views, source spans, and best-effort extraction. Parser CST nodes remain the authority for structure; semantic layers own meaning.

## Current Behavior

`AstSourceFile` exposes declarations for classes, enums, typedefs, functions, fields, members, callables, attributes, modifiers, raw leading documentation comments, and empty semicolon declarations. `TextValue` keeps extracted names, type text, defaults, and spans backed by the original source. Unknown or incomplete syntax returns `Option` rather than invented facts.

Fields and locals read parser-owned `TypeRef`, `DeclaratorList`, and `Declarator` boundaries. This preserves comma-separated fields, static-array suffixes, brace defaults, `for` initializer locals, and `foreach` variables without token re-splitting. Parameter views preserve raw text, type/default text, declaration modifiers, and the distinction between true declaration parameters and preserved non-declaration callable fragments.

Expression views cover names, literals, calls, member/index access, casts, `new`, unary/binary/assignment/ternary expressions, initializer expressions, and parenthesized forms. Offset lookup, member-access context, and named-argument-label helpers are the shared syntax-understanding path for resolver consumers and attribute/body expressions alike.

## Dependencies and Boundaries

Depends only on lexer spans/tokens and parser syntax types. It does not resolve symbols, evaluate values or aliases, infer inheritance, inspect files, query game data or Workbench, emit diagnostics, or handle LSP requests.

## Verification

Focused AST tests cover declaration extraction, attributes, callable classification, declarators, locals, defaults, and expression lookup. Parser fixtures provide broader source-shape coverage.

## Future Direction

Lexical scope, normalized type shapes, semantic modeling, and workspace indexing remain separate layers. AST views should stay source-faithful and file-local.
