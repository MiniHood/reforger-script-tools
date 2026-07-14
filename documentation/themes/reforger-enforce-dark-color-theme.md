# themes/reforger-enforce-dark-color-theme.json

## Purpose

Defines the bundled `Reforger Enforce Dark` color theme for readable Enforce script editing.

## Architecture Role

This is presentation-only VS Code theme data. It maps Rust LSP semantic token names to a fixed Enforce color standard without adding user settings or embedding colors in runtime language logic.

## Current Behavior

The theme enables semantic highlighting and uses only the provided color set through `semanticTokenColors`:

- class/enum/type/type parameter: `#40b5ac`
- function/method: `#f3ad58`
- enum member/variable/field/parameter/number: `#cfcfcf`
- keyword: `#59A6E9`
- comment: `#59aa59`
- string: `#c178dd`
- operator/punctuation: `#bfbfbf`
- preprocessor/decorator: `#d4fd95`

Hover-debug output includes a semantic-token coloring table derived from the same Rust LSP semantic-token builder and this palette. That table is a troubleshooting aid for theme work; it is not a direct VS Code token-inspector API result.

Language primitive/value words such as `int`, `void`, `float`, `bool`, `typename`, `true`, and `false` are emitted as `keyword` by the language server and therefore use the keyword color. Source-backed type-position words such as `string`, `vector`, `array`, `map`, `set`, `ResourceName`, `LocalizedString`, `Curve`, and `Color` are emitted through source-backed type facts and use the type/class color. Enum owners use the enum color, while enum-member values use the variable color.

## Dependencies and Boundaries

The theme depends on VS Code theme contribution support and the semantic token legend advertised by the Rust language server. It must not add settings, commands, parser behavior, or LSP behavior.

## Change Notes

- Added the first bundled Enforce theme using the user-provided palette.
- Connected hover-debug review expectations to the same fixed palette so color issues can be inspected from `logs/hover-debug/latest.md`.
- Removed TextMate token colors; Enforce coloring now comes from semantic tokens only.
- Adjusted enum-member values to use the variable color and allowed source-backed `string` / `vector` type positions to use type-family coloring while keeping `bool` / `int` / `float` / `typename` keyword-blue.
- Switched field semantic tokens from the generic `property` name to the Enforce-facing `field` name so theme, reports, hover, and backend symbol terminology match.

## Future Improvements

- Add additional generated/custom themes only after a palette-generation workflow is explicitly needed.
- Tune colors only through theme files, not runtime code.
