# server/src/parser.rs

## Purpose

Owns declaration, statement, and expression parsing for Enfusion Script source text.

## Architecture Role

This file is the Rust parser scaffold. It consumes lexer tokens and produces a full-fidelity syntax tree with parse diagnostics. It is syntax-only and exists below future AST, model, index, diagnostics, formatting, and LSP layers.

## Current Behavior

The parser structures classes, attributes, modifiers, generic class parameters, inheritance, enums, typedefs, methods/functions, fields, empty semicolon declarations, parameter lists, preprocessor directives, initializer expressions, and callable body blocks. Parameter parsing is depth-aware so commas inside generic types, nested calls, brackets, brace initializer-list defaults, or default-value expressions do not split parameters. Field parsing is also depth-aware so nested brace initializer expressions stay part of the field declaration, while direct top-level `= { ... }` field initializers produce structured `InitializerExpression` nodes rather than token-preserved lists.

Callable bodies and attribute argument lists now parse into full-fidelity statement and expression syntax nodes instead of token-only balanced blocks. Covered body forms include control flow, `for`, `foreach`, `switch/case/default`, flow statements, local declarations, calls, member access, indexing, casts, unary/binary/assignment/ternary expressions, named arguments, `new`, `thread`, `delete`, and initializer expressions. Attribute arguments reuse the same expression and named-argument parser so values such as `UIWidgets.ComboBox` and `ParamEnumArray.FromEnum(...)` are structurally visible to AST/resolver tooling. This is syntax-only: it does not evaluate expressions, resolve overloads, infer types, evaluate preprocessor directives, or claim Workbench/compiler truth.

`ForHeader` contains bounded `ForInitializer`, `ForCondition`, and `ForIncrement` child nodes. Declaration-shaped initializers are represented as a nested `LocalDeclStatement` inside `ForInitializer`; expression-form initializers remain expression-list syntax. Condition and increment sections parse expression syntax. AST local-variable extraction reads `for` initializer locals from the nested declaration node.

`ForeachHeader` contains `ForeachVariableList`, one `ForeachVariable` per declared header variable, and `ForeachIterable` for the iterable expression after the top-level colon. The parser preserves typed variables, `auto`, and index/value pairs without assigning semantic meaning.

`SwitchStatement` groups each run of `case`/`default` labels plus following statements into a `SwitchSection`. `CaseClause` and `DefaultClause` remain label nodes inside the section. This is syntax grouping for future folding, scope, and control-flow groundwork; it is not fallthrough or reachability analysis.

Optional semicolons after method bodies and between attribute lists and decorated declarations are preserved because both forms appear in game data. Extra standalone semicolons are preserved as `EmptyDecl` nodes so future AST and index layers can ignore them explicitly. Fields may also terminate at a class closing brace for generated-source tolerance; this is parser recovery behavior and must not be treated as Workbench/compiler proof that field semicolons are optional. Fixture coverage includes larger game-data-derived editor and Workbench class excerpts.

Lexer error tokens are forwarded as parse diagnostics. Parser recovery records diagnostics for malformed constructs and keeps returning a source-file tree instead of panicking.

Malformed declaration-context text is recovered as a bounded `Error` node. Recovery stops at semicolons, braces, EOF, or the next clear declaration-start keyword such as `class`, `enum`, `typedef`, `modded`, or `vanilla`, so invalid top-level text does not swallow later real declarations.

Corpus reports classify the known `Game\game.c` `#ifdef BREAK_COMPILATION` invalid branch as expected preprocessor-test recovery when the preserved source matches that pattern. This is review evidence only and does not evaluate macros.

## Dependencies and Boundaries

The parser depends on the lexer and syntax modules only. It must not resolve symbols, infer types, evaluate preprocessor directives, parse Workbench/compiler behavior, index files, or handle LSP requests.

## Change Notes

- Added initial declaration-level parser scaffold and parser fixture tests.
- Added depth-aware parameter parsing and structured initializer-expression syntax nodes for field initializers.
- Added larger editor preview and Workbench formatter fixtures to parser preservation tests.
- Added support for game-data-observed optional semicolons after callable bodies and after attribute lists.
- Added depth-aware field initializer parsing for call expressions that contain nested brace initializer expressions.
- Added brace-depth-aware parameter parsing for defaults such as static-array vector initializer lists.
- Added `EmptyDecl` preservation for standalone semicolons in declaration context.
- Added generated-source tolerance for field declarations that reach a class closing brace without a semicolon.
- Added full-fidelity statement and expression parsing for callable bodies.
- Structured declaration-form `for` initializers, `foreach` headers, and switch sections.
- Structured attribute argument lists with expression and named-argument children instead of preserving them as token-only balanced parens.
- Tightened declaration-context error recovery so invalid top-level text stops before the next real declaration.
- Tightened local-declaration detection so compound assignment expression statements such as `addonsDir += absPath;` are not parsed as `LocalDeclStatement`.
- Replaced token-preserved field `InitializerList` nodes with the same structured `InitializerExpression` shape used by body/local initializer expressions so resolver-backed tooling can consume enum/static values inside field defaults.

## Future Improvements

- Update resolver hover to consume expression syntax for receiver/member chains in a later slice.
