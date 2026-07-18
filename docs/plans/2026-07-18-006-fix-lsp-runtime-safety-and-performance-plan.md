---
title: LSP Runtime Safety And Performance - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: code-review-2026-07-18
execution: code
---

# LSP Runtime Safety And Performance - Plan

## Goal Capsule

| Field | Plan |
| --- | --- |
| Objective | Make the Rust LSP remain correct and responsive during large-file edits, workspace startup, overlay updates, and semantic-token refreshes. |
| Product authority | Fix the confirmed code-review findings without adding new language features or changing Enfusion language semantics. |
| Execution profile | Rust runtime concurrency and memory-safety work with regression tests, stress evidence, and matching source-reference documentation. |
| Primary risks | Full-sync document traffic can grow unbounded; startup can publish stale workspace symbols; overlay locks and rich-token scheduling can stall or exhaust runtime resources. |

## Product Contract

### Requirements

- R1. LSP framing and ingress must have explicit bounded memory behavior for oversized headers, bodies, and bursts of full-sync messages.
- R2. A workspace file update or deletion received during startup indexing must never be overwritten by the older startup disk snapshot.
- R3. Foreground completion, hover, definition, signature help, and semantic-token requests must not hold the external-overlay mutex while running resolver or rendering work.
- R4. Workspace overlay updates must not rebuild and publish a stale aggregate while another update is in flight.
- R5. Rich semantic-token scheduling must use bounded worker resources under rapid edits; cancelling obsolete work must also avoid unbounded sleeping-thread creation.
- R6. A panic caught in external-index startup must surface as a terminal failed state instead of leaving the server permanently `Building`.
- R7. Completion and semantic-token context detection must reuse cached lexical facts rather than repeatedly lexing unchanged open-document text.
- R8. Existing LSP feature behavior, source-priority order, full-sync document contract, and Workbench truth boundaries must remain unchanged.

### Scope Boundaries

- In scope: stdio message framing, runtime event ingress, open-document analysis cache, external overlay publication, semantic-token scheduling, completion lexical reuse, targeted stress/report tooling, tests, and matching reference pages.
- Out of scope: incremental parsing, semantic `modded` merging, workspace-wide rename/references, new editor features, Workbench integration changes, and new runtime dependencies.
- Out of scope: unrelated Clippy cleanup. The current warning baseline is recorded but is not bundled into this runtime-behavior fix.

### Acceptance Examples

- AE1. An oversized or malformed LSP frame fails cleanly before a large allocation; normal VS Code-sized frames retain current behavior.
- AE2. During startup, a watcher update or delete wins over the older disk-scanned version of the same workspace file.
- AE3. A long-running completion or hover query does not prevent an external-index snapshot from publishing.
- AE4. A save updates one workspace file without publishing an aggregate derived from an older workspace generation.
- AE5. A rapid sequence of edits and semantic-token requests leaves at most the configured bounded scheduler resources active and only the latest valid projection becomes cached.
- AE6. A caught external-index panic reports `Failed` with an error, rather than remaining `Building`.
- AE7. Completion output remains equivalent for existing fixtures while request-time lexical scans are removed from unchanged-document paths.

## Planning Contract

### Key Technical Decisions

- KTD1. Use bounded backpressure for incoming stdio messages before attempting sophisticated notification coalescing. This fixes unbounded retained payloads without changing JSON-RPC ordering semantics.
- KTD2. Treat workspace startup output as a baseline snapshot and live watcher changes as versioned overrides, including deletion tombstones. A startup scan may never blindly replace live state.
- KTD3. Publish immutable `Arc<SymbolIndex>` snapshots. Lock only to read or swap state; merge/index work and feature queries run outside the mutex.
- KTD4. Replace one-thread-per-idle-delay with a bounded scheduler that keeps only current rich-token work eligible for execution. Cancellation is a resource policy, not merely a stale-result filter.
- KTD5. Extend `FileIndexAnalysis` with reusable lexical facts and pass those facts into resolver/completion helpers. Preserve the existing parser, AST, model, and resolver authority.
- KTD6. Keep measured thresholds in dev-only reports. Do not introduce incremental parsing or a new workspace-index architecture unless the post-fix evidence justifies it.

### Runtime Shape

```text
stdio reader --bounded ingress--> LSP event loop --replace/snapshot--> immutable state
                                             |                         |
                                             |                         +--> feature queries outside lock
                                             +--> bounded token scheduler --> latest valid projection

startup disk scan + live watcher versions --> generation-aware workspace snapshot publication
```

### Risks And Mitigations

- Backpressure can delay the client writer under bursts. Use a bounded capacity sized for normal traffic, preserve protocol order, and record queue wait so capacity can be tuned from evidence.
- Startup merge logic can mishandle deletes. Store per-path live revision/tombstone state and test update/delete interleavings explicitly.
- Snapshot retry can starve under continuous writes. Keep publish work outside the lock, use generation comparison, and bound/retry through the event loop rather than spin indefinitely.
- A single rich-token scheduler can delay another open document. Preserve per-document revision cancellation and measure queue delay before adding more workers.
- Cached lexer facts can drift from parse/source. Build them in the same `OpenDocument` replacement transaction and invalidate them only with that analysis.

## Implementation Units

### U1. Bound LSP Framing And Ingress

- **Goal:** Prevent malformed or bursty full-sync traffic from retaining unbounded message payloads or causing unbounded frame allocation.
- **Requirements:** R1, R8; AE1
- **Dependencies:** None
- **Files:** `server/src/lsp.rs`, `docs/reference/server/src/lsp.md`
- **Patterns to follow:** Preserve the current reader-thread/main-loop separation in `run_stdio` and the existing protocol error shape from `read_message`.
- **Approach:** Introduce explicit message and header limits before body allocation. Replace the unbounded reader-to-event-loop queue with bounded ingress that applies backpressure while preserving JSON-RPC ordering. Keep server-generated internal events distinguishable from client ingress so a saturated client queue cannot create a deadlock in rich-token completion reporting.
- **Test scenarios:** Accepted normal frame; missing/invalid content length; oversized header; oversized body rejected before allocation; burst of full-sync notifications stays within configured queue capacity and exits cleanly when input closes; ordinary framed LSP smoke tests remain unchanged.
- **Verification:** Targeted `lsp` framing tests plus `cargo test --manifest-path server/Cargo.toml`.

### U2. Publish Versioned Workspace Overlay Snapshots

- **Goal:** Make startup scans and watcher updates converge on the latest workspace state without stale overwrite or long mutex ownership.
- **Requirements:** R2-R4, R6, R8; AE2-AE4, AE6
- **Dependencies:** U1
- **Files:** `server/src/lsp/external_overlay.rs`, `server/src/lsp.rs`, `docs/reference/server/src/lsp/external_overlay.md`, `docs/reference/server/src/lsp.md`
- **Patterns to follow:** Preserve workspace-over-game-data priority and separate external layers. Follow the existing `Arc<SymbolIndex>` snapshot pattern used by worker-owned semantic-token projection.
- **Approach:** Record a live workspace revision for every changed path and a tombstone revision for every delete. Startup builds a baseline outside the lock, then merges it with newer live changes before publication. Build replacement workspace aggregates from immutable file-index snapshots outside the mutex; atomically publish only when the captured generation remains current, otherwise retry from a new snapshot. Replace `with_indexes` borrowing with a short-lock snapshot API so feature handlers resolve against cloned `Arc` layers. Convert caught startup panics into a `Failed` state with stored error and generation change.
- **Test scenarios:** Update during startup wins; delete during startup remains deleted; multiple rapid updates publish the latest text; workspace update during a held query can complete/publish; feature query observes a consistent old or new snapshot, never a partially rebuilt aggregate; caught startup panic transitions to `Failed`; workspace precedence over game data remains unchanged.
- **Verification:** Focused external-overlay and framed-LSP tests, existing workspace-overlay report, and `cargo test --manifest-path server/Cargo.toml`.

### U3. Replace Per-Revision Rich Token Threads

- **Goal:** Bound semantic-token scheduling resources while preserving fast-first coloring and revision/generation correctness.
- **Requirements:** R5, R8; AE5
- **Dependencies:** U1, U2
- **Files:** `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, `server/src/lsp/semantic_tokens.rs`, `docs/reference/server/src/lsp.md`, `docs/reference/server/src/lsp/open_documents.md`, `docs/reference/server/src/lsp/semantic_tokens.md`
- **Patterns to follow:** Preserve the existing fast projection, rich projection cache key, external-generation checks, and cancellation semantics.
- **Approach:** Create a bounded server-owned rich-token scheduler instead of spawning an idle-delay thread for each revision. It should retain only eligible latest work, observe document cancellation/revision and external-generation changes before expensive projection, and deliver completion through the existing internal event mechanism. Keep rich computation off the foreground LSP loop.
- **Test scenarios:** Repeated requests for one revision schedule one job; rapid revisions cancel/replace older jobs without growing worker count; a stale job cannot populate cache; an external-generation change cancels pending work; a valid unchanged revision still triggers refresh and serves cached rich tokens.
- **Verification:** Targeted LSP scheduler tests that expose worker/job counts through test-only instrumentation, plus the full Rust test suite.

### U4. Reuse Open-Document Lexical Facts

- **Goal:** Remove duplicate full-document lexing from completion and related context paths without changing completion behavior.
- **Requirements:** R7, R8; AE7
- **Dependencies:** U3
- **Files:** `server/src/lsp/open_documents.rs`, `server/src/resolver.rs`, `server/src/lsp/completion.rs`, `server/src/lsp/semantic_tokens.rs`, `docs/reference/server/src/lsp/open_documents.md`, `docs/reference/server/src/resolver.md`, `docs/reference/server/src/lsp/completion.md`, `docs/reference/server/src/lsp/semantic_tokens.md`
- **Patterns to follow:** Keep the existing span-based resolver path used by semantic tokens and keep parser/AST/model/index ownership unchanged.
- **Approach:** Store the lexer token stream with `FileIndexAnalysis` as part of the single replacement transaction. Accept borrowed cached tokens in member/top-level completion context and callable-argument helpers, and reuse them in semantic-token projection when compatible. Do not add TypeScript-side lexical behavior or a second token authority.
- **Test scenarios:** Existing member, top-level, argument-label, enum-placeholder, comment/string suppression, and semantic-token fixtures retain their output; changed documents replace cached lexical facts; a test-only lexer invocation counter demonstrates no request-time re-lexing for cached completion paths.
- **Verification:** Focused resolver/completion/semantic-token tests, relevant corpus report if its existing fixture set covers the path, and `cargo test --manifest-path server/Cargo.toml`.

### U5. Measure And Document The Runtime Contract

- **Goal:** Leave maintainers with repeatable evidence for queue pressure, overlay update latency, and token-scheduler behavior.
- **Requirements:** R1-R8; AE1-AE7
- **Dependencies:** U1-U4
- **Files:** `server/examples/lsp_workspace_overlay_report.rs`, `server/examples/lsp_corpus_report.rs` if queue/token timing needs a focused extension, `docs/reference/server/examples/lsp_workspace_overlay_report.md`, `docs/reference/server/examples/lsp_corpus_report.md`, `docs/solutions/` only if execution establishes a new durable lesson
- **Patterns to follow:** Reports remain dev-only, write ignored output, and do not become runtime feature paths or source truth.
- **Approach:** Extend existing stress evidence only where it can measure the new runtime contract: bounded ingress, generation-safe publication, and scheduler queue/cancellation timing. Keep thresholds advisory and use results to decide whether future incremental parsing or wider workspace-index changes are justified.
- **Test scenarios:** Report runs on synthetic burst/update inputs, records bounded queue/job behavior, and does not alter server runtime configuration or source state.
- **Verification:** Run the affected report command(s), inspect generated ignored output, validate reference-doc links, run `git diff --check`, and preserve any remaining environment-specific performance uncertainty.

## Verification Contract

| Check | Applies To | Done Signal |
| --- | --- | --- |
| Framed LSP protocol tests | U1 | Oversized/malformed frames fail safely; normal requests still respond. |
| Overlay interleaving tests | U2 | Startup, update, and delete paths converge on the newest workspace state. |
| Scheduler tests | U3 | Rapid edits do not create unbounded threads/jobs and stale work never caches. |
| Completion/token regression tests | U4 | Existing feature outputs stay stable while cached lexical facts are reused. |
| Runtime stress report | U5 | Queue, overlay, and scheduler timings are captured from the new path. |
| `cargo test --manifest-path server/Cargo.toml` | U1-U5 | Entire Rust suite passes. |
| `cargo clippy --manifest-path server/Cargo.toml -- -D warnings` | U1-U5 | Current lint baseline is reported; unrelated existing warnings are not fixed in this plan. |
| Documentation review and `git diff --check` | U1-U5 | Runtime contract and source-owner pages agree with code. |

## Definition Of Done

- The server applies bounded memory behavior to incoming LSP frames and full-sync ingress.
- No live workspace change or deletion is lost during startup indexing.
- External index queries and aggregate construction do not hold the mutex through expensive feature work.
- Rich semantic tokens use bounded scheduler resources and preserve valid fast-to-rich refresh behavior.
- Completion/context logic reuses current open-document lexical facts.
- Panic, queue, overlay, and scheduler behavior have targeted regression coverage.
- All matching source-reference documentation describes the new runtime contract.
