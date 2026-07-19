# `server/src/lsp/external_overlay.rs`

## Purpose

Maintains the runtime workspace and game-data symbol layers consumed by LSP
feature projection.

## Ownership

Owns background construction, replacement, snapshots, generation identity,
status summaries, game-data cache use, workspace path normalization, and
workspace-file update/delete application. It does not own LSP dispatch,
open-document analysis, TypeScript watching, feature presentation, semantic
`modded` merging, or persisted workspace caches.

## Current Behavior

The overlay builds immutable workspace and game-data indexes off the request
path, then publishes short-lock `Arc` snapshots. Workspace facts take precedence
over game-data facts. A monotonically advancing external generation lets feature
caches distinguish computations made before or after an overlay update.

Workspace indexing follows configured roots, recursively collects physical
script files, rejects directory links, and uses lexical normalized absolute
paths as identity. Change and deletion notifications carry per-path sequences;
older events cannot resurrect or roll back newer workspace state. Status and
phase markers make cache/index startup failures observable without moving
indexing work into the client.

Each workspace-file entry retains the validated, versioned public
`FileContribution` that admitted it. Its `SymbolIndex` is a query projection
from the same compiler-owned semantic file, rather than the publication
contract.

## Dependencies and Boundaries

Depends on parser/AST/model/index/index-cache and standard synchronization.
The TypeScript language client resolves workspace paths and emits watcher
events; it must not build competing semantic indexes.

## Verification

Run focused overlay tests and `cargo test` from `server/`. Cover workspace
precedence, update/delete ordering, normalized path identity, linked
directories, game-data cache phases, generation invalidation, and snapshot
safety.

## Future Direction

Add source-backed `modded` behavior and persistent workspace caches only after
their semantics and invalidation rules are independently verified.
