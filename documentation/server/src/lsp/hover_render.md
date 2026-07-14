# server/src/lsp/hover_render.rs

## Purpose

Owns Markdown presentation for normal LSP hover content.

## Architecture Role

This module sits below `server/src/lsp/hover.rs`. Hover selection remains resolver-owned in `hover.rs`; this renderer only turns `SymbolDisplayInfo` and optional `IndexQuery` context into compact Markdown.

## Current Behavior

The renderer emits an old-hover-inspired structure using the current Rust display stack: colored kind header, fenced Enforce declaration/signature block, structured documentation sections, attribute parameter and constructor sections, bounded class member sections, enum member sections, and bottom metadata for modifiers, attributes, and source path when available. Class summaries use `IndexQuery::completion_members_for_class`, sample direct/overlay members, and show inherited members as a count so inherited constructors do not look like constructors declared by the hovered class.

Color is best-effort Markdown HTML for VS Code hover display. It reuses the semantic-token palette and is not a second coloring or classification system. The renderer emits sanitized inline spans using only VS Code-supported hover CSS such as `style="color:#40b5ac;"`; the TypeScript language client is responsible only for enabling HTML-capable Markdown rendering. The hover remains readable when a client ignores inline HTML color.

The previous TypeScript extension used custom hover-only TextMate grammars for colored code fences. This renderer intentionally does not revive those grammars. It keeps one source-backed presentation path through `SymbolDisplayInfo` and Rust Markdown rendering.

## Dependencies and Boundaries

Depends on `SymbolDisplayInfo`, `IndexQuery`, semantic-token palette helpers, and LSP symbol-kind labels. It must not select hover targets, resolve symbols, parse source, infer types, call Workbench, implement semantic-token behavior, or introduce TextMate/regex hover classification.

## Change Notes

- Added to keep hover presentation separate from resolver selection and LSP request handling while avoiding a parallel symbol-display system.
- Adopted the useful surface format from the old extension: attribute params/constructor sections, class member sections, enum member sections, and declaration-first layout. The implementation is new Rust renderer code over indexed facts, not old TypeScript or TextMate grammar reuse.

## Future Improvements

Keep member summaries bounded. If hover needs richer source provenance or raw docs, route that through debug-hover or explicit display fields rather than dumping raw index data into normal hover.
