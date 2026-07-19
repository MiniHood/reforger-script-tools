## Why

Editing a Reforger script replaces resolver-backed semantic colors with a
weaker lexical response before current analysis finishes. External types such
as `SCR_GameModeEndData` visibly change from their semantic color to white and
then back, making normal typing distracting in large files.

## What Changes

- Keep the editor's existing semantic-token display stable while a newer
  document revision is being analyzed.
- Publish a semantic-token response for an edited document only when the
  matching current revision has rich, resolver-backed token facts.
- Cancel or supersede pending token responses on newer edits without returning
  stale token ranges or reintroducing fixed idle delays.
- Retain a safe lexical response only when rich coloring is unavailable for an
  initial document or cannot be produced.

## Capabilities

### New Capabilities

- `semantic-token-color-stability`: revision-safe delivery of rich semantic
  tokens without interim visual color loss during edits.

### Modified Capabilities

- None.

## Impact

The Rust LSP semantic-token request lifecycle, token cache, refresh/cancellation
handling, semantic-token tests, and their reference documentation are affected.
The VS Code extension remains transport-only; no game-data format or user
setting changes are required.
