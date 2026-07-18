---
title: Bound LSP work at final side effects
date: 2026-07-18
category: best-practices
module: server/lsp
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - "A bounded intermediate LSP result can expand before it is sent to the client"
  - "A server-initiated LSP request may be triggered repeatedly before client acknowledgement"
tags: [lsp, semantic-tokens, bounds, refresh, backpressure]
---

# Bound LSP work at final side effects

## Context

An intermediate bound is not sufficient when a later transformation expands it,
and a request trigger is not sufficient when the client has not acknowledged a
previous request. Both cases can turn normal editor input into unbounded output
or request traffic.

## Guidance

Apply resource limits after every expanding transformation and again at the
encoder boundary. Semantic tokens now cap both the multiline-split stream and
the encoded stream; a single unterminated comment cannot turn one raw token
into more than the configured output limit.

For server-initiated requests, represent the lifecycle explicitly: permit one
in-flight request, mark the request dirty when another trigger arrives, and
send one follow-up only after the matching client response arrives. The
semantic-token refresh path uses this model for rich projections and external
overlay changes.

## Why This Matters

Limits must protect the resource actually consumed by the client. Likewise,
coalescing before acknowledgement prevents refresh storms while preserving the
fact that a newer result became available.

## When to Apply

- Splitting, flattening, expanding, or encoding bounded LSP data.
- Emitting server-to-client refresh or invalidation requests from several
  independent state changes.

## Examples

```text
raw tokens -> multiline split -> final cap -> delta encode -> defensive cap

refresh trigger while request in flight -> dirty
matching client response -> send exactly one follow-up refresh
```

## Related

- [LSP document revision consistency](../conventions/lsp-document-revision-consistency.md)
  covers accepting only current document snapshots.
- [Ordered LSP overlay notifications](ordered-lsp-overlay-notifications.md)
  covers ordering filesystem-overlay events.
