# `server/src/lsp/completion.rs`

## Purpose

Projects source-backed completion lists, insertion edits, and bounded developer
reports for the Rust language server.

## Ownership

Owns completion context detection, file-local/external candidate combination,
visibility filtering, ranking, keyword and override skeleton suggestions,
callable insertion text, and LSP item rendering. It does not own protocol
dispatch, document caching, resolver policy, or TypeScript retrigger parsing.

## Current Behavior

Completion reuses cached file analysis to recognize member, top-level, type,
override, and callable-argument contexts. It combines local, workspace, and
game-data candidates without rebuilding a merged index, preserves source-backed
precedence, and caps output at 250 items. Member access uses receiver/owner
resolution; static owners, typedefs, enum members, attributes, and `new`
expressions have dedicated source-backed paths.

Keywords are LSP-owned so language suggestions do not depend on VS Code word
suggestions. Callable completions share [callable.md](callable.md) parameter
parsing with signature help, provide snippets/follow-up commands when safe, and
avoid duplicate named argument labels case-insensitively.

## Dependencies and Boundaries

Depends on cached open-document analysis, `ReferenceResolver`, `IndexQuery`,
`SymbolIndex`, display facts, and callable helpers. `lsp.rs` dispatches the
request; the TypeScript client only decides whether to retrigger editor UI and
must not reproduce completion context parsing.

## Verification

Run focused completion/callable tests and `cargo test` from `server/`. Cover
member/type/top-level contexts, visibility, workspace overlay updates, keyword
precedence, overloads, named labels, snippets, comments/strings, and the
output cap.

## Future Direction

Add ranking improvements, richer overload selection, and optional ghosting only
when supported by resolver/model facts. Keep future completion semantics in
Rust rather than text matching in the extension.
