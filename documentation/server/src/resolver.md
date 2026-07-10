# server/src/resolver.rs

## Purpose

Owns the first reference-resolution scaffold for identifier tokens.

## Architecture Role

This file sits above the raw symbol index and below future LSP hover/definition behavior. It turns a cursor byte offset into a source-backed resolution record with the identifier token, candidate symbols, selected symbol, selection reason, and provenance.

## Current Behavior

The resolver accepts source text, a file-local `SymbolIndex`, and an optional external workspace/game-data `SymbolIndex`. It resolves identifier tokens only. Whitespace, punctuation, comments, strings, keywords, and error tokens produce no resolution.

Candidate ordering is intentionally narrow and best-effort. Declaration hits in the open file stay first. For ordinary value/callable identifiers, the resolver checks containing callable locals, containing callable parameters, containing class members, file-local top-level declarations, then optional external preferred declarations. For identifiers detected inside source-backed type spans, such as return types, parameter types, field/local types, base types, typedef targets, and generic type arguments, file-local type-like top-level declarations are preferred before class members so constructor names do not hide class/type names.

Each resolution records an identifier context: declaration name, type position, or value/callable. Local variables are approximate source facts; block-accurate scope and expression typing are future work. LSP hover uses the resolver for identifier tokens with a file-local index and, when ready, a background-loaded game-data index as external context.

## Dependencies and Boundaries

The resolver depends on lexer tokens, index symbols, model symbol kinds, and copied source metadata. It must not parse source, build indexes, evaluate expressions, infer receiver types, resolve overloads, evaluate macros, merge `modded` classes, call Workbench, handle LSP requests, or mutate index state.

## Change Notes

- Added `ReferenceResolver` with `resolve_at_offset`.
- Added `ReferenceResolution`, `ReferenceCandidate`, `CandidateSource`, and `ResolutionReason`.
- Added file-local and optional external candidate provenance for future hover and definition debugging.
- Wired file-local resolver output into LSP hover for identifier-token usages.
- Added source-span-based identifier context so type-position hovers prefer class, enum, and typedef declarations before constructor/member candidates.
- Wired optional external game-data index candidates into LSP hover so file-local misses can resolve to downloaded game-data symbols.

## Future Improvements

- Add `textDocument/definition` using the same resolver result.
- Add block-accurate local scope, receiver type inference, member access resolution, and overload handling as separate semantic slices.
