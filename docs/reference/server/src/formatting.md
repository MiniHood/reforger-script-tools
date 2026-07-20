# server/src/formatting.md

## Purpose

Records the Rust-owned formatting and typing-assist boundary.

## Ownership

Formatting belongs in the Rust language engine and consumes lexer/parser/AST/model facts. TypeScript only exposes LSP capabilities, commands, settings, and keybindings. The formatting core owns safe layout edits; completion owns symbol/template choice and signature help owns callable explanation.

## Current Behavior

The current first slice is a Rust `textDocument/onTypeFormatting` semicolon
assist in `server/src/lsp/on_type_formatting.rs`. It adds a single zero-width
`;` edit only after Enter when the preceding physical line is a complete
standalone call/member-call expression. It inserts before a trailing `//`
comment.

The assist is deliberately fail-closed. Controls, callable declarations and
constructors, attributes, existing semicolons, incomplete expressions,
strings/comments, directives, brace bodies, malformed source, large snapshots,
multi-caret edits, and selections all produce no edit. The classifier is
bounded to 64 KiB and does not consult semantic/index/workspace data or
schedule analysis work.

The parser preserves declarations, bodies, expressions, attributes,
comments/trivia, loops, and switch structure needed for a conservative future
formatter.

The planned feature surfaces are document/range formatting, on-type edits, typing assists, comment/Doxygen formatting, and formatting of already-inserted completion snippets. They share syntax-aware context rather than implementing separate text rewrite systems.

## Dependencies and Boundaries

Future formatting depends on lexer spans/trivia, parser syntax, and AST/model facts; targeted assists may use resolver/type facts. It must not become a second parser, use regex-only structural rewrites, re-order declarations, change semantics, evaluate macros, or apply broad ambiguous edits while a user is typing.

Only auto-apply an edit when the syntax leaves no realistic alternate user action. If an action chooses a symbol, callable, inherited member, argument label, enum owner, or source-backed template, it belongs to completion or an explicit assist. If it explains the active callable without editing, it belongs to signature help. Formatting changes only presentation of already-chosen text.

Generated/read-only sources require explicit policy before mutation. Documentation-comment formatting preserves raw user comments; any generated Doxygen scaffolding needs an explicit declaration-aware assist contract.

## Verification

The semicolon assist has table-driven safety cases for valid calls, member
calls, comments, CRLF/UTF-16, controls, declarations, malformed expressions,
comments/strings, directives, and large input. A future broader formatter must
define parser-backed edit safety cases, idempotence, malformed-source behavior,
and LSP edit projection tests before enabling edits.

## Future Direction

Keep completion, signature help, formatting, and documentation generation as
distinct owners. Future full formatting needs a parser-backed model; the
semicolon assist is intentionally not a general formatter.
