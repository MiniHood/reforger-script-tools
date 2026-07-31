# Language Engine

`server/` is the repository's Enfusion language authority. It turns Enfusion
source, workspace scripts, and resolved game data into facts consumed by LSP
and MCP projections. The TypeScript extension transports those facts and
applies editor-only behaviour; it does not duplicate parsing or semantic
decisions.

PAC1 archive inspection is also Rust-owned. Its pack module builds a bounded
logical file catalogue and reads only caller-selected entries. The add-on
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

The base-game layer is published from the current Workbench graph. On an
offline warm start, the dependency scope uses each exact `(GUID, source-root)`
instance key to locate its self-describing `symbols.bin` directly. The binary
header establishes format compatibility and embedded add-on identity without a
separate manifest read. The hydrated layer is reference-counted at construction
and published without cloning its symbol graph. Cache hydration also restores
the locator-rich packed-source registry from `manifest.json`, so hover and
definition navigation can open a `reforger-pak:` source before Workbench
connects; reading a source still validates its PAC artifacts at navigation time.
The later authoritative Workbench graph validates the exact packed and loose
sources and atomically replaces only changed instances. Unchanged validation
inspects bounded PAC catalogues and hashes only selected compressed script
payloads, not the full multi-gigabyte archives. The resulting strong revision
identity detects same-size script changes. A changed artifact decodes only
selected `.c` entries. The semantic cache and locator-rich manifest are written
directly at the exact loaded-instance root as one current pair (`symbols.bin`
and `manifest.json`), with a compact `manifest-header.json` companion that is
digest-bound to the locator manifest for warm validation. Legacy cache roots
without that companion fall back to the full manifest shape. Retired
revision/pointer layouts are discarded and rebuilt rather than read as a
compatibility path. Cache roots not named by the current Workbench graph are
also removed before indexing. A loaded add-on whose graph source root contains
a workspace root is supplied only through the live workspace layer; any packed
cache for that exact instance is removed. If the authoritative Workbench graph
is unavailable, malformed, cancelled, or fails to acquire an instance, the
Workbench-sourced layer is unavailable; the engine never reuses an earlier graph
or substitutes a local source.

The extension's explicit `loaded` startup path provides a provisional dependency
scope derived from the opened project's `.gproj` dependency GUIDs. Rust also
uses the authoritative source roots recorded by cached add-on manifests to
locate each cached add-on's root `.gproj`, then follows the transitive
descriptor closure (including installed dependencies such as `core` referenced
by `ArmaReforger.gproj`). It always includes the base-game GUID, resolves
matching cache instances by GUID, and prefers an unpacked source-root cache over
a packed-only duplicate. It hydrates compatible cached indexes only; it does
not scan arbitrary filesystem locations or inspect/build source before
Workbench is available. The offline cache is the first usable source, and a
later Workbench graph is the authoritative reconciliation that validates and
builds missing or changed source roots. The result is explicitly labelled
`project-dependencies-provisional`, is not a live Workbench graph, and is
replaced by the next authoritative graph publication.

The binary payload persists canonical public symbol facts, source metadata, and
source line starts; dense symbol IDs and lookup maps are structural or derived
runtime facts and are not duplicated in the payload. Cache-sized integers use
fixed-width `u32` records, symbol option/list presence shares one flag word,
and line starts use fixed-width positive deltas. Cold indexing builds the final
runtime-cache projection directly and encodes that index without constructing a
second cache object graph. Warm hydration decodes directly into runtime records
and builds lookup maps once, so neither path materializes an intermediate copy
of every symbol.

An empty add-on cache store is a cold build. Instances without a compatible
current cache pair are omitted from optimistic delivery and joined by the
validation build. After authoritative inspection has measured each loaded
instance's script count, at most four workers rebuild the largest sets first.
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
