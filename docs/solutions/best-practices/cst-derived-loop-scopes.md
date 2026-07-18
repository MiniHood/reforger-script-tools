---
title: Derive Loop Variable Visibility from CST Structure
date: 2026-07-18
category: best-practices
module: rust-language-engine
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "Adding lexical visibility for parser-discovered locals"
  - "Projecting control-flow declarations into resolver, completion, or semantic tokens"
tags: [scope, cst, resolver, completion, semantic-tokens]
---

# Derive Loop Variable Visibility from CST Structure

## Context

Indexed local symbols retain declaration spans, but a span alone cannot express a loop variable's lifetime. Attaching all locals to the innermost enclosing block leaks `for` and `foreach` bindings after the loop and can expose a `foreach` binding while its iterable is evaluated.

## Guidance

Use parser-owned statement structure to create lexical visibility regions:

1. Map declaration-form `ForInitializer` nodes to their enclosing `ForStatement`; the resulting scope covers condition, increment, and body only.
2. Map `ForeachVariableList` declarations to the following statement body, not the header or iterable expression.
3. Attach indexed locals through those CST-derived declaration regions; keep ordinary local variables block-owned.
4. Test resolver/completion/semantic-token lookup both inside each loop and immediately after it.

## Why This Matters

The resolver, completion, receiver inference, and semantic tokens all use the lexical scope model. One incorrect attachment therefore creates several editor defects at once: stale locals resolve after a loop and `foreach` variables can shadow the iterable before they exist.

## Related

- [Scope reference](../../reference/server/src/scope.md)
- [AST/CST boundary practice](parser-owned-cst-boundaries.md)
