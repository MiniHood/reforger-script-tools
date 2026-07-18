---
title: LSP Diagnostic Versioning - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: lsp-code-review-2026-07-18
execution: code
---

# LSP Diagnostic Versioning - Plan

## Goal Capsule

Prevent stale full-sync document changes and parser diagnostics from regressing the editor's current document state.

## Product Contract

- R1. Parser diagnostics include the LSP document version that produced them.
- R2. `didOpen` and full-sync `didChange` require document versions.
- R3. A repeated or older change never replaces current cached analysis, outline state, or diagnostics.

## Implementation Units

### U1. Version Parser Diagnostics

- **Files:** `server/src/lsp/diagnostics.rs`, `server/src/lsp.rs`, `docs/reference/server/src/lsp.md`
- **Approach:** Thread document version into publish diagnostics while preserving the clear-on-close notification shape.
- **Test scenarios:** Open and changed diagnostics contain their matching versions; close still clears diagnostics.

### U2. Reject Stale Full-Sync Changes

- **Files:** `server/src/lsp.rs`, `server/src/lsp/open_documents.rs`, `docs/reference/server/src/lsp/open_documents.md`
- **Approach:** Treat open/change versions as required and retain only strictly newer changes for an open URI; log ignored stale events without rebuilding analysis.
- **Test scenarios:** v1 open, v3 change, delayed v2 replay leaves symbols and diagnostics on v3; same-version replay is ignored.

## Verification Contract

- Focused framed LSP diagnostics/document-symbol regressions.
- `cargo test` from `server/` and `git diff --check`.
