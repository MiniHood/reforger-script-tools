## Why

Accepting the `RplRpc` attribute completion opens an empty VS Code suggestion
widget even though the language server returns the expected enum choices. The
server currently emits an invalid `InsertReplaceEdit` range pair, which the
VS Code client preserves unchanged and can reject during presentation.

## What Changes

- Make enum completion edits obey VS Code's insert/replace range invariant.
- Preserve the canonical `RplRpc` snippet, event-driven Suggest dispatch for
  each authored enum placeholder, and no-delay typing behavior.
- Keep enum members first while retaining the complete normal value fallback
  set below them: visible locals, containing-class members, top-level symbols,
  and keywords.
- Add regression coverage and bounded diagnostics for the protocol-to-editor
  completion boundary.
- Verify the Rust-to-extension bridge command as a cross-layer contract.

## Capabilities

### New Capabilities

- `enum-argument-completion`: Reliable, editor-compatible completion for
  selected enum arguments inserted by callable snippets.

### Modified Capabilities

- None.

## Impact

Affected areas are Rust LSP completion rendering and tests, the thin VS Code
language-client diagnostic/command bridge, extension command configuration,
and their reference documentation. The change does not add a typing debounce,
background analysis work, or a second language-completion implementation.
