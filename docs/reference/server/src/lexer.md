# server/src/lexer.rs

## Purpose

Tokenizes Enfusion Script source into full-fidelity tokens and byte spans.

## Ownership

The lexer is the first Rust language-engine layer. It owns token classification and span preservation; later parser, model, formatter, and LSP layers consume its output.

## Current Behavior

It emits identifiers, keywords, number/string literals, whitespace, comments, documentation comments, punctuation, operators, `#`, unknown characters, invalid numbers, and unterminated strings/comments. Tokens retain `TokenKind` and `TextSpan`; source text remains external.

`//!` and `/*! ... */` receive documentation-comment kinds. Doxygen tags remain raw comment text rather than a lexer-level documentation model. Keyword and number handling is exercised against focused snippets and committed source-derived fixtures.

## Dependencies and Boundaries

Uses only Rust standard-library behavior. It does not parse declarations, resolve symbols, evaluate preprocessor directives, crawl files, call Workbench, or handle LSP.

## Verification

Lexer unit tests cover token shapes, keyword/operator recognition, malformed literals/comments, and fixtures in `tools/fixtures/lexer/`.

## Future Direction

Token kinds evolve only with verified source/compiler evidence. Parser utilities remain parser-owned until a concrete shared need exists.
