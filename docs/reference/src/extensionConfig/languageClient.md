# src/extensionConfig/languageClient.ts

## Purpose

Centralizes extension-owned constants for the language-client subsystem.

## Ownership

This file is TypeScript extension-shell configuration. It keeps language-client IDs, bundled binary folder names, log filenames, and document selectors out of runtime feature code.

## Current Behavior

Exports constants for the language client ID/name, hover-debug and completion-debug output channel names, language-client command IDs, server binary name, packaged server folder, development fallback path, log locations, startup timing log filename, index-cache locations, contributed language id, custom LSP request/notification names, and language-client document selector. This includes the Rust-owned `reforger/blockCommentPair` typing-assist request. The completion-debug command remains the single Ctrl+F2 debug entrypoint for both autocomplete and Signature Help.

Also exports the language-client crash handling constants: the default-equivalent restart count, the restart window, and the concise final crash notification text.

Also exports deletion and insertion completion retrigger debounces. These control only when the VS Code shell asks the Rust LSP for completion after backspace/delete or typed identifier characters; they do not provide candidates or duplicate completion logic.

The runtime game-data index cache file retains the compatible path `index-cache/game-data-symbol-index.v9.bin`. Its current payload is the v10 binary cache: it prunes external game-data local variables, strips source-only detail spans, rebuilds lookup maps after load, stores repeated strings through an interned string table, preserves compacted per-file symbol ranges, stores an explicit index-shape marker, and preserves parameters, type parameters, and declaration facts needed by hover/signature-style display. Rust migrates a strictly validated v9 payload in that same path atomically to v10; invalid or stale files rebuild from source.

The language id is `enforce`. The document selector targets that language id; path-scoped `.c` association belongs to `package.json`, not this TypeScript constants file.

## Dependencies and Boundaries

This file has no VS Code API calls, filesystem access, mutable state, parser logic, or LSP process management. Runtime behavior belongs in `src/languageClient/languageClient.ts`.

## Verification

Run `npm test` after changing a constant or its consumer. Exercise the affected client lifecycle or command in a fresh Extension Development Host when the value crosses the VS Code/Rust boundary.
