# themes/reforger-enforce-dark-color-theme.json

## Purpose

Defines the bundled `Reforger Enforce Dark` color theme for readable Enforce script editing.

## Architecture Role

This is presentation-only VS Code theme data. It maps TextMate scopes and future semantic token names to a fixed Enforce color standard without adding user settings or embedding colors in runtime language logic.

## Current Behavior

The theme uses only the provided color set:

- class/enum/type: `#40b5ac`
- function/method: `#f3ad58`
- variable/parameter/number: `#cfcfcf`
- keyword: `#59A6E9`
- comment: `#59aa59`
- string: `#c178dd`
- punctuation: `#bfbfbf`
- preprocessor: `#d4fd95`

Hover-debug output includes an expected token coloring table derived from the lexer and this palette. That table is a troubleshooting aid for theme and grammar work; it is not a direct VS Code token-inspector API result.

## Dependencies and Boundaries

The theme depends on VS Code theme contribution support and the `source.enforce` grammar scopes. It must not add settings, commands, parser behavior, or LSP behavior.

## Change Notes

- Added the first bundled Enforce theme using the user-provided palette.
- Connected hover-debug review expectations to the same fixed palette so color issues can be inspected from `logs/hover-debug/latest.md`.

## Future Improvements

- Add additional generated/custom themes only after a palette-generation workflow is explicitly needed.
- Tune colors only through theme files, not runtime code.
