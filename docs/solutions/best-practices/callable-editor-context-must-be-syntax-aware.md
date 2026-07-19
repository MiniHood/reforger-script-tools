---
title: Keep Callable Editor Context Syntax-Aware
date: 2026-07-18
category: best-practices
module: lsp-callable-interaction
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "Completion and Signature Help share callable argument context"
  - "Editor input may be nested, incomplete, or contain quoted defaults"
tags: [lsp, completion, signature-help, callable-context]
---

# Keep Callable Editor Context Syntax-Aware

## Context

Completion and Signature Help both depend on locating the active callable argument list and interpreting its parameters.
Small text-scanning shortcuts therefore affect both editor features at once.

## Guidance

Prefer parser structure when selecting the active argument list, and continue traversal so the smallest enclosing call wins.
When a fallback scanner is necessary, keep delimiter, quote, and escape state together.
Do not treat every `<` as a generic opener: relational expressions are valid arguments while the user is editing. A fallback generic scan must mirror the parser's continuation rule: after its matching `>` (or `>>`), a generic expression continues only into `.`, `(`, or `[`. Count `>>` as two generic closers.

Compute active parameters per callable candidate, then expose only a valid selected-candidate value.
Normalize named labels at collection time because parameter matching is case-insensitive.

## Why This Matters

Returning from the first enclosing call selects `Outer` while editing `Inner`.
Ignoring quoted defaults splits source-backed signatures on commas or closing parentheses inside strings.
Sharing a mutable active index across overloads can produce an index outside a candidate's parameter array.

## When to Apply

- A language server offers both parameter-label completion and Signature Help.
- Indexed signatures preserve default-value source text.
- The parser recovers incomplete expressions used during live editing.

## Examples

For `Outer(Inner(value))`, the cursor in `Inner` must select `Inner`.
For `Use(left < right, next)`, the comma begins the second argument rather than remaining nested under a generic bracket.
For `Use(Outer<Inner<A, B>>(), next)`, the `>>` closes both generic levels before the second argument begins.
For `Log(string separator = ",")`, the default comma must not split the signature.

## Related

- [Callable helpers](../../reference/server/src/lsp/callable.md)
- [Completion projection](../../reference/server/src/lsp/completion.md)
- [Signature Help projection](../../reference/server/src/lsp/signature_help.md)
