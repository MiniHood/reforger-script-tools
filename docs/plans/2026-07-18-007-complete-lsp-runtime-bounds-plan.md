---
title: Complete LSP Runtime Bounds - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: lsp-review-2026-07-18
execution: code
---

# Complete LSP Runtime Bounds - Plan

## Goal Capsule

| Field | Plan |
| --- | --- |
| Objective | Complete the unfinished runtime-bound guarantees identified after `1fdae54`: latest-wins rich tokens, bounded rich work, and short-lock workspace publication. |
| Product authority | The post-change LSP code review and the preceding runtime-safety plan. |
| Non-goal | Do not change Enfusion semantics, add incremental parsing, or broaden LSP feature scope. |
| Primary risk | A scheduling or publication redesign can silently leave stale coloring, lose watcher changes, or regress workspace-over-game-data precedence. |

## Product Contract

### Requirements

- R1. Each document has at most one eligible pending rich-token projection; a newer revision or request replaces older pending work rather than being dropped.
- R2. Rich semantic-token computation has a fixed, explicit worker bound. Slow projections cannot create an unbounded number of live threads.
- R3. The rich-token path keeps fast-first responses, revision/external-generation validation, cancellation, and refresh behavior intact.
- R4. Workspace update, delete, and startup publication must build aggregates outside the external-overlay mutex.
- R5. Publication must be generation-checked: it may publish only a coherent snapshot derived from the latest file map, or retry from a newer snapshot.
- R6. Startup baseline plus live updates/deletion tombstones must still converge on the newest workspace state.
- R7. Workspace symbols must retain priority over game-data symbols, and readers must observe an old or new complete snapshot, never partial state.
- R8. Source reference documentation must describe the actual snapshot API and runtime scheduling behavior.

### Scope Boundaries

- In scope: `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, `server/src/lsp/external_overlay.rs`, focused Rust tests, and matching `docs/reference/` pages.
- Out of scope: new LSP features, incremental parsing/indexing, semantic `modded` merge behavior, TypeScript client changes, Workbench integration, new runtime dependencies, and unrelated Clippy cleanup.

### Acceptance Examples

- AE1. Rapid token requests for one document retain only its newest revision; the newest request is never discarded because another job occupies a queue slot.
- AE2. Long-running rich projections do not grow the number of rich-token threads beyond the configured worker count.
- AE3. A workspace update on a large workspace does not retain the overlay mutex while `SymbolIndex::merged` runs.
- AE4. A concurrent workspace update/delete cannot be overwritten by an older aggregate publication.
- AE5. Feature queries see a coherent `Arc` snapshot and preserve workspace-over-game-data results.

## Planning Contract

### Key Technical Decisions

- KTD1. Replace the one-slot `SyncSender` queue with a server-owned scheduler state that coalesces pending jobs by document URI and revision. Replacing pending work is required; rejecting newest work is not an acceptable overload policy.
- KTD2. Capture immutable source analysis and external-index `Arc` snapshots when scheduling. A fixed scheduler/worker boundary performs idle delay, cancellation checks, and rich projection without creating a thread per due event.
- KTD3. Scheduler capacity is explicit and bounded. If it must evict a pending document, it emits a matching skipped/cancelled event so the main loop clears only that job's pending marker and a later token request can retry.
- KTD4. Represent workspace indexed files as cheap immutable shared values so a workspace-file map can be snapshotted under the mutex and merged outside it.
- KTD5. Use a workspace-map generation as a compare-and-publish guard. A writer captures a map plus generation, builds aggregate/summary outside the lock, and swaps only if the generation remains current; otherwise it retries from a newer snapshot.
- KTD6. Preserve the startup live-change overlay until startup baseline publication succeeds, then discard startup-only tombstone bookkeeping. This remains separate from steady-state workspace publication.

### Runtime Shape

```text
semantic token request
  -> fast response + immutable job snapshot
  -> replaceable per-URI pending scheduler state
  -> one bounded rich worker
  -> revision/generation-validated ready event
  -> cache + refresh

workspace watcher/startup
  -> short-lock file-map snapshot + generation
  -> aggregate merge outside lock
  -> generation-checked atomic publication or retry
  -> immutable reader snapshots
```

### Risks And Mitigations

- A per-URI replacement map can starve less-active documents. Bound capacity, make eviction explicit in logs/tests, and preserve retry on a subsequent request.
- A single worker can delay rich refinement under multi-file load. Keep fast tokens immediate and collect queue-delay timing before considering a larger fixed pool.
- A generation retry loop can churn under continuous saves. Avoid spinning while holding locks; bound retries through event-loop rescheduling and verify progress with stress tests.
- Shared indexed-file ownership can accidentally change source priority or aggregate ordering. Preserve the current ordered `BTreeMap` path and existing overlay behavior tests.

## Implementation Units

### U1. Implement Latest-Wins Bounded Rich Scheduling

- **Goal:** Replace queue-full request dropping and per-due worker spawning with coalesced pending work and a fixed rich-token worker boundary.
- **Requirements:** R1-R3; AE1-AE2
- **Dependencies:** None
- **Files:** `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, `server/src/lsp/semantic_tokens.rs`, `docs/reference/server/src/lsp.md`, `docs/reference/server/src/lsp/open_documents.md`, `docs/reference/server/src/lsp/semantic_tokens.md`
- **Patterns to follow:** Keep the existing fast projection, `SemanticTokenCache` revision/generation key, cancellation token checks, and internal ready/refresh event behavior.
- **Approach:** Introduce a server-owned coalescing scheduler that replaces pending work for the same URI and retains bounded pending jobs across documents. Its fixed worker executes the idle-delay and rich projection from immutable job snapshots. It must report eviction/cancellation with enough identity for `SemanticTokenCache` to clear only the matching pending state.
- **Test scenarios:** Repeated same-revision requests produce one job; a later revision replaces older pending work; a full pending set evicts deterministically and leaves the affected document retryable; long projection plus rapid requests never exceeds the worker bound; stale revision/external generation cannot cache; normal fast-to-rich refresh still succeeds.
- **Verification:** Targeted scheduler/state tests with test-only job/worker instrumentation, semantic-token smoke tests, and `cargo test --manifest-path server/Cargo.toml`.

### U2. Publish Workspace Aggregates Outside The Lock

- **Goal:** Remove `SymbolIndex::merged` and workspace-summary computation from mutex-held update, delete, and startup publication paths.
- **Requirements:** R4-R7; AE3-AE5
- **Dependencies:** U1
- **Files:** `server/src/lsp/external_overlay.rs`, `server/src/lsp.rs`, `docs/reference/server/src/lsp/external_overlay.md`, `docs/reference/server/src/lsp.md`
- **Patterns to follow:** Preserve the current `ExternalIndexSnapshot` reader API, `Arc<SymbolIndex>` layers, workspace priority, and startup live-change/tombstone contract.
- **Approach:** Make indexed workspace-file entries immutable/shared, capture the file map and workspace generation under a short lock, build aggregate index/summary outside the lock, then compare generation and atomically publish or retry. Apply the same protocol to startup baseline merging so a watcher change/delete remains authoritative.
- **Test scenarios:** Update/delete while a query holds an old snapshot; update during startup wins; delete during startup remains deleted; competing updates publish the last revision; readers see only complete old/new aggregates; workspace symbols still outrank game data; a large synthetic file map demonstrates merge occurs after lock release through test-only synchronization hooks.
- **Verification:** Focused external-overlay concurrency/interleaving tests, existing workspace-overlay LSP test/report, and full Rust tests.

### U3. Correct Runtime Reference Drift

- **Goal:** Make source-owner documentation match the completed runtime implementation.
- **Requirements:** R8
- **Dependencies:** U1-U2
- **Files:** `docs/reference/server/src/lsp.md`, `docs/reference/server/src/lsp/external_overlay.md`, `docs/reference/server/src/lsp/open_documents.md`, `docs/reference/server/src/lsp/semantic_tokens.md`
- **Approach:** Replace stale `with_indexes` wording with the actual snapshot API, document latest-wins bounded scheduling and short-lock publication, and keep future-performance work clearly deferred.
- **Test scenarios:** Reference pages name only current APIs and agree with the runtime paths.
- **Verification:** Manual source/doc comparison and `git diff --check`.

## Verification Contract

| Check | Applies To | Done Signal |
| --- | --- | --- |
| Rich scheduler regression tests | U1 | Latest work survives pressure, stale work cannot cache, and live worker count remains bounded. |
| Overlay publication interleaving tests | U2 | Startup/live updates converge correctly and readers are never blocked by aggregate construction. |
| Existing semantic-token and workspace-overlay tests | U1-U2 | Current LSP behavior and source-priority rules remain stable. |
| `cargo test --manifest-path server/Cargo.toml` | U1-U3 | Full Rust suite passes. |
| `cargo clippy --manifest-path server/Cargo.toml -- -D warnings` | U1-U3 | Report the existing unrelated warning baseline; do not bundle cleanup. |
| Documentation comparison and `git diff --check` | U3 | References match code and the diff is clean. |

## Definition Of Done

- Latest rich-token requests replace older pending work rather than being silently discarded.
- Rich-token worker resources have an explicit fixed bound.
- Workspace aggregate construction occurs outside the overlay mutex and publishes only a current generation.
- Startup watcher updates/deletes and workspace-over-game-data priority remain correct.
- Targeted scheduler and overlay interleaving coverage protects the failure modes found in review.
- LSP source-owner documentation matches current runtime APIs and contracts.
