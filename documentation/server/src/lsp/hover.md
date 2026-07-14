# server/src/lsp/hover.rs

## Purpose

Owns normal LSP hover selection and delegates Markdown rendering.

## Architecture Role

This module sits inside the Rust LSP layer and turns cached file-local analysis plus an optional external workspace/game-data overlay into `textDocument/hover` responses and hover fixture/corpus report data. `server/src/lsp.rs` keeps protocol dispatch and open-document cache lifecycle. `server/src/lsp/hover_render.rs` owns Markdown presentation.

## Current Behavior

Hover is resolver-owned. The module asks `ReferenceResolver` for identifier or syntax-span hover resolution, renders the selected symbol through `SymbolDisplay`, and returns compact Markdown. File-local candidates are preferred over external overlay candidates according to resolver policy. External symbols use the hovered token range in the current document while displaying facts from the external index.

Syntax-span hover is intentionally limited to useful declaration syntax inside source-backed type, return-type, or base-type detail spans, such as a callable return type keyword. Comments, whitespace, strings, punctuation, modifiers such as `protected` / `private` / `static` / `const`, and other non-symbol token classes return no hover instead of selecting a containing class or method.

The hover Markdown shows a colored kind label, fenced Enforce signature or label, kind-specific detail text, structured documentation sections, bounded class member summaries, modifiers, attributes, and concise source path when available. Color is best-effort Markdown presentation only; semantic tokens remain the editor coloring source.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`, `SymbolIndex`, hover rendering, and LSP range/position helpers. It does not own debug-hover reports, protocol dispatch, document storage, workspace indexing, completion, definition, diagnostics, or semantic tokens.

Debug-hover may reuse `render_hover_markdown`, but debug report assembly belongs in its own path.

## Change Notes

Extracted from `server/src/lsp.rs` without behavior changes. Hover presentation later moved to `hover_render.rs` so selection and rendering have separate owners while retaining one authoritative hover path.

## Future Improvements

Keep future hover enhancements routed through resolver and `SymbolDisplay`. Do not add a parallel span or string-scanning hover path in the LSP dispatch layer.
