# server/src/lsp/hover_render.rs

## Purpose

Owns Markdown presentation for normal LSP hover content.

## Architecture Role

This module sits below `server/src/lsp/hover.rs`. Hover selection remains resolver-owned in `hover.rs`; this renderer only turns `SymbolDisplayInfo` and optional `IndexQuery` context into compact Markdown.

## Current Behavior

The renderer emits an old-hover-inspired structure using the current Rust display stack: a larger bold colored kind header directly followed by the renderer-colored declaration/signature line, structured documentation text, attribute parameter and constructor sections, class API-surface sections, and enum member sections. Normal hover avoids duplicate detail/footer noise: types, defaults, and base classes are folded into the declaration line where possible, while source provenance remains debug-hover territory. Low-value callable modifiers such as `override`, `proto`, `external`, and `event` are hidden from hover declarations; useful context modifiers such as `protected`, `private`, and `static` remain visible when hovering the member itself. Method hovers use the user-facing label `Function`; field hovers use the user-facing label `field`; enum-member hovers use `Enum Value`; owner containers render as plain `in` plus a separately class-colored owner name. Class and enum hover kind headers use keyword-blue, but class/enum type names in declarations remain green. Class declarations render with spaced inheritance punctuation, such as `class Child : Base`. Section labels such as `Members`, `Functions`, `Fields`, `Params`, and `Constructor` render as real Markdown `###` headings with a blank line before section content, matching the enum `Members` sizing and the old hover's visual weight instead of small inline bold text. Class summaries use `IndexQuery::completion_members_for_class`, render public-facing direct/overlay constructors, functions, and fields with the same Rust declaration-coloring path as the main hover line, and merge public inherited methods/fields into the normal `Functions` and `Fields` sections without a separate inherited dump or per-row owner suffix. Destructors and `protected`/`private` members are intentionally omitted from normal class hover summaries to keep the summary focused on the class's public API surface; hovering those declarations directly still shows their own symbol facts. When hover selects a file-local class declaration and an external overlay is available, direct members come from the selected file-local index while inherited member summaries can come from the external overlay. That keeps declaration hovers and type-usage hovers consistent without changing resolver selection or hiding open-document facts.

Color is best-effort Markdown HTML for VS Code hover display. It reuses the semantic-token palette and is not a second coloring or classification system. The renderer emits sanitized inline spans using only VS Code-supported hover CSS such as `style="color:#40b5ac;"`; the TypeScript language client is responsible only for enabling HTML-capable Markdown rendering. The hover remains readable when a client ignores inline HTML color.

The previous TypeScript extension used custom hover-only TextMate grammars for colored code fences. This renderer intentionally does not revive those grammars. Primary declarations are colored directly from `SymbolDisplayInfo` facts because VS Code hover code fences do not consume the Rust semantic-token classifier. It keeps one source-backed presentation path through `SymbolDisplayInfo` and Rust Markdown rendering.

## Dependencies and Boundaries

Depends on `SymbolDisplayInfo`, `IndexQuery`, semantic-token palette helpers, and LSP symbol-kind labels. It must not select hover targets, resolve symbols, parse source, infer types, call Workbench, implement semantic-token behavior, or introduce TextMate/regex hover classification.

## Change Notes

- Added to keep hover presentation separate from resolver selection and LSP request handling while avoiding a parallel symbol-display system.
- Adopted the useful surface format from the old extension: kind line, declaration block, plain documentation text, attribute params/constructor sections, class member sections, enum member sections, and declaration-first layout. The implementation is new Rust renderer code over indexed facts, not old TypeScript or TextMate grammar reuse.
- Class member sections render public-facing direct/overlay constructors, functions, and fields instead of sampled code-fence snippets, so class hover shows the useful API surface with Rust-rendered colors.
- Public inherited methods/fields are merged into the normal `Functions` and `Fields` sections without an owner suffix. Destructors and `protected`/`private` members are intentionally hidden from normal class hover summaries to avoid noisy internals.
- Callable declarations suppress low-value modifiers such as `override`, `proto`, `external`, and `event` so class hover reads like an API surface instead of a source echo.
- Documentation commands render as small VS Code-theme badge labels such as `param[in]`, `return`, `warning`, and `note` so Doxygen-style facts are visible without exposing raw comment markup or reusing semantic-token colors.
- Normal hover intentionally suppresses source path/provenance and duplicate modifier/detail rows. Debug hover remains the raw troubleshooting surface.

## Future Improvements

If hover needs richer source provenance or raw docs, route that through debug-hover or explicit display fields rather than dumping raw index data into normal hover.
