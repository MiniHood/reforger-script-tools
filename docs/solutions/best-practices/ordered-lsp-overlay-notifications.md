---
title: Order Stateful LSP Overlay Notifications
date: 2026-07-18
category: best-practices
module: lsp-runtime
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "A client reads files asynchronously before sending stateful LSP notifications"
  - "Delete events can race with delayed change events"
tags: [lsp, workspace-overlay, async-ordering, tombstones]
---

# Order Stateful LSP Overlay Notifications

## Context

The language client debounces workspace file events but reads changed files asynchronously.
A delayed read can therefore arrive after a delete or a newer save, and arrival order alone cannot represent the newest workspace state.

## Guidance

Assign a monotonic sequence when the client captures each file event, before any asynchronous read starts.
Use one existence-independent path key at both endpoints: absolute lexical normalization, normalized separators, and Windows case folding without canonicalization.
Send the sequence on both change and delete notifications.

The Rust overlay owns last-applied sequence state, including delete tombstones.
It must discard equal or older notifications before parsing text, rebuilding an index, advancing a generation, or publishing an aggregate.

## Why This Matters

Canonical paths are not suitable for ordering delete events because the deleted path may no longer exist.
Without a tombstone, a stale change can resurrect removed symbols; without the same path-key rule, aliases can create independent sequence streams for one file.

## When to Apply

- A thin editor client sends full source text to a server-owned index.
- A change notification can perform I/O or await work before transport.
- The server needs latest-wins state rather than append-only events.

## Examples

`change(path, 1)` begins an async read, then `delete(path, 2)` arrives first.
The overlay records tombstone `2`; when the delayed changed text for `1` arrives, it is ignored and cannot restore the deleted file.

## Related

- [LSP external overlay reference](../../reference/server/src/lsp/external_overlay.md)
- [Language client reference](../../reference/src/languageClient/languageClient.md)
