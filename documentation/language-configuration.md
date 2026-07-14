# language-configuration.json

## Purpose

Provides minimal VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment, bracket, and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, braces, brackets, parentheses, and double-quote auto-closing/surrounding pairs. Enforce coloring is intentionally not handled here; it comes from Rust LSP semantic tokens and the bundled theme.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, syntax highlighting, or broad formatting rules.

## Change Notes

- Added initial editor configuration for the `enforce` language contribution.
- Clarified that coloring is owned by semantic tokens, not language configuration or TextMate grammar.

## Future Improvements

- Add indentation or folding rules only after they are source-backed and tested against real Reforger script patterns.
