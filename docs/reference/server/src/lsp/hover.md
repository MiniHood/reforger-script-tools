# `server/src/lsp/hover.rs`

## Purpose

Selects the source-backed symbol and range for normal LSP hover, then delegates
Markdown presentation.

## Ownership

Owns cursor-to-offset handling, resolver-driven selection across file-local and
external layers, hover response projection, and selection-source reporting. It
does not render Markdown, assemble debug reports, dispatch requests, or create
a parallel span/string-scanning lookup path.

## Current Behavior

Hover uses cached open-document analysis plus workspace/game-data snapshots
when that analysis belongs to the accepted document revision. While analysis
is pending, it instead reads only the runtime-owned current snapshot: keyword,
literal, and comment hovers are lexical facts; identifiers and every
resolution-dependent syntax span return no hover. It never defers a hover or
joins current source to a prior local analysis.
The resolver decides identifier and non-identifier syntax-span selection, while
the projection returns the cursor token range for file-local identifiers and
the appropriate source range for external candidates. Ambiguous and
non-symbol positions return no hover rather than guessing. Batch helpers reuse
the same selection path for reports.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `FileIndexAnalysis`, `SymbolIndex`,
`IndexQuery`, external snapshots, and [hover_render.md](hover_render.md).
[debug_hover.md](debug_hover.md) may reuse rendering but owns diagnostic output.

## Verification

Run focused hover tests and `cargo test` from `server/`. Cover local and
external symbols, workspace precedence, syntax spans, comments/whitespace
misses, Unicode ranges, and local/block scope behavior.

## Future Direction

Route richer hover facts through resolver and display models. Do not add
file-local LSP shortcuts that bypass resolver policy.
