# src/extensionConfig/languageClient.ts

## Purpose

Centralizes extension-owned constants for the language-client subsystem.

## Architecture Role

This file is TypeScript extension-shell configuration. It keeps language-client IDs, bundled binary folder names, log filenames, and document selectors out of runtime feature code.

## Current Behavior

Exports constants for the language client ID/name, hover-debug output channel name, language-client command IDs, server binary name, packaged server folder, development fallback path, log locations, index-cache locations, contributed language id, custom LSP request names, and language-client document selector.

The runtime game-data index cache file is `index-cache/game-data-symbol-index.v4.json`. The v4 cache is disposable and prunes external game-data local variables, strips source-only detail spans, rebuilds lookup maps after load, and preserves parameters and declaration facts needed by hover/signature-style display.

The language id is `enforce`. The document selector targets that language id; path-scoped `.c` association belongs to `package.json`, not this TypeScript constants file.

## Dependencies and Boundaries

This file has no VS Code API calls, filesystem access, mutable state, parser logic, or LSP process management. Runtime behavior belongs in `src/languageClient/languageClient.ts`.

## Change Notes

- Added centralized constants for the first bundled Rust language-server client.
- Added the centralized `enforce` language id used by the language-client selector.
- Added centralized constants for the hover-debug command and `reforger/debugHover` request.
- Added centralized storage names for the single-record hover-debug report under `logs/hover-debug/latest.md`.
- Added centralized storage names for the disposable game-data symbol index cache under `index-cache/`.

## Future Improvements

- Add future language-client command IDs or storage names here before use.
