---
title: Background Open-Document Analysis for Typing Latency - Plan
type: fix
date: 2026-07-19
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: "2026-07-19 GC_MarkerArea Ctrl+F2 runtime capture"
execution: code
---

# Background Open-Document Analysis for Typing Latency - Plan

## Goal Capsule

- **Objective:** Keep the LSP ingress loop responsive during large-file typing by accepting document text/version immediately and completing full language analysis only for the latest paused revision.
- **Measured problem:** The fresh `GC_MarkerArea.c` capture shows each accepted `didChange` spends 126-134 ms rebuilding its catalog. Edits arrive at approximately that cadence, so completion queue wait grows to 869 ms and perceived completion p95 is 974 ms. Completion execution itself is not the dominant delay; document-symbol projection is already reduced to 3 ms.
- **Non-goals:** Incremental parsing, TypeScript language logic, returning stale semantic facts for current text, changing Enforce semantics, or removing bounded ingress backpressure.

## Product Contract

### Requirements

- R1. `didChange` accepts a strictly newer full-text revision without running lexer/parser/catalog/scope construction on the ingress thread. It immediately replaces the current text/version, invalidates text-derived caches, cancels obsolete rich-token work, and schedules latest-wins analysis after a bounded idle delay.
- R2. At most one pending analysis job is retained per URI. A newer accepted revision replaces a not-yet-running job; a completed result installs only when its URI, revision, and text identity are still current.
- R3. Diagnostics, outline, completion, hover, definition, signature help, and semantic tokens never use a mismatched text/analysis revision. Requests that need a pending revision's analysis are retained only for that revision and either replay after installation or receive the standard content-modified response when superseded/closed.
- R4. The server continues receiving and ordering incoming events while analysis runs. Internal worker completions remain bounded and cannot starve ingress.
- R5. Existing `didOpen` behavior remains available for an initial document snapshot; all document-close, stale-version, diagnostic-clear, cache, and rich-token generation gates remain correct.
- R6. Runtime logging and the developer report distinguish accepted/pending/installed/superseded analysis and report foreground queue time separately from background analysis duration.
- R7. The large-file controlled capture reaches perceived completion p95 <=400 ms after typing settles, with no completion queue sample above 250 ms after the first accepted edit. If analysis-ready completion response time still misses this target, profile completion lookup independently before expanding scope.

### Acceptance Examples

- AE1. `didChange(v2)`, `didChange(v3)`, `didChange(v4)` at typing speed accepts all three quickly but runs full analysis only for v4 after the idle delay; diagnostics publish only for v4.
- AE2. A completion request for v3 waits for v3 analysis. If v4 arrives first, the v3 request receives `ContentModified` and cannot produce v3 or v4 completion data.
- AE3. A document-symbol request, hover, definition, or semantic-token request received for a pending revision cannot read the previous analysis against new text.
- AE4. A close during pending/running work clears diagnostics, drops queued requests, and prevents installation or refresh from the old result.
- AE5. `GC_Sounds.c` remains responsive and `GC_MarkerArea.c` no longer accumulates a foreground edit backlog during the controlled typing protocol.

## Key Technical Decisions

- KTD1. Add a dedicated Rust latest-wins analysis scheduler rather than moving analysis into TypeScript or weakening analysis correctness. It mirrors the existing rich-token scheduler's worker/event pattern but owns only immutable text snapshots and `FileIndexAnalysis` results.
- KTD2. Use a 150 ms per-URI idle delay. It is longer than the observed ~130 ms edit cadence, allowing ordinary bursts to collapse, while leaving room for one ~130 ms analysis plus completion under the 400 ms acceptance target once typing stops.
- KTD3. `OpenDocument` has explicit pending versus ready analysis state. Current text/version/revision remain authoritative immediately; a ready analysis is usable only when it carries the same accepted revision.
- KTD4. Defer source-backed requests by URI/revision in Rust. On supersession or close, send `ContentModified` rather than serving a mismatched analysis or blocking the transport thread. Keep the pending-request cap bounded and reject overflow predictably.
- KTD5. Preserve synchronous `didOpen` analysis for the initial snapshot. It avoids changing initialization semantics and leaves this vertical slice focused on typing updates.

## Scope Boundaries

- **In scope:** `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, targeted Rust tests, runtime-report tooling, and matching LSP reference documentation.
- **Out of scope:** incremental parser/index internals, completion ranking changes, client retry policy, persistent analysis caches, and changing the rich-token resolver model.

## Implementation Units

### U1. Model pending document analysis

- **Goal:** Separate accepted text identity from ready analysis without allowing mismatched use.
- **Files:** Modify `server/src/lsp/open_documents.rs`; update `docs/reference/server/src/lsp/open_documents.md`.
- **Approach:** Replace synchronous `OpenDocument::replace` rebuilding with an immediate accepted-revision transition that clears symbols and semantic-token state, records pending analysis identity, and exposes ready analysis only when installed for that identity. Keep initial construction synchronous. Make installation and close/supersession validation explicit and non-panicking.
- **Test scenarios:** accepted replacement is pending; stale installation is ignored; current installation becomes usable; close prevents later installation; symbol and token caches reset exactly once.
- **Verification:** Focused open-document tests and `cargo test` from `server/`.

### U2. Schedule latest-wins analysis and preserve request ordering

- **Goal:** Run expensive full analysis off the ingress loop and deliver results back through bounded internal events.
- **Files:** Modify `server/src/lsp.rs`; add scheduler/channel tests in `server/src/lsp.rs`; update `docs/reference/server/src/lsp.md`.
- **Approach:** Add immutable analysis jobs/results with URI, revision, source snapshot, cancellation/supersession identity, and scheduled time. Reuse the rich scheduler's one-worker/latest-per-URI model with a 150 ms idle delay. On worker result, install only current analysis, publish diagnostics, then service retained same-revision requests. Keep internal event passes bounded before and after ingress batches.
- **Test scenarios:** typing-speed v2/v3/v4 yields one v4 analysis; interleaved URIs stay independent; stale worker result cannot install; internal completion does not starve a queued request; analysis logs record pending/ready/superseded transitions.
- **Verification:** Channel-runtime tests, full Rust suite, and runtime-log inspection.

### U3. Defer or supersede source-backed requests safely

- **Goal:** Avoid stale analysis while ensuring the transport remains responsive during a pending revision.
- **Files:** Modify `server/src/lsp.rs`; add lifecycle/request tests in `server/src/lsp.rs`; update `docs/reference/server/src/lsp.md` and matching feature references if behavior wording changes.
- **Approach:** Classify source-backed request methods at dispatch. When the target document's current revision is pending, retain the original request with that revision rather than running it against the previous analysis. When a new revision or close supersedes it, respond with JSON-RPC `ContentModified`; when analysis installs, replay it through the normal handler. Bound retained requests per URI and preserve JSON-RPC IDs and timestamps for logging.
- **Test scenarios:** completion waits then returns current analysis; superseded completion receives content-modified; outline/hover/definition/signature/semantic-token requests do not read stale analysis; close clears retained requests; cap overflow is deterministic.
- **Verification:** Focused LSP request tests and full Rust suite.

### U4. Measure and validate the new latency path

- **Goal:** Prove the fix with comparable runtime evidence and document the changed ownership/state model.
- **Files:** Modify `tools/lsp-runtime-performance-report.mjs` and tests only if new log fields require parsing; update `docs/reference/tools/lsp-runtime-performance-report.md`, `docs/reference/server/src/lsp/open_documents.md`, `docs/reference/server/src/lsp.md`, and the relevant `docs/solutions/` convention.
- **Approach:** Add only fixed, source-free analysis lifecycle fields needed to distinguish foreground acceptance from background build/install and request deferral. Run the existing controlled protocol after a fresh packaged server. If p95 remains high after the settled-revision path is measured, isolate completion lookup before proposing another architecture change.
- **Verification:** `node --test tools/lsp-runtime-performance-report.test.mjs`, `cargo test`, `npm test`, fresh `GC_MarkerArea.c` and `GC_Sounds.c` captures, and `git diff --check`.

## Verification Contract

| Scope | Evidence | Done signal |
| --- | --- | --- |
| Revision safety | Channel lifecycle/request tests | No source-backed feature combines current text with old analysis. |
| Throughput | Burst scheduler tests | Multiple edits accept quickly; only the latest paused revision installs. |
| Protocol behavior | Deferred/superseded request tests | Pending requests replay once current or receive `ContentModified`; no hangs. |
| Server regression | `cargo test` from `server/` | Full suite passes. |
| Extension integration | `npm test` | Packaged server and extension tests pass. |
| Runtime acceptance | Fresh three-burst large/small capture | Large-file perceived completion p95 <=400 ms and queue <=250 ms after first edit. |

## Definition of Done

- Full document analysis is no longer performed by the ingress handler for `didChange`.
- Only analysis matching the accepted text revision can feed diagnostics or language features.
- Large-file typing no longer creates a serial foreground analysis queue.
- Runtime capture verifies the target or isolates a completion-lookup bottleneck with evidence.
