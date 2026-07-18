# `server/src/lsp/semantic_tokens.rs`

## Purpose

Projects Enforce source facts into LSP semantic tokens for editor coloring.

## Ownership

Owns lexical/declaration and resolver-backed token projection, priority
resolution, multiline splitting, UTF-16 delta encoding, palette facts, and
bounded debug/report data. It does not own request scheduling, cache lifecycle,
external indexing, or any TextMate/TypeScript coloring path.

## Current Behavior

The fast pass returns lexical, declaration, and cached scope-reference colors
immediately. A rich pass overlays resolver-backed references and external
workspace/game-data facts. Both use the same cached document analysis; the
runtime accepts rich output only when its document revision and external
generation still match.

Token priorities preserve comments over code-like text, declarations/type
positions over weaker references, and scope facts over generic variable
fallbacks. Attributes, calls, static members, preprocessor directives/macros,
base types, constructors, and enum owners receive source-backed treatment.
Multiline spans are split before encoding. Raw tokens, post-split tokens, and
encoded output are all capped at 200,000 tokens to bound malformed or huge
input.

Semantic tokens are the only Enforce editor-coloring source. The palette helper
is shared only for best-effort hover presentation; it does not create a second
editor coloring pipeline.

## Dependencies and Boundaries

Depends on lexer/parser/model/index/resolver facts, cached `FileIndexAnalysis`,
and external snapshots. [open_documents.md](open_documents.md) owns cache
identity; `lsp.rs` owns workers, cancellation, and refresh coalescing.

## Verification

Run focused semantic-token tests and `cargo test` from `server/`. Cover fast to
rich replacement, stale revision/generation rejection, cancellation, Unicode
and CRLF delta encoding, multiline comments, malformed source, token priority,
and the final output cap.

## Future Direction

Add range or delta requests only if full-document refresh cost warrants their
additional invalidation complexity. Keep new classifications source-backed.
