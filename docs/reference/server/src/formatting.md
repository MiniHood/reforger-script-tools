# server/src/formatting.md

## Purpose

Records the Rust-owned formatting and typing-assist boundary.

## Ownership

Formatting belongs in the Rust language engine and consumes lexer/parser/AST/model facts. TypeScript only exposes LSP capabilities, commands, settings, and keybindings. The formatting core owns safe layout edits; completion owns symbol/template choice and signature help owns callable explanation.

## Current Behavior

The current first slice is a Rust `textDocument/onTypeFormatting` semicolon
assist in `server/src/lsp/on_type_formatting.rs`. A thin extension document
change bridge admits exactly one plain Enter edit and forwards its current
revision and caret. Rust adds a single zero-width `;` edit only when the
preceding physical line is a complete standalone call/member-call expression,
typed variable declaration, a bare `return`, or a value-return statement. It inserts before a
trailing `//` comment.

The assist is deliberately fail-closed. Controls, callable declarations and
constructors, attributes, existing semicolons, incomplete expressions,
strings/comments, directives, brace bodies, malformed source, large snapshots,
multi-caret edits, and selections all produce no edit. The classifier is
bounded to 64 KiB and does not consult semantic/index/workspace data or
schedule analysis work.

Value-return support is additionally limited to complete value-expression
tokens. It rejects control keywords, adjacent primary values, and incomplete
`new Type(...)` construction. Separately, standalone call statements must end
in a call rather than a bare member or index access. These checks do not treat
delimiter balance alone as proof that a user has finished an expression.

The parser preserves declarations, bodies, expressions, attributes,
comments/trivia, loops, and switch structure needed for a conservative future
formatter.

The current comment-formatting core provides a Rust-only,
`format_comment_region` operation. It accepts one explicitly selected,
comment-only region and returns zero or more byte-span edits. It aligns each
contiguous comment group to its first line while preserving delimiters, tags,
prose, trailing text, and line endings. Block continuation lines beginning
with `*` use one space after the group indentation. Code, strings, directives,
trailing comments, partial comment tokens, malformed ranges, and mixed
code/comment selections return no edits. It is exposed only through the
explicit standard textDocument/rangeFormatting request.

The planned feature surfaces are document/range formatting, on-type edits, typing assists, comment/Doxygen formatting, and formatting of already-inserted completion snippets. They share syntax-aware context rather than implementing separate text rewrite systems.

## Dependencies and Boundaries

Future formatting depends on lexer spans/trivia, parser syntax, and AST/model facts; targeted assists may use resolver/type facts. It must not become a second parser, use regex-only structural rewrites, re-order declarations, change semantics, evaluate macros, or apply broad ambiguous edits while a user is typing.

The comment-region core consumes full-fidelity lexer trivia only. Its caller
must project LSP UTF-16 ranges to valid source spans and may expose it only as
an explicit range-formatting action; it must not run on type, on save, or over
mixed code regions.

Only auto-apply an edit when the syntax leaves no realistic alternate user action. If an action chooses a symbol, callable, inherited member, argument label, enum owner, or source-backed template, it belongs to completion or an explicit assist. If it explains the active callable without editing, it belongs to signature help. Formatting changes only presentation of already-chosen text.

Generated/read-only sources require explicit policy before mutation. Documentation-comment formatting preserves raw user comments; any generated Doxygen scaffolding needs an explicit declaration-aware assist contract.

## Verification

The semicolon assist has table-driven safety cases for valid calls, member
calls, comments, CRLF/UTF-16, controls, declarations, malformed expressions,
comments/strings, directives, and large input. A future broader formatter must
define parser-backed edit safety cases, idempotence, malformed-source behavior,
and LSP edit projection tests before enabling edits.

Comment/Doxygen work has an explicit evidence gate before it may change source:
run the development-only `tools/comment-formatting-corpus-report.mjs` against a
known corpus, validate `tools/fixtures/formatting/comment_doxygen_matrix.c` in
Workbench, and record the versioned result. Corpus frequencies and parser
acceptance are discovery evidence only; neither establishes edit eligibility.

## Future Direction

Keep completion, signature help, formatting, and documentation generation as
distinct owners. Future full formatting needs a parser-backed model; the
semicolon assist is intentionally not a general formatter.

The next comment slice is parser/trivia-backed, region-scoped formatting that
preserves comment payload byte-for-byte. Explicit missing-documentation
generation remains a separate Rust-owned action after the fixture gate is
recorded. Do not add automatic comment conversion, prose reflow, trailing-doc
movement, or save-time documentation generation as part of that slice.

Range formatting uses only the current open-document snapshot and projects its
byte-span edits back to UTF-16 LSP ranges. Whole-document whitespace formatting,
on-type comment formatting, and documentation generation remain separate
future slices.
