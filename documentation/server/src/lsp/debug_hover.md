# server/src/lsp/debug_hover.rs

## Purpose

Owns the custom `reforger/debugHover` report rendering.

## Architecture Role

This module sits inside the Rust LSP layer as a targeted debug/report path. It uses the same cached file-local analysis, resolver, symbol display, and semantic-token projection as normal editor features, then renders a bounded Markdown report for human/Codex troubleshooting.

## Current Behavior

The report includes cursor/source context, nearby lexer tokens, nearby semantic-token coloring, parse diagnostics, resolver resolution, external-index status, selected symbol display facts, rendered hover Markdown, span candidates, parent/child context, and symbol-kind counts. It is intentionally heavier than runtime logs and should only run through the explicit debug-hover command/request.

Debug-hover selection follows the same resolver hover decision as normal hover. It may still list syntax-span candidates as debug evidence, but comments, whitespace, strings, and other non-symbol token classes should not be reported as selected symbols just because they are inside a broader declaration span.

The rendered hover Markdown section uses `server/src/lsp/hover_render.rs`, so Ctrl+F1 previews the same structured documentation, colored kind label, metadata, and bounded class member summary as normal hover.

The module also exposes a tiny label extraction helper used by `lsp.rs` request logging after a debug-hover request.

## Dependencies and Boundaries

Depends on lexer tokens, semantic-token projection, `ReferenceResolver`, `IndexQuery`, `SymbolDisplay`, cached `FileIndexAnalysis`, and the external-index status summary from `lsp.rs`.

It does not own normal hover behavior, protocol dispatch, document storage, workspace indexing, completion, definition, or diagnostics publishing. Keep full AST/index dumps out of this path unless a future targeted debug mode explicitly bounds them.

## Change Notes

Extracted from `server/src/lsp.rs` without behavior changes so the custom debug report has one owner and request dispatch remains focused.

## Future Improvements

Keep this report bounded and human-readable. If new language layers are added, expose concise input/output facts here rather than duplicating the layer’s implementation logic.
