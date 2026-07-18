---
title: Hover and Definition Projection Fix - Plan
type: fix
date: 2026-07-18
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
product_contract_source: ce-plan-bootstrap
execution: code
---

# Hover and Definition Projection Fix - Plan

## Goal Capsule

- **Objective:** Make LSP hover and definition projection faithfully describe the cursor selection and produce valid file URIs for Windows network paths.
- **Scope:** Rust LSP projection and its focused regression coverage only.
- **Non-goals:** Change resolver selection policy, parser behavior, external-index lifecycle, or TypeScript language-client behavior.

---

## Product Contract

### Requirements

- R1. A resolver-selected file-local identifier hover reports the cursor token's UTF-16 range, matching external identifier hover behavior.
- R2. Hover selected from a syntax span retains its declaration-oriented range behavior.
- R3. Definition links for Windows UNC and extended UNC source paths encode the server as the file-URI authority and percent-encode the share path.
- R4. Existing local, POSIX, and drive-letter URI behavior remains unchanged.

### Acceptance Examples

- AE1. Hovering a file-local identifier selects only the identifier token, not the declaration span that contains it.
- AE2. A path such as `\\server\share\File Name.c` projects as `file://server/share/File%20Name.c`.
- AE3. A path such as `\\?\UNC\server\share\File.c` projects as `file://server/share/File.c`.

---

## Planning Contract

### Key Technical Decisions

- KTD1. Preserve resolver ownership: projection consumes `resolution.token_span`; it does not introduce token scanning or a second lookup path. This keeps hover and definition selection consistent with resolver policy.
- KTD2. Normalize extended UNC syntax before generic extended-path handling, then emit a URI authority for ordinary UNC paths. Treat only the host as authority; percent-encode the share-and-file path as URI path data.
- KTD3. Keep regression tests in `server/src/lsp.rs`, the established integration-style test host for the public LSP projection helpers.

### Sequencing

U1 and U2 both add tests in `server/src/lsp.rs`, so execute them serially. U1 validates hover range projection first; U2 validates URI projection without changing resolver or document-analysis behavior.

---

## Implementation Units

### U1. Project file-local hover token ranges

- **Goal:** Return the resolver token range for file-local identifier hover while retaining syntax-span hover semantics.
- **Files:** Modify `server/src/lsp/hover.rs`; add or strengthen focused tests in `server/src/lsp.rs`; verify `docs/reference/server/src/lsp/hover.md` remains accurate.
- **Approach:** Pass `range_for_span(source, resolution.token_span)` to the existing file-local identifier projection path, mirroring the external branch. Do not alter `HoverSelectionSource::ResolverSyntaxSpan` handling.
- **Patterns:** Follow the external identifier branch in `server/src/lsp/hover.rs` and existing hover selection tests in `server/src/lsp.rs`.
- **Execution note:** Use proof-first coverage: establish the file-local range expectation before relying on the production change.
- **Test scenarios:** Add a local declaration/use fixture with a multibyte prefix and assert the hover range is the usage token's UTF-16 range rather than the declaration range; external identifier range remains token-scoped; syntax-span selection remains declaration-oriented.
- **Verification:** Run the focused hover tests in `server/`, then include them in the final `cargo test` run.

### U2. Serialize UNC definition URIs correctly

- **Goal:** Emit network-share file URIs with a host authority for both normal and extended UNC paths.
- **Files:** Modify `server/src/lsp/definition.rs`; add focused tests in `server/src/lsp.rs`; verify `docs/reference/server/src/lsp/definition.md` remains accurate.
- **Approach:** Normalize `\\?\UNC\` to ordinary UNC form before stripping generic extended-path prefixes. Split normal UNC at the first slash into host and share path, then reuse the existing percent encoder for both URI components.
- **Patterns:** Follow `file_uri_for_path` and its existing POSIX/drive-letter tests in `server/src/lsp.rs`.
- **Execution note:** Use proof-first coverage: add the expected Windows-only UNC cases before validating the implementation.
- **Test scenarios:** Normal UNC with a space; extended UNC; unchanged POSIX URI; unchanged drive-letter URI; non-absolute paths still return no URI.
- **Verification:** Run focused URI tests on Windows, then include them in the final `cargo test` run.

---

## Verification Contract

| Scope | Command | Done signal |
| --- | --- | --- |
| Formatting | `cargo fmt --check` from `server/` | Rust formatting is clean. |
| Focused behavior | `cargo test hover --lib` and `cargo test file_uri_for_path --lib` from `server/` | Hover and URI regressions pass. |
| Server regression | `cargo test` from `server/` | Full Rust server suite passes. |
| Extension integration | `npm test` from repository root | Type, lint, build, and extension tests pass. |
| Documentation | `git diff --check` and manual owner-page review | Current owner docs describe final behavior without duplicating history. |

## Definition of Done

- U1 and U2 satisfy R1-R4 and their focused regression coverage passes.
- The resolver remains the sole source of hover/definition candidate selection.
- Windows UNC and extended UNC links use an authority-form `file://host/share/...` URI.
- The final diff is reviewed, required documentation is accurate, and task-scoped changes are committed and pushed or the push-authentication failure is recorded.
