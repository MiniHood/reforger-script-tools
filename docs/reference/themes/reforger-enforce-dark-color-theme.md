# themes/reforger-enforce-dark-color-theme.json

## Purpose

Defines the bundled `Reforger Enforce Dark` color theme for readable Enforce script editing.

## Ownership

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

## Verification

Run `npm test`, then inspect semantic-token coloring in a fresh Extension Development Host. Use the hover-debug token table only as a Rust/theme consistency aid; VS Code remains the rendering authority.
