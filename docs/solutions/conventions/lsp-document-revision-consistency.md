---
title: LSP document revision consistency
date: 2026-07-18
category: conventions
module: server/lsp
problem_type: convention
component: tooling
severity: medium
applies_when:
  - "Handling textDocument/didOpen or textDocument/didChange notifications"
  - "Publishing diagnostics or cached projections derived from open-document text"
related_components:
  - development_workflow
tags: [lsp, document-version, diagnostics, document-symbols, cache-invalidation]
---

# LSP document revision consistency

## Context

Parser diagnostics, cached analysis, and document symbols must describe the same
accepted open-document snapshot. Treat the client-provided document version as
the client-facing boundary of that snapshot; the server also maintains an
internal revision for its accepted analysis identity.

## Guidance

Require an integer version on both `didOpen` and `didChange`. Before rebuilding
analysis or mutating a cache, reject a change for a document that is not open,
or whose version is equal to or older than the accepted version. The protocol
types use required `i32` versions and the handler performs this gate before
calling `OpenDocument::replace` ([`server/src/lsp.rs`](../../../server/src/lsp.rs)).

For an accepted replacement, update the text and protocol version together,
then rebuild analysis and invalidate all text-derived caches. `OpenDocument`
increments its internal revision, clears its document-symbol projection, and
cancels pending semantic-token work during replacement
([`server/src/lsp/open_documents.rs`](../../../server/src/lsp/open_documents.rs)).

The transport may coalesce a contiguous burst of full-document replacements
for one URI before this acceptance gate. It must retain the newest version and
treat a ranged change, another URI, a request, or an internal event as a hard
ordering barrier. Coalescing reduces redundant work; it does not weaken version
ordering or allow a partial edit to be interpreted as a full replacement.

Publish parser diagnostics with the accepted document version. The diagnostic
message includes `version` for live documents, while the empty diagnostic
notification sent after `didClose` intentionally has no version because that
document is no longer open ([`server/src/lsp/diagnostics.rs`](../../../server/src/lsp/diagnostics.rs)).

Keep protocol versions and internal revisions separate: the former orders edits
received from the client; the latter identifies accepted server-side analysis
snapshots and can advance independently of the client's numbering scheme.

## Why This Matters

Delayed or duplicate LSP notifications must not make editor-visible state move
backward. Without an acceptance gate, an older change can replace current text,
recompute diagnostics for it, invalidate the outline cache, and cause the next
document-symbol request to project stale declarations.

The regression coverage sends version 1, then version 3, then a late version 2.
It verifies that diagnostics are published only for versions 1 and 3 and that
the symbol response still contains the version-3 declarations
([`server/src/lsp.rs`](../../../server/src/lsp.rs)).

## When to Apply

- Adding any LSP result derived from mutable open-document text.
- Introducing a cache or deferred projection such as symbols, semantic tokens,
  folding ranges, inlay hints, or code actions.
- Reviewing lifecycle handling for out-of-order, duplicate, or stale document
  notifications.

For asynchronous work, capture the accepted internal revision at scheduling
time and install its result only when that revision is still current.

For full-file analysis during typing, accept the new text/revision first and
mark analysis pending. A request for that revision must wait for its matching
analysis or receive `ContentModified` when superseded; serving the previous
analysis against current text is a revision-consistency violation.

## Examples

```text
accepted document: version 1, text = "class Initial {}"
incoming change:   version 3, text = "class Current { void Run() {} }"
result:            accept; rebuild analysis; clear cached symbols; publish diagnostics version 3

incoming change:   version 2, text = "class Stale { void OldRun() {} }"
result:            ignore before analysis or cache mutation
```

After the stale notification, `documentSymbol` must describe `Current` and
`Run`, never `Stale` or `OldRun`.

## Related

- [Ordered LSP overlay notifications](../best-practices/ordered-lsp-overlay-notifications.md)
  covers a complementary ordering domain: filesystem-overlay event sequences.
- [Physical line boundaries across the language engine](../best-practices/physical-line-boundaries-across-language-engine.md)
  covers coordinate correctness for parser-diagnostic ranges, including UTF-16
  and CRLF behavior.
