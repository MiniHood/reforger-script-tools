# language-configuration.json

## Purpose

Provides minimal VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment, bracket, and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, braces, brackets, parentheses, and double-quote auto-closing/surrounding pairs. Enforce coloring is intentionally not handled here; it comes from Rust LSP semantic tokens and the bundled theme. Bracket-pair colorization is disabled for Enforce in `package.json` because it is a separate VS Code color overlay and can color bracket characters inside comments when no TextMate scopes are present.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, syntax highlighting, or broad formatting rules.

## Change Notes

- Added initial editor configuration for the `enforce` language contribution.
- Clarified that coloring is owned by semantic tokens, not language configuration or TextMate grammar.
- Clarified that bracket-pair colorization is controlled by the manifest default, not this language configuration.

## Future Improvements

- Add indentation or folding rules only after they are source-backed and tested against real Reforger script patterns.
