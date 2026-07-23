# Architecture Review Journal

Status: in progress  
Started: 2026-07-23  
Scope: every repository-owned coding file: TypeScript, Rust, JavaScript tooling,
and executable configuration. The current authoritative inventory is 192 files:
91 under `server/`, 74 under `tools/` (including fixtures), 22 under `src/`,
and five root executable/configuration files. Fixtures are reviewed where their
structure or coverage affects the code they evidence; generated output and
dependencies are excluded.

This journal is an evidence log for the ongoing architecture review. It records
each reviewed slice, the files covered, and findings that survive the deletion
test. It intentionally does not prescribe implementation interfaces; those
belong to a later design discussion.

## Review method

- Trace responsibility from each module's callers to its implementation and
  tests.
- Check ownership against the documented TypeScript-shell/Rust-engine split and
  the LSP Runtime terminology in `CONTEXT.md`.
- Look for special cases without a proven semantic distinction, repeated work
  on hot paths, accidental duplicate state, and seams that scatter knowledge.
- Apply the deletion test before recording a deepening recommendation:
  deleting a useful module should make complexity reappear at its callers.
- Mark observations as findings only when source and test evidence establish a
  concrete concern. Every slice is recorded even when it yields no finding.

## Slice ledger

| Slice | Coding files | Status | Notes |
| --- | --- | --- | --- |
| Repository inventory, build/configuration | 5 | completed | `package.json`, `esbuild.js`, `eslint.config.mjs`, `tsconfig.json`, `language-configuration.json` |
| Extension composition and configuration | 8 | completed | `src/extension.ts`, `src/extensionConfig/`, diagnostics, game data |
| Editor/LSP bridges and extension tests | 15 | completed | 14 language-client modules and `src/test/extensionActivation.test.ts` |
| Rust language foundations | pending | pending | `server/src/*.rs` excluding LSP subtree |
| Rust LSP runtime and features | pending | pending | `server/src/lsp/` and submodules |
| Rust binary and examples | pending | pending | `server/src/bin/`, `server/examples/` |
| Developer tools and tool tests | pending | pending | `tools/*.mjs` and tests |
| Cross-cutting synthesis | pending | pending | Findings, performance, overlaps, coverage |

## Findings

### AR-001 — Manual game-data validation repeatedly serializes a full directory traversal

**Strength:** Strong
**Files:** `src/gameData/gameData.ts:100-116, 200-249`

`registerGameDataFeatures` starts validation on every extension activation when
the manual-folder setting is populated. Validation recursively visits every
directory and awaits each `readdir` before starting the next one. A valid
Reforger scripts tree is expected to contain at least 5,000 files, so this is a
full serial filesystem traversal in the normal, not exceptional, startup path.
It also repeats immediately after selecting the same folder.

The server then independently performs another recursive traversal before each
manual-folder cache load, collecting the file count, byte count, and latest
mtime (`server/src/index_cache.rs:2530-2626`). This doubles the normal
activation filesystem work without sharing a source-identity fact.

The warning needs the *threshold result*, not an exact count, and neither the
server restart nor source resolution needs the count. Make manual-source
validation a deep module whose interface answers whether the source is usable
and, only until the threshold, how many scripts were observed. It can stop at
the threshold, bound concurrent directory reads, and cache the successful
identity (canonical path plus directory metadata) in extension runtime state.
That concentrates the performance policy and gives callers leverage without
changing the source-resolution contract.

### AR-002 — Game-data source resolution is split across its owner and the language-client composition root

**Strength:** Worth exploring  
**Files:** `src/gameData/gameData.ts:57-67, 215-229, 390-400`; `src/languageClient/languageClient.ts:438-445`

`gameData` owns the manual-source user setting, normalization, and the rule
that a selected folder may itself be `scripts/`. The language-client module
separately reads that setting and imports one helper to reconstruct the source
path. The result is a shallow seam: a change to source selection, canonical
paths, or validation must be understood in both modules, while neither caller
gets a single resolved-source fact.

Expose one source-resolution interface from the game-data module that returns
the current supplied scripts path (or the installed path) as a value. The
language-client composition root should consume that value when building server
arguments; game-data acquisition should notify it only after publishing a new
value. The deletion test passes: deleting the helper and duplicate setting read
would otherwise re-create this policy at every server-start caller. This would
improve locality while preserving the documented ownership split.

### AR-003 — Replacing downloaded scripts can leave the last known-good data absent

**Strength:** Strong  
**Files:** `src/gameData/gameData.ts:271-315`

Installation verifies extraction in staging, then permanently removes
`scriptsRoot` before renaming the staged directory into place. If that rename,
the subsequent metadata write, or the process fails after removal, the previous
external source has already been discarded. The catch removes staging but
cannot restore it. A restart then starts without the previously usable game
data.

Treat publication as a small transactional module: retain the old directory as
a uniquely named sibling, rename the completed staging directory into the
published location, write metadata, then remove the backup. On failure before
commit, restore the backup. This is not a fallback language path; it protects a
single authoritative external-index input. The smaller publication interface
has leverage for both startup updates and manual update commands.

### AR-004 — The completion UI bridge contains a language-specific `case` rewrite

**Strength:** Strong  
**Files:** `src/languageClient/completionUiBridge.ts:317-343`

When the current snippet placeholder is `default` and a completion item's label
is `case`, the TypeScript bridge replaces the item text with `case ${1:value}`
and attaches a new placeholder command. That is a special case based on
Enfusion completion semantics and is not merely rendering or applying a
Rust-authored edit. It can diverge from the server's insert range, filter text,
documentation, or any future `case` completion whose label happens to match.

Make the server return the complete `case` snippet and its next-placeholder
intent as part of the same completion item that establishes the switch-arm
context. The UI bridge's interface can then be generic: execute a declared
snippet transaction, rather than recognize a language label. This moves the
language rule behind the Rust module's interface, increases leverage for every
client surface, and restores locality for testing it against parser context.

### AR-005 — Completion debug retention is unbounded by document count and is active in ordinary sessions

**Strength:** Worth exploring  
**Files:** `src/languageClient/completionUiBridge.ts:14, 683-708`

Every completion response is converted into an array of presentation records
and retained in `completionPresentationObservations`, keyed by URI. Unlike the
80-event lifecycle trace beside it, this map has no eviction or document-close
cleanup, and registration does not depend on diagnostics or development mode.
Long-lived sessions that visit many files retain the latest full completion
list for each one solely for a debug command.

Make debug observation a bounded module: retain the active document and a
small LRU of recent documents, or collect only while the debug command is
armed. Its interface need only answer the debug command's request for a recent
observation, which bounds memory and avoids putting production completion work
behind a forensic data-retention policy.

### AR-006 — Restart requests are dropped while a restart is running

**Strength:** Worth exploring  
**Files:** `src/languageClient/languageClient.ts:109-145, 370-402`; `src/languageClient/developmentServerWatchBridge.ts:10-46`

`restartLanguageClient` returns immediately when `restartingClient` is true.
The game-data source-change callback and development-binary watcher can both
request restarts, but a request arriving after the active restart has built its
server arguments is discarded. The final client can therefore retain the older
source path or binary until a further event occurs.

Replace the boolean guard with a small restart-coalescing module: record a
monotonic requested generation (and latest reason), run until the started
generation catches up, then stop. Its interface stays one `requestRestart`
operation while implementation hides overlap and preserves the latest
authoritative inputs. This removes a timing special case and improves locality
for lifecycle tests.

### AR-007 — Two declaration-extraction pipelines duplicate language facts

**Strength:** Strong
**Files:** `server/src/model.rs:303-1090`; `server/src/semantic_file.rs:146-971`; `server/src/index.rs:229-376, 381-709`; `server/examples/{symbol*,resolver_report,lsp_completion_report,lsp_signature_help_report,reference_finder*,scope_corpus_report,expression_type_corpus_report}.rs`

Production document and external-index paths build `SemanticFile` from the
parser and project it into `SymbolIndex`. The retained `SymbolCatalog` path
independently walks AST declarations and separately implements callable-form,
macro, and conditional-branch extraction. The production paths documented in
`open_documents`, `external_overlay`, and `index_build` use `SemanticFile`.
The retained `SymbolCatalog` construction sites include tests, compatibility
constructors, and developer reports, so even corpus evidence can observe a
different declaration projection from the runtime.

This is two parallel implementations of Enfusion declaration truth. A parser
edge case or preprocessor change must be fixed and verified twice, and the
legacy route can silently disagree with the runtime one. The deletion test
passes: deleting `SymbolCatalog` would concentrate its compatibility
projection at the index ingestion seam, not force callers to rediscover
declarations. Define a removal condition and migrate test/index constructors
to a single `SemanticFile`-to-index projection. If a legacy constructor must
remain temporarily, make it a thin adapter over semantic facts rather than an
independent traversal. That restores one language authority and improves
locality for corpus verification.

### AR-008 — Manual game-data cache fingerprints can treat changed source as current

**Strength:** Strong
**Files:** `server/src/index_cache.rs:112-122, 2530-2626`

The manual-source fingerprint consists only of canonical root path, aggregate
file count, aggregate byte count, and the maximum modification timestamp. Two
different script trees can have the same values: for example, replacing a
non-latest file with equal-length contents while preserving (or not exceeding)
the existing maximum timestamp. The cache then loads an external index built
from old source under a fingerprint that appears current.

Put manual source identity behind one cache-source adapter that uses an exact
manifest of relative path, byte length, and per-file modification identity (or
a durable content hash when metadata cannot be trusted). The adapter can
produce both the cache key and the user-facing validation summary, eliminating
the duplicate traversal called out in AR-001. Its interface gives cache loading
a single authoritative identity fact and makes correctness tests local.

### AR-009 — Top-level completion does full-index work before enforcing its result limit

**Strength:** Strong
**Files:** `server/src/index_query.rs:182-255`; `server/src/lsp/completion.rs:4021-4029`

Every top-level completion request scans every top-level name, performs fuzzy
matching, allocates string completion keys, groups duplicate candidates, sorts
each group, builds display data, and finally truncates to its caller's limit.
The normal LSP path invokes it for both local and external indexes. An empty
type-prefix request is explicitly allowed, making this full scan a normal
editor interaction rather than an exceptional search.

Give `SymbolIndex` a completion-candidate retrieval interface that can supply
prefix groups and a bounded ranked frontier; keep fuzzy matching/ranking inside
that module. `IndexQuery` should project only the candidates it will return.
The deletion test passes because callers would otherwise each need to learn
index traversal, shadowing, and cap behavior. This concentrates a hot-path
performance policy and preserves one ranking authority.

### AR-010 — Every workspace file event rebuilds the whole workspace index on the request path

**Strength:** Strong
**Files:** `server/src/lsp/workspace_requests.rs:26-85`; `server/src/lsp/feature_dispatch.rs:148-166`; `server/src/lsp/external_overlay.rs:117-215, 820-838`

Workspace change notifications are dispatched synchronously by the LSP feature
dispatcher. Updating one file parses that file, then `publish_workspace_change`
clones the entire retained workspace-file map and rebuilds a `SymbolIndex` from
every contribution before the notification handler returns. The TypeScript
watcher emits these notifications for ordinary saves/changes, so a large
workspace pays full aggregation cost per changed file on the server's incoming
request path.

Make workspace publication an immutable-generation module with a bounded,
coalescing background builder. The notification interface should only publish
the latest per-file contribution and generation; one builder captures a batch,
constructs the next aggregate outside the lock, and atomically publishes it if
its generation remains current. Queries retain the last immutable External
Index Snapshot until the new one is ready. This matches the documented
snapshot contract, removes editor-path whole-workspace work, and keeps
generation correctness local.

### AR-011 — External definitions synchronously reread their source file for each navigation

**Strength:** Strong
**Files:** `server/src/lsp/definition.rs:205-223`; `server/src/lsp/feature_dispatch.rs:148-166`

For every selected external declaration, `definition_link_for_candidate` calls
`fs::read_to_string` on the target before it can convert the indexed byte spans
to LSP ranges. Definition handling is synchronous on the LSP request path, so
navigation into game data or a workspace overlay can block the message loop on
disk I/O. Repeated navigation to the same target repeats that read even though
the index already owns the declaration's source identity and spans.

Give the external-index module a source-coordinate projection interface that
retains (or lazily, boundedly caches) the position data required for indexed
files. The definition module should ask it for a target URI and ranges, rather
than independently reopening a file. The deletion test passes: without this
module, every consumer of external spans must rediscover file loading and UTF-16
coordinate conversion. This keeps source-coordinate policy local and removes
avoidable foreground I/O.

### AR-012 — Hover rendering owns an unindexed hard-coded `Attribute` language contract

**Strength:** Strong
**Files:** `server/src/lsp/hover_render.rs:11-21, 966-1080`; `server/src/lsp/completion.rs:7271`; `server/examples/lsp_completion_report.rs:99`

The presentation module embeds the full `Attribute` constructor signature and
duplicates its parameter names/types again as `attribute_param_specs`. It then
parses attribute text against that private schema to manufacture hover details.
The same language fact also appears in completion fixtures and reports, while
normal callable display is derived from the indexed semantic model. This makes
one built-in behave through a renderer-only semantic path: a Workbench or game
data change can leave hover, completion fixtures, and indexed callable facts
out of agreement.

Move the exceptional built-in declaration into a single compiler-owned
language-facts module (or consume the indexed declaration when available), and
have hover rendering receive a generic callable/attribute projection. The
renderer then owns only presentation. The deletion test passes: deleting its
embedded schema removes special Enfusion interpretation from presentation
without making callers recreate it. This restores one language authority and
lets parser, completion, signature help, and hover verify the same fact.

### AR-013 — Cached LSP requests bypass the runtime-owned position index

**Strength:** Strong
**Files:** `server/src/analysis_runtime.rs:22-97`; `server/src/lsp/open_documents.rs:104-127`; `server/src/lsp.rs:1049-1103`; `server/src/lsp/completion.rs:241-251`; `server/src/lsp/hover.rs:221-245`; `server/src/lsp/definition.rs:139-157`

The foreground runtime builds and retains a `PositionIndex` for every current
document revision before semantic analysis becomes available. Yet cached
completion, hover, and definition reports accept only source text and an LSP
position, then call the standalone `offset_for_position`, which scans from the
start of the document. Their result projection also uses `range_for_span`,
which constructs another position index from the same text. The retained
`PositionIndex` itself provides byte-to-position lookup but implements its
inverse as another whole-table linear search. On ordinary cursor requests this
repeats source-wide UTF-16/line conversion despite the exact snapshot already
owning coordinate state.

Make bidirectional source-coordinate conversion a runtime-query fact:
`DocumentQuery` should expose a revision-owned coordinate projection with a
direct or line-bounded position-to-offset lookup, and report functions should
accept it rather than raw text alone. The deletion test passes because callers
would otherwise need to build or scan coordinate state independently. This
deepens the snapshot interface, removes repeated editor-path work, and
guarantees every feature applies the same CRLF/UTF-16 conversion policy.

### AR-014 — Developer report commands duplicate and disagree on Cargo execution policy

**Strength:** Strong
**Files:** `tools/ast-corpus-report.mjs:3-45`; `tools/lsp-hover-report.mjs:3-35`; `tools/lsp-signature-help-report.mjs:2-27`; `tools/lsp-workspace-overlay-report.mjs:2-28`, plus the analogous report wrappers

The report wrappers repeat the same Cargo-example launch in many files, but
the copies disagree: some honor `CARGO`, some only probe the Windows user Cargo
location; some set the repository as `cwd`, some inherit the caller's working
directory; and Windows shell selection varies. The report name is the only
meaningful difference, yet equivalent developer commands therefore have
different environment and path behavior.

Introduce one developer-only Cargo example runner whose interface accepts the
example name, display label, and forwarded arguments. Keep each report command
as a tiny executable declaration of those values. The deletion test passes:
removing the runner would force every wrapper to reconstruct Cargo discovery,
process policy, and error handling. This gives the tool layer a single,
deterministic execution policy without becoming an extension runtime
dependency.

### AR-015 — The verified auto-commit receipt does not bind the verified change set

**Strength:** Strong
**Files:** `tools/verified-refactor-auto-commit.mjs:69-107, 109-168`; `tools/verified-refactor-auto-commit.test.mjs:49-67`

`verify` records only branch, `HEAD`, title, and time after its command passes.
`stop` then runs `git add -A` and commits the entire working tree. It never
records or compares the index/worktree diff that was actually verified. Any
edit created after verification—by a person, another process, or a generated
artifact—therefore satisfies the receipt's `HEAD` check and is committed as
though it were verified. The existing test codifies the broad staging behavior
but does not exercise this post-verification mutation.

Make the receipt a change-set module: capture a deterministic staged or
working-tree patch identity during `verify`, and have `stop` reject any added,
removed, or modified path outside that identity. Alternatively, make `verify`
stage the exact reviewed paths and require an unchanged index. The deletion
test passes because without this policy owner, each auto-commit caller must
independently reason about verification freshness and staged scope. This keeps
the tool's stated guarantee local and preserves unrelated user work.

### AR-016 — Semantic-file construction scans preprocessor lines three times for overlapping facts

**Strength:** Strong
**Files:** `server/src/semantic_file.rs:134-159, 497-531, 789-922`

For every parsed file, `SemanticFile::build` constructs `DirectiveContextMap`
by walking all source lines, then `add_preprocessor_macro_definitions` walks all
lines again, then `source.lines().count()` walks them a third time solely for
telemetry. The first two passes both recognize preprocessor directives and
derive adjacent semantic facts, but use separate textual recognizers. This is
normal work for every workspace and game-data source file, not just recovery
input.

Make preprocessor extraction one compiler-owned module that produces line
contexts, macro definitions, and the line count in a single scan. `SemanticFile`
can consume that immutable fact rather than interpret directive text twice.
The deletion test passes because otherwise every semantic consumer needs to
recreate directive classification and traversal policy. This improves locality,
prevents drift between `#define` and conditional recognition, and cuts repeated
index-build work.

### AR-017 — Resolver and expression typing independently implement member lookup semantics

**Strength:** Strong
**Files:** `server/src/resolver.rs:1062-1139, 1604-1693`; `server/src/expression_type.rs:787-815, 1130-1417`

Reference resolution and expression-type inference each traverse typedef/class
owner expansion, search inherited class members, recursively locate enum
members, filter static members, and recognize the same four pseudo-members
(`ClassName`, `IsInherited`, `ToString`, and `Type`). The resolver already
imports `member_lookup_owners` from expression typing, but retains private
copies of the remaining rules. The two paths are then used together on member
access: inference determines the receiver and resolution independently decides
which declaration that receiver member names. A new member rule can therefore
make a hover/type result disagree with definition/reference navigation.

Extract an index-query module that owns member candidate discovery and its
semantic distinctions (including enum inheritance, static filtering, and
pseudo-members). Give resolver and expression typing projections suited to
their results rather than two rule sets. The deletion test passes: without this
module, both callers must reconstruct the same language lookup traversal. This
deepens the index query seam, gives language changes one authority, and keeps
feature modules focused on their respective result projections.

### AR-018 — The language-server executable silently discards invalid launch arguments

**Strength:** Strong
**Files:** `server/src/bin/reforger_language_server.rs:7-46`; `src/languageClient/languageClient.ts:430-481`

The executable's argument loop ignores every unknown flag and also silently
accepts a recognized flag when its value is missing. Those options carry the
external-index contract: `--game-data-scripts`, `--index-cache`, metadata, and
workspace roots. A spelling or version mismatch between the thin TypeScript
launcher and the bundled binary can therefore start a working stdio server
without the intended index rather than producing an actionable startup error.
The binary has no parser tests for this public invocation contract.

Make command-line admission one small module that returns either validated
`LspServerOptions` or a concise usage error. It should reject unknown flags,
require a value for value-taking flags, and have table-driven tests for the
launcher-owned option set. The deletion test passes: without a single parser,
each startup path has to rediscover argument validity. This keeps the
TypeScript-to-Rust process seam explicit and turns configuration loss into a
diagnosable failure.

### AR-019 — Signature help parses the index's display signature back into semantic parameters

**Strength:** Strong
**Files:** `server/src/index.rs:1110-1157, 1689-1711`; `server/src/index_query.rs:26-27, 435-445, 650-658`; `server/src/lsp/callable.rs:59-86, 557-641`; `server/src/lsp/signature_help.rs:137-157, 487-647`

`SymbolIndex` renders a callable into a string such as
`Owner.Run(out int value = 1) -> void`, `IndexQuery` carries that display string
in a completion candidate, and signature help then reparses it to recover
parameter names, modifiers, defaults, and result text. The re-parser contains
its own parenthesis/generic/string/default splitting logic. This is not a
presentation-only convenience: active named-parameter selection and parameter
documentation depend on the reconstructed facts. Any change in signature
rendering or a source form the re-parser does not preserve can make signature
help disagree with the indexed declaration.

Give the index-query module a typed callable projection built directly from
the callable symbol and its indexed parameter children. Signature help can use
that projection for parameter identity and render the final LSP label from the
same fact. Keep source-call argument context in `callable`, but delete its
display-signature parser from the semantic result path. The deletion test
passes: without this projection, every callable consumer must parse a display
format to regain language facts. This keeps parameter semantics local to the
index and makes rendering a one-way operation.

### AR-020 — Rust corpus reports repeat their source-discovery and invocation policy

**Strength:** Strong
**Files:** `server/examples/{lexer,parser,ast,expression,index}_corpus_report.rs` and analogous corpus/report examples

The Rust report programs independently implement the same `--scripts`/`--out`
parser, default VS Code storage location, repository-root resolution, recursive
`.c` discovery, lossy decode behavior, report-directory creation, and Markdown
write policy. The only material variation is the analysis projection and report
name. This repeats the same discovery and path semantics already duplicated at
the JavaScript wrapper layer (AR-014), so a storage-location or traversal change
must be made and verified across many tools.

Extract a developer-only corpus runner module that owns invocation, source
enumeration, decoding, and report publication, and let each example provide its
analysis projection. The deletion test passes: removing that module forces each
report to recreate the same developer environment policy. This makes report
analysis modules deeper and keeps corpus-source semantics in one place without
adding an extension runtime dependency.

### AR-021 — Parser corpus reporting embeds a path-and-text-specific recovery exception

**Strength:** Worth exploring
**Files:** `server/examples/parser_corpus_report.rs:160-169`

`expected_recovery_node_count` recognizes exactly a relative path ending in
`Game\\game.c` and source text containing `#ifdef BREAK_COMPILATION`, then
declares every error node in that file expected. This exceptional evidence is
not represented in the fixture corpus or a review-input manifest; it is hidden
inside a general parser report. A path rename or a second known malformed
source silently changes report interpretation, while the exception cannot be
reviewed alongside other corpus allowances.

Move known corpus exceptions into a small data manifest keyed by stable source
identity and an explicit reason, or report them as ordinary diagnostics until
such evidence exists. The report runner should consume that manifest rather
than recognize source prose. The deletion test passes because without the
manifest, every report must encode its own exceptional evidence. This makes the
special case reviewable and preserves the distinction between parser behavior
and corpus policy.

## Reviewed slices

### 2026-07-23 — Repository inventory, build/configuration

Reviewed `package.json`, `esbuild.js`, `eslint.config.mjs`, `tsconfig.json`,
and `language-configuration.json`. The manifest command identifiers match the
central configuration constants (including the established `sript` spelling),
so that spelling is compatibility debt rather than a broken route. The build
correctly keeps Rust server construction outside the editor bundle. No
additional architecture or performance finding was established in this slice.

### 2026-07-23 — Extension composition, configuration, diagnostics, and game data

Reviewed `src/extension.ts`, all five files in `src/extensionConfig/`,
`src/diagnostics/diagnostics.ts`, and `src/gameData/gameData.ts`. Activation is
a narrow composition module and game-data completion is correctly wired to the
language-client refresh callback. The current findings are AR-001 through
AR-003. Diagnostics have a single serialized writer and an explicitly bounded
log; no overlap requiring a recommendation was found.

### 2026-07-23 — Editor/LSP bridges and extension-host tests

Reviewed all fourteen `src/languageClient/*.ts` modules and
`src/test/extensionActivation.test.ts`. The generic input-route bridge follows
the documented Pre-Native Input Route and stale-decision contracts; its lack of
an arbitrary request timeout is intentional. Versioned editor transactions
provide a useful internal seam for the remaining post-edit block-comment
assist. Findings AR-004 through AR-006 concern the completion bridge's
language-specific rewrite, unbounded debug retention, and non-coalescing
server restarts. Workspace notifications are deliberately debounced and carry
per-file sequence numbers; their Rust-side ordering behavior remains to be
reviewed with the runtime slice.

### 2026-07-23 — Language-foundation and fixture evidence (in progress)

Reviewed the lexer, syntax tree, parser recovery paths, AST/model entry points,
preprocessor completion classifier, semantic-file construction, scope model,
symbol display, index query, resolver/type-inference member paths, and the
fixture inventory (34 Enfusion-script fixtures). The fixtures are explicitly
labelled by evidence status and cover parser recovery, lexical forms,
documentation formatting, semantic contribution ID remapping, and workspace
overlay precedence. No fixture was treated as Workbench truth merely because it
was present. AR-016 and AR-017 were established in this continuing slice; the
remaining language-foundation modules and the full report/example programs are
still under review.

### 2026-07-23 — Developer tooling execution and discovery (in progress)

Reviewed every tool wrapper's invocation shape, the verified-commit tool and
its test, the build-and-replace server tool, startup tracing, runtime-performance
reporting, and the two corpus-discovery scripts. Cargo-launch duplication is
AR-014; receipt scope is AR-015. The corpus scripts label their regular
expression/text scans as discovery-only and do not claim compiler truth. The
build tool intentionally stops only binaries resolved within this repository
before replacement. Remaining work in this slice is the report-program source
that those wrappers run and a final per-path reconciliation.

### 2026-07-23 — LSP feature projections (in progress)

Reviewed diagnostics, definition, hover, semantic tokens, on-type formatting,
callable context, signature help, collection declarations, and debug hover,
alongside their dispatch/runtime callers. Definition's override target rule is
a real semantic distinction, and the on-type assists are bounded conservative
edits, so neither is recorded as an unjustified special case. AR-019 records
the signature-help display-to-semantics reversal. Coordinate conversion in
diagnostics and navigation is covered by AR-013; external definition source
loading is covered by AR-011. Completion and the remaining protocol/runtime
test source still require the final reconciliation pass.

### 2026-07-23 — LSP protocol, document, runtime, and feature tests

Reviewed `server/src/lsp/tests.rs` and its `support`, `protocol`, `documents`,
`runtime`, and `features` inclusions. The shared harness constructs the same
runtime and transport-facing objects as production tests rather than a parallel
implementation. Coverage explicitly exercises scheduler lanes and capacity,
snapshot/revision replacement, UTF-16 and CRLF coordinate handling, deferred
request cancellation, protocol error continuation, and feature projections.
This establishes regression evidence for AR-009 through AR-013 and AR-019;
no additional test-ownership or performance finding was established.

### 2026-07-23 — Rust corpus/report program family (in progress)

Reviewed the lexer, parser, AST, expression, index, cache, LSP, resolver,
scope, symbol, and reference report families by their shared invocation,
source-acquisition, semantic-pipeline, and output contracts, with detailed
source review of representative lexer/parser/AST/index corpus programs. AR-020
records the repeated runner policy; AR-021 records the parser report's hidden
path/text-specific exception. Legacy `SymbolCatalog` use in this family is
included in AR-007. Per-report analysis projection and remaining report files
continue into the final audit.

## Coverage audit ledger

The inventory command excludes generated/dependency output and yields 192
repository-owned coding/configuration files. The buckets below are exhaustive
and disjoint; their counts sum to 192. A bucket is only marked complete when
the corresponding source review and its test/report evidence have both been
recorded above.

| Bucket | Files | Status | Review basis |
| --- | ---: | --- | --- |
| Root configuration and test runner | 5 | completed | Build/configuration slice |
| Extension, configuration, diagnostics, game-data | 7 | completed | Extension composition slice |
| Language-client bridges and extension test | 15 | completed | Editor/LSP bridge slice |
| Rust language foundations and composition root | 21 | in progress | Foundation slice and AR-007/016/017 |
| Rust LSP runtime, transport, and features | 31 | in progress | Feature and LSP-test slices |
| Rust executable | 1 | completed | AR-018 executable argument review |
| Rust report programs | 37 | in progress | Report-program family and AR-007/020/021 |
| JavaScript developer tools and tests | 40 | in progress | Tooling slice and AR-014/015 |
| Enfusion-script fixtures | 34 | in progress | Foundation fixture-evidence slice |
| Server crate manifest | 1 | completed | Build/configuration review |

Before closure, each in-progress bucket receives a final per-path comparison
against this accounting and its status is updated only if no path remains
unreviewed.
