# Language Engine

`server/` is the repository's Enfusion language authority. It turns Enfusion
source, workspace scripts, and resolved game data into facts consumed by LSP
and MCP projections. The TypeScript extension transports those facts and
applies editor-only behaviour; it does not duplicate parsing or semantic
decisions.

PAC1 archive inspection is also Rust-owned. Its pack module builds a bounded
logical file catalogue and reads only caller-selected entries. The add-on
source module receives the Workbench-loaded graph, composes that
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

The base-game layer is published only after its complete GUID-scoped cache has
loaded or rebuilt. Unchanged startup inspects bounded PAC catalogues and hashes
only the selected compressed script payloads, not the full multi-gigabyte
archives. The resulting strong revision identity detects same-size script
changes. A changed artifact decodes only selected `.c` entries. The semantic
cache and locator-rich manifest are written beneath a new immutable revision
before an atomic current-pointer publication, after which stale persisted
revisions are removed. A loaded add-on whose graph source root contains a
workspace root is supplied only through the live workspace layer; any packed
cache for that exact instance is removed. If the authoritative Workbench
graph is unavailable, malformed, cancelled, or fails to acquire an instance,
the Workbench-sourced layer is unavailable; the engine never reuses an earlier
graph or substitutes a local source.

Packed files carry a typed virtual-source identity in semantic metadata rather
than overloading a filesystem path. The identity includes the loaded add-on
instance (GUID and canonical source root through its revision), semantic
revision, logical script path, and URI. Definition serving retains
immutable revision registries. Indexing and definition serving hash and decode
each entry from the same captured compressed bytes, so a concurrent pack
update cannot apply an old symbol span to newer bytes.

The language-server diagnostic log records each external-index startup as a
bounded performance trace: end-to-end game-data, workspace, and publication
durations; graph-read and layer-composition durations; and one record per non-workspace
loaded add-on. An add-on record identifies its cache outcome and rebuild
reason, source and cache sizes, and inspection, cache read/decode/validation,
runtime-map reconstruction, rebuild, and cache-write durations. It contains no
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
