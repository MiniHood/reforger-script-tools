# `server/src/lsp/semantic_tokens.rs`

## Purpose

Projects Enforce source facts into LSP semantic tokens for editor coloring.

## Ownership

Owns lexical/declaration and resolver-backed token projection, priority
resolution, multiline splitting, UTF-16 delta encoding, palette facts, and
bounded debug/report data. It does not own request scheduling, cache lifecycle,
external indexing, or any TextMate/TypeScript coloring path.

## Current Behavior

The foreground lexical projection reads only the current source and lexer
output, so it remains available while syntax and semantic analysis for a newer
revision is pending. It deliberately emits no parser diagnostics, declaration,
scope, or resolver facts. Each LSP response selects the current revision's
cached lexical `TokenSnapshot` baseline unless a rich overlay has exactly the
same revision and external generation. A rich pass replaces that baseline with
resolver-backed references and external workspace/game-data facts only after
those identity checks pass. Responses are full only, with an opaque result ID
for `revision:lexical` or `revision:rich:generation`; semantic-token deltas are
not advertised.

Token priorities preserve comments over code-like text, declarations/type
positions over weaker references, and scope facts over generic variable
fallbacks. Attributes, calls, static members, preprocessor directives/macros,
base types, constructors, and enum owners receive source-backed treatment.
Multiline spans are split before encoding. Raw tokens, post-split tokens, and
encoded output are all capped at 200,000 tokens to bound malformed or huge
input.

Rich-token cancellation is cooperative across lexical/resolver projection,
declaration overlays, filtering, multiline splitting, and UTF-16 encoding.
The latter phases poll at bounded intervals, so a superseded revision cannot
spend an unbounded tail encoding tokens that will be discarded.

Resolver-backed rich refinement is additionally capped at 96 identifier
resolutions per document projection. The full lexical/fast token baseline still
classifies every token; only optional cross-file resolver overlays stop at the
budget, so a large file cannot turn rich coloring into a foreground competitor.

Semantic tokens are the only Enforce editor-coloring source. The palette helper
is shared only for best-effort hover presentation; it does not create a second
editor coloring pipeline.

## Dependencies and Boundaries

Depends on lexer/parser/model/index/resolver facts, cached `FileIndexAnalysis`,
and external snapshots. The lexical foreground entrypoint depends only on the
current source and lexer output. [open_documents.md](open_documents.md) owns
cache identity; `analysis_runtime` owns rich-task admission, latest-wins
cancellation, and publication eligibility, including the only retained
job/byte capacity limit. `lsp.rs`'s rich worker merely coalesces
already-admitted work through its idle delay and executes it with the runtime
cancellation token.

## Verification

Run focused semantic-token tests and `cargo test` from `server/`. Cover fast to
rich replacement, stale revision/generation rejection, cancellation, Unicode
and CRLF delta encoding, multiline comments, malformed source, token priority,
and the final output cap.

## Future Direction

Add range or delta requests only if full-document refresh cost warrants their
additional invalidation complexity. Keep new classifications source-backed.
