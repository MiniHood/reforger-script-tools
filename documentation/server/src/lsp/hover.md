# server/src/lsp/hover.rs

## Purpose

Owns normal LSP hover selection and Markdown rendering.

## Architecture Role

This module sits inside the Rust LSP layer and turns cached file-local analysis plus an optional external workspace/game-data overlay into `textDocument/hover` responses and hover fixture/corpus report data. `server/src/lsp.rs` keeps protocol dispatch and open-document cache lifecycle.

## Current Behavior

Hover is resolver-owned. The module asks `ReferenceResolver` for identifier or syntax-span hover resolution, renders the selected symbol through `SymbolDisplay`, and returns compact Markdown. File-local candidates are preferred over external overlay candidates according to resolver policy. External symbols use the hovered token range in the current document while displaying facts from the external index.

Syntax-span hover is intentionally limited to useful declaration syntax inside source-backed type, return-type, or base-type detail spans, such as a callable return type keyword. Comments, whitespace, strings, punctuation, modifiers such as `protected` / `private` / `static` / `const`, and other non-symbol token classes return no hover instead of selecting a containing class or method.

The Markdown renderer shows a fenced Enforce signature or label, optional detail text, documentation preview, modifiers, and attribute names. It intentionally does not show source provenance in normal hover output.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`, `SymbolIndex`, and LSP range/position helpers. It does not own debug-hover reports, protocol dispatch, document storage, workspace indexing, completion, definition, diagnostics, or semantic tokens.

Debug-hover may reuse `render_hover_markdown`, but debug report assembly belongs in its own path.

## Change Notes

Extracted from `server/src/lsp.rs` without behavior changes. This keeps hover selection/rendering as one authoritative implementation path while reducing LSP dispatch size.

## Future Improvements

Keep future hover enhancements routed through resolver and `SymbolDisplay`. Do not add a parallel span or string-scanning hover path in the LSP dispatch layer.
