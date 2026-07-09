# tools/build-language-server.mjs

## Purpose

Builds the current-platform Rust language server binary and copies it into the extension distribution folder.

## Architecture Role

This is repo-only developer/build tooling. It bridges Cargo output into the VS Code extension package layout so marketplace users receive a bundled binary and do not need Rust installed.

## Current Behavior

The script runs Cargo for the `reforger_language_server` binary, using debug mode by default or release mode with `--release`. It copies the resulting executable to `dist/server/<platform>-<arch>/reforger_language_server(.exe)` and marks it executable on non-Windows platforms.

## Dependencies and Boundaries

Uses Node built-in modules and local Cargo. It must not become runtime extension code, register a VS Code command, download game data, or manage cross-platform release matrices. `tools/` remains excluded from VSIX packages.

## Change Notes

- Added the build helper for the first bundled Rust LSP server path.

## Future Improvements

- Add explicit cross-target packaging only when release automation needs multi-platform VSIX artifacts.
