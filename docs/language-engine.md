# Language Engine

`server/` is the repository's Enfusion language authority. It turns Enfusion
source, workspace scripts, and resolved game data into facts consumed by LSP
and MCP projections. The TypeScript extension transports those facts and
applies editor-only behaviour; it does not duplicate parsing or semantic
decisions.

PAC1 archive inspection is also Rust-owned. Its pack module builds a bounded
logical file catalogue and reads only caller-selected entries. The add-on
source module composes that mechanism with GUID identity, pack-set
fingerprints, one durable cache per add-on, and virtual source reads. The pack
module itself does not infer add-on identity, load order, cache keys, or
indexing policy.

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

Workspace and game-data indexes are separate immutable external layers. A
request captures one layer generation, so background indexing cannot change
the meaning of an in-flight response. Do not introduce per-feature revision
tables or mutable shared feature state that bypasses this model.

The base-game layer is published only after its complete GUID-scoped cache has
loaded or rebuilt. Unchanged startup inspects bounded PAC catalogues and hashes
only the selected compressed script payloads, not the full multi-gigabyte
archives. The resulting strong revision identity detects same-size script
changes. A changed artifact decodes only selected `.c` entries. The semantic
cache and locator-rich manifest are written beneath a new immutable revision
before an atomic current-pointer publication; failed or cancelled rebuilds do
not replace the previously complete cache.

Packed files carry a typed virtual-source identity in semantic metadata rather
than overloading a filesystem path. The identity includes add-on GUID,
semantic revision, logical script path, and URI. Definition serving retains
immutable revision registries. Indexing and definition serving hash and decode
each entry from the same captured compressed bytes, so a concurrent pack
update cannot apply an old symbol span to newer bytes.

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
