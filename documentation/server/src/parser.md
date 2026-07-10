# server/src/parser.rs

## Purpose

Owns declaration, statement, and expression parsing for Enfusion Script source text.

## Architecture Role

This file is the Rust parser scaffold. It consumes lexer tokens and produces a full-fidelity syntax tree with parse diagnostics. It is syntax-only and exists below future AST, model, index, diagnostics, formatting, and LSP layers.

## Current Behavior

The parser structures classes, attributes, modifiers, generic class parameters, inheritance, enums, typedefs, methods/functions, fields, empty semicolon declarations, parameter lists, preprocessor directives, initializer lists, and callable body blocks. Parameter parsing is depth-aware so commas inside generic types, nested calls, brackets, brace initializer-list defaults, or default-value expressions do not split parameters. Field parsing is also depth-aware so nested brace lists inside call initializers stay part of the field declaration, while direct top-level `= { ... }` field initializers still produce `InitializerList`.

Callable bodies now parse into full-fidelity statement and expression syntax nodes instead of token-only balanced blocks. Covered body forms include control flow, `for`, `foreach`, `switch/case/default`, flow statements, local declarations, calls, member access, indexing, casts, unary/binary/assignment/ternary expressions, named arguments, `new`, `thread`, `delete`, and initializer expressions. This is syntax-only: it does not evaluate expressions, resolve overloads, infer types, evaluate preprocessor directives, or claim Workbench/compiler truth.

`ForHeader` now contains bounded `ForInitializer`, `ForCondition`, and `ForIncrement` child nodes. Initializer declarations preserve their declaration tokens while parsing default expressions where possible; condition and increment sections parse expression syntax. Existing AST local-variable extraction still recognizes `for` initializer locals from header tokens, and a later AST cleanup can read them directly from these section nodes.

Optional semicolons after method bodies and between attribute lists and decorated declarations are preserved because both forms appear in game data. Extra standalone semicolons are preserved as `EmptyDecl` nodes so future AST and index layers can ignore them explicitly. Fields may also terminate at a class closing brace for generated-source tolerance; this is parser recovery behavior and must not be treated as Workbench/compiler proof that field semicolons are optional. Fixture coverage includes larger game-data-derived editor and Workbench class excerpts.

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
- Added full-fidelity statement and expression parsing for callable bodies.

## Future Improvements

- Move AST local-variable extraction from token scanning to the new statement syntax after body parsing stabilizes.
- Update resolver hover to consume expression syntax for receiver/member chains in a later slice.
