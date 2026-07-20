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
strictly newer revision, stores its text identity, and admits a latest-wins
foreground job. That worker builds the UTF-16 position index, lexical state,
and syntax off the protocol loop; only its matching installation publishes
parser diagnostics and admits semantic/index convergence. A newer revision or
close cancels both dependency stages. The completed semantic result installs
only for the still-current revision and never republishes parser diagnostics.
Close removes the document and clears diagnostics. Repeated Outline requests
reuse cached symbol projection for the accepted revision.

`RuntimeWorkExecutor` has one bounded shared pending-work map and CPU-aware
fixed worker capacity. It always reserves one `Foreground` worker; it starts
one semantic/rich-token/debug worker only when `available_parallelism` reports
another logical CPU. On a one-CPU host, the foreground worker advances
background convergence only while foreground work is idle, so the runtime does
not oversubscribe the CPU with a competing background worker. This is a
capacity reservation inside one executor, not a second scheduler. Shared
admission remains latest-wins by `(TaskClass, URI)`; when its queue is full,
higher-priority foreground work evicts lower-priority queued work first, while
rich/debug work remains best effort.

The bounded ingress queue remains the backpressure and ordering boundary. When
several contiguous `didChange` notifications each contain exactly one
full-document replacement for the same URI, the runtime retains only the
newest version before rebuilding analysis. Ranged edits, mixed URIs, requests,
and internal events are ordering barriers and are never merged. The dispatch
log records queue time plus coalesced and superseded counts so a capture can
separate queue delay from analysis work.

While a current revision is pending analysis, semantic feature requests are
held by URI/revision instead of reading the prior analysis against new text.
They replay after that exact analysis installs; a new edit or close receives
the standard `ContentModified` response for retained requests. Document Outline
is the explicit exception: it returns a current-revision lexical projection of
top-level classes, enums, and typedefs immediately. That result intentionally
omits members and uncertain declarations until exact syntax/semantic facts are
installed, and it never reads the prior cached outline.

Hover and definition are current-snapshot exceptions to that retained-request
behavior. A pending document returns immediately with only lexical hover facts
(keywords, literals, and comments) from the accepted snapshot; identifiers and
resolver-dependent syntax spans return `null` until matching analysis exists.
Pending definition returns only a cursor already on a lexically proven current
top-level declaration, linked to that declaration; all references and other
semantic targets return an empty result. These contracts preserve responsive
navigation without presenting stale local semantic facts.

Signature help is also a current-snapshot exception. Before semantic analysis
publishes, it can return one unique, complete declaration named by an
unqualified current-source call. Member calls, constructors, attributes,
external symbols, recovered declarations, and ambiguous names return `null`.
It never delays a request or combines current text with a cached semantic
analysis from another revision.

The current parser/catalog analysis is an indivisible worker operation. A newer
revision cancels a queued job immediately and causes a running obsolete result
to be skipped after that operation returns; the runtime log records the
background elapsed time so this residual worker cost stays visible rather than
being mistaken for ingress queue time.

Feature requests combine the open document's cached file-local analysis with a
short-lived snapshot of the workspace/game-data overlay. Rich semantic-token
work is delayed and admitted through `analysis_runtime`'s `TaskClass::Rich`
lane by URI/revision/generation; late, cancelled, or non-current task results
cannot replace current token state. Refresh notifications are coalesced until
the client acknowledges the in-flight request.

The module exposes bounded developer-only debug requests and wires workspace
file-change notifications to the external overlay. It is deliberately a small
dispatcher, not a general application framework.

`textDocument/onTypeFormatting` is handled as a Rust-owned, standard-shape
request for the semicolon typing assist. The extension forwards one plain
Enter document-change event after its document synchronization listener has
run; it rejects selections, multiple carets, replacement edits, and any other
text change before transport. The server intentionally does not advertise an
automatic on-type-formatting capability because VS Code did not reliably invoke
the provider in the active editor. The request carries the captured document
version as an extension field and Rust returns no edits unless it exactly
matches the installed immutable snapshot. Its concise request log records the
version, UTF-16 request position, outcome, and elapsed time, never source text.

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
