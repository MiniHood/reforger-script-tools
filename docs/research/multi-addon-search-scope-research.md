# Multi-add-on search-scope research

Research date: 2026-08-02. This note defines the path from accepted Search
Scope Prototype C to a functional multi-add-on search scope. It is an
implementation recommendation, not product code.

## Recommendation

Replace the current `SEARCH IN` buttons and prototype switcher with Prototype
C as the only scope control. Its selectable set should be:

1. **Workspace**, pinned first and represented by the reserved scope ID
   `workspace`;
2. the non-workspace add-ons in the current searchable add-on scope, identified
   in requests by canonical uppercase GUID;
3. **Official Wiki**, separated from add-ons and present only in Text mode, as
   required by the existing mode boundary.

The initial checked set should be Workspace, Arma Reforger
(`58D0FB3206B6F859`), Enfusion Core (`5614BBCCBB55ED1C`), and the existing
mode-eligible Wiki default when those sources are available. Never create a
checked but unavailable placeholder. The two add-on defaults must be keyed by
GUID, not by mutable title text: the repository already defines the Reforger
GUID as the base-game identity and launches Workbench with both of these GUIDs
as required base add-ons
([`addon_sources.rs:28`](../../server/src/addon_sources.rs#L28),
[`workbench.rs:130`](../../server/src/workbench.rs#L130)). The accepted UI
already pins Workspace and defaults Workspace, Arma Reforger, and Enfusion Core
in Prototype C, but its add-on IDs and script counts are hard-coded and its
selection is explicitly non-functional
([`searchUiPrototype.ts:830-870`](../../src/searchPrototype/searchUiPrototype.ts#L830-L870)).

The implementation must deepen the existing Rust catalogue rather than add a
TypeScript add-on registry. TypeScript should render discovery facts, send the
selected IDs, merge the existing Workspace/add-on/Wiki query streams, and open
returned handoffs. Rust should own scope validation, cache loading, filtering,
identity, search, cursors, and source reads, consistent with the repository
router and one-authoritative-path rule
([AGENTS.md, “Router”](../../AGENTS.md#router),
[AGENTS.md, “Taste”](../../AGENTS.md#taste)).

## Authoritative selectable set

The raw cache catalogue is **not** the selectable set. It is a maintenance
index of cache manifest headers, sorted and deduplicated by `(GUID,
source-root)`; it can contain an instance that is no longer loaded
([`addon_sources.rs:1166-1299`](../../server/src/addon_sources.rs#L1166-L1299)).
The current Workbench-loaded graph is the live scope authority, and the
published inventory is explicitly the complete Workbench-owned identity/root
graph
([`localSourceInventory.ts:25-50`](../../src/gameData/localSourceInventory.ts#L25-L50),
[`architecture.md:10-149`](../architecture.md#L10-L149)).
An offline startup may instead expose the labelled provisional project-
dependency scope already supported by the language engine; it must not silently
present every compatible cache as though it were loaded
([`architecture.md:22-55`](../architecture.md#L22-L55),
[`language-engine.md:104-126`](../language-engine.md#L104-L126)).

Therefore discovery should return the intersection of:

- the current authoritative or explicitly provisional scope;
- the exact compatible cache instance selected for each `(GUID, source-root)`;
- instances that successfully loaded and contain at least one script.

Also return unavailable scope members as warnings or disabled diagnostic
entries when useful, but do not make them selectable. `LoadedAddonIndexResult`
already separates searchable instances, exact scope identities, missing
instances, and workspace exclusions, while each loaded instance carries GUID,
display ID, script count, and cache outcome
([`addon_sources.rs:53-114`](../../server/src/addon_sources.rs#L53-L114)).
The discovery projection needs to add the graph title (or a canonical display
label) without exposing `source_root`.

Workspace is a live overlay rather than an add-on cache. If a loaded instance's
root contains a workspace root, current indexing deliberately removes/skips
that packed cache to prevent duplicate or shadowed source
([`addon_sources.rs:637-675`](../../server/src/addon_sources.rs#L637-L675),
[`addon_sources.rs:1983-1997`](../../server/src/addon_sources.rs#L1983-L1997)).
The selector must mirror that rule: show Workspace, not a second cached row for
the same working add-on.

Use GUID as the public selection key only within a returned `scopeRevision`.
Internally, canonical `(GUID, source-root)` remains the durable instance
identity; Workbench selects one root per GUID and the revision invalidates
requests if that root or content changes
([`architecture.md:120-149`](../architecture.md#L120-L149),
[`addon_sources.rs:1531-1543`](../../server/src/addon_sources.rs#L1531-L1543)).

## UI behavior and select-all semantics

Prototype C becomes a normal `SEARCH SCOPE` panel: selected chips, **Edit
selected sources**, a filter box, Workspace first, then pinned defaults, then
remaining add-ons sorted by display label. Remove the A/B/C switcher,
`PROTOTYPE` badge/note, and the old All sources / Workspace / Game Data buttons.
Official Wiki remains a distinct authority and appears in this same panel only
in Text mode; Semantic mode must neither show nor query it. This preserves the
existing client rule that Semantic excludes Wiki
([`mcpSearchClient.ts:331-408`](../../src/searchPrototype/mcpSearchClient.ts#L331-L408),
[`searchUi.test.ts:121-136`](../../src/test/searchUi.test.ts#L121-L136)).

The single bulk button has deterministic two-state behavior:

- if every currently available, mode-eligible scope is selected, label it
  **Unselect all** and clear the complete set;
- otherwise label it **Select all** and select the complete set;
- filtering changes only the displayed rows, never the meaning of “all”;
- disabled/unavailable entries are excluded from both operations;
- an empty selection is valid and displays “No search scopes selected” without
  issuing a search. It must not fall back to all sources.

On discovery refresh, retain explicit selections that still exist, remove
retired IDs, and report removed selections. Apply defaults only on the first
usable discovery (or an explicit reset), not whenever the set is empty; a
`selectionTouched`/equivalent state is required so **Unselect all** stays empty.
Keep the selection across Semantic/Text mode changes, but query only the
mode-eligible subset.

## Required runtime and MCP contract

The present MCP/search process cannot support this UI: its launch arguments
contain one `--index-cache` path resolved specifically to the base-game cache,
plus independent Workspace roots
([`mcpConfiguration.ts:12-34`](../../src/mcp/mcpConfiguration.ts#L12-L34),
[`mcpSearchClient.ts:480-488`](../../src/searchPrototype/mcpSearchClient.ts#L480-L488)).
`GameDataCatalogue` likewise stores one cache path and one merged index
([`game_data_catalogue.rs:37-59`](../../server/src/game_data_catalogue.rs#L37-L59),
[`game_data_catalogue.rs:656-693`](../../server/src/game_data_catalogue.rs#L656-L693)).
The LSP already has the correct acquisition shape: add-on index storage, an
authoritative/provisional graph, exact per-instance caches, and one immutable
layered snapshot that retains originating layers
([`languageClient.ts:435-483`](../../src/languageClient/languageClient.ts#L435-L483),
[`index.rs:449-478`](../../server/src/index.rs#L449-L478)).

Implement the following contract changes:

1. **MCP startup/catalogue.** Add MCP inputs for the add-on index-storage root
   and current graph/dependency scope. Reuse the add-on loader's read-only
   selection/validation path; MCP must not independently scan installed add-on
   folders or rebuild caches. Retain per-add-on index, cache path, revision,
   GUID, display facts, and script count before composing a query view.
2. **Discovery.** Extend `game_data_status` rather than add a second overlapping
   registry tool. Return `scopeRevision`, `scopeAuthority`, and bounded
   `addons[]` entries containing `addonGuid`, display ID/title, script count,
   availability, and default/pinned hints. The existing status tool is already
   the readiness/cache discovery surface
   ([`docs/mcp-api.md:62-80`](../mcp-api.md#L62-L80),
   [`game_data_catalogue.rs:538-553`](../../server/src/game_data_catalogue.rs#L538-L553)).
3. **Search filters.** Add optional, unique `addonGuids` to
   `search_game_data_symbols` and `search_game_data_text`. Omitted means every
   add-on in this process's searchable scope for backward compatibility; an
   explicitly empty array is invalid (the UI makes no Game Data call for zero
   selected add-ons). Unknown, unloaded, duplicate, or malformed GUIDs return a
   stable invalid-scope error rather than being ignored. Workspace and Wiki
   remain separate tools/authorities.
4. **Result identity.** Add `addonGuid` and display label to every add-on search
   hit and relevant inspection result. Include the GUID in deterministic sort
   keys. Do not infer it from path: packed metadata has a GUID in
   `VirtualSourceIdentity`, but loose files do not currently have a general
   add-on identity field
   ([`model.rs:95-113`](../../server/src/model.rs#L95-L113)). Preserve add-on
   identity explicitly in indexed file metadata or in an equally authoritative
   per-file catalogue map.
5. **Handoffs and reads.** Add `addonGuid` to symbol references,
   `readSourceInput`, text hits, inspection/member references, and source-read
   requests. Bump opaque reference versions. Today semantic references and
   reads identify a file only by catalogue revision plus relative path, and
   lookup returns the first matching path; this is ambiguous as soon as two
   add-ons publish the same logical path
   ([`game_data_search.rs:623-679`](../../server/src/game_data_search.rs#L623-L679),
   [`game_data_inspection.rs:240-303`](../../server/src/game_data_inspection.rs#L240-L303),
   [`game_data_catalogue.rs:335-405`](../../server/src/game_data_catalogue.rs#L335-L405)).
6. **Cursors and caches.** Canonicalize selected GUIDs to sorted uppercase and
   bind that set plus `scopeRevision` into semantic cursors, text cursors, and
   both Rust and TypeScript page-cache keys. Current semantic cursors bind
   query/kind/owner/category but not add-on scope; text cursors bind query and
   match options only
   ([`game_data_search.rs:169-179`](../../server/src/game_data_search.rs#L169-L179),
   [`text_search.rs:204-241`](../../server/src/text_search.rs#L204-L241),
   [`mcpSearchClient.ts:557-609`](../../src/searchPrototype/mcpSearchClient.ts#L557-L609)).
7. **Client fan-out.** If Workspace is selected, issue the existing Workspace
   request once. If one or more add-ons are selected, issue one Game Data
   request carrying all selected GUIDs. In Text mode, query Wiki only when its
   scope ID is selected. Keep the existing total-result merge/pagination owner
   in the TypeScript client; do not issue one MCP request per add-on.

## Performance implications

Filtering must happen before expensive work. Semantic search currently walks
every symbol and only then applies source filters
([`game_data_search.rs:289-336`](../../server/src/game_data_search.rs#L289-L336)).
An initial correct implementation may add a cheap file/add-on membership check,
but the retained per-add-on layers should ultimately let narrow scopes iterate
only selected indexes. Preserve the existing deterministic cross-add-on order.

The full-text path is more sensitive: it currently materializes source for
every file—including a batched packed-source read—before scanning and caches
the complete result set by revision/query/options
([`game_data_catalogue.rs:132-275`](../../server/src/game_data_catalogue.rs#L132-L275)).
Build the corpus only from selected add-ons, group packed reads by each
instance's exact `symbols.bin`, and include the canonical GUID set in the
bounded cache key. A mixed-add-on batch cannot use today's one `cache_path`.
The scanner itself is cancellable and capped at 100,000 retained matches, so
report truncation rather than widening that bound
([`text_search.rs:9-15`](../../server/src/text_search.rs#L9-L15),
[`text_search.rs:139-201`](../../server/src/text_search.rs#L139-L201)).

Do not rebuild a new layered index on every checkbox click. Load immutable
per-add-on snapshots once per MCP process, capture one `scopeRevision`, and
project/filter them per request. Selection should improve narrow text-search
latency and should add only membership/sort overhead to all-add-on semantic
search.

## Migration order

1. Add explicit add-on identity to indexed files, result DTOs, references, and
   source reads; version caches/references and prove path-collision handling.
2. Give MCP read-only access to the same exact graph + cache-storage scope as
   the LSP and retain per-add-on snapshots. Keep existing one-cache startup as
   a temporary compatibility route with an explicit removal condition: remove
   it once extension launch, generated MCP configuration, and integration tests
   all supply multi-add-on scope.
3. Extend `game_data_status` discovery and add `addonGuids` filtering to both
   Game Data searches, including cursor/cache identity and per-add-on text
   source acquisition.
4. Update the TypeScript client to discover scope, normalize selections, send
   filters, merge sources, and log scope facts while the old source buttons
   still provide a verification path.
5. Promote Prototype C, implement bulk selection/defaults/empty state, then
   remove the old buttons, prototype variants, hard-coded add-ons, and
   compatibility UI state in the same verified slice.
6. Measure one-, default-three-, and all-add-on semantic/text searches; optimize
   selected-layer iteration only from those measurements. Regenerate MCP docs
   when the public schemas change, following the documentation lifecycle
   ([`docs/README.md`, “Documentation Lifecycle”](../README.md#documentation-lifecycle)).

## Tests and diagnostics

Rust unit tests should cover canonical GUID validation, omitted/all and
multi-add-on filters, unknown/unloaded GUID errors, deterministic ordering,
scope-bound semantic/text cursors, cache-key separation, same-relative-path
collisions, exact source reads, per-cache packed reads, workspace-cache
exclusion, stale graph/cache rejection, zero-script omission, and cancellation.
Add-on loader tests already exercise layered identity and cache selection; add
MCP catalogue cases beside them rather than inventing fixtures in release
paths.

MCP stdio tests should assert discovery/output schemas, filter forwarding,
revision-bound handoffs, stable errors, and copy-ready search-to-read behavior.
TypeScript tests should cover discovery normalization, pinned/default order,
partial/all/empty toggle transitions, filter-independent bulk selection,
selection retention across refresh/mode changes, no request for empty scope,
one combined add-on request, Wiki mode gating, cache-key identity, totals, and
stale-response suppression. The current tests only prove the hard-coded
prototype is rendered
([`searchUi.test.ts:138-151`](../../src/test/searchUi.test.ts#L138-L151)).

Extend Ctrl+F3 with `scopeRevision`, authority, available and unavailable scope
IDs, selected and mode-eligible IDs, removed selections, per-add-on result
counts, per-add-on source-read failures/timings, cache hits, and total
discovery/search/merge/render durations. Do not log source roots or source text;
the existing indexing diagnostic contract intentionally excludes both
([`language-engine.md:171-181`](../language-engine.md#L171-L181)).

## Principal risks and acceptance conditions

- **Path/symbol collisions:** GUID-qualified results, opaque references, and
  reads must select the exact add-on even when logical paths and declarations
  collide.
- **Stale or unloaded caches:** discovery must be graph/dependency-scope driven,
  cache-compatible, revision-labelled, and explicit about missing instances;
  never substitute an arbitrary cached root.
- **Workspace duplication:** the corresponding packed instance must remain
  excluded while Workspace supplies live scripts.
- **Selection drift:** a scope refresh may remove choices but cannot silently
  select replacements or undo an intentional empty selection.
- **Text-search cost:** selected add-ons must be applied before source
  acquisition; all-add-on scans remain explicit, cancellable, bounded, and
  measured.
- **API complexity:** keep one optional `addonGuids` field on each Game Data
  search and extend the existing status/read contracts. Do not create one tool,
  catalogue manager, or MCP process per add-on.

Acceptance requires that the displayed scope, search corpus, result add-on,
cursor, preview, and opened source all agree for one immutable revision. If a
checkbox changes only UI totals, or a hit can open another add-on's same-named
path, the feature is not complete.
