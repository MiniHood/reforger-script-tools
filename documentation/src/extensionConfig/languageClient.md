# src/extensionConfig/languageClient.ts

## Purpose

Centralizes extension-owned constants for the language-client subsystem.

## Architecture Role

This file is TypeScript extension-shell configuration. It keeps language-client IDs, bundled binary folder names, log filenames, and document selectors out of runtime feature code.

## Current Behavior

Exports constants for the language client ID/name, server binary name, packaged server folder, development fallback path, log location, contributed language id, and language-client document selector.

The language id is `enforce`. The document selector targets that language id; path-scoped `.c` association belongs to `package.json`, not this TypeScript constants file.

## Dependencies and Boundaries

This file has no VS Code API calls, filesystem access, mutable state, parser logic, or LSP process management. Runtime behavior belongs in `src/languageClient/languageClient.ts`.

## Change Notes

- Added centralized constants for the first bundled Rust language-server client.
- Added the centralized `enforce` language id used by the language-client selector.

## Future Improvements

- Add future language-client command IDs or storage names here before use.
