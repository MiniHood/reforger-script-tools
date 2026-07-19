## Why

When an editor session is slow, crashes, or returns an unexpected language
feature result, the extension and language server do not provide a coherent,
separate account of the work that led to it. Existing timing and server logs
are useful point diagnostics but are not a reliable, low-overhead support log
for reconstructing a user session.

## What Changes

- Add structured, append-only diagnostic logs for the VS Code extension host
  and Rust language server under extension global storage.
- Record lifecycle events, server startup/configuration, commands, protocol
  requests/notifications, scheduling/index events, failures, and elapsed time
  for completed work.
- Make diagnostic logging enabled by default for this release, with one
  extension setting to disable it.
- Bound log retention and avoid recording document contents or high-volume
  per-token data so logging remains useful without materially affecting editor
  responsiveness.

## Capabilities

### New Capabilities

- `diagnostic-performance-logging`: Separate, privacy-conscious extension and
  language-server diagnostic logs that support performance and support-case
  investigation.

### Modified Capabilities

- None.

## Impact

- Affects `src/extensionConfig/`, `src/languageClient/`, `src/gameData/`, and
  the Rust LSP transport/runtime layers.
- Adds a user-facing setting enabled by default and new files beneath
  `globalStorageUri/logs/`.
- Does not change Enfusion language behavior, game-data acquisition results,
  or LSP feature semantics.
