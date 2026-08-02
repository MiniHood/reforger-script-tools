# Language Engine

`server/` is the repository's Enfusion language authority. It turns Enfusion
source, workspace scripts, and resolved game data into facts consumed by LSP
and MCP projections. The TypeScript extension transports those facts and
applies editor-only behaviour; it does not duplicate parsing or semantic
decisions.

PAC1 archive inspection is also Rust-owned. Its pack module builds a bounded
logical file catalogue and reads only caller-selected entries, either from an
inspection or from previously validated locators. The add-on
source module receives the Workbench-loaded graph, reads loose source only
from each add-on's top-level `Scripts` directory, and composes that
mechanism with canonical `(GUID, source-root)` instance identity, pack-set and
loose-source fingerprints, one persisted current cache revision per instance,
and virtual source
reads. The pack module itself does not infer add-on identity, load order, cache
keys, or indexing policy.

## Contract

The engine accepts source snapshots and external source layers, then projects
shared facts into diagnostics, formatting, symbols, completion, hover,
definition, signature help, and semantic tokens. A feature should use a shared
layer whenever the fact can serve more than one feature; adapters only project
those facts to a client protocol.

```text
source text
  -> lexer and parser
  -> syntax, scopes, and symbols
  -> resolver and type facts
  -> LSP or MCP feature projection
```

Semantic-token classification is a language contract, not a colour palette.
Rust identifies language roles; VS Code settings and the TypeScript shell own
their presentation, including hover rendering where editor token colours are
not available.
The fixed collection type names `array`, `set`, and `map` retain their class
role in type positions even when no external index is available; other
source-backed class names still require indexed facts.

## Snapshot Rules

Open documents are immutable, revisioned snapshots. The analysis runtime owns
their admission, cancellation, and publication. A request may use local facts
only when they match the snapshot it captured.

Workspace and game-data indexes are separate immutable external layers. The
game-data layer retains its per-loaded-add-on indexes and composes only their
stable lookup identities; it never eagerly copies all symbol records into a
second merged index. A request captures one layer generation, so background
indexing cannot change the meaning of an in-flight response. Do not introduce
per-feature revision tables or mutable shared feature state that bypasses this
model.

Workspace-file notifications build one compiler-owned contribution on the
incoming path and enqueue it for a coalescing workspace-generation worker.
Multiple changes to the same batch are aggregated once, then published as one
new immutable generation and external-index event. Request handling never
rebuilds the workspace-wide index synchronously.

The base-game layer is published from the current Workbench graph. On an
offline warm start, the dependency scope uses each exact `(GUID, source-root)`
instance key to locate its self-describing `symbols.bin` directly. The binary
header establishes format compatibility and embedded add-on identity without a
separate manifest read. The hydrated layer is reference-counted at construction
and published without cloning its symbol graph. Cache hydration registers the
cache's source identity without reading the locator-rich `manifest.json`;
hover and definition navigation lazily materialize that registry on the first
`reforger-pak:` source read, which still validates its PAC artifacts at
navigation time.
Explicit Game Data full-text search uses that same immutable locator registry
as one batch: it validates each referenced source revision once, groups entries
by PAC archive, opens each archive once without reparsing its catalogue, and
reads selected scripts in archive offset order before scanning. Entry bounds,
archive identity, expansion limits, and the captured compressed-payload digest
remain verified at read time. Its bounded result statistics report source-read
wall time separately from scan time, along with per-add-on read time and failures.
After the first full-text query, the catalogue may retain one decoded source
corpus for the exact catalogue revision and selected add-on GUIDs. Retention is
limited to 64 MiB including source identity metadata; another scope replaces
the slot, and an oversized corpus is not retained. Later distinct queries and
their pages may share those immutable strings. The cache is filled only by an
explicit text query, so it adds no startup work and never retains a PAC archive
or an unselected payload.
Individual navigation remains a one-entry lazy
read through the same validated-locator path. The matcher uses case-insensitive
literal substrings by default and
accepts explicit match-case, whole-word, and regular-expression options. Those
options and the selected add-on GUIDs are part of both the bounded result-cache
identity and opaque paging cursor, so pages from different matching modes or
scopes cannot be combined. Search exposes at most 10,000 results. Text search
stops after detecting match 10,001; semantic search must examine every symbol
to preserve ranking, but selects and sorts only the best 10,000 candidates.
Both result shapes report truncation rather than presenting the bound as an
exact total. Semantic Game Data search follows the same scope
identity: candidates are filtered by loaded add-on GUID before ranking, opaque
symbol references include that GUID, and Game Data source reads require the
returned GUID for every handoff so path uniqueness is never assumed. Layered
queries iterate the immutable child
indexes directly rather than relying on an eagerly flattened symbol allocation.
Within the same semantic match-quality tier, original declarations rank before
`modded` or `override` declarations and before members declared inside those
overlays. Remaining ties use the stable symbol identity and source ordering.
The later authoritative Workbench graph validates the exact packed and loose
sources and atomically replaces only changed instances. Unchanged validation
inspects bounded PAC catalogues and hashes only selected compressed script
payloads, not the full multi-gigabyte archives. The resulting strong revision
identity detects same-size script changes. A changed artifact decodes only
selected `.c` entries. The semantic cache and locator data are written directly
at the exact loaded-instance root. `symbols.bin` contains a required
semantic-index section and an optional binary locator section; the latter is
read lazily when a packed source URI is first materialized. The locator section
omits derived URIs, interns repeated pack paths, and stores payload digests in
raw form. The full `manifest.json` remains available for repair/debug, while
`manifest-header.json` is the compact warm-validation record. Legacy cache roots
without the binary locator section fall back to the full manifest shape. Retired
revision/pointer layouts are discarded and rebuilt rather than read as a
compatibility path. Cache roots not named by the current Workbench graph are
also removed before indexing. A loaded add-on whose graph source root contains
a workspace root is supplied only through the live workspace layer; any packed
cache for that exact instance is removed. If the authoritative Workbench graph
is unavailable, malformed, cancelled, or fails to acquire an instance, the
Workbench-sourced layer is unavailable; the engine never reuses an earlier graph
or substitutes a local source.

The cache root also maintains a compact `cache-catalogue.json` for exact
instance selection. Enumerating cache roots is restricted to catalogue
recovery and maintenance; the normal dependency warm path does not inspect
every cache directory.

The extension's explicit `loaded` startup path provides a provisional dependency
scope derived from the opened project's `.gproj` dependency GUIDs. Rust uses
the Workbench project-list registry (`.projectList_app1874910_*`) as the offline
source catalogue, follows the transitive dependency closure, and uses the same
Workbench-owned game-install discovery for installed base projects. It always
includes the base-game GUID, resolves one source candidate per GUID, and
prefers an unpacked source root over a packed-only duplicate. The offline cycle
first loads compatible cached indexes by exact `(GUID, source-root)` identity.
If the scope has no usable cache entries, it builds the selected dependency
sources before publishing the first semantic layer; this is the cold/offline
fallback rather than a second warm path. A valid warm scope does not scan or
inspect source roots. The result is explicitly labelled
`project-dependencies-provisional`, is not a live Workbench graph, and is
replaced by the next authoritative graph publication. That second cycle
compares the live loaded add-on instances, performs a delta load/build for
different or missing roots, and validates changed sources before replacing the
warm generation. If the graph has the same canonical `(GUID, source-root)`
sequence as the warm scope, the existing immutable layer remains published and
graph scope authority is promoted without decoding or composing a replacement
snapshot. Source validation retains the warm snapshot if validation fails.

The binary payload persists canonical public symbol facts, source metadata, and
source line starts; dense symbol IDs and lookup maps are structural or derived
runtime facts and are not duplicated in the payload. Cache-sized integers use
fixed-width `u32` records, symbol option/list presence shares one flag word,
and line starts use fixed-width positive deltas. Cold indexing builds the final
runtime-cache projection directly and encodes that index without constructing a
second cache object graph. Warm hydration decodes directly into runtime records
and builds lookup maps once, so neither path materializes an intermediate copy
of every symbol.

An empty add-on cache store is an offline cold build. Instances without a
compatible current cache pair are built from the selected project-list sources
before the first offline layer is published; the later Workbench reconciliation
can then replace only instances whose live source identity differs. After
authoritative inspection has measured each loaded instance's script count, at
most four workers rebuild the largest sets first.
Within a sufficiently large add-on, source parsing and semantic modelling use
that add-on's bounded share of the same logical-CPU budget; small add-ons keep
the lower-overhead sequential path. The outer worker count and each inner
source-build share multiply to no more than the available logical CPUs, so a
large base-game add-on can use idle cores without an unbounded nested pool.
Completed instances are restored to canonical graph order before immutable
layering and publication. Authoritative source inspection is bounded to four
independent workers. A compact manifest header validates cache format, exact
instance, strong revision, archive facts, and cache bytes before the
locator-rich manifest is materialized and published for a rebuild. A valid warm
hit does not rewrite that already-validated manifest.

Packed files carry a typed virtual-source identity in semantic metadata rather
than overloading a filesystem path. The identity includes the loaded add-on
instance (GUID and canonical source root through its revision), semantic
revision, logical script path, and URI. Definition and hover links preserve
that identity so navigation opens the indexed source rather than the requesting
document. Class references navigate to the original non-`modded` declaration;
the effective `modded` overlay remains authoritative for semantic lookup,
completion, and member behavior. Definition serving retains immutable revision
registries. Indexing and definition serving hash and decode each entry from the
same captured compressed bytes, so a concurrent pack update cannot apply an
old symbol span to newer bytes.

The language-server diagnostic log records each external-index startup as a
bounded performance trace: end-to-end game-data, workspace, and publication
durations; graph-read, strong source-inspection, cache-load-or-build, and
layer-composition durations; and one record per non-workspace
loaded add-on. An add-on record identifies its cache outcome and rebuild
reason, source and cache sizes, and inspection, cache read/decode/validation,
runtime-map reconstruction, rebuild, and cache-write durations. Rebuilt
add-ons further split source rebuilding into file discovery, source acquisition,
read/decode, parsing, semantic modelling, and index aggregation, and separately
record runtime-cache preparation before encoding/writing and metadata
publication after it. It contains no
source text or filesystem paths, so a fresh and warm start can be compared
from the same diagnostic-log stream.

## Boundaries and Evidence

The engine owns language behaviour, not VS Code UI, extension settings, or
game-data downloads. Enfusion behaviour follows the evidence hierarchy in the
[system overview](overview.md): Workbench/compiler behaviour first, then
official documentation, verified extracted data, and labelled examples.

Typing assists and completion edits are Rust-authored structural decisions;
the TypeScript client can screen impossible editor states, apply a versioned
result, and use an explicit native fallback. It must not infer Enfusion source
shape or recreate an edit after the fact. The enduring editor-input ownership
policy is in [Key input routing](key-input-routing.md).

## Change Rules

- Put new language behaviour in the shared layer that owns its facts.
- Preserve one language model across LSP and MCP projections.
- Keep request results bounded and revision-correct rather than guessing from
  stale or partial text.
- Keep hover payloads valid and bounded at complete presentation entries; when
  a repeated member summary exceeds its budget, report the exact omitted count
  instead of allowing the client to truncate markup.
- Keep presentation, storage, configuration, and process lifecycle outside the
  engine.

Code and tests define exact parsing recovery, ranking, token classes, limits,
and editor transactions. This document preserves why those details share one
language authority and where changes belong.
