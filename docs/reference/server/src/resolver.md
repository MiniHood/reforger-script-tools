# server/src/resolver.rs

## Purpose

Selects source-backed symbol candidates for a cursor token and records why the selection was made or withheld.

## Ownership

`ReferenceResolver` owns identifier context classification, candidate selection, precedence, and resolution reasons for hover, definition, completion, semantic tokens, and reference analysis. AST owns expression shape, scope owns lexical visibility, `expression_type` owns receiver facts, and index/query layers own stored lookup facts.

## Current Behavior

The resolver returns `ReferenceResolution` with selected candidate, provenance, alternatives, and explicit reason. It handles declaration names, type positions, value/callable positions, local/parameter scope, file-local symbols, and ordered workspace/game-data external indexes. File-local facts win where appropriate; external order is preserved rather than pre-merged.

Member access uses AST expression views and `ExpressionTypeEnvironment` for names, calls, indexing, casts, `new`, parenthesized chains, `this`, `super`, static type-like receivers, typedef/base expansion, generic return substitution, literals, and auto-local defaults. It supports source-backed enum members, pseudo `Class` members, and `Type.Cast` handling without raw receiver text scanning. The resolver remains best effort: it neither evaluates expressions nor resolves overloads.

Named call and attribute labels intentionally return explicit non-symbol reasons; their values remain resolvable. Preprocessor directives are non-symbol targets; `#define` uses resolve to indexed macros when present and otherwise receive an explicit macro reason. Syntax-span safeguards exclude comments, whitespace, literals, punctuation, and modifiers from misleading enclosing-declaration matches.

## Dependencies and Boundaries

Depends on lexer tokens, parser syntax, AST expressions, scope, index/query facts, model kinds, `type_facts`, and `expression_type`. It may construct parse/scope in standalone helpers, but it does not build indexes, evaluate values/macros, merge `modded` classes, call Workbench, mutate index state, or handle LSP protocol requests.

## Verification

Resolver tests cover local shadowing, type/value contexts, external precedence, member chains, casts, static owners, named arguments, macros/directives, incomplete source, and hover/definition projections.

## Future Direction

Deeper expression inference, overload selection, and semantic inheritance require a real semantic model. Receiver typing remains outside resolver in `expression_type`.
