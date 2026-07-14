# package.json

## Purpose

Defines the VS Code extension manifest, contribution points, commands, settings, dependencies, and build/test scripts.

## Architecture Role

This file is the public VS Code extension contract. It tells VS Code how to activate the extension, which commands and settings exist, and which language ids the extension contributes.

## Current Behavior

The manifest contributes the `enforce` language id for Reforger/Enfusion Script. It associates only `.c` files under `Scripts/` or `scripts/` path segments through `filenamePatterns`; it does not claim the `.c` extension globally. This keeps ordinary C files owned by C tooling while Reforger script files can use the Rust language server.

The manifest contributes the `Reforger Enforce Dark` theme and the lightweight language configuration. It does not contribute a TextMate grammar. Enforce token coloring is owned by Rust LSP semantic tokens plus the theme's `semanticTokenColors`.

The manifest sets the language-specific default `editor.bracketPairColorization.enabled = false` for `enforce`. The language configuration also avoids contributing editor bracket pairs. Together these prevent VS Code's built-in bracket-pair/matching overlays from coloring brackets inside comments, since Enforce deliberately has no TextMate grammar scopes and Rust semantic tokens are the single authoritative coloring path.

The manifest also contributes game-data commands, the manual game-data folder setting, and the developer-facing hover debug command. `Reforger Script Tools: Debug Hover At Cursor` is bound to `Ctrl+F1` only when an Enforce editor has focus. It asks the running Rust language server for a targeted cursor-position hover report.

## Dependencies and Boundaries

Do not add global `.c` language association for Enforce. Do not add user-facing settings unless they are real end-user controls. Runtime dependencies must remain bundled and invisible to marketplace users.

## Change Notes

- Added the `enforce` language contribution with path-scoped Reforger script file association.
- Added the hover debug command and `Ctrl+F1` keybinding scoped to Enforce editors.
- Removed the TextMate grammar contribution so Enforce coloring has one path: Rust semantic tokens.
- Disabled VS Code bracket-pair colorization by default for Enforce documents and stopped contributing editor bracket pairs so brackets inside comments remain comment-colored by semantic tokens.

## Future Improvements

- Add additional theme variants only through semantic-token theme files unless a future feature explicitly needs another presentation mechanism.
