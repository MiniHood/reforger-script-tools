# language-configuration.json

## Purpose

Provides minimal VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, and brace/bracket/paren/double-quote auto-closing/surrounding pairs. It narrows `autoCloseBefore` so typing `{`, `[`, or `(` immediately before an existing closing bracket does not create a duplicate closer. It intentionally does not define editor `brackets`; VS Code's bracket-pair and matching-bracket layers do not have Enforce TextMate comment scopes and can color bracket characters inside comments. Enforce coloring is owned by Rust LSP semantic tokens and the bundled theme.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, syntax highlighting, or broad formatting rules.

## Verification

Run `npm test` after changing this configuration. In an Extension Development
Host, verify comment insertion, auto-closing behavior, and that bracket
characters inside comments retain comment coloring.

## Future Improvements

- Add indentation, folding, or richer bracket typing rules only after they are source-backed and tested against real Reforger script patterns.
