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

Each accepted edit creates one immutable `DocumentSnapshot` and cancels every
retained task for that URI. `TaskAdmission` retains at most one task per URI
and lane; replacement cancels the previous
task, and publication succeeds only for its exact identity. `QueryQuality`
distinguishes exact/recovery local facts from deterministic unavailable
fallbacks, which may not use stale local state.

Rich token refinement is admitted through `TaskClass::Rich`. Its executor
receives the runtime-owned cancellation token and must return the exact task
identity before the LSP can publish a token-cache result; document caches may
only mirror that token for cooperative cancellation.

## Verification

Run `cargo test analysis_runtime --lib` and the full Rust suite from `server/`.
Cover UTF-16/CRLF conversion, stale versions, delayed close, cancellation,
priority, overload, and stale-publication rejection.
