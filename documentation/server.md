# server/

## Purpose

Owns the Rust language-tooling foundation for Reforger Script Tools.

## Architecture Role

This folder is the future language-engine side of the project. It should contain compiler-style language intelligence that must stay out of the TypeScript VS Code shell. Current slices include the lexer, the declaration-level parser scaffold, the first syntax-backed AST declaration wrapper layer, the first file-local declaration model/symbol catalog layer, and the first in-memory symbol index layer.

## Current Behavior

The Rust crate exposes a full-fidelity lexer that returns token kinds and byte spans without copying source text into tokens. It also exposes a declaration-level parser that consumes lexer tokens and returns a full-fidelity syntax tree preserving trivia, tokens, and balanced method-body blocks. The AST layer provides file-local declaration views over that syntax tree without semantic resolution. The model layer converts those AST views into source-backed file-local symbol records with stable IDs and parent-child relationships. The index layer aggregates many file-local catalogs into global symbol handles and lookup maps without semantic resolution or persistence.

## Dependencies and Boundaries

The crate currently has no external Rust dependencies. It must not import VS Code APIs, implement extension activation, perform Workbench downloads, or mix LSP behavior into lower layers before those slices exist.

## Change Notes

- Added the initial `server/` crate as a single focused Rust library for language tooling.
- Added lexer-only tokenization for identifiers, keywords, literals, trivia, punctuation, operators, preprocessor marker tokens, and unterminated string/comment errors.
- Added the first parser scaffold for declaration-level syntax, full-fidelity token preservation, parse diagnostics, and parser fixture reporting.
- Added the first AST declaration wrapper layer over parser syntax nodes.
- Added the first file-local model/symbol catalog layer over AST declarations.
- Added the first in-memory symbol index over file-local catalogs.

## Future Improvements

- Expand parser coverage into statements and expressions in separate verified slices.
- Expand workspace indexing with incremental updates and workspace-over-game-data review.
- Add LSP process wiring from TypeScript only after the Rust side has useful language-server behavior.
- Consider splitting into `crates/` only if multiple Rust crates become necessary.
