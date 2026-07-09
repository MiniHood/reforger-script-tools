# server/examples/symbol_debug.rs

## Purpose

Prints compact per-file symbol debug output to stdout.

## Architecture Role

This is developer/Codex inspection tooling for one source file. It is not VS Code runtime behavior, not an LSP command, not workspace indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example requires `--file <path>` and accepts optional `--symbol <name>` or `--line <line>`. It parses the file, builds an `AstSourceFile`, creates a `SymbolCatalog` with file metadata, and prints compact Markdown-style output. Files under `tools/fixtures/` are marked as fixture source; other inspected files are marked as workspace source. Output includes source kind, absolute path, root path when known, relative path when known, and priority.

Without filters it prints the full file-local symbol tree. With `--symbol`, it prints matching symbol records with parent chains and immediate children. With `--line`, it prints symbols whose declaration or selection span touches the requested 1-based line, plus parent chains and immediate children. Rendered records include line/column plus byte spans, attribute names resolved through the catalog API, modifiers, doc-comment counts, and cleaned doc previews. Paths may be absolute or repo-relative.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, and model modules. It must not resolve symbols across files, normalize type text, call Workbench, become a VS Code command, or become runtime extension code.

## Change Notes

- Added targeted symbol debugging by file, symbol name, and line.
- Improved debug readability with line/column locations, attribute names, `<none>` list markers, and cleaned doc previews.
- Attribute name rendering now uses `SymbolCatalog::attribute_name()` instead of local parsing.
- Debug output now displays catalog-level source metadata for the inspected file.

## Future Improvements

- Add optional JSON output only if a future tool genuinely needs machine-readable symbol debug records.
