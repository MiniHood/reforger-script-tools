# `server/src/lsp/open_documents.rs`

## Purpose

Owns accepted open-document state and cached file-local language analysis.

## Ownership

Owns only analysis/caches derived from a runtime-owned immutable document
snapshot: cached lexer/parser/syntax/index/scope analysis, lazy document-symbol
projection, and per-revision semantic-token cache state. `analysis_runtime`
owns accepted text, client version, revision allocation, UTF-16 positions, and
runtime task admission.
This module does not dispatch JSON-RPC, choose external symbols, render
features, or own document admission.

## Current Behavior

A document cache is created from an already-accepted runtime snapshot and
replaced only after `DocumentStore` accepts a strictly newer `didChange`
version. In the production runtime, both an initial open and a replacement make
the snapshot authoritative first and mark full analysis pending. The shared
runtime-work executor only executes an `AnalysisRuntime`-admitted task and
reports its identity back
before installation, so runtime cancellation is the sole authority for stale
publication. This keeps a large initial open from blocking the LSP message loop.
A pending document has current text identity only. The foreground worker
installs its immutable query snapshot (lexer tokens, top-level declaration
summary, and safe callable-declaration summary), UTF-16 positions, and syntax before parser
diagnostics publish; only then may semantic admission begin. Until that
installation, features use deterministic lexical fallbacks and cannot combine
current text with old semantic facts. The full analysis later supplies
semantic/query state only.
Tests without a worker may construct an immediately-ready cache
to exercise deterministic feature projection. The full analysis contains lexer
tokens, syntax, parse diagnostics, file index, lexical scope, and timings; its
syntax is compatibility/query state rather than the publication authority for
parser diagnostics.
Outline projection is built lazily then retained for the ready revision.

Semantic tokens keep one `TokenSnapshot` per document revision. Its lexical
baseline is cached from current snapshot text and is always the first valid
response. A rich projection is an optional replacement overlay valid only for
that revision and external-overlay generation. Full-token result IDs encode the
published revision and layer (`lexical` or `rich:<generation>`); token deltas
are not advertised. Pending/cancelled state and revision/generation checks
discard obsolete overlays, leaving the lexical baseline authoritative.
Close removes all document-local state.

Before this owner receives a change, the runtime may collapse a contiguous run
of safe full-text replacements for one URI to its newest version. That is an
ingress optimization only: this module still sees a normal accepted version,
performs its usual cache invalidation, and never applies a partial/ranged edit
through that path.

The ready-analysis identity is separate from the accepted document revision.
Feature dispatch must require them to match; it may defer a semantic request,
but must never combine current text with an earlier analysis. Hover, definition,
and signature-help pending projections consume this foreground snapshot directly:
request handlers never re-lex source, traverse the CST, or construct an AST. The lexical
semantic-token projection is explicitly allowed during this pending state
because it reads only current source text. Document Outline is likewise allowed
through a documented lower-quality lexical contract: it returns only top-level
class, enum, and typedef names with ranges from the runtime-owned current
`PositionIndex`; members and ambiguous declarations remain absent until exact
analysis installs. Top-level completion is also allowed in this state, but only
through its current lexer prefix and captured external indexes; it deliberately
excludes local/member/argument facts until their current analysis contract is
available. Replacing or closing a document cancels the tracked analysis work
and invalidates its caches.

If bounded semantic admission rejects a revision, deferred semantic requests
receive the standard content-modified response rather than remaining queued
indefinitely. A later accepted revision is a fresh admission attempt; lexical
tokens and lexical/top-level completion remain available for the rejected
current revision. `OpenDocument` does not own a second document-analysis
cancellation flag.

## Dependencies and Boundaries

Depends on the compiler-owned document runtime, lexer, parser, AST/model/index/
scope construction, document-symbol projection in `lsp.rs`, and semantic-token
data types. External workspace and game-data facts remain in
[external_overlay.md](external_overlay.md).

## Verification

Run focused document-cache/LSP tests and `cargo test` from `server/`. Cover
version acceptance, stale-change rejection, cache reuse/invalidation,
diagnostics and symbols after edits, close removal, and token revision/generation
matching.

## Future Direction

Introduce incremental parsing/indexing only after profiling shows full-revision
analysis is the bottleneck and the same cache invariants can be preserved.
