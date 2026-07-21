# language-configuration.json

## Purpose

Provides native VS Code editing behavior for the `enforce` language id.

## Architecture Role

This file belongs to the VS Code shell contribution layer. It gives VS Code comment and auto-closing-pair behavior before richer language features come from the Rust LSP.

## Current Behavior

The configuration defines `//` line comments, `/* */` block comments, and a
native `/*` -> `*/` auto-closing pair. VS Code owns that immediate delimiter
pairing, so it respects the user's standard autoclose preferences. The Rust
typing assist receives only VS Code's precise native `**/` pair-change event,
never general `*` typing, and replaces a proven empty standalone pair with a
three-line comment plus Rust-authored interior caret position. The pair is
suppressed in editor-recognized string and comment contexts. Brace/bracket/paren and
double-quote pairs remain unchanged. It narrows `autoCloseBefore` so typing
`{`, `[`, or `(` immediately before an existing closing bracket does not create
a duplicate closer. It intentionally does not define editor `brackets`; VS
Code's bracket-pair and matching-bracket layers do not have Enforce TextMate
comment scopes and can color bracket characters inside comments. Enforce
coloring is owned by Rust LSP semantic tokens and the bundled theme.

Its indentation configuration has one coordinated native path: a paired
`onEnterRules` contract. The first rule gives VS Code immediate extra
indentation after a standalone `if (...)`, `else if (...)`, or `else` header
with no braces, semicolon, comment, or trailing body. The second rule
outdents only when that header is the physical previous line and the current
line is a non-comment body statement. This prevents blank lines from carrying
a temporary control-body indentation into unrelated later lines. It does not
inspect language semantics, insert braces, or use a language-server request.

## Dependencies and Boundaries

This file must stay lightweight. It must not encode parser semantics, Workbench truth, syntax highlighting, or broad formatting rules.

## Verification

Run `npm test` after changing this configuration. In an Extension Development
Host, verify comment insertion, auto-closing behavior, native unbraced
if-family indentation, and that bracket characters inside comments retain
comment coloring.

## Future Improvements

- Add Rust-owned comment Enter continuation as a separate assist only after a
  focused editor journey proves the paired block's final caret and newline
  behavior. Do not turn the explicit comment formatter into an on-type fallback.
- Add another control form, folding, or richer bracket typing rule only after
  it is source-backed and tested against real Reforger script patterns.
