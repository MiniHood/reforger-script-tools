# language-configuration.json

## Purpose

Provides minimal VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, and brace/bracket/paren/double-quote auto-closing/surrounding pairs. It intentionally does not define editor `brackets`; VS Code's bracket-pair and matching-bracket layers do not have Enforce TextMate comment scopes and can color bracket characters inside comments. Enforce coloring is owned by Rust LSP semantic tokens and the bundled theme.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, syntax highlighting, or broad formatting rules.

## Change Notes

- Added initial editor configuration for the `enforce` language contribution.
- Clarified that coloring is owned by semantic tokens, not language configuration or TextMate grammar.
- Removed editor `brackets` from the Enforce language configuration so VS Code's bracket-pair/matching layer does not color bracket characters inside comments.

## Future Improvements

- Add indentation or folding rules only after they are source-backed and tested against real Reforger script patterns.
