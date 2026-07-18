# `server/src/lsp/hover_render.rs`

## Purpose

Renders compact Markdown for resolver-selected normal hover symbols.

## Ownership

Owns display headers, declarations, documentation, type links, member/enum
summaries, attributes, escaping, and command-link encoding. It does not select
symbols, query cursor spans, or own external-index lifecycle.

## Current Behavior

The renderer converts `SymbolDisplayInfo` and optional index context into
readable Markdown with semantic-token-inspired colors. It formats callable,
attribute, typedef, class, enum, field, parameter, and metadata details while
escaping code and command-link data. Class and enum summaries are bounded to
public-facing/source-backed facts rather than raw index dumps.

## Dependencies and Boundaries

Depends on display/index query types and the semantic-token palette helper.
[hover.md](hover.md) owns selection; `lsp.rs` owns request dispatch. Rendering
must not invent semantic facts or use TypeScript coloring.

## Verification

Run focused hover-render tests and `cargo test` from `server/`. Cover Markdown
escaping, links, declaration kinds, documentation, attributes, member bounds,
and unknown/missing display details.

## Future Direction

Expose richer provenance or raw docs only through explicit display fields or
the developer debug path; keep normal hover concise.
