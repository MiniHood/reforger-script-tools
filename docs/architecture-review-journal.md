# Architecture Review Journal

Status: in progress  
Started: 2026-07-23  
Scope: every repository-owned coding file: TypeScript, Rust, JavaScript tooling,
and executable configuration. Fixtures are reviewed where their structure or
coverage affects the code they evidence; generated output and dependencies are
excluded.

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
**Files:** `server/src/model.rs:303-1090`; `server/src/semantic_file.rs:146-971`; `server/src/index.rs:229-376, 381-709`

Production document and external-index paths build `SemanticFile` from the
parser and project it into `SymbolIndex`. The retained `SymbolCatalog` path
independently walks AST declarations and separately implements callable-form,
macro, and conditional-branch extraction. The production paths documented in
`open_documents`, `external_overlay`, and `index_build` use `SemanticFile`;
the remaining `SymbolCatalog` construction sites are tests and compatibility
constructors.

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
