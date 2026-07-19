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
is pending, the foreground path uses a fixed 16KiB-before/2KiB-after
current-snapshot lexer window: a `ReceiverResolutionQuery` for one bare local,
parameter, or field receiver, a `LocalScopeQuery` for ordinary callable-local
prefixes, and an `ArgumentLabelQuery` for one bare externally indexed callable.
These queries recover only brace-scoped declarations, parameter-body ownership,
receiver type, and call-label facts wholly proven inside that window. A
one-or-more-step zero-argument external call path rooted in an externally
indexed global function is also admitted when every return type can be proved
from the captured external indexes; this preserves immediate completion both
after the root call (`GetGame().GetPlayer...`) and through common game API
chains such as `GetGame().GetPlayerController().`. They never
call `file_index_for_source`, construct `SemanticFile`/`SymbolIndex`/
`LexicalScopeModel` from document text, walk a CST root, or read prior document
analysis on the request thread. The receiver query captures workspace/game-data
indexes once and otherwise admits only a simple identifier receiver; local,
argument-bearing, indexed, static, malformed/unterminated, unproven, and
deadline-exceeded chains use only current lexer prefix facts and captured
workspace/game-data indexes for a deterministic top-level result. The
argument query returns only parameter-label items for a bare captured-index
function or method; member/delegate calls, constructors, malformed text, values
after a label, locally declared callables, and over-budget work remain
unavailable. No pending path combines current text with older local facts.
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

If the fixed window begins or ends inside an otherwise valid multi-line comment
or string, a current-snapshot lexical-state recovery is allowed only for files
up to 128KiB. It lexes that bounded snapshot once, then still restricts
receiver/local recovery to the original cursor window. This lets externally
proven chains such as `GetGame().GetPlayerController()` remain immediately
available in ordinary large files without consulting stale facts. Oversized,
malformed, or deadline-exceeded snapshots retain the independently indexed
top-level/keyword fallback; pending requests never infer local, member, or
argument facts without current lexical proof.

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
