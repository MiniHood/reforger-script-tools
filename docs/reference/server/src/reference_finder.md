# server/src/reference_finder.rs

## Purpose

Owns the first file-local reference search foundation for future references and rename work.

## Architecture Role

The module sits above the lexer, resolver, file-local index, parser, and lexical scope model. It scans identifier tokens in one source file and asks the resolver to select the symbol at each token. A token is reported as a reference only when the resolver selects the exact requested file-local `GlobalSymbolId`.

## Current Behavior

`find_file_local_references` accepts source text, the file-local `SymbolIndex`, parsed syntax tree, lexical scope model, and a target symbol id. It returns all declaration and usage tokens that resolve to that target, plus the number of identifier tokens scanned.

`scan_file_local_references` performs the same resolver-backed work once per file and groups all file-local references by selected symbol id. `scan_file_local_references_with_external` accepts an optional external workspace/game-data index so corpus reports can separate real unresolved identifiers from ordinary cross-file references. It also records unresolved identifier tokens and external selections for review. This is the preferred path for reports because it avoids rescanning the same file once per declaration.

References are exact and resolver-backed. Shadowed locals, parameters, fields, and member accesses follow the same rules used by hover, definition, completion, and semantic tokens. Unresolved records are review evidence only; they are not references and are not candidates for rename.

`analyze_file_local_rename_at_offset` is the first rename architecture shape. It resolves the symbol under a byte offset, requires a file-local selected symbol with a stable name, returns resolver-confirmed file-local references, and attaches safety metadata such as same-name symbol count and declaration/usage reference counts. It does not create text edits or perform workspace-wide rename.

## Dependencies and Boundaries

Depends on `lexer`, `resolver`, `index`, `scope`, and `syntax`. It does not perform workspace-wide search, text-only matching, semantic rename edits, cross-file references, Workbench validation, or LSP protocol handling.

## Change Notes

Added as the first references/rename foundation so reference correctness can be reported before adding any editor-visible reference or rename feature. Added a single-pass scan API for corpus-scale review. Added the first file-local rename analysis shape without edit application.

## Future Improvements

Add workspace-wide symbol reference search only after file-local behavior is proven. Rename should build on this resolver-backed path instead of introducing a separate text matcher.
