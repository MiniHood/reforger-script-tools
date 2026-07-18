# server/src/expression_type.rs

## Purpose

Provides source-backed expression owner and receiver-type facts for resolver inference.

## Ownership

`ExpressionTypeEnvironment` owns expression traversal and reusable type-text interpretation below the resolver. It centralizes receiver inference so hover, definition, and completion do not reconstruct receiver chains independently.

## Current Behavior

The environment accepts source text, a file index, parse tree, lexical scope model, and optional external index. It infers best-effort owner/static facts for names, calls, member/index access, `new`, constructor-style typedef/class calls, casts, parentheses, literals, `this`, `super`, type parameters, fields, and `auto` locals. Helpers strip source qualifiers, read `TypeFacts`, expand typedef/base owners, infer collection/static-array indexing, substitute class generic return parameters, and classify primitive literals.

Inference is deliberately source-backed. It supplies candidates and receiver facts; resolver still selects a symbol and reports resolution reasons.

## Dependencies and Boundaries

Depends on AST expressions, index/model records, lexical scope, parser `Parse`, and `TypeFacts`. It does not parse independently, handle LSP, mutate indexes, resolve overloads, evaluate expressions or macros, or claim compiler truth.

## Verification

Unit tests cover owner extraction, collection index results, generic-return substitution, and resolver consumers exercise chained receiver inference.

## Future Direction

Deeper inference and overload handling require a dedicated semantic layer; this module remains reusable source-backed infrastructure.
