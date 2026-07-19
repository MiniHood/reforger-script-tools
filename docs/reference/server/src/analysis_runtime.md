# `server/src/analysis_runtime.rs`

## Purpose

Owns revision-safe document snapshots and the language engine's bounded task
admission contracts outside the LSP protocol layer.

## Ownership

`AnalysisRuntime` owns accepted document versions, immutable UTF-16 position
indexes, revision allocation, close-safe removal, task identity, latest-wins
cancellation, priority lanes, and retained job/byte limits. LSP translates
messages into runtime operations and publishes only matching results.

## Current Behavior

Each accepted edit creates one immutable text `DocumentSnapshot` and cancels every
retained task for that URI. Its UTF-16 position index is installed only by the
matching foreground task, so requests before foreground installation use their
documented lexical fallback. `TaskAdmission` retains at most one task per URI
and lane; replacement cancels the previous
task, and publication succeeds only for its exact identity. `QueryQuality`
distinguishes exact/recovery local facts from deterministic unavailable
fallbacks, which may not use stale local state.

Completion reports carry that enum instead of reconstructing quality from LSP
cache state. `Unavailable` logs a stable reason and returns only independently
proven lexical/top-level candidates. A bounded exact argument-label query is
available only for one bare resolver-proven function or method in valid current text;
every other pending argument form remains unavailable.

Rich token refinement and developer debug captures are admitted through
`TaskClass::Rich`. This runtime admission is the sole retained job and snapshot
byte capacity boundary for those jobs. The LSP's one `RuntimeWorkExecutor`
owns one shared pending-work map and a CPU-aware fixed worker capacity. It
always reserves one foreground slot. When `available_parallelism` reports at
least two logical CPUs, it also starts one background slot; on a one-CPU host,
it starts no competing background worker and the foreground slot advances
semantic/rich work only while no foreground work is runnable. It coalesces by
`(TaskClass, URI)`, dispatches ready semantic work before ready rich work, and
does not create a separate rich-token worker or a per-request debug thread.
At executor capacity, an incoming class may evict only queued work of equal or
lower priority: foreground evicts rich then semantic work first, semantic
evicts rich before semantic work, and rich is dropped when no rich work remains.
Its executors receive the
runtime-owned cancellation token and must return the exact task identity before
the LSP can publish a token-cache result or answer a capture request. A newer
edit, close, or replacement rich job makes that identity ineligible; debug
callers receive `Content modified` rather than a report from an obsolete
snapshot. Document caches may only mirror the token for cooperative
cancellation.

## Verification

Run `cargo test analysis_runtime --lib` and the full Rust suite from `server/`.
Cover UTF-16/CRLF conversion, stale versions, delayed close, cancellation,
priority, overload, and stale-publication rejection.
