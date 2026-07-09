# package.json

## Purpose

Defines the VS Code extension manifest, contribution points, commands, settings, dependencies, and build/test scripts.

## Architecture Role

This file is the public VS Code extension contract. It tells VS Code how to activate the extension, which commands and settings exist, and which language ids the extension contributes.

## Current Behavior

The manifest contributes the `enforce` language id for Reforger/Enfusion Script. It associates only `.c` files under `Scripts/` or `scripts/` path segments through `filenamePatterns`; it does not claim the `.c` extension globally. This keeps ordinary C files owned by C tooling while Reforger script files can use the Rust language server.

The manifest also contributes game-data commands and the manual game-data folder setting.

## Dependencies and Boundaries

Do not add global `.c` language association for Enforce. Do not add user-facing settings unless they are real end-user controls. Runtime dependencies must remain bundled and invisible to marketplace users.

## Change Notes

- Added the `enforce` language contribution with path-scoped Reforger script file association.

## Future Improvements

- Add syntax highlighting only when a real grammar slice is planned and verified.
