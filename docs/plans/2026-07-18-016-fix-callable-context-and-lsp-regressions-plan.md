---
title: Callable Context and LSP Regression Fixes - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: lsp-code-review-2026-07-18
execution: code
---

# Callable Context and LSP Regression Fixes - Plan

## Goal Capsule

- **Objective:** Correct callable argument boundaries and prove LSP document-change and completion-retrigger contracts exposed by review.
- **Scope:** Shared Rust callable context, framed LSP lifecycle coverage, and the thin TypeScript retrigger bridge.
- **Non-goals:** Add generic-language features, alter resolver policy, change LSP protocol shapes, or move language context into TypeScript.

---

## Product Contract

### Requirements

- R1. Nested generic callable arguments closed by `>>` do not hide the following top-level comma from signature help or argument-label completion.
- R2. Compact relational expressions using `<` and `>` remain ordinary expressions, not generic nesting.
- R3. Versioned `didChange` before `didOpen`, and equal-version replay after open, leave document analysis, diagnostics, and symbols unchanged.
- R4. Completion retriggering remains an editor-state bridge: it forwards valid Enforce identifier edits to Rust and suppresses inactive, wrong-language, selection, and invalid-prefix cases.

### Acceptance Examples

- AE1. In `Use(Outer<Inner<A, B>, C>(), next)`, `next` is argument index 1.
- AE2. In `Use(first<second>third, next)`, `next` is argument index 1.
- AE3. A `didChange` for an unopened URI produces no open-document state or diagnostics; a same-version change cannot replace accepted state.
- AE4. A comment/string identifier edit may trigger VS Code suggestion UI, but Rust remains the sole authority that suppresses inappropriate candidates.

---

## Planning Contract

### Key Technical Decisions

- KTD1. Keep callable interpretation in Rust. Generic-angle recognition must be constrained by valid callable expression shape, not merely identifier adjacency and a later close token.
- KTD2. Use the existing framed LSP test host in `server/src/lsp.rs` for protocol lifecycle coverage, so assertions exercise dispatch and cached-document state together.
- KTD3. Test TypeScript retrigger predicates as editor-state guards. Do not reintroduce comment/string source scanning; those language decisions belong to Rust completion.

### Sequencing

U1 establishes the shared callable contract first. U2 is independent but shares `server/src/lsp.rs` with broader integration tests, so execute it after U1. U3 is TypeScript-only and follows the existing U1 source boundary.

---

## Implementation Units

### U1. Correct callable argument boundaries

- **Goal:** Count only true top-level commas after nested generic calls and compact comparisons.
- **Files:** Modify `server/src/lsp/callable.rs`; update `docs/reference/server/src/lsp/callable.md` only if final behavior differs from its current contract.
- **Approach:** Make generic-angle recognition reject relational/operator expression shapes and make `>>` close two active generic depths when a generic was accepted. Preserve existing literal and nested-delimiter handling.
- **Patterns:** Follow `argument_index_at_offset`, `generic_angle_opens`, and the existing comparison/generic callable tests.
- **Execution note:** Use proof-first regression tests for both `>>` and no-whitespace comparisons before changing production logic.
- **Test scenarios:** Nested `Outer<Inner<A, B>, C>()` followed by another argument; no-whitespace chained comparison; existing one-level generic control; signature-help or completion path that consumes the shared argument index.
- **Verification:** Focused callable, signature-help, and completion tests, then `cargo test` from `server/`.

### U2. Prove rejected document changes are inert

- **Goal:** Demonstrate that missing open state and non-increasing versions cannot create or replace cached LSP state.
- **Files:** Modify `server/src/lsp.rs`; update `docs/reference/server/src/lsp/open_documents.md` only if final behavior differs from its current contract.
- **Approach:** Extend framed JSON-RPC tests rather than adding a parallel state harness. Assert no published diagnostics or symbols are produced for a pre-open change, and that an equal-version replay preserves accepted analysis.
- **Patterns:** Follow `framed_lsp_ignores_stale_changes_without_regressing_diagnostics_or_symbols` and existing didOpen/didChange message helpers.
- **Execution note:** Add the failing lifecycle cases before relying on existing rejection paths.
- **Test scenarios:** Versioned change before open; equal-version replay; currently accepted version remains available for hover/symbol/diagnostic projection.
- **Verification:** Focused framed-LSP lifecycle tests, then `cargo test` from `server/`.

### U3. Cover completion retrigger guards

- **Goal:** Lock the TypeScript bridge to editor-state gating while Rust owns comment/string semantics.
- **Files:** Modify `src/languageClient/languageClient.ts`; add focused extension tests under `src/test/`; update `docs/reference/src/languageClient/languageClient.md` only if final behavior differs from its current contract.
- **Approach:** Expose or extract the minimal deterministic retrigger predicate needed for extension-host tests. Assert valid identifier prefixes pass without inspecting syntax text; assert wrong language, non-active document, selected text, and invalid prefixes remain blocked by the bridge.
- **Patterns:** Follow the existing `triggerCompletionWhenActive` guard and `languageClientCompletion` configuration.
- **Execution note:** Characterize the current editor-state predicate before changing visibility or test seams.
- **Test scenarios:** Valid identifier insertion in normal text, comment-like text, and string-like text; selection, short/invalid prefix, inactive editor, and non-Enforce document suppression.
- **Verification:** Focused extension test, then `npm test` from the repository root.

---

## Verification Contract

| Scope | Command | Done signal |
| --- | --- | --- |
| Rust formatting | `cargo fmt --check` from `server/` | Rust formatting is clean. |
| Callable behavior | Focused `cargo test` filters for callable, signature help, and completion | Generic/comparison regressions pass through shared context. |
| LSP lifecycle | Focused framed-LSP tests | Pre-open and equal-version changes remain inert. |
| Full Rust regression | `cargo test` from `server/` | Full server suite passes. |
| Extension bridge | `npm test` from repository root | TypeScript test, lint, build, and extension tests pass. |
| Diff hygiene | `git diff --check` | Task diff is whitespace-clean. |

## Definition of Done

- R1-R4 and AE1-AE4 have focused regression coverage.
- Rust remains the only language-context authority for completion and callable parsing.
- Existing owner documentation remains accurate or is updated with the implementation.
- The reviewed task diff is committed and pushed, or an authentication failure is recorded after the commit exists.
