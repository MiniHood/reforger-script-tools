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

Completion reuses matching-revision file analysis to recognize member,
top-level, type, override, and callable-argument contexts. While full analysis
is pending, the foreground path instead uses only current lexer prefix facts
and captured workspace/game-data indexes for a deterministic top-level result;
it never combines current text with an older local analysis. It combines local,
workspace, and game-data candidates without rebuilding a merged index,
preserves source-backed precedence, and caps output at 250 items. Member access
uses receiver/owner resolution; static owners, typedefs, enum members,
attributes, and `new` expressions have dedicated source-backed paths once their
current analysis is available.

Request logs record `query_quality=Exact` for matching local analysis and
`query_quality=Unavailable` for the pending lexical/top-level contract. The
latter is a deterministic fallback for facts it can prove, not stale local
state or a signal that the client should retrigger completion.

Keywords are LSP-owned so language suggestions do not depend on VS Code word
suggestions. Callable completions share [callable.md](callable.md) parameter
parsing with signature help, provide snippets/follow-up commands when safe, and
avoid duplicate named argument labels case-insensitively.

## Dependencies and Boundaries

Depends on current open-document source/analysis, `ReferenceResolver`,
`IndexQuery`, `SymbolIndex`, display facts, and callable helpers. `lsp.rs`
dispatches the request; the TypeScript client contains no ordinary-edit
completion retrigger or completion-context parsing.

## Verification

Run focused completion/callable tests and `cargo test` from `server/`. Cover
member/type/top-level contexts, visibility, workspace overlay updates, keyword
precedence, overloads, named labels, snippets, comments/strings, and the
output cap.

## Future Direction

Add ranking improvements, richer overload selection, and optional ghosting only
when supported by resolver/model facts. Keep future completion semantics in
Rust rather than text matching in the extension.
