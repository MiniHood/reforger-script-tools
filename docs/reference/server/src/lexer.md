# server/src/lexer.rs

## Purpose

Owns tokenization for Enfusion Script source text.

## Architecture Role

This is the first Rust language-engine layer. It produces full-fidelity tokens with byte spans for future parser, formatter, model, and LSP layers. It does not parse declarations, resolve symbols, evaluate preprocessor directives, or apply Workbench/compiler behavior.

## Current Behavior

The lexer emits tokens for identifiers, keywords, numbers, strings, whitespace, comments, documentation comments, punctuation, operators, `#`, unknown characters, invalid numbers, and unterminated strings/comments. Tokens store `TokenKind` and `TextSpan`; source text stays external and can be sliced by span.

Documentation comments are classified as `DocLineComment` for `//!` and `DocBlockComment` for `/*! ... */`. Doxygen-style contents such as `\param`, `\return`, `\warning`, and `\code` remain raw comment text and are not parsed by the lexer.

Unit tests cover focused snippets and committed lexer fixtures under `tools/fixtures/lexer/`, including larger game-data-derived Core/class/config/editor/Workbench fixtures. Keyword coverage includes documented/game-data-observed script words such as `auto`, `event`, `thread`, `vanilla`, `debug`, and `func`.

## Dependencies and Boundaries

The lexer uses only Rust standard library behavior. It must stay independent of VS Code APIs, Workbench processes, file-system crawling, parser logic, semantic analysis, indexing, and LSP request handling.

## Change Notes

- Added lexer token/span types and keyword/operator recognition.
- Added fixture smoke coverage for committed lexer fixtures.
- Added keyword coverage for documented/game-data-observed `auto`, `event`, `thread`, `vanilla`, `debug`, and `func`, plus focused tests for hex, float, and scientific number literals.
- Added documentation comment token kinds, invalid-number reporting, token classification helpers, and focused parser-facing shape tests.
- Added larger game-data-derived editor preview and Workbench formatter fixture coverage.

## Future Improvements

- Use the corpus report to validate tokenization against larger game-data updates.
- Add parser-facing token utilities only when parser implementation starts.
- Adjust token kinds as Workbench-confirmed syntax cases require.
