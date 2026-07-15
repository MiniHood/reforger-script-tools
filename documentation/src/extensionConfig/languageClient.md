# src/extensionConfig/languageClient.ts

## Purpose

Centralizes extension-owned constants for the language-client subsystem.

## Architecture Role

This file is TypeScript extension-shell configuration. It keeps language-client IDs, bundled binary folder names, log filenames, and document selectors out of runtime feature code.

## Current Behavior

Exports constants for the language client ID/name, hover-debug and completion-debug output channel names, language-client command IDs, server binary name, packaged server folder, development fallback path, log locations, startup timing log filename, index-cache locations, contributed language id, custom LSP request/notification names, and language-client document selector.

Also exports the language-client crash handling constants: the default-equivalent restart count, the restart window, and the concise final crash notification text.

Also exports the deletion-completion retrigger debounce. This controls only when the VS Code shell asks the Rust LSP for completion after backspace/delete; it does not provide candidates or duplicate completion logic.

The runtime game-data index cache file is `index-cache/game-data-symbol-index.v9.bin`. The v9 binary cache is disposable and prunes external game-data local variables, strips source-only detail spans, rebuilds lookup maps after load, stores repeated strings through an interned string table, preserves compacted per-file symbol ranges, stores an explicit index-shape marker, and preserves parameters, type parameters, and declaration facts needed by hover/signature-style display.

The language id is `enforce`. The document selector targets that language id; path-scoped `.c` association belongs to `package.json`, not this TypeScript constants file.

## Dependencies and Boundaries

This file has no VS Code API calls, filesystem access, mutable state, parser logic, or LSP process management. Runtime behavior belongs in `src/languageClient/languageClient.ts`.

## Change Notes

- Added centralized constants for the first bundled Rust language-server client.
- Added the centralized `enforce` language id used by the language-client selector.
- Added centralized constants for the hover-debug command and `reforger/debugHover` request.
- Added centralized constants for the completion-debug command and `reforger/debugCompletion` request.
- Added the centralized hover symbol-link command used by Rust-generated trusted Markdown links.
- Added centralized constants for workspace overlay file-change notifications.
- Added centralized storage names for the single-record hover-debug report under `logs/hover-debug/latest.md`.
- Added centralized storage names for the single-record completion-debug report under `logs/completion-debug/latest.md`.
- Added centralized storage names for the disposable game-data symbol index cache under `index-cache/`.
- Updated the disposable game-data cache file to v8 after replacing the JSON payload with the binary runtime cache format.
- Updated the disposable game-data cache file to v9 after adding binary string-table storage.
- Added centralized crash handling constants so repeated language-server crashes show a short user-facing notification while keeping detailed output in logs.
- Added the deletion-completion retrigger debounce constant for Enforce editor integration.
- Added the centralized TypeScript startup timing log filename `logs/language-client-startup.log`.

## Future Improvements

- Add future language-client command IDs or storage names here before use.
