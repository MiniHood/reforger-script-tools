# `server/src/lsp/callable.rs`

## Purpose

Provides shared syntax-backed callable signature parsing and active-argument
context for completion and signature help.

## Ownership

Owns callable parameter splitting, optional/default metadata, call/new target
recognition, active argument index, and named-argument label detection. It does
not choose candidates, rank completion items, render signature-help responses,
or dispatch LSP requests.

## Current Behavior

The helper walks CST nodes and source spans rather than scanning arbitrary
text. It distinguishes calls and `new Type(...)`, handles nesting, strings,
escapes, and generic-angle syntax, and ignores comparison operators when
determining argument boundaries. Named labels are recovered from the current
argument and supplied labels are normalized for duplicate suppression.

## Dependencies and Boundaries

Depends on lexer tokens, CST/AST expression views, and `TextSpan`. It is shared
by [completion.md](completion.md) and [signature_help.md](signature_help.md) so
parameter interpretation has one authoritative implementation.

## Verification

Run focused callable/completion/signature-help tests and `cargo test` from
`server/`. Cover nested calls, quoted/escaped commas, generic syntax,
comparisons, named arguments, and malformed-source recovery.

## Future Direction

Add generic type-argument context only with a dedicated feature. Preserve the
syntax-backed path; do not add a competing string-scanning implementation.
