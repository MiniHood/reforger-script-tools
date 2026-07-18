---
title: Keep Declaration Boundaries in the Parser CST
date: 2026-07-18
category: best-practices
module: rust-language-engine
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "Adding a syntax-backed AST or model view for Enfusion declarations"
  - "Recovering malformed or deeply nested editor input"
tags: [parser, cst, ast, recovery, language-server]
---

# Keep Declaration Boundaries in the Parser CST

## Context

The language engine must stay responsive while an editor document is incomplete or deeply nested, while still giving AST, model, and index consumers one authoritative view of declarations. Splitting declaration tokens again in the AST creates a second grammar that can drift from parser recovery.

## Guidance

Make the parser own both safeguards and syntax boundaries:

1. Route every recursive parser entry through the shared depth budget in [`server/src/parser.rs`](../../../server/src/parser.rs). At the limit, emit one diagnostic, create an `Error` region, and leave the parent's delimiter available for normal unwind.
2. Structure recognized fields, local declarations, declaration-form `for` initializers, and `foreach` variables as direct CST children. [`structure_declaration`](../../../server/src/parser.rs) and [`structure_foreach_variable`](../../../server/src/parser.rs) are the boundary owners.
3. Let AST extraction read `TypeRef`, `DeclaratorList`, and `Declarator` children. [`declaration_declarators`](../../../server/src/ast.rs) and [`push_foreach_variable`](../../../server/src/ast.rs) should not reclassify raw declaration tokens.
4. Preserve malformed tails as recovery syntax instead of manufacturing declarators or symbols from guessed token shapes.

## Why This Matters

One parser-owned shape keeps source spans, defaults, comma declarators, and recovery behavior consistent for AST, model, index, and LSP consumers. The shared recursion budget prevents hostile or half-typed nesting from exhausting the language-server process without claiming an Enfusion language limit.

## When to Apply

- A new declaration form needs AST/model/index facts.
- A parser recovery path is introduced or changed.
- An AST helper begins scanning tokens to rediscover a syntactic boundary the parser can represent.

## Examples

`FieldDecl` and `LocalDeclStatement` use a `TypeRef` plus `DeclaratorList`; `ForeachVariable` keeps its header role but directly owns one `TypeRef` and `Declarator`. This lets typed, `auto`, and index/value `foreach` facts use the same parser-owned contract.

## Related

- [Parser reference](../../reference/server/src/parser.md)
- [AST reference](../../reference/server/src/ast.md)
- [Parser resilience plan](../../plans/2026-07-18-008-fix-parser-resilience-and-cst-boundaries-plan.md)
