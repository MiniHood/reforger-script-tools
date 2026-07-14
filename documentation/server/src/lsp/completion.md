# server/src/lsp/completion.rs

## Purpose

Owns LSP completion projection for the Rust language server.

## Architecture Role

This module sits inside the LSP layer and maps cached open-document analysis plus the optional external workspace/game-data overlay into LSP `CompletionList` responses and dev-report data. `server/src/lsp.rs` remains responsible for protocol dispatch and document/cache lifecycle.

## Current Behavior

Completion is resolver-owned. The module asks `ReferenceResolver` for member-completion context or top-level/type prefix context, then queries the file-local index and optional external overlay separately. It combines candidates without constructing a full merged index per request, preserving open-document candidates ahead of workspace/game-data candidates.

Member completion supports instance and static owners. Instance members use `IndexQuery::completion_members_for_class`; static owners use `IndexQuery::completion_static_members_for_type`. Top-level completion supports type-position and value/callable prefixes. Completion items use `SymbolDisplay`-derived candidate data for labels, signatures/details, documentation previews, label details, source-aware sort text, and prefix-replacing text edits.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`-backed completion candidates, `SymbolIndex`, and basic LSP range/position helpers from `server/src/lsp.rs`.

This module must not own request dispatch, open-document storage, workspace file watching, game-data cache lifecycle, TypeScript client behavior, diagnostics, hover, definition, or semantic-token projection. It must not add a second completion path through raw index APIs when `IndexQuery` has an editor-facing API for the same result.

## Change Notes

Extracted from the monolithic `server/src/lsp.rs` without behavior changes. This keeps completion candidate lookup, ranking, item rendering, and timing in one owner while leaving protocol dispatch in `lsp.rs`.

## Future Improvements

Add `completionItem/resolve`, richer overload presentation, and snippet placeholders in separate verified slices. Keep future completion behavior routed through resolver context and `IndexQuery` rather than direct raw aggregate index access.
