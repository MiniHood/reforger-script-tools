# syntaxes/enforce.tmLanguage.json

## Purpose

Defines the first TextMate grammar for the `enforce` language id so Reforger/Enfusion Script files get readable baseline syntax highlighting.

## Architecture Role

This grammar is editor presentation support. It gives VS Code scopes that themes can color before the Rust language server provides richer semantic tokens. It is intentionally separate from parser, AST, model, index, and LSP logic.

## Current Behavior

The grammar scopes comments, documentation comments, preprocessor lines, strings, attributes, declaration names, keywords, primitive and PascalCase-like types, function-like identifiers, numbers, and punctuation/operators.

## Dependencies and Boundaries

The grammar is declarative JSON consumed by VS Code. It must not encode semantic truth, workspace lookup, inheritance, or compiler validation. Keep complex language intelligence in Rust semantic-token/LSP features later.

## Change Notes

- Added baseline `source.enforce` scopes for theme coloring.
- Fixed the punctuation/operator regex so the grammar can emit child scopes instead of falling back to only `source.enforce`.

## Future Improvements

- Replace or supplement broad regex scopes with Rust semantic tokens when editor-facing semantic highlighting is implemented.
- Add token-scope reports/tests if grammar complexity grows.
