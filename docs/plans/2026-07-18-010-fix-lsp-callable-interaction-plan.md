---
title: LSP Callable Interaction Integrity - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: lsp-code-review-2026-07-18
execution: code
---

# LSP Callable Interaction Integrity - Plan

## Goal Capsule

Fix confirmed completion and signature-help defects in incomplete, nested, and overloaded callable editing without adding new language features.

## Product Contract

### Requirements

- R1. Nested calls select the innermost callable argument list.
- R2. Comparison and literal expressions do not corrupt argument or signature splitting.
- R3. Active signature parameters are candidate-local and always in range.
- R4. Named parameter labels are case-insensitively deduplicated.
- R5. VS Code retrigger plumbing does not classify Enforce source text in TypeScript.

## Planning Contract

- KTD1. Reuse parser/lexer facts for callable context wherever available; lexical fallback must preserve literal and delimiter state.
- KTD2. Rust remains the sole authority for comment/string completion suppression.

## Implementation Units

### U1. Correct Callable Context And Signature Scanning

- **Goal:** Select nested argument lists correctly and parse realistic callable signatures safely.
- **Requirements:** R1, R2
- **Files:** `server/src/lsp/callable.rs`, `server/src/lsp/signature_help.rs`, `server/src/lsp/completion.rs`
- **Approach:** Continue traversal after an enclosing call candidate so the innermost list wins. Derive argument boundaries from syntax where possible and make fallback scanning literal-aware; do not treat relational brackets as generic nesting. Make signature/default splitting respect quoted and escaped literals.
- **Test scenarios:** Nested `Outer(Inner(...))`; comparison before a comma; generic nesting; defaults containing commas, closing parens, and escaped quotes.
- **Verification:** Focused callable/completion/signature tests.

### U2. Make Active Parameters And Labels Candidate-Safe

- **Goal:** Prevent invalid active parameter indices and duplicate named-label completions.
- **Requirements:** R3, R4
- **Dependencies:** U1
- **Files:** `server/src/lsp/signature_help.rs`, `server/src/lsp/callable.rs`, `server/src/lsp/completion.rs`
- **Approach:** Compute and clamp active parameter per candidate, selecting the top-level value only from the selected signature. Normalize supplied labels before set membership checks.
- **Test scenarios:** Overloads with different layouts; missing named label in an overload; extra positional arguments; differently cased supplied labels.
- **Verification:** Focused signature-help and argument-label completion tests.

### U3. Remove Client-Side Source Heuristics

- **Goal:** Let Rust, not TypeScript, reject completion in comments and strings.
- **Requirements:** R5
- **Files:** `src/languageClient/languageClient.ts`, `docs/reference/src/languageClient/languageClient.md`
- **Approach:** Retain editor/document/selection guards only; remove string/comment text scanning from the retrigger bridge.
- **Test scenarios:** Identifier edits after `https://` and quoted `/*`; actual comments/strings still produce no Rust completion candidates.
- **Verification:** Extension workflow plus existing Rust comment/string completion tests.

### U4. Document And Verify The Callable Contract

- **Goal:** Align source references and dev reports with nested/literal/overload behavior.
- **Requirements:** R1-R5
- **Dependencies:** U1-U3
- **Files:** `docs/reference/server/src/lsp/callable.md`, `docs/reference/server/src/lsp/completion.md`, `docs/reference/server/src/lsp/signature_help.md`, `server/examples/lsp_signature_help_report.rs` if a focused fixture adds durable coverage
- **Verification:** Full Rust suite, `npm test`, reference comparison, and `git diff --check`.

## Definition Of Done

- Nested, comparison, quoted-default, overload, and label-case regressions are covered.
- Completion retrigger ownership stays in Rust for language context.
- All focused and full verification passes.
