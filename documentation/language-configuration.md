# language-configuration.json

## Purpose

Provides minimal VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment, bracket, and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, braces, brackets, parentheses, and double-quote auto-closing/surrounding pairs.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, or broad formatting rules. Syntax highlighting is intentionally out of scope for this slice.

## Change Notes

- Added initial editor configuration for the `enforce` language contribution.

## Future Improvements

- Add indentation or folding rules only after they are source-backed and tested against real Reforger script patterns.
