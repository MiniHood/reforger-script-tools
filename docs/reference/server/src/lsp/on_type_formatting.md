# `server/src/lsp/on_type_formatting.rs`

## Purpose

Owns bounded, fail-closed Rust classifiers for immediate editor typing assists.

## Architecture Role

This is formatting logic below the LSP transport. It receives only the current
immutable document snapshot and an already-admitted cursor position, then
returns a source span, replacement text, and when needed a Rust-authored caret
position. It has no VS Code APIs, workspace reads, semantic analysis, or
background work.

## Current Behavior

The module supports three narrow assists:

- inserts a semicolon after a complete standalone call, declaration, or return
  statement when Enter makes that the only supported completion;
- expands VS Code's exact empty native `/**/` pair into an indented multiline
  block comment; and
- when a plain Enter splits a complete single-line condition in an unfinished
  unbraced `if (` header, restores it as `if (<condition>)` and returns the
  immediate unbraced-body indentation/caret.

The incomplete-`if` path accepts only a current-line split with whitespace
between the inserted newline and the captured caret. It preserves the existing
condition text, including a split identifier, and refuses complete headers,
comments, unterminated strings, directives, braces, attributes, malformed delimiters,
unfinished expressions, and documents above the bounded source size. Ordinary
complete `if`, `else if`, and `else` indentation remains native VS Code
language-configuration behavior.

## Dependencies and Boundaries

Uses the lexer and source spans only. It must not resolve symbols, inspect a
workspace, wait for foreground analysis, or become a general document
formatter. The TypeScript client only transports and applies its plans after
version/caret validation.

## Change Notes

- Added the incomplete-`if` Enter plan so pressing Enter inside a condition
  does not leave a broken multi-line header or rely on a second formatting
  system.

## Future Improvements

- Add other control forms only as separately source-backed, fail-closed
  interaction slices.
- Keep broad whitespace formatting behind explicit formatting commands rather
  than expanding these typing assists.
