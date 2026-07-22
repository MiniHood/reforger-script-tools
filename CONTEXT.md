# Reforger Script Tools

This glossary records the durable language used for the language engine's LSP
runtime boundaries.

## LSP Runtime

**Document Runtime**:
The owner of open-document snapshots, their analysis lifecycle, and the
admission of document-backed LSP queries.
_Avoid_: document coordinator, document state

**Document Query**:
A request-local view that captures an open-document snapshot together with the
external-index snapshot that the request may use.
_Avoid_: request context, analysis context

**External Index Snapshot**:
The immutable workspace and game-data index generation captured for one
document query or scheduled analysis job.
_Avoid_: current index, global index

**Runtime Effect**:
An observable action emitted by the Document Runtime for the LSP composition
root to deliver, such as a response, notification, refresh request, or log.
_Avoid_: callback, side effect
