# server/src/formatting.rs

## Purpose

Records the architecture boundary for a future Rust-side formatter and typing-assist system.

## Ownership

Formatting belongs in the Rust language engine and consumes lexer/parser/AST/model facts. TypeScript only exposes LSP capabilities, commands, settings, and keybindings. The future formatting core owns safe layout edits; completion owns symbol/template choice and signature help owns callable explanation.

## Current Behavior

No formatter is implemented. The parser currently preserves declarations, bodies, expressions, attributes, comments/trivia, loops, and switch structure needed for a conservative future formatter.

The planned feature surfaces are document/range formatting, on-type edits, typing assists, comment/Doxygen formatting, and formatting of already-inserted completion snippets. They share syntax-aware context rather than implementing separate text rewrite systems.

## Dependencies and Boundaries

Future formatting depends on lexer spans/trivia, parser syntax, and AST/model facts; targeted assists may use resolver/type facts. It must not become a second parser, use regex-only structural rewrites, re-order declarations, change semantics, evaluate macros, or apply broad ambiguous edits while a user is typing.

Only auto-apply an edit when the syntax leaves no realistic alternate user action. If an action chooses a symbol, callable, inherited member, argument label, enum owner, or source-backed template, it belongs to completion or an explicit assist. If it explains the active callable without editing, it belongs to signature help. Formatting changes only presentation of already-chosen text.

Generated/read-only sources require explicit policy before mutation. Documentation-comment formatting preserves raw user comments; any generated Doxygen scaffolding needs an explicit declaration-aware assist contract.

## Verification

No formatter verification exists yet. A future slice must define parser-backed edit safety cases, source-derived/Workbench-confirmed style evidence, idempotence, malformed-source behavior, and LSP edit projection tests before enabling edits.

## Future Direction

Design a small verified vertical slice before adding implementation: conservative document formatting or a single unambiguous on-type edit. Keep completion, signature help, formatting, and documentation generation as distinct owners.
