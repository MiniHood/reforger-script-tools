---
title: Keep Physical Line Boundaries Consistent Across the Language Engine
date: 2026-07-18
category: best-practices
module: rust-language-engine
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "Changing CR, LF, or CRLF handling in lexer, parser, or LSP code"
  - "Adding line-sensitive directive, range, or semantic-token behavior"
tags: [line-endings, parser, lsp, semantic-tokens, resolver]
---

# Keep Physical Line Boundaries Consistent Across the Language Engine

## Context

Physical line endings are consumed by more than the parser. LSP byte-offset conversion, semantic-token encoding and splitting, and resolver checks for directive context all need the same CR, LF, and CRLF interpretation.

## Guidance

When changing physical-line behavior, audit every layer that turns source offsets into lines or scans a line-sensitive construct:

1. Keep parser directive termination and LSP position conversion aligned.
2. Treat CRLF as one terminator and support lone CR and LF in semantic-token segmentation and line indexes.
3. Update resolver checks that find directive or attribute boundaries; a `\n`-only search can make the LSP disagree with the parser.
4. Test CR-only and CRLF source through position round-trips and the affected LSP projection.

## Why This Matters

A parser may correctly separate `#define X\rclass A {}`, while an LF-only LSP layer keeps `class A` on line zero or treats it as directive text. That produces misranged diagnostics and can suppress semantic tokens, hover, or definition even though parsing succeeds.

## When to Apply

- Adding or changing preprocessor recovery.
- Building offset/range conversion or semantic-token line splitting.
- Adding source-text scans that use line starts or ends.

## Related

- [LSP reference](../../reference/server/src/lsp.md)
- [Semantic-token reference](../../reference/server/src/lsp/semantic_tokens.md)
- [Resolver reference](../../reference/server/src/resolver.md)
