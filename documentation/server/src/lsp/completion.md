# server/src/lsp/completion.rs

## Purpose

Owns LSP completion projection for the Rust language server.

## Architecture Role

This module sits inside the LSP layer and maps cached open-document analysis plus the optional external workspace/game-data overlay into LSP `CompletionList` responses and dev-report data. `server/src/lsp.rs` remains responsible for protocol dispatch and document/cache lifecycle.

## Current Behavior

Completion is resolver-owned. The module asks `ReferenceResolver` for member-completion context or top-level/type prefix context, then queries the file-local index and optional external overlay separately. It combines candidates without constructing a full merged index per request, preserving open-document candidates ahead of workspace/game-data candidates.

Member completion supports instance and static owners. Instance members use `IndexQuery::completion_members_for_class`; static owners use `IndexQuery::completion_static_members_for_type`. Static-owner completion is the only editor-facing path that returns enum members, so values such as `DEBUG` or `STATIC` require their enum owner/container, for example `LogLevel.`. Unqualified value completion uses the cached lexical scope for visible locals/parameters, current-class member completion for methods/fields available without a receiver, top-level value/callable completion for functions/globals/types, and language keyword items such as `return` and `static`. Prefix matching is case-insensitive for completion display, while inserted text remains the real source-backed symbol or keyword spelling. Keyword items are ranked before source-symbol matches for the same prefix. Type-position completion stays type-only and can include primitive/type keywords such as `int` and `typename`. Completion items use `SymbolDisplay`-derived candidate data for labels, signatures/details, documentation previews, label details, source-aware sort text, prefix-replacing text edits, and VS Code standard completion item kinds. Methods/destructors use Method, functions use Function, constructors use Constructor, fields use Field, parameters/locals/non-const globals use Variable, const fields/globals and preprocessor macros use Constant, classes use Class, enums use Enum, enum values use EnumMember, keywords use Keyword, and typedefs/type parameters use TypeParameter because VS Code has no dedicated typedef completion icon.

The module also owns the bounded Markdown formatter for the custom `reforger/debugCompletion` request. That request runs the same cached-analysis completion projection as `textDocument/completion`, then reports request context, receiver/owner inference, prefix, candidate counts, source/origin counts, phase timings, and the first completion items. It is command-triggered only and must not add background or per-completion logging cost.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`-backed completion candidates, `SymbolIndex`, and basic LSP range/position helpers from `server/src/lsp.rs`.

This module must not own request dispatch, open-document storage, workspace file watching, game-data cache lifecycle, TypeScript client behavior, diagnostics, hover, definition, or semantic-token projection. It must not add a second completion path through raw index APIs when `IndexQuery` has an editor-facing API for the same result.

## Change Notes

Extracted from the monolithic `server/src/lsp.rs` without behavior changes. This keeps completion candidate lookup, ranking, item rendering, and timing in one owner while leaving protocol dispatch in `lsp.rs`.

Added the `reforger/debugCompletion` report formatter for Ctrl+F2 completion troubleshooting without adding normal request hot-path overhead.

Aligned completion item kind projection with the VS Code IntelliSense icon table, including Constant for const source facts and macros.

Added unqualified value-prefix completion for visible locals/parameters and current-class members so symbols such as `owner` and `GetOwner` come from the LSP completion list with Variable/Method icons instead of falling through to VS Code word suggestions.

Made completion prefix matching case-insensitive so typing `get` can offer the source-backed `GetOwner()` item. Comment/string autocomplete startup is controlled by Enforce language defaults in `package.json`; the LSP still returns no candidates when the resolver reports no completion context.

Added LSP-owned keyword completions so disabling VS Code word suggestions does not remove language words such as `return`.

Expanded keyword completion to include declaration/modifier words such as `static`, `protected`, and `private`, and verified keyword results rank before matching source symbols.

Removed standalone enum-member results from unqualified top-level completion. Enum members remain available from static-owner completion when their enum owner is present.

## Future Improvements

Add `completionItem/resolve`, richer overload presentation, and snippet placeholders in separate verified slices. Keep future completion behavior routed through resolver context and `IndexQuery` rather than direct raw aggregate index access.
