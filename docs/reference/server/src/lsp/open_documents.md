# `server/src/lsp/open_documents.rs`

## Purpose

Owns accepted open-document state and cached file-local language analysis.

## Ownership

Owns document text, accepted LSP version, cached lexer/parser/syntax/index/scope
analysis, lazy document-symbol projection, and per-revision semantic-token
cache state. It does not dispatch JSON-RPC, choose external symbols, render
features, or keep persistent workspace state.

## Current Behavior

A document is created from `didOpen` text and replaced only by a strictly newer
`didChange` version. Each accepted revision receives one `FileIndexAnalysis`
containing lexer tokens, syntax, parse diagnostics, file index, lexical scope,
and timings. Outline projection is built lazily then retained for that revision.

Semantic tokens keep separate fast and rich cache entries. Rich results are
valid only for the revision and external-overlay generation that produced them;
pending/cancelled state prevents obsolete background work from becoming current.
Close removes all document-local state.

## Dependencies and Boundaries

Depends on lexer, parser, AST/model/index/scope construction, document-symbol
projection in `lsp.rs`, and semantic-token data types. External workspace and
game-data facts remain in [external_overlay.md](external_overlay.md).

## Verification

Run focused document-cache/LSP tests and `cargo test` from `server/`. Cover
version acceptance, stale-change rejection, cache reuse/invalidation,
diagnostics and symbols after edits, close removal, and token revision/generation
matching.

## Future Direction

Introduce incremental parsing/indexing only after profiling shows full-revision
analysis is the bottleneck and the same cache invariants can be preserved.
