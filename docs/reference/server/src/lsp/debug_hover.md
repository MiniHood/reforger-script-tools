# `server/src/lsp/debug_hover.rs`

## Purpose

Renders the bounded, developer-triggered `reforger/debugHover` report used to
diagnose hover selection and semantic-token context.

## Ownership

Owns human-readable debug report assembly and bounded excerpts of lexer,
parser, resolver, semantic-token, display, and external-index facts. It does
not change normal hover selection, own logging policy, or provide a production
editor feature.

## Current Behavior

For an open-document position, the report includes source-line context, nearby
tokens, semantic-token coloring, parser diagnostics, resolver result, selected
display facts, bounded candidates, hierarchy context, rendered hover Markdown,
and symbol-kind counts. The token palette is the same Rust semantic-token
palette used by normal semantic tokens, so the report does not introduce a
second coloring model. A small label extractor supports request logging.

## Dependencies and Boundaries

Depends on cached `FileIndexAnalysis`, resolver/index/display data,
semantic-token projection, and external-overlay status. `lsp.rs` owns custom
request dispatch; [hover.md](hover.md) remains the normal hover path.

## Verification

Run targeted debug-hover tests and `cargo test` from `server/`. Check bounded
output for hits, misses, local/external candidates, malformed source, and
Unicode positions.

## Future Direction

Add concise facts from new language layers only when they help a concrete
debugging workflow. Keep the report bounded and command-triggered.
