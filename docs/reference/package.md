# package.json

## Purpose

Defines the VS Code extension manifest, contribution points, commands, settings, dependencies, and build/test scripts.

## Architecture Role

This file is the public VS Code extension contract. It tells VS Code how to activate the extension, which commands and settings exist, and which language ids the extension contributes.

## Current Behavior

The manifest contributes the `enforce` language id for Reforger/Enfusion Script. It associates only `.c` files under `Scripts/` or `scripts/` path segments through `filenamePatterns`; it does not claim the `.c` extension globally. This keeps ordinary C files owned by C tooling while Reforger script files can use the Rust language server.

The manifest contributes the `Reforger Enforce Dark` theme and the lightweight language configuration. It does not contribute a TextMate grammar. Enforce token coloring is owned by Rust LSP semantic tokens plus the theme's `semanticTokenColors`.

The manifest sets language-specific defaults for `enforce`: `editor.autoIndent = full`, `editor.bracketPairColorization.enabled = false`, comment/string quick suggestions disabled, normal-code quick suggestions enabled, Enter acceptance disabled (Tab remains the standard acceptance key), keyword suggestion display enabled, and word-based suggestions disabled. Full auto indentation is required for VS Code to evaluate the language configuration's paired `onEnterRules`, which provide immediate unbraced if-family body indentation. The bracket settings prevent VS Code's built-in bracket-pair/matching overlays from coloring brackets inside comments, since Enforce deliberately has no TextMate grammar scopes and Rust semantic tokens are the single authoritative coloring path. Lightweight auto-closing pairs still come from `language-configuration.json`, including duplicate-closer prevention through `autoCloseBefore`. The suggestion settings keep autocomplete owned by the Rust language server instead of mixing in plain VS Code word suggestions, especially in comments. Keyword display stays enabled so source-owned LSP keyword items such as `if`, `override`, `return`, and `static` are not hidden by broader editor settings. Accepting `if` with Tab inserts Rust-authored empty parentheses and places the cursor inside them; Enter remains a line break rather than a completion acceptance key.

The manifest also contributes game-data commands, the manual game-data folder setting, and the developer-facing hover debug command. `Reforger Script Tools: Debug Hover At Cursor` is bound to `Ctrl+F1` only when an Enforce editor has focus. It asks the running Rust language server for a targeted cursor-position hover report.

## Dependencies and Boundaries

Do not add global `.c` language association for Enforce. Do not add user-facing settings unless they are real end-user controls. Runtime dependencies must remain bundled and invisible to marketplace users.

## Verification

Run `npm test` after changing contributions, commands, settings, dependencies,
or scripts. In an Extension Development Host, confirm that only `Scripts/*.c`
files select the Enforce language mode and that each changed contribution is
available through VS Code.

## Future Improvements

- Add additional theme variants only through semantic-token theme files unless a future feature explicitly needs another presentation mechanism.
