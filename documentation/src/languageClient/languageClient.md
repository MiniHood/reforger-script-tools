# src/languageClient/languageClient.ts

## Purpose

Owns VS Code-side startup and shutdown of the bundled Rust language server.

## Architecture Role

This file is TypeScript shell code. It resolves the packaged or development server binary, configures `vscode-languageclient`, passes extension-owned paths to the server, and starts the LSP process. Serious language intelligence remains in Rust.

## Current Behavior

On activation, the module creates a VS Code log output channel, resolves `dist/server/<platform>-<arch>/reforger_language_server(.exe)` first, and falls back to `server/target/debug/reforger_language_server(.exe)` for development. It starts the server over stdio for files matching `**/{Scripts,scripts}/**/*.c`.

The client passes `globalStorageUri/logs/language-server.log` and the resolved game-data scripts path to the server. The game-data path uses the manual-folder setting when present, otherwise the downloaded global-storage `game-data/scripts` folder.

## Dependencies and Boundaries

Uses VS Code APIs, Node path/filesystem APIs, `vscode-languageclient`, and extension config constants. It must not parse Enfusion Script, build indexes, inspect symbols, or implement language features directly.

## Change Notes

- Added the first VS Code language-client startup path for the bundled Rust LSP server.
- Kept document selection conservative so the extension does not claim every `.c` file globally.

## Future Improvements

- Add user-facing restart/status commands only after there is a concrete need.
- Add richer runtime logging controls after the language server owns more expensive work.
- Pass workspace roots once runtime workspace indexing is implemented.
