# server/src/resolver.rs

## Purpose

Owns the first reference-resolution scaffold for identifier tokens.

## Architecture Role

This file sits above the raw symbol index and below LSP hover/definition behavior. It turns a cursor byte offset into a source-backed resolution record with the identifier token, candidate symbols, selected symbol, selection reason, and provenance.

## Current Behavior

The resolver accepts source text, a file-local `SymbolIndex`, and ordered external indexes. Live LSP callers pass workspace first and game data second, preserving open-document > workspace > game-data priority without requiring a merged external index. It resolves identifier tokens for reference-style hovers and exposes syntax-span hover selection for keyword positions inside source-backed type, return-type, or base-type detail spans, such as hovering `void` in a method return type. Live LSP callers pass the cached parse tree into the resolver; standalone callers can let the resolver parse once for expression context. Whitespace, punctuation, comments, strings, preprocessor punctuation, numbers, modifiers such as `protected` / `private` / `static` / `const`, and error tokens do not select enclosing declarations just because their byte offset lies inside a broader class or method span.

Interactive cursor features usually call `resolve_at_offset`, which finds the token under one cursor position. Batch projections that already have lexer tokens, such as semantic tokens, call the span-based identifier entrypoint instead. That keeps one authoritative resolver path while avoiding repeated whole-source lexing for every identifier in a document. Completion context detection treats comments and strings as hard stops: it must not fall back to the previous declaration boundary when the cursor is inside `//`, `/* ... */`, doc comments, unterminated block comments, or string tokens.

Candidate ordering is intentionally narrow and best-effort. Declaration hits in the open file stay first. For ordinary value/callable identifiers, the resolver asks the lexical scope model for visible locals and parameters, then checks containing class members, file-local top-level declarations, and external preferred declarations in layer order. For identifiers detected inside source-backed type spans, such as return types, parameter types, field/local types, base types, typedef targets, and generic type arguments, file-local type-like top-level declarations are preferred before class members so constructor names do not hide class/type names. Type-position detection rejects obvious parser-recovery spans that cross into a later declaration keyword before the declared name, so an incomplete expression statement on one line does not suppress value/function completion on that line. Keyword tokens that name source-backed generated types, such as `string`, `vector`, `bool`, `int`, `float`, and `typename`, also resolve through the type-position path when they appear inside declaration type spans; value keywords such as `true` and `false` do not.

Each resolution records an identifier context: declaration name, member access, type position, or value/callable. For member access, the resolver asks AST expression wrappers for the `MemberAccessExpression` whose member name contains the hovered token. Receiver inference then walks `Expression` views for calls, member access, indexing, parenthesized expressions, casts, direct `new Type(...)` expressions, and names instead of string-scanning text before a dot. Supported receiver shapes include identifier receivers, `this`, `super`, static-looking type receivers, direct `new Type(...)` receivers, simple call receivers with known return types, `Type.Cast(...)` returning `Type`, and shallow left-to-right chains when every step has a known type. Static enum-member access such as `RplChannel.Reliable` resolves through enum children. Member completion reuses the same receiver inference, selects the full receiver expression ending at the completion dot, and passes the static-owner flag to `IndexQuery`, which handles enum-member and static-class completion policy. Engine pseudo members such as `ClassName`, `Type`, `ToString`, and `IsInherited` resolve through explicit pseudo `Class` member rules. `Type.Cast(...)` hover uses an explicit engine class-cast rule against `Class.Cast` when the concrete receiver class does not expose its own copied static `Cast` member.

Named call-argument labels such as `level:` are recognized through AST expression views and intentionally produce no reference target with the explicit `named-argument-label` reason. Attribute argument labels such as `desc:`, `defvalue:`, `params:`, and `uiwidget:` are classified as `attribute-named-argument`. The value side of normal call named arguments and expression-shaped attribute values remains hoverable through the shared expression AST path. Preprocessor directive tokens such as `ifdef` / `endif` are classified as explicit non-symbol targets with `preprocessor-directive`. Preprocessor macro identifiers such as `ENABLE_DIAG` resolve to indexed `#define ENABLE_DIAG` symbols when present and otherwise keep the explicit `preprocessor-macro` unresolved reason instead of becoming generic unresolved identifiers.

Local and parameter visibility is backed by `server/src/scope.rs`. Live LSP callers pass the cached scope model from open-document analysis; standalone resolver helpers build the scope once with the parse/index. The scope model is still lexical and source-backed only: it does not evaluate control flow, macros, semantic type compatibility, or Workbench/compiler behavior. LSP hover and definition use the resolver for identifier tokens with a file-local index and, when ready, workspace and game-data indexes as external context.

Raw declared type text, return type text, base type text, typedef targets, enum values, and defaults are accessed through `server/src/type_facts.rs` where the resolver only needs copied indexed facts. Expression owner/type inference lives in `server/src/expression_type.rs` through `ExpressionTypeEnvironment`, including AST expression traversal, typedef/base owner expansion, collection index result typing, primitive literal typing, auto-local default inference, static type detection, `this`/`super`, call/member/index chains, generic return substitution, and ordered external-index lookup. Resolver still owns candidate selection, resolution reasons, and provenance, but it asks the expression type environment for receiver and expression owner facts.

## Dependencies and Boundaries

The resolver depends on lexer tokens, parser syntax, AST expression views, lexical scope, index symbols, model symbol kinds, and copied source metadata. It may use an existing parse/scope pair, or parse source and build scope in standalone helper mode, but it must not build indexes, evaluate expressions, resolve overloads, evaluate macros, merge `modded` classes, call Workbench, handle LSP requests, or mutate index state.

## Change Notes

- Added `ReferenceResolver` with `resolve_at_offset`.
- Added `ReferenceResolution`, `ReferenceCandidate`, `CandidateSource`, and `ResolutionReason`.
- Added file-local and external candidate provenance for hover and definition debugging.
- Wired file-local resolver output into LSP hover for identifier-token usages.
- Added source-span-based identifier context so type-position hovers prefer class, enum, and typedef declarations before constructor/member candidates.
- Wired optional external game-data index candidates into LSP hover so file-local misses can resolve to downloaded game-data symbols.
- Added resolver-owned syntax-span hover selection for keyword positions inside source-backed type, return-type, and base-type detail spans.
- Restricted syntax-span hover selection away from comments, whitespace, strings, numbers, punctuation, and other non-symbol token classes so comment hovers do not misleadingly show the containing class or method.
- Restricted syntax-span hover away from broad enclosing class/member spans for modifiers, so hovering `protected` or `static` on a field does not show the containing class.
- Added shallow receiver/member-call resolution for simple member access hovers, including enum static members, `super`, explicit pseudo `Class` members, and the engine class-cast rule.
- Moved receiver/member-access detection and receiver traversal to AST expression views.
- Added direct `new Type(...)` receiver inference and full receiver-expression selection for member completion after chained receivers.
- Added named-argument label suppression so labels do not appear as unresolved symbol misses.
- Added explicit resolver reasons for named argument labels, attribute named arguments, preprocessor directives, and preprocessor macro names so hover/definition reports do not count syntax tokens as actionable misses.
- Attribute value/member expressions now resolve through the same expression AST path as body expressions.
- Wired resolver-selected symbols into LSP definition locations.
- Replaced resolver-local callable child scans with lexical-scope lookup for local and parameter visibility.
- Routed resolver raw type/detail fact reads through the `type_facts` facade where possible.
- Extracted reusable receiver/type-text helpers into `expression_type`.
- Moved receiver-chain expression typing into `ExpressionTypeEnvironment`; resolver now delegates expression owner inference to that layer.
- Added a span-based identifier resolution entrypoint for batch projections that already own lexer token spans.
- Added keyword type-position resolution for generated script type names so definition can navigate from declaration type words without adding LSP-specific lookup rules.
- Added source-backed preprocessor macro resolution from macro uses such as `#ifdef ENABLE_DIAG` to matching `#define ENABLE_DIAG` symbols when present.
- Added ordered external-index support so live LSP requests can query workspace and game-data layers directly instead of consuming a pre-merged overlay.
- Made completion context detection stop inside comments and strings before falling back to earlier significant tokens, so block comments after code do not inherit the previous `;` declaration context.
- Made type-position detection reject recovered multi-line type spans that run into a following declaration keyword before the declared name, preserving value/function completion for incomplete statement prefixes such as `getgam` before a later `int testnum` declaration.

## Future Improvements

- Add deeper expression type inference and overload handling as separate semantic slices.
- Keep moving future type inference improvements into `expression_type` or a later semantic facts layer instead of re-growing resolver-owned typing logic.
