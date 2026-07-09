# server/src/bin/reforger_language_server.rs

## Purpose

Provides the executable entrypoint for the bundled Rust language server binary.

## Architecture Role

This binary is the runtime process launched by the VS Code TypeScript language client. It delegates protocol behavior to `server/src/lsp.rs` and keeps command-line handling minimal.

## Current Behavior

The binary accepts:

- `--log <path>` for language-server log output.
- `--game-data-scripts <path>` for game-data source provenance.

It starts the stdio LSP loop and exits with a nonzero status if the server returns an error.

## Dependencies and Boundaries

This file must stay thin. Do not add parser, index, Workbench, VS Code, cache, or feature logic here. New runtime behavior belongs in `server/src/lsp.rs` or lower language-engine layers.

## Change Notes

- Added the first binary entrypoint for packaging a self-contained language server with the VS Code extension.

## Future Improvements

- Add explicit version or diagnostic command-line flags only if packaging/debug workflows require them.
