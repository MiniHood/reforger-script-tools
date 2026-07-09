# server/

## Purpose

Owns the Rust language-tooling foundation for Reforger Script Tools.

## Architecture Role

This folder is the future language-engine side of the project. It should contain compiler-style language intelligence that must stay out of the TypeScript VS Code shell. Current slices include the lexer, the declaration-level parser scaffold, the first syntax-backed AST declaration wrapper layer, the first file-local declaration model/symbol catalog layer, and the first in-memory symbol index layer.

## Current Behavior

The Rust crate exposes a full-fidelity lexer that returns token kinds and byte spans without copying source text into tokens. It also exposes a declaration-level parser that consumes lexer tokens and returns a full-fidelity syntax tree preserving trivia, tokens, and balanced method-body blocks. The AST layer provides file-local declaration views over that syntax tree without semantic resolution. The model layer converts those AST views into source-backed file-local symbol records with stable IDs and parent-child relationships. The index layer aggregates many file-local catalogs into global symbol handles and lookup maps without semantic resolution or persistence. The index build layer owns reusable source-root scanning, parser/AST/model/index construction, and build timing summaries. The index query layer wraps the raw index with editor-facing lookup APIs so future LSP features use preferred, overlay-aware paths instead of debug aggregate maps by accident. The symbol display layer converts copied indexed facts into editor-ready labels, details, signatures, documentation previews, and provenance output. The LSP layer is the first stdio protocol boundary for VS Code and currently exposes document symbols and file-local hover for open files.

## Dependencies and Boundaries

The crate currently uses only minimal bundled Rust dependencies for JSON protocol support. It must not import VS Code APIs, implement extension activation, perform Workbench downloads, or mix LSP behavior into lower compiler-style layers.

## Change Notes

- Added the initial `server/` crate as a single focused Rust library for language tooling.
- Added lexer-only tokenization for identifiers, keywords, literals, trivia, punctuation, operators, preprocessor marker tokens, and unterminated string/comment errors.
- Added the first parser scaffold for declaration-level syntax, full-fidelity token preservation, parse diagnostics, and parser fixture reporting.
- Added the first AST declaration wrapper layer over parser syntax nodes.
- Added the first file-local model/symbol catalog layer over AST declarations.
- Added the first in-memory symbol index over file-local catalogs.
- Added the reusable index build pipeline for explicit game-data/workspace source roots.
- Added the first editor-facing index query facade over the raw symbol index.
- Added the first indexed symbol display layer for shared hover/completion/debug presentation facts.
- Added a dev-only index build baseline report for measuring debug/release game-data index construction cost without corpus report rendering.
- Added the first stdio LSP scaffold and bundled binary entrypoint with document-symbol support.
- Added a dev-only LSP fixture report for reviewing document-symbol output across committed parser fixtures.
- Added a dev-only LSP corpus report for reviewing document-symbol projection over downloaded game-data scripts.
- Added file-local LSP hover support and a dev-only hover fixture report.

## Future Improvements

- Expand parser coverage into statements and expressions in separate verified slices.
- Expand workspace indexing with incremental updates and workspace-over-game-data review.
- Expand LSP behavior through small verified features such as diagnostics, completion, and definition.
- Consider splitting into `crates/` only if multiple Rust crates become necessary.
