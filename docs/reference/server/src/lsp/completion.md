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
is pending, it first maps the LSP UTF-16 position through the matching
foreground position index when present, or directly through the immutable
current snapshot when foreground publication is still in flight. It then uses a
fixed 16KiB-before/2KiB-after
current-snapshot lexer window: an `OverrideQuery` for a class header and direct
base within a 17KiB before-cursor budget, a `ReceiverResolutionQuery` for one bare local,
parameter, or field receiver, a `LocalScopeQuery` for ordinary callable-local
prefixes, and an `ArgumentLabelQuery` for one bare externally indexed callable.
These queries recover only a proven class base, brace-scoped declarations, parameter-body ownership,
receiver type, and call-label facts wholly proven inside that window. A
one-or-more-step zero-argument external call path rooted in an externally
indexed global function is also admitted when every return type can be proved
from the captured external indexes; this preserves immediate completion both
after the root call (`GetGame().GetPlayer...`) and through common game API
chains such as `GetGame().GetPlayerController().`. A bare external static
owner (`RplChannel.` or another indexed enum/class) is also admitted when the
same bounded facts prove no visible local, parameter, or field binding shadows
that name; the response then contains only its externally indexed static
members. They never
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
unavailable. No pending path combines current text with older local facts. A successful
`OverrideQuery` returns only externally proven override skeletons and
declaration keywords (including `override`), plus a bounded, prefix-filtered
external type/attribute projection. This preserves canonical attribute
insertions such as `RplRpc` alongside matching inherited overrides without
appending the generic top-level fallback. If neither the contextual nor bounded
external candidates match, the query declines so the ordinary local/top-level
path can provide its normal results.
It combines local,
workspace, and game-data candidates without rebuilding a merged index,
preserves source-backed precedence, and caps output at 250 items. Member access
uses receiver/owner resolution; static owners, typedefs, enum members,
attributes, and `new` expressions have dedicated source-backed paths once their
current analysis is available.

Completion is suppressed inside line, documentation, block, and unfinished
block comments for both cached and pending snapshots. This check uses the
request's existing lexer tokens; only an already-rejected bounded window may
use the existing 128KiB current-snapshot lexical-recovery cap to recognize an
unfinished block comment rather than fall back to top-level items.

Every Rust `LspCompletionReport` carries `QueryQuality` and a recovery reason.
`Exact` is the matching-revision analyzed path. `RecoveryExact` is reserved
until a bounded recovery query proves candidate equivalence. `Unavailable`
records the deterministic pending contract and its reason in request/debug
logs. Pending member and argument positions deliberately return a
top-level/lexical fallback tagged `member-unavailable-top-level-fallback` or
`argument-unavailable-top-level-fallback`; they never expose receiver, local,
or argument facts from an older revision. This is not an `isIncomplete` signal
or a request for the client to retrigger completion.

The normal completion log's `foreground_ready` and `cached_analysis` fields
separately record position-index availability and whether matching-revision full
analysis was actually used; neither is inferred from query quality. Its bounded
`response_labels` field records the first three labels sent in the actual LSP
response, so typing-path diagnostics can be distinguished from the separate
cached-analysis debug command. Completion timings report request latency
independently from bounded context, lookup, and render work. Semantic-analysis logs record `semantic_idle_delay_ms=0`, build
timings, and total job latency so scheduler delay cannot be mistaken for parse
or index cost. When a completed call receiver is syntactically current but its
externally indexed zero-argument chain cannot be proved, the fallback records
`current-revision-external-call-chain-unresolved`. This keeps the response
safe while distinguishing external-chain lookup from ordinary unavailable
local receiver facts. It adds only bounded lexer/index work already required
by the pending query and no source content to runtime logs.

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
suggestions. Across every completion context, exact names and case-insensitive
prefixes rank before boundary abbreviations and subsequence matches; the score
bands are non-overlapping so a long fuzzy name cannot outrank a direct prefix.
Override completion replaces only the incomplete declaration prefix. When the
current lexer tokens prove a following block already exists (including after a
comment), it emits only the resolved signature and preserves that body; it
otherwise emits the normal method skeleton.
Callable completions share [callable.md](callable.md) parameter
parsing with signature help, provide snippets/follow-up commands when safe, and
avoid duplicate named argument labels case-insensitively.

`RplRpc` attribute shorthand is a verified exception to placeholder-only
callable insertion. When its constructor has the engine-defined required
`RplChannel` and `RplRcver` parameters, it expands to the canonical
request-to-server annotation with its snippet cursor retained inside the call:
`[RplRpc(${1:RplChannel.Reliable}, ${2:RplRcver.Server})]`. Each complete enum
expression is selected, so typing replaces it and resumes ordinary value
completion. The built-in `editor.action.triggerSuggest` command opens the
Rust-owned enum completion policy: qualified enum values rank first, while
general value candidates remain below them. Every item replaces the full enum
expression, so selecting a general value cannot form an invalid
`RplChannel.<unrelated value>` expression. Tab advances to the selected
`RplRcver.Server` expression. The completion item's signature detail still
documents the optional condition and custom-condition inputs.
The same template applies inside an already typed `[` without duplicating its
brackets. This is deliberately signature-checked and does not infer defaults
from enum declaration order for other attributes.

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
