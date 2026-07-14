# server/src/lsp/definition.rs

## Purpose

Owns LSP definition projection for Ctrl+click/navigation.

## Architecture Role

This module sits inside the Rust LSP layer and converts resolver-selected symbols into standard LSP `LocationLink[]` responses. `server/src/lsp.rs` keeps request dispatch, document storage, and external-overlay lifecycle.

## Current Behavior

Definition uses the resolver as the single reference path. It accepts cached open-document analysis, the current document URI, the requested LSP position, and an optional external workspace/game-data overlay index. File-local candidates return the current URI, origin selection range, full target declaration range, and selected symbol name range. External candidates return a `file://` URI built from indexed absolute path metadata plus target ranges computed from the external source file.

Definition returns no links for unresolved identifiers, named argument labels, external symbols without readable absolute paths, value keywords such as `true` / `false`, and invalid positions. Identifier references and keyword type positions such as `string`, `vector`, `bool`, `int`, `float`, and `typename` are both resolved through the resolver when they appear inside source-backed declaration type spans. Preprocessor macro identifiers resolve to matching indexed `#define NAME` symbols when such a definition exists; directive words such as `ifdef` and `endif` remain non-symbol targets. Reports still keep `Location[]`-style rows for compatibility, derived from the target selection range.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `SymbolIndex`, file-local `FileIndexAnalysis`, and LSP range/position helpers. It may read external source files only to convert cached byte spans into LSP ranges for definition targets.

This module does not own hover, completion, references, workspace indexing, file watching, diagnostics, or protocol dispatch.

## Change Notes

Extracted from the monolithic `server/src/lsp.rs` without behavior changes. The file URI helper remains crate-visible for existing tests and workspace overlay utilities. Definition projection later moved from live `Location[]` responses to `LocationLink[]` while keeping report compatibility rows. Keyword type-position navigation and preprocessor macro-definition navigation are handled by the resolver so generated script declarations and `#define` symbols can be targeted without duplicating lookup logic in this module.

## Future Improvements

Keep all future definition lookup rules routed through the resolver rather than duplicating symbol search in the LSP layer. Multi-target definition can extend the report/link list once resolver deliberately selects more than one target.
