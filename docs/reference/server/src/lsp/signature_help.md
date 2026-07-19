# `server/src/lsp/signature_help.rs`

## Purpose

Projects source-backed callable signatures and active parameters into
`textDocument/signatureHelp` responses.

## Ownership

Owns callable candidate lookup, overload presentation, candidate-local active
parameter calculation, parameter documentation, and signature-help Markdown.
It does not parse callable argument context independently, rank normal
completion, dispatch requests, or infer argument types beyond available source
facts.

## Current Behavior

Signature help uses cached document analysis and the shared syntax-backed
[callable.md](callable.md) context when matching semantic analysis is ready. It resolves functions, methods,
constructors, `new Type(...)`, attributes, enum-typed parameters, optional
parameters, and named arguments across local/workspace/game-data indexes.
Each overload computes its active parameter against its own parameter list, so
shorter candidates remain bounded. Non-call positions return no signature help.

While the accepted revision is still pending semantic analysis, signature help
responds immediately from that revision only. The fallback permits exactly one
syntactically complete current-source callable declaration for an unqualified
call. It does not resolve receivers, constructors, attributes, external
symbols, recovered declarations, or ambiguous names; those cases return no
help rather than reading a previous revision's analysis.

The existing developer completion-debug path can render a bounded signature
summary without creating a separate user-facing command.

## Dependencies and Boundaries

Depends on cached analysis, immutable document snapshots, callable helpers,
`ReferenceResolver`, `IndexQuery`, `SymbolIndex`, and external snapshots. `lsp.rs` owns protocol dispatch and
[completion.md](completion.md) owns normal suggestion presentation.

## Verification

Run focused signature-help/callable tests and `cargo test` from `server/`. Cover
nested calls, constructors, attributes, optional/default parameters, named
labels, comparison syntax, malformed calls, overload bounds, and non-call
misses. Pending-snapshot coverage must prove current-source signatures can
return without semantic publication and that stale, ambiguous, and member
targets never leak a prior analysis result.

## Future Direction

Add type-aware overload ranking, generic type-argument help, or optional
argument ghosting only as independent source-backed features.
