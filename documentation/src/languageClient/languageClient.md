# src/languageClient/languageClient.ts

## Purpose

Owns VS Code-side startup and shutdown of the bundled Rust language server.

## Architecture Role

This file is TypeScript shell code. It resolves the packaged or development server binary, configures `vscode-languageclient`, passes extension-owned paths to the server, and starts the LSP process. Serious language intelligence remains in Rust.

## Current Behavior

On activation, the module creates a VS Code log output channel and a separate hover-debug output channel. In Extension Development Host mode, it resolves `server/target/debug/reforger_language_server(.exe)` first so newly compiled server changes are used without depending on the packaged `dist` copy. Outside development mode, it resolves `dist/server/<platform>-<arch>/reforger_language_server(.exe)` first and falls back to the development binary only when needed. It starts the server over stdio for file documents whose VS Code language id is `enforce`.

The `enforce` language id is contributed through `package.json` and is path-associated only for `.c` files under `Scripts/` or `scripts/`. The language client should target the language id, not duplicate path-glob logic.

The client passes `globalStorageUri/logs/language-server.log`, `globalStorageUri/index-cache/game-data-symbol-index.v1.json`, and the resolved game-data scripts path to the server. The game-data path uses the manual-folder setting when present, otherwise the downloaded global-storage `game-data/scripts` folder. For downloaded game data, the client also passes `game-data/metadata.json` so the Rust cache can invalidate by commit SHA. For manual folders, metadata is omitted and Rust uses a file-metadata fingerprint.

The module also registers `Reforger Script Tools: Debug Hover At Cursor`. The command sends the active Enforce editor URI and cursor position to the Rust server through the custom `reforger/debugHover` request, writes the returned report to the hover-debug output channel, and overwrites `globalStorageUri/logs/hover-debug/latest.md`. That file is a single-record debug artifact; each command run replaces it completely so Codex and humans have one stable place to inspect the latest hover/AST pipeline state. TypeScript does not inspect source text or duplicate language analysis.

## Dependencies and Boundaries

Uses VS Code APIs, Node path/filesystem APIs, `vscode-languageclient`, and extension config constants. It must not parse Enfusion Script, build indexes, inspect symbols, or implement language features directly.

## Change Notes

- Added the first VS Code language-client startup path for the bundled Rust LSP server.
- Kept document selection conservative so the extension does not claim every `.c` file globally.
- Switched the client document selector to the contributed `enforce` language id.
- Added the cursor-position hover debug command that delegates analysis to the Rust server.
- Changed development-host server resolution to prefer `server/target/debug` before the packaged `dist` binary, avoiding stale custom-request behavior while iterating on Rust LSP code.
- Added global-storage hover-debug report writing to `logs/hover-debug/latest.md`, overwriting the file on each command run.
- Added game-data index cache and metadata paths to the server launch arguments so runtime hover can use a cached external game-data index.

## Future Improvements

- Add user-facing restart/status commands only after there is a concrete need.
- Add richer runtime logging controls after the language server owns more expensive work.
- Pass workspace roots once runtime workspace indexing is implemented.
