# server/

## Purpose

Owns the Rust language-tooling foundation for Reforger Script Tools.

## Architecture Role

This folder is the future language-engine side of the project. It should contain compiler-style language intelligence that must stay out of the TypeScript VS Code shell. The first slice is a lexer only.

## Current Behavior

The Rust crate exposes a full-fidelity lexer that returns token kinds and byte spans without copying source text into tokens. It includes trivia tokens for whitespace and comments so later parser and formatting layers can preserve source shape.

## Dependencies and Boundaries

The crate currently has no external Rust dependencies. It must not import VS Code APIs, implement extension activation, perform Workbench downloads, or mix in parser/model/index/LSP behavior before those slices exist.

## Change Notes

- Added the initial `server/` crate as a single focused Rust library for language tooling.
- Added lexer-only tokenization for identifiers, keywords, literals, trivia, punctuation, operators, preprocessor marker tokens, and unterminated string/comment errors.

## Future Improvements

- Add parser modules only after lexer behavior is stable against fixtures.
- Add LSP process wiring from TypeScript only after the Rust side has useful language-server behavior.
- Consider splitting into `crates/` only if multiple Rust crates become necessary.
