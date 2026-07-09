# src/extensionConfig/languageClient.ts

## Purpose

Centralizes extension-owned constants for the language-client subsystem.

## Architecture Role

This file is TypeScript extension-shell configuration. It keeps language-client IDs, bundled binary folder names, log filenames, and document selectors out of runtime feature code.

## Current Behavior

Exports constants for the language client ID/name, server binary name, packaged server folder, development fallback path, log location, and conservative Reforger script document selector.

## Dependencies and Boundaries

This file has no VS Code API calls, filesystem access, mutable state, parser logic, or LSP process management. Runtime behavior belongs in `src/languageClient/languageClient.ts`.

## Change Notes

- Added centralized constants for the first bundled Rust language-server client.

## Future Improvements

- Add future language-client command IDs or storage names here before use.
