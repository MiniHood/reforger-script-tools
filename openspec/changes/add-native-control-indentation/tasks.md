## 1. Native control-header contract

- [x] 1.1 Add paired narrow VS Code `onEnterRules` for complete standalone `if (...)` headers and cover accepted/rejected header shapes.
- [ ] 1.2 Validate in the existing Extension Development Host that a real Enter after an eligible header indents the body and the following real Enter returns to the enclosing level. (The automated test host's injected newline bypasses native indentation.)

## 2. Retire deferred layout correction

- [x] 2.1 Remove Rust scope-exit planning and LSP response edits while retaining the independent semicolon assist.
- [x] 2.2 Remove client/test assumptions for deferred Enter indentation and update formatting and language-configuration reference contracts.

## 3. Verification

- [x] 3.1 Run focused Rust and extension tests, then `npm run compile`; record the remaining live Development Host validation. (`cargo test on_type_formatting --lib` 9 passed; `npm test` 9 passed; final `npm run compile` passed; live keypress validation remains task 1.2.)
