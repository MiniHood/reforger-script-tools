# `server/src/lsp/diagnostics.rs`

## Purpose

Projects parser diagnostics into LSP diagnostics and publish/clear
notifications.

## Ownership

Owns stable parser diagnostic source/code values, visible UTF-16 range
projection, and JSON-RPC notification payloads. `lsp.rs` owns when a document
revision is accepted and when notifications are sent.

## Current Behavior

Parser diagnostics retain their message and span while receiving the stable
`reforger.parser.syntax` code and parser source label. Zero-width or
end-of-file spans are projected to a visible nearby character where possible,
so editor diagnostics remain selectable. Publish notifications include the
accepted document version. The LSP publishes from the parser output captured by
the newly accepted syntax snapshot, before any deferred semantic/index analysis
begins; later semantic completion never republishes parser diagnostics. Close
emits an empty diagnostic list to clear stale editor state.

## Dependencies and Boundaries

Depends on parser `ParseDiagnostic`, `TextSpan`, and shared LSP range helpers.
It does not add semantic diagnostics, decide document revision ordering, parse
source, or own editor rendering.

## Verification

Run focused diagnostics tests and `cargo test` from `server/`. Cover malformed
source, zero-width/end-of-file spans, CRLF and Unicode ranges, versioned publish
payloads, and close clearing.

## Future Direction

Add semantic diagnostics in a separate owner and preserve parser diagnostics as
a distinct source/code family.
