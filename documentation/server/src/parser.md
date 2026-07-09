# server/src/parser.rs

## Purpose

Owns declaration-level parsing for Enfusion Script source text.

## Architecture Role

This file is the Rust parser scaffold. It consumes lexer tokens and produces a full-fidelity syntax tree with parse diagnostics. It is syntax-only and exists below future AST, model, index, diagnostics, formatting, and LSP layers.

## Current Behavior

The parser structures classes, attributes, modifiers, generic class parameters, inheritance, enums, typedefs, methods/functions, fields, empty semicolon declarations, parameter lists, preprocessor directives, initializer lists, and balanced blocks. Parameter parsing is depth-aware so commas inside generic types, nested calls, brackets, brace initializer-list defaults, or default-value expressions do not split parameters. Field parsing is also depth-aware so nested brace lists inside call initializers stay part of the field declaration, while direct top-level `= { ... }` field initializers still produce `InitializerList`. Method bodies are preserved as token-containing block nodes instead of being parsed as statements or expressions. Optional semicolons after method bodies and between attribute lists and decorated declarations are preserved because both forms appear in game data. Extra standalone semicolons are preserved as `EmptyDecl` nodes so future AST and index layers can ignore them explicitly. Fields may also terminate at a class closing brace for generated-source tolerance; this is parser recovery behavior and must not be treated as Workbench/compiler proof that field semicolons are optional. Fixture coverage includes larger game-data-derived editor and Workbench class excerpts.

Lexer error tokens are forwarded as parse diagnostics. Parser recovery records diagnostics for malformed constructs and keeps returning a source-file tree instead of panicking.

## Dependencies and Boundaries

The parser depends on the lexer and syntax modules only. It must not resolve symbols, infer types, evaluate preprocessor directives, parse Workbench/compiler behavior, index files, or handle LSP requests.

## Change Notes

- Added initial declaration-level parser scaffold and parser fixture tests.
- Added depth-aware parameter parsing and distinct initializer-list syntax nodes for field initializers.
- Added larger editor preview and Workbench formatter fixtures to parser preservation tests.
- Added support for game-data-observed optional semicolons after callable bodies and after attribute lists.
- Added depth-aware field initializer parsing for call expressions that contain nested brace initializer lists.
- Added brace-depth-aware parameter parsing for defaults such as static-array vector initializer lists.
- Added `EmptyDecl` preservation for standalone semicolons in declaration context.
- Added generated-source tolerance for field declarations that reach a class closing brace without a semicolon.

## Future Improvements

- Add statement and expression parsing in separate verified slices.
- Add typed AST wrappers over syntax nodes after declaration parsing stabilizes.
- Add corpus-level parser reporting once fixture behavior is stable.
