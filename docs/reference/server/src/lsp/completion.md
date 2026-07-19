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
is pending, the foreground path first attempts a valid-syntax, size- and
deadline-bounded `LocalScopeQuery` against the current source revision. That
query constructs an ephemeral parser/scope view and returns current callable
locals and parameters without waiting for background analysis. Malformed,
oversize, deadline-exceeded, member, and argument contexts use only current
lexer prefix facts and captured workspace/game-data indexes for a deterministic
top-level result; they never combine current text with an older local analysis.
It combines local,
workspace, and game-data candidates without rebuilding a merged index,
preserves source-backed precedence, and caps output at 250 items. Member access
uses receiver/owner resolution; static owners, typedefs, enum members,
attributes, and `new` expressions have dedicated source-backed paths once their
current analysis is available.

Every Rust `LspCompletionReport` carries `QueryQuality` and a recovery reason.
`Exact` is the matching-revision analyzed path. `RecoveryExact` is reserved
until a bounded recovery query proves candidate equivalence. `Unavailable`
records the deterministic pending contract and its reason in request/debug
logs. Pending member and argument positions deliberately return a
top-level/lexical fallback tagged `member-unavailable-top-level-fallback` or
`argument-unavailable-top-level-fallback`; they never expose receiver, local,
or argument facts from an older revision. This is not an `isIncomplete` signal
or a request for the client to retrigger completion.

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
