---
title: Reduce Large-File Completion Latency - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: "GC_MarkerArea Ctrl+F2 capture and runtime performance review"
execution: code
---

# Reduce Large-File Completion Latency - Plan

## Goal Capsule

- **Objective:** Make autocomplete responsive while editing large Enforce files without weakening document-version, diagnostic, outline, or semantic-token correctness.
- **Measured problem:** The fresh `GC_MarkerArea.c` capture (86,049 bytes) completes `getgame` projection in 36 ms after dispatch, but the matching ten-minute runtime report shows completion queue p95 of 840 ms and perceived completion p95 of 964 ms. Repeated full `didChange` catalog rebuilds, lazy document-symbol projection, and rich semantic-token CPU work create the backlog.
- **Non-goals:** Remove bounded ingress/backpressure, move language intelligence into TypeScript, alter Enforce semantics, or introduce general incremental parsing before the measured lower-risk fixes prove insufficient.

## Product Contract

### Requirements

- R1. A burst of explicitly classified full-sync `textDocument/didChange` notifications for one URI performs analysis only for the highest valid version before the next protocol/lifecycle cut point; no request, close/open transition, malformed, multi-change, incremental-range notification, or other URI may be reordered across that boundary.
- R2. Completion, diagnostics, document symbols, and semantic-token caches observe only an accepted current document version. Coalesced intermediate changes publish no diagnostics or cache state.
- R3. Document-symbol projection preserves all current symbol hierarchy and UTF-16/CRLF/Unicode ranges while converting each projected endpoint through a per-projection UTF-16 index or ordered sweep, including long physical lines, instead of rescanning source per range.
- R4. Rich semantic-token work probes cancellation between bounded resolver/token chunks and before/during every remaining linear projection stage. It cannot install or refresh stale output and remains bounded/latest-wins; any indivisible resolver-call bound is measured and logged rather than described as interruptible.
- R5. The developer performance report can associate large-file edit bursts with queue wait, analysis phases, document-symbol projection, completion execution, and rich-token cancellation so before/after results are comparable.
- R6. The representative large-file protocol runs three fresh-server bursts per file, each with at least ten qualified completion requests after external indexing is idle and no unrelated editor activity. It reduces the combined qualified large-file perceived completion p95 from the 964 ms baseline to at most 400 ms, with no completion queue sample above 250 ms after the first accepted edit in a burst. The report marks a capture with fewer qualified samples as insufficient evidence. If this target is missed, the plan requires an explicit architecture checkpoint rather than silently broadening the implementation.

### Acceptance Examples

- AE1. After `didOpen(v1)`, queued full-text `didChange(v2)`, `didChange(v3)`, and `didChange(v4)`, followed by completion, the server analyzes/publishes only v4 and completion uses v4 text.
- AE2. A completion request between `didChange(v3)` and `didChange(v4)` sees v3, never v4; a stale v2 cannot displace v3.
- AE3. A large file containing nested symbols and non-ASCII/CRLF source returns the same document-symbol tree and ranges before and after the range-projection optimization.
- AE4. A resolver-heavy rich-token run cancelled by a new edit exits before completing its full traversal and cannot trigger a stale refresh.
- AE5. The controlled `GC_MarkerArea.c` typing burst and the small `GC_Sounds.c` control produce comparable report sections; startup/index-build activity is excluded.
- AE6. An incremental-range change or a multi-change notification is a FIFO barrier and continues through the existing handler uncoalesced.

## Planning Contract

### Key Technical Decisions

- KTD1. Retain the 64-event ingress queue. It provides bounded stdio backpressure and one authoritative owner for mutable open-document state; it exposes overload but does not create the expensive work.
- KTD2. First retain and classify LSP change shape. Coalesce only contiguous notifications that have exactly one full-text change (`range` absent), a usable version, and the same URI. Requests, lifecycle notifications, invalid/no-op, multi-change, incremental-range changes, and another URI are FIFO cut points. Select the highest version within the bounded run and retain the existing version gate and `OpenDocument::replace` mutation path.
- KTD3. Optimize measured foreground work before changing analysis ownership. Document-symbol range conversion is a current, isolated hotspot; latest-wins background document analysis is an architecture checkpoint only if the targeted fixes fail the measured contract.
- KTD4. Preserve Rust as the single language authority. TypeScript supplies no syntax classification or completion policy; its 75 ms suggestion retrigger remains an editor bridge.
- KTD5. Use the existing runtime log, Ctrl+F2 completion debug report, and developer report as the evidence source. Do not copy the user’s game source into the repository; any repeatable automated fixture must be synthetic or passed by path.

### Scope Boundaries

- **In scope:** `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, `server/src/lsp/semantic_tokens.rs`, focused Rust tests, the dev-only runtime report, and matching owner documentation.
- **Out of scope:** changing LSP transport limits, arbitrary completion-result reduction, source-file persistence, marketplace behavior, incremental parsing, or background analysis redesign unless the final architecture checkpoint is explicitly reached.

## Implementation Units

### U1. Establish replayable latency evidence

- **Goal:** Make the observed typing burst attributable by URI, revision, and operation without making runtime logging noisy or storing user source.
- **Requirements:** R5, AE5.
- **Files:** Modify `tools/lsp-runtime-performance-report.mjs`; create `tools/lsp-runtime-performance-report.test.mjs`; update `docs/reference/tools/lsp-runtime-performance-report.md` if report fields/sections change.
- **Approach:** Define the fixed `didChange` log schema first: `coalesced_changes`, `superseded_changes`, selected document version/revision, and queue time default to zero when absent. Teach the report to group operations by URI/revision and explicit capture window, expose a burst-oriented comparison section, and mark an undersampled capture insufficient. Preserve the current aggregate report and its read-only input behavior. Document the manual protocol: fresh server; wait for external index idle; no other editor activity; three bursts per file; each burst contains ten gibberish-to-real-prefix cycles followed by Ctrl+F2; then generate a same-window report for `GC_MarkerArea.c` and `GC_Sounds.c`.
- **Patterns:** Follow the current `queue_ms`, `analysis_*_ms`, `document_symbol_ms`, and rich-token report parsing. Keep source text and completion payloads out of runtime logs.
- **Execution note:** Characterize the existing report parsing with representative log records before extending it.
- **Test scenarios:** URI/revision grouping; fixed-field defaults; qualified/undersampled capture classification; mixed startup and typing records; completed/cancelled/discarded rich jobs; report generation without writing the input log.
- **Verification:** Run `node --test tools/lsp-runtime-performance-report.test.mjs`, run the report against the captured runtime log and inspect the generated comparison, then run `git diff --check`.

### U2. Coalesce contiguous full-sync document changes

- **Goal:** Eliminate redundant full-file analysis during a queued typing burst while retaining exact protocol ordering at request and lifecycle boundaries.
- **Requirements:** R1, R2, R5; AE1, AE2.
- **Dependencies:** U1 supplies the before/after evidence shape but U2 may be implemented first if its test harness is self-contained.
- **Files:** Modify `server/src/lsp.rs`; add focused channel-runtime/LSP lifecycle tests in `server/src/lsp.rs`; update `docs/reference/server/src/lsp.md`, `docs/reference/server/src/lsp/open_documents.md`, and `docs/solutions/conventions/lsp-document-revision-consistency.md`.
- **Approach:** Extend the change-event model to retain `range`/`rangeLength`, then add a pure eligibility classifier that uses the same parameter-deserialization/validation contract as the handler. It permits only one full-text change with no range; malformed, missing, multi-change, and ranged notifications remain FIFO barriers for the existing path. Add a small deferred-event buffer to the channel runtime. When its next event is eligible, non-blockingly inspect only the contiguous same-URI run up to the ingress bound, select the greatest usable version, and defer the first cut-point event with its original receive timestamp for FIFO processing before any later incoming event. Feed the selected change through the existing `handle_message` path so `OpenDocument::replace`, diagnostics, cache invalidation, and semantic cancellation remain singular. Emit U1's fixed selected/coalesced/superseded log fields. After each bounded batch, service the same bounded internal-event pass as the normal loop before accepting another ingress event.
- **Patterns:** Follow `run_message_channels`, current notification validation, strictly-newer document-version gates, and `SemanticTokenCache::cancel_pending`. Keep the synchronous reader path uncoalesced unless an explicit test exercises channel behavior.
- **Execution note:** Start with a failing channel-driven regression proving v2/v3/v4 currently rebuild independently; do not collapse messages across a request.
- **Test scenarios:** Runtime-channel (not direct framed-reader) test with v2/v3/v4 followed by completion asserts one accepted-change analysis log and one v4 diagnostics publish after v1 open; v3 then stale v2; request and document-symbol cut points; close/open boundaries; interleaved URIs; invalid/missing-version/no-text, incremental-range, and multi-change barriers; bounded burst fairness with a ready internal rich-token event; existing rich job cancellation after selected replacement. Retain direct framed-reader lifecycle coverage separately.
- **Verification:** Focused LSP lifecycle/channel tests, `cargo test` from `server/`, fresh Extension Development Host capture, and U1 report comparison.

### U3. Make document-symbol range projection linear in source size

- **Goal:** Remove the repeated position-from-zero scans that make large-file Outline projection add ~210 ms foreground spikes.
- **Requirements:** R3, R5; AE3.
- **Dependencies:** U2 may reduce how often this projection queues, but this unit is otherwise independent.
- **Files:** Modify `server/src/lsp.rs` and `server/src/lsp/semantic_tokens.rs`; add focused symbol-range tests in `server/src/lsp.rs` and line-index parity/cancellation tests in `server/src/lsp/semantic_tokens.rs`; update `docs/reference/server/src/lsp.md` or the matching owner page if the helper’s boundary changes.
- **Approach:** Profile the current symbol projection by phase, then extract one shared UTF-16 position strategy at the LSP boundary that guarantees linear projection: either a sorted span-endpoint sweep that advances once through source or a per-physical-line byte-to-UTF-16 prefix table. Do not reuse the current semantic-token line-start index unchanged because it rescans every long line per endpoint. Give the projection a test-only counter/seam proving one index build and indexed conversion for every projected range; include a long-single-line source and do not use timing assertions in CI. Replace the semantic-token-private equivalent only after CRLF/Unicode parity tests prove the contracts match; retain `range_for_span` as the compatibility entry point for unrelated callers.
- **Patterns:** Follow `range_for_span`, existing UTF-16 conversion tests, recursive document-symbol conversion, and cached-per-revision symbol projection.
- **Execution note:** Add a characterization test covering nested symbols, CRLF, and surrogate-pair text before changing range conversion.
- **Test scenarios:** Equal hierarchy/kinds/ranges for ASCII, CRLF, and Unicode; nested declaration ranges; repeated symbol requests reuse cache; accepted edit invalidates/rebuilds only the new revision; a synthetic large nested-symbol source verifies no per-symbol full-source rescan.
- **Verification:** Focused document-symbol/UTF-16 tests, `cargo test` from `server/`, and U1 before/after report showing the large-file document-symbol spike reduction.

### U4. Tighten rich semantic-token cancellation at expensive boundaries

- **Goal:** Stop resolver-heavy rich-token work sooner once a newer accepted revision makes it obsolete, reducing CPU contention with foreground typing.
- **Requirements:** R4, R5; AE4.
- **Dependencies:** U2 preserves timely delivery of the newer accepted change; U3 is independent.
- **Files:** Modify `server/src/lsp/semantic_tokens.rs` and, only if scheduling/log identity needs it, `server/src/lsp.rs`; add cancellation-focused tests; update `docs/reference/server/src/lsp/semantic_tokens.md` if cancellation granularity or timings change.
- **Approach:** Audit cancellation probes around the resolver-backed loop and thread the probe through every potentially linear rich-projection stage: resolver/token chunks, source-backed type overlays, recursive `new` expression overlays, multiline splitting, and encoding. Probe before and after each safe bounded chunk without changing token priority, rich/fast replacement, revision/generation validation, or the 250 ms idle delay. If one resolver call is indivisible, record its observed upper bound separately. Emit enough bounded timing/cancellation evidence to distinguish cancellation observed during resolver work, projection/encoding, and stale completion after work.
- **Patterns:** Follow `semantic_tokens_for_cached_analysis_with_external_indexes_cancelled`, the scheduler’s per-URI replacement model, and current revision/generation cache-install gates.
- **Execution note:** Use a resolver-heavy synthetic analysis and a deterministic cancellation hook/test seam; do not lower correctness by publishing partial rich output.
- **Test scenarios:** Deterministic cancellation before work, during a resolver-heavy pass, during multiline split/encoding, and after projection before install; newest revision can still obtain rich output after idle; stale/cancelled jobs do not refresh; worker/pending bounds remain unchanged.
- **Verification:** Focused semantic-token scheduler/cancellation tests, `cargo test` from `server/`, fresh large-file capture, and U1 report comparison of rich ready/cancelled CPU time.

### U5. Gate any completion-lookup or analysis-architecture expansion on measurements

- **Goal:** Resolve any remaining latency with the narrowest evidence-backed follow-up rather than adding speculative concurrency.
- **Requirements:** R6; AE5.
- **Dependencies:** U1-U4.
- **Files:** Conditionally modify `server/src/lsp/completion.rs` with focused completion tests, or create a follow-up plan for background analysis; update the matching owner documentation only for the chosen path.
- **Approach:** Re-run the exact three-run large/small control capture after U2-U4. If queue/perceived targets pass, stop. If queue is low but broad real-prefix lookup remains high, profile local/workspace/game-data lookup and ranking before changing candidate behavior. If one accepted `didChange` catalog build remains the dominant cost and targets fail, stop for the documented architecture checkpoint: design revision-safe latest-wins background analysis with explicit behavior for requests while current analysis is pending. Do not implement that redesign as an opportunistic extension of this plan.
- **Test scenarios:** Broad real prefix versus gibberish retains source-backed ordering/cap; all accepted revision/diagnostic/symbol/token invariants still hold; final reports exclude startup and show per-file comparison.
- **Verification:** Full Rust suite, `npm test` after server packaging, fresh Extension Development Host, controlled reports, and manual acceptance in both `GC_MarkerArea.c` and `GC_Sounds.c`.

## Verification Contract

| Scope | Evidence | Done signal |
| --- | --- | --- |
| Ordering | Channel-driven Rust lifecycle tests | Only latest coalesced text is analyzed before a request; no boundary is crossed. |
| Document ranges | Symbol/UTF-16/CRLF tests | Symbol hierarchy and LSP ranges are unchanged. |
| Rich work | Deterministic cancellation tests | Obsolete work exits without install/refresh; bounds stay intact. |
| Server regression | `cargo test` from `server/` | Full Rust suite passes. |
| Extension integration | `npm test` from repository root | Fresh packaged server and extension tests pass. |
| Runtime acceptance | Three controlled captures per large/small file, external index idle | Large-file perceived completion p95 ≤400 ms and post-first-edit completion queue samples ≤250 ms, or a documented architecture checkpoint is produced. |
| Diff hygiene | `git diff --check` and reference review | No whitespace errors; owner documentation matches runtime behavior. |

## Deferred to Implementation

- The actual `GC_MarkerArea.c` is user-local and must not be copied into the repository. Choose a synthetic fixture or path-driven developer harness only if the current log report cannot demonstrate a repeatable before/after result.
- Confirm whether the VS Code Outline request cadence can be observed from existing client output before changing any client behavior. The server must continue to answer standard `documentSymbol` requests correctly.
- If U3’s profiler disproves repeated UTF-16 position scanning as the dominant symbol cost, keep the characterization test and redirect the optimization to the measured phase rather than forcing a shared line-index extraction.

## Definition of Done

- The bounded ingress queue remains, but large-file typing no longer accumulates obsolete full-analysis work before a completion request.
- Diagnostics, outline, completion, and semantic tokens remain revision-safe and source-backed.
- Large-file document-symbol and stale rich-token work no longer produce the measured foreground/CPU spikes without evidence in the report.
- The fresh Ctrl+F2 capture plus controlled runtime report demonstrate the acceptance target, or the remaining single-revision analysis bottleneck is isolated with a separately approved architecture plan.
