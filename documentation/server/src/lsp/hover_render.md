# server/src/lsp/hover_render.rs

## Purpose

Owns Markdown presentation for normal LSP hover content.

## Architecture Role

This module sits below `server/src/lsp/hover.rs`. Hover selection remains resolver-owned in `hover.rs`; this renderer only turns `SymbolDisplayInfo` and optional `IndexQuery` context into compact Markdown.

## Current Behavior

The renderer emits an old-hover-inspired structure using the current Rust display stack: a larger bold colored kind header directly followed by the renderer-colored declaration/signature line, structured documentation text, attribute parameter and constructor sections, class API-surface sections, and enum value sections. Normal hover avoids duplicate detail/footer noise: types, defaults, and base classes are folded into the declaration line where possible, while source provenance remains debug-hover territory. Low-value callable modifiers such as `override`, `proto`, `external`, and `event` are hidden from hover declarations; useful context modifiers such as `protected`, `private`, and `static` remain visible when hovering the member itself. Method hovers use the user-facing label `Function`; constructor hovers use a keyword-blue `Constructor` label with a class-green constructor name; destructor hovers use a class-green `Destructor` label and class-green owner name; field hovers use the user-facing label `field`; enum-member hovers use `Enum Value`; owner containers render the kind and owner name as bold while the separator word `in` stays plain text. Class and enum hover kind headers use keyword-blue, but class/enum type names in declarations remain green. Class declarations render with spaced inheritance punctuation, such as `class Child : Base`. Documentation parameter rows render directly under the summary with compact native Markdown inline-code tags such as `in`, `out`, `inout`, or `param`; there is intentionally no separate `Parameters` heading, and parameter names render as plain text so only the direction tag looks like a boxed badge. Section labels such as `Enum Values`, `Functions`, `Fields`, `Params`, and `Constructor` render as real Markdown `###` headings with a blank line before section content, matching the old hover's visual weight instead of small inline bold text. The synthetic Attribute constructor section uses the same Rust HTML declaration renderer instead of a fenced code block, so `void` is keyword-blue, `Attribute` is class-green, and parameter types follow the normal type-color/link path. Class summaries use `IndexQuery::completion_members_for_class`, render public-facing direct/overlay constructors, functions, and fields with the same Rust declaration-coloring path as the main hover line, and merge public inherited methods/fields into the normal `Functions` and `Fields` sections without a separate inherited dump or per-row owner suffix. Enum value summaries render all source-backed enum values, not fenced code blocks or bounded samples, so every visible enum value can be command-linked to its declaration. Destructors and `protected`/`private` members are intentionally omitted from normal class hover summaries to keep the summary focused on the class's public API surface; hovering those declarations directly still shows their own symbol facts. When hover selects a file-local class declaration and an external overlay is available, direct members come from the selected file-local index while inherited member summaries can come from the external overlay. That keeps declaration hovers and type-usage hovers consistent without changing resolver selection or hiding open-document facts.

Color is best-effort Markdown HTML for VS Code hover display. It reuses the semantic-token palette and is not a second coloring or classification system. The renderer emits sanitized inline spans using only VS Code-supported hover CSS such as `style="color:#40b5ac;"`; the TypeScript language client is responsible only for enabling HTML-capable Markdown rendering. The hover remains readable when a client ignores inline HTML color.

Known symbol labels in hover declarations and class summaries can render as trusted command links when the target URI and byte range are available from indexed facts. Link creation is source-backed: type names are resolved through `IndexQuery` class/enum/typedef facts, member rows link to their `EditorCompletionCandidate` display target, and no regex or text-shape linkifier is used. The TypeScript client only opens the target URI/range supplied by Rust. Links inside the renderer's HTML declaration blocks must be emitted as HTML `<a href="command:...">` anchors, not Markdown `[label](command:...)` links, because VS Code does not parse Markdown link syntax inside raw HTML hover blocks.

The previous TypeScript extension used custom hover-only TextMate grammars for colored code fences. This renderer intentionally does not revive those grammars. Primary declarations are colored directly from `SymbolDisplayInfo` facts because VS Code hover code fences do not consume the Rust semantic-token classifier. It keeps one source-backed presentation path through `SymbolDisplayInfo` and Rust Markdown rendering.

## Dependencies and Boundaries

Depends on `SymbolDisplayInfo`, `IndexQuery`, semantic-token palette helpers, and LSP symbol-kind labels. It must not select hover targets, resolve symbols, parse source, infer types, call Workbench, implement semantic-token behavior, or introduce TextMate/regex hover classification.

## Change Notes

- Added to keep hover presentation separate from resolver selection and LSP request handling while avoiding a parallel symbol-display system.
- Adopted the useful surface format from the old extension: kind line, declaration block, plain documentation text, attribute params/constructor sections, class member sections, enum member sections, and declaration-first layout. The implementation is new Rust renderer code over indexed facts, not old TypeScript or TextMate grammar reuse.
- Class member sections render public-facing direct/overlay constructors, functions, and fields instead of sampled code-fence snippets, so class hover shows the useful API surface with Rust-rendered colors.
- Public inherited methods/fields are merged into the normal `Functions` and `Fields` sections without an owner suffix. Destructors and `protected`/`private` members are intentionally hidden from normal class hover summaries to avoid noisy internals.
- Callable declarations suppress low-value modifiers such as `override`, `proto`, `external`, and `event` so class hover reads like an API surface instead of a source echo.
- Documentation commands render as native Markdown inline-code badge labels such as `in`, `out`, `return`, `warning`, and `note` so Doxygen-style facts are visible without exposing raw comment markup, relying on custom hover CSS, or reusing semantic-token colors.
- Normal hover intentionally suppresses source path/provenance and duplicate modifier/detail rows. Debug hover remains the raw troubleshooting surface.
- Added source-backed hover links for known type, enum value, and member labels. Links are emitted only when indexed symbol display metadata can produce a target URI and byte range. Declaration-block and enum-value links use HTML anchors so they remain clickable inside the renderer's HTML-colored declaration output.
- Attribute constructor reference output now uses the same HTML declaration renderer as normal hover declarations instead of a fenced `enforce` code block.

## Future Improvements

If hover needs richer source provenance or raw docs, route that through debug-hover or explicit display fields rather than dumping raw index data into normal hover.
