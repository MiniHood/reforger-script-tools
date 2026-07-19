# `server/src/lsp.rs`

## Purpose

Owns Rust LSP transport, request dispatch, protocol lifecycle, and the runtime
coordination that connects open documents, external indexes, and feature
projections. It is the server boundary between JSON-RPC/stdio and the language
engine.

## Ownership

`lsp.rs` owns bounded `Content-Length` framing, JSON-RPC validation and error
responses, initialize/shutdown/exit lifecycle, document synchronization,
request routing, document-symbol projection, UTF-16 position conversion, and
rich semantic-token scheduling. It also owns runtime logging and the bridge to
background external-index and semantic-token work.

Feature-specific selection and rendering stay in child modules:

- [open_documents.md](lsp/open_documents.md) — open text, revisions, cached
  analysis, and per-revision token caches.
- [external_overlay.md](lsp/external_overlay.md) — workspace and game-data
  external-index layers.
- [diagnostics.md](lsp/diagnostics.md) — parser diagnostic notifications.
- [hover.md](lsp/hover.md), [hover_render.md](lsp/hover_render.md), and
  [debug_hover.md](lsp/debug_hover.md) — hover selection, presentation, and
  targeted troubleshooting output.
- [definition.md](lsp/definition.md), [completion.md](lsp/completion.md),
  [signature_help.md](lsp/signature_help.md), and [callable.md](lsp/callable.md)
  — source-backed editor interaction projection.
- [semantic_tokens.md](lsp/semantic_tokens.md) — lexical and resolver-backed
  token projection.

It must not own Enfusion language rules, parsing, semantic resolution, index
construction, game-data acquisition, or VS Code UI behavior.

## Current Behavior

The server reads and writes stdio JSON-RPC frames with explicit header, body,
and ingress-queue limits before allocation. Invalid request parameters receive
JSON-RPC errors; invalid notifications are ignored. After `shutdown`, requests
are rejected and `exit` terminates the process as required by the lifecycle.

Open/change notifications require document versions. The server accepts only a
strictly newer revision. `didOpen` builds the initial analysis synchronously;
`didChange` immediately accepts text/version, clears text-derived caches, and
schedules latest-wins analysis after a short idle delay. The completed result
installs only for the still-current revision, then publishes parser diagnostics.
Close removes the document and clears diagnostics. Repeated Outline requests
reuse cached symbol projection for the accepted revision.

The bounded ingress queue remains the backpressure and ordering boundary. When
several contiguous `didChange` notifications each contain exactly one
full-document replacement for the same URI, the runtime retains only the
newest version before rebuilding analysis. Ranged edits, mixed URIs, requests,
and internal events are ordering barriers and are never merged. The dispatch
log records queue time plus coalesced and superseded counts so a capture can
separate queue delay from analysis work.

While a current revision is pending analysis, source-backed requests are held
by URI/revision instead of reading the prior analysis against new text. They
replay after that exact analysis installs; a new edit or close receives the
standard `ContentModified` response for retained requests. This keeps the
transport responsive without exposing mismatched language facts.

The current parser/catalog analysis is an indivisible worker operation. A newer
revision cancels a queued job immediately and causes a running obsolete result
to be skipped after that operation returns; the runtime log records the
background elapsed time so this residual worker cost stays visible rather than
being mistaken for ingress queue time.

Feature requests combine the open document's cached file-local analysis with a
short-lived snapshot of the workspace/game-data overlay. Rich semantic-token
work is delayed and scheduled by URI/revision/generation; late or cancelled
worker results cannot replace current token state. Refresh notifications are
coalesced until the client acknowledges the in-flight request.

The module exposes bounded developer-only debug requests and wires workspace
file-change notifications to the external overlay. It is deliberately a small
dispatcher, not a general application framework.

## Dependencies and Boundaries

Depends on `serde`/`serde_json`, the parser/model/index stack, and
`server/src/index_cache.rs` through the external overlay. The TypeScript client
owns process launch, VS Code events, and user-facing rendering; this module
only consumes and emits LSP protocol messages.

Keep position/range conversion here as the shared LSP boundary. Source spans
are UTF-8 byte offsets internally; LSP positions are UTF-16 code units and
must remain correct for Unicode and CRLF text. A per-source position index is
built once for multi-span projections, avoiding repeated source scans.

## Verification

Run `cargo test` from `server/`. Focused tests in this module cover framing
limits, lifecycle and parameter validation, document revision ordering,
diagnostic clearing, document-symbol caching, UTF-16 conversion, and
semantic-token worker/refresh scheduling. Use matching feature reports in
`server/examples/` only for targeted developer inspection, not at runtime.

## Future Direction

Add protocol endpoints only as separate source-backed slices. Keep semantic
diagnostics, references, rename, and incremental analysis behind their own
owners rather than growing dispatch-specific language shortcuts here.
