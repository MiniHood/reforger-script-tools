# server/src/expression_type.rs

## Purpose

Owns the first source-backed expression type and receiver-owner facts used by resolver inference.

## Architecture Role

This file sits above `index`, `scope`, AST expression views, and `type_facts`, and below the resolver. It centralizes expression owner/type inference, type-text interpretation, receiver owner extraction, generic return substitution, typedef/base owner expansion, primitive literal typing, and indexed-symbol-to-expression-type conversion.

## Current Behavior

`ExpressionType` records the inferred owner type, whether the expression is static/type-like, and the raw source-backed type text when available. `ExpressionTypeEnvironment` is the authoritative source-backed expression typing path for resolver consumers. It accepts source text, file-local index, parse tree, lexical scope model, and optional external index, then infers expression types for names, calls, member access, index access, `new`, constructor-style type calls, casts, parenthesized/nested expressions, primitive literals, boolean/null literal names, `this`, `super`, class type parameters, top-level symbols, class members, and auto locals.

The module exposes helpers for:

- extracting owner names from raw type text
- stripping source-backed qualifiers such as `ref`, `notnull`, `autoptr`, `const`, `out`, and `inout`
- reading indexed symbol type/return/base facts through `TypeFacts`
- expanding typedef owners and exact base owners for member lookup
- inferring collection index result owners for `array<T>`, `set<T>`, `map<K, V>`, strings, vectors, and static arrays
- substituting class generic return type parameters from the current receiver type text
- classifying source-backed literal values including strings, booleans, decimal/hex numbers, and `null`
- inferring `auto` locals from the full default-expression span, including member-access defaults such as `auto value = receiver.field`
- treating a call whose callee is a known class or typedef as a constructor-style instance expression after normal function lookup fails

Resolver now asks `ExpressionTypeEnvironment` for receiver-chain typing instead of owning that traversal itself.

## Dependencies and Boundaries

This file depends on AST expression views, `SymbolIndex`, `IndexedSymbol`, `SymbolKind`, lexical scope, parser `Parse`, and `TypeFacts`. It must not parse source on its own, handle LSP requests, query Workbench, perform overload resolution, evaluate expressions, choose active preprocessor branches, mutate indexes, or claim compiler truth.

## Change Notes

- Extracted reusable receiver/type-text helpers from `resolver.rs`.
- Added `ExpressionTypeEnvironment` and moved receiver-chain expression typing from resolver into this module.
- Added unit tests for owner extraction, collection index result typing, and generic return substitution.

## Future Improvements

- Add report coverage for expression type inference reasons once resolver delegates traversal here.
- Keep semantic type checking separate; this module should remain source-backed inference infrastructure until a richer semantic layer is designed.
