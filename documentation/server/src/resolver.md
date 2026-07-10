# server/src/resolver.rs

## Purpose

Owns the first reference-resolution scaffold for identifier tokens.

## Architecture Role

This file sits above the raw symbol index and below LSP hover/definition behavior. It turns a cursor byte offset into a source-backed resolution record with the identifier token, candidate symbols, selected symbol, selection reason, and provenance.

## Current Behavior

The resolver accepts source text, a file-local `SymbolIndex`, and an optional external workspace/game-data `SymbolIndex`. It resolves identifier tokens for reference-style hovers and exposes syntax-span hover selection for non-identifier positions inside declaration spans. Live LSP callers pass the cached parse tree into the resolver; standalone callers can let the resolver parse once for expression context. Whitespace, punctuation, comments, strings, keywords, and error tokens outside symbol spans produce no resolution.

Candidate ordering is intentionally narrow and best-effort. Declaration hits in the open file stay first. For ordinary value/callable identifiers, the resolver checks containing callable locals, containing callable parameters, containing class members, file-local top-level declarations, then optional external preferred declarations. For identifiers detected inside source-backed type spans, such as return types, parameter types, field/local types, base types, typedef targets, and generic type arguments, file-local type-like top-level declarations are preferred before class members so constructor names do not hide class/type names.

Each resolution records an identifier context: declaration name, member access, type position, or value/callable. For member access, the resolver asks AST expression wrappers for the `MemberAccessExpression` whose member name contains the hovered token. Receiver inference then walks `Expression` views for calls, member access, indexing, parenthesized expressions, casts, and names instead of string-scanning text before a dot. Supported receiver shapes include identifier receivers, `this`, `super`, static-looking type receivers, simple call receivers with known return types, `Type.Cast(...)` returning `Type`, and shallow left-to-right chains when every step has a known type. Static enum-member access such as `RplChannel.Reliable` resolves through enum children. Engine pseudo members such as `ClassName`, `Type`, `ToString`, and `IsInherited` resolve through explicit pseudo `Class` member rules. `Type.Cast(...)` hover uses an explicit engine class-cast rule against `Class.Cast` when the concrete receiver class does not expose its own copied static `Cast` member.

Named call-argument labels such as `level:`, `desc:`, `defvalue:`, and `uiwidget:` are recognized through AST expression views and intentionally produce no reference resolution. The value side of a named argument is still hoverable normally.

Local variables are approximate source facts; block-accurate scope and full expression typing are future work. LSP hover and definition use the resolver for identifier tokens with a file-local index and, when ready, a background-loaded game-data index as external context.

## Dependencies and Boundaries

The resolver depends on lexer tokens, parser syntax, AST expression views, index symbols, model symbol kinds, and copied source metadata. It may use an existing parse tree, or parse source in standalone helper mode, but it must not build indexes, evaluate expressions, resolve overloads, evaluate macros, merge `modded` classes, call Workbench, handle LSP requests, or mutate index state.

## Change Notes

- Added `ReferenceResolver` with `resolve_at_offset`.
- Added `ReferenceResolution`, `ReferenceCandidate`, `CandidateSource`, and `ResolutionReason`.
- Added file-local and optional external candidate provenance for hover and definition debugging.
- Wired file-local resolver output into LSP hover for identifier-token usages.
- Added source-span-based identifier context so type-position hovers prefer class, enum, and typedef declarations before constructor/member candidates.
- Wired optional external game-data index candidates into LSP hover so file-local misses can resolve to downloaded game-data symbols.
- Added resolver-owned syntax-span hover selection for non-identifier positions inside symbol spans.
- Added shallow receiver/member-call resolution for simple member access hovers, including enum static members, `super`, explicit pseudo `Class` members, and the engine class-cast rule.
- Moved receiver/member-access detection and receiver traversal to AST expression views.
- Added named-argument label suppression so labels do not appear as unresolved symbol misses.
- Wired resolver-selected symbols into LSP definition locations.

## Future Improvements

- Add block-accurate local scope, deeper expression type inference, enum/static constant handling, and overload handling as separate semantic slices.
