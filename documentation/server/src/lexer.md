# server/src/lexer.rs

## Purpose

Owns tokenization for Enfusion Script source text.

## Architecture Role

This is the first Rust language-engine layer. It produces full-fidelity tokens with byte spans for future parser, formatter, model, and LSP layers. It does not parse declarations, resolve symbols, evaluate preprocessor directives, or apply Workbench/compiler behavior.

## Current Behavior

The lexer emits tokens for identifiers, keywords, numbers, strings, whitespace, comments, punctuation, operators, `#`, unknown characters, and unterminated strings/comments. Tokens store `TokenKind` and `TextSpan`; source text stays external and can be sliced by span.

Unit tests cover focused snippets and committed lexer fixtures, including a larger game-data-derived class/config fixture.

## Dependencies and Boundaries

The lexer uses only Rust standard library behavior. It must stay independent of VS Code APIs, Workbench processes, file-system crawling, parser logic, semantic analysis, indexing, and LSP request handling.

## Change Notes

- Added lexer token/span types and keyword/operator recognition.
- Added fixture smoke coverage for committed lexer fixtures.

## Future Improvements

- Validate tokenization against a larger game-data corpus once a corpus test harness exists.
- Add parser-facing token utilities only when parser implementation starts.
- Adjust token kinds as Workbench-confirmed syntax cases require.
