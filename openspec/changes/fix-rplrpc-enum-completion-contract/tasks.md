## 1. Completion edit contract

- [x] 1.1 Add regression tests that enforce valid insert/replace range relationships and enum-first fallback candidates across attributes, calls, and constructors.
- [x] 1.2 Render the shared static-enum completion list with valid full-expression edits and retain the complete normal contextual fallback candidates, including the bounded current-snapshot path used by immediate snippet completion.

## 2. Bridge contract and diagnostics

- [x] 2.1 Add bounded client-side protocol-boundary diagnostics without logging source or full completion payloads, including multi-placeholder bridge progression.
- [x] 2.2 Add a cross-layer bridge-command contract check and update command ownership as needed.

## 3. Documentation and verification

- [x] 3.1 Update the completion and language-client reference contracts.
- [ ] 3.2 Run focused and complete Rust/extension checks, validate a fresh Extension Development Host journey, and record results.
- [ ] 3.3 Remove the temporary RplRpc snippet-bridge forensic trace before release after a freshly loaded Extension Development Host proves the multi-placeholder lifecycle.
- [ ] 3.4 Remove the temporary erased-prefix completion-lifecycle trace before release after a live Extension Development Host proves recovery or identifies its remaining VS Code boundary.
