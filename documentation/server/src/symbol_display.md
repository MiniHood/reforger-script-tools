# server/src/symbol_display.rs

## Purpose

Owns editor-ready presentation views for indexed symbols.

## Architecture Role

This file sits above the raw symbol index and below future LSP hover, completion, document-symbol, and debug output. It turns copied indexed facts into stable display records without reparsing source text.

## Current Behavior

`SymbolDisplay::for_symbol()` returns `SymbolDisplayInfo` for one indexed symbol. The display record includes label, kind, detail text, callable signature, raw `doc_comments`, a bounded `documentation_preview`, modifiers, attributes, source provenance, spans, conditional context, and callable form.

Callable symbols use `SymbolIndex::callable_signature()`, including source-backed parameter modifiers such as `out`, `inout`, and `notnull`. Non-callable symbols use existing indexed detail text such as type text, base type, enum value, default value, typedef aliased type, parameter detail, or local-variable detail. Documentation comments are preserved as raw copied comment text. `documentation_preview` is display-only: it strips comment markers and separator noise, trims common block-comment `*` prefixes, and lightly renders first-useful Doxygen-style tags such as `\brief`, `\param`, `\return`, `\warning`, and `\note` into readable preview text.

## Dependencies and Boundaries

This file depends on `server/src/index.rs` and model/source metadata types. It must not parse source files, resolve symbols, evaluate types/defaults/enum values, perform full Doxygen extraction, call Workbench, persist caches, or handle LSP protocol requests.

## Change Notes

- Added the first symbol display layer so future editor features and debug tools can share one presentation shape.
- Raw documentation storage is named `doc_comments`; `documentation_preview` is the only display-rendered documentation field.
- Kept display source-backed through copied indexed facts because `SymbolIndex` does not retain source text.
- Added lightweight doc-preview rendering so hover/completion previews avoid exposing raw Doxygen tags while raw comments remain unchanged.
- Added display support for `LocalVariable` symbols using the same type/default detail shape as parameters and fields.

## Future Improvements

- Add LSP-specific conversion separately when hover, completion, or document-symbol handlers exist.
- Add structured documentation extraction only after Doxygen/tag behavior is intentionally designed.
