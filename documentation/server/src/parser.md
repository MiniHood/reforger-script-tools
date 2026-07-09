# server/src/parser.rs

## Purpose

Owns declaration-level parsing for Enfusion Script source text.

## Architecture Role

This file is the Rust parser scaffold. It consumes lexer tokens and produces a full-fidelity syntax tree with parse diagnostics. It is syntax-only and exists below future AST, model, index, diagnostics, formatting, and LSP layers.

## Current Behavior

The parser structures classes, attributes, modifiers, generic class parameters, inheritance, enums, typedefs, methods/functions, fields, parameter lists, preprocessor directives, initializer lists, and balanced blocks. Parameter parsing is depth-aware so commas inside generic types, nested calls, brackets, or default-value expressions do not split parameters. Method bodies are preserved as token-containing block nodes instead of being parsed as statements or expressions. Fixture coverage includes larger game-data-derived editor and Workbench class excerpts.

Lexer error tokens are forwarded as parse diagnostics. Parser recovery records diagnostics for malformed constructs and keeps returning a source-file tree instead of panicking.

## Dependencies and Boundaries

The parser depends on the lexer and syntax modules only. It must not resolve symbols, infer types, evaluate preprocessor directives, parse Workbench/compiler behavior, index files, or handle LSP requests.

## Change Notes

- Added initial declaration-level parser scaffold and parser fixture tests.
- Added depth-aware parameter parsing and distinct initializer-list syntax nodes for field initializers.
- Added larger editor preview and Workbench formatter fixtures to parser preservation tests.

## Future Improvements

- Add statement and expression parsing in separate verified slices.
- Add typed AST wrappers over syntax nodes after declaration parsing stabilizes.
- Add corpus-level parser reporting once fixture behavior is stable.
