## Why

Typing a complete unbraced Enfusion control header currently has no native
language indentation contract. The extension tried to correct indentation after
Enter through an asynchronous language-server request, which can visibly move
the caret after VS Code has already rendered the editor's initial line.

## What Changes

- Add native, header-driven indentation for complete standalone unbraced
  Enfusion control headers.
- Preserve the established unbraced style: a control header indents its one
  following statement without inserting braces or inspecting that statement.
- Remove the asynchronous Rust scope-exit/caret-correction path from Enter
  typing assistance; semicolon assistance remains a separate concern.
- Add regression coverage and concise diagnostics for the native indentation
  contract.

## Capabilities

### New Capabilities

- `native-control-indentation`: Immediate, VS Code-owned indentation for
  complete unbraced Enfusion control-statement headers.

### Modified Capabilities

- None.

## Impact

- `language-configuration.json` gains deliberately narrow indentation rules.
- The TypeScript language-client bridge and Rust Enter assist stop applying
  asynchronous scope/caret layout edits.
- Rust and extension tests, plus the language-configuration and formatting
  reference documentation, define and verify the no-flicker boundary.
