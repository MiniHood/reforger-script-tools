# server/src/ast.rs

## Purpose

Owns typed, source-backed AST wrapper views over the parser syntax tree.

## Architecture Role

This file is the first Rust AST layer. It sits above syntax parsing and below future semantic model, indexing, diagnostics, formatting, and LSP behavior. It converts parser `SyntaxNode` shapes into ergonomic declaration views without changing parser output.

## Current Behavior

The AST layer exposes `AstSourceFile` for one parsed source file and best-effort declaration wrappers for classes, enums, typedefs, functions, top-level fields/globals, class fields, class methods, constructors, destructors, parameters, attributes, modifiers, doc comments, and empty semicolon declarations. Extracted names and text are returned as span-backed slices into the original source text. Uncertain names or type text return `Option` instead of inventing semantic facts. Class, enum, function, field, and method attributes are exposed through declaration wrappers. Leading `//!` and `/*! ... */` documentation comments are exposed as raw `DocComment` values attached from immediate leading trivia; tags and comment text are not parsed or normalized. Top-level fields are returned as `Declaration::Field`, while class fields remain `ClassMember::Field`. Field extraction ignores preserved semicolons that belong to leading attribute lists before scanning the actual declaration tokens. Enum members expose source-backed explicit value text when present, but values are not evaluated or resolved. Destructor methods such as `void ~Example()` are detected through `MethodDecl::is_destructor()`, keep `name()` as `Example`, and return `void` from `return_type_text()` rather than `void ~`. Constructor classification is exposed through `ClassDecl::classify_method()` because it requires comparing a method name with the containing class name. Parameters expose raw text plus best-effort name, leading type text, default text, and top-level `out`/`inout`/`notnull` modifiers. `ref` remains part of parameter type text because it is strong-reference/type ownership syntax in Reforger sources. `ParameterKind` distinguishes real declaration parameters from preserved non-declaration callable fragments such as literal-only `false` inside declaration-shaped conditional source; `parameters()` returns declaration parameters and `parameter_fragments()` returns those preserved fragments.

## Dependencies and Boundaries

This file depends only on lexer span/token types and parser syntax types. It must not resolve symbols, evaluate type aliases, understand inheritance, inspect workspace files, read game data, call Workbench, emit diagnostics, or handle LSP requests. It must preserve the parser as the source of structure and keep all source text external.

## Change Notes

- Added the initial syntax-backed AST declaration extraction layer.
- Added span-backed text wrappers for names, type text, attributes, modifiers, parameters, and declaration spans.
- Added fixture-oriented tests that ensure committed parser fixtures expose extractable declarations.
- Fixed field name/type extraction for declarations preceded by semicolon-terminated attribute lists.
- Added destructor classification so `~` is not folded into method return type text.
- Added top-level field/global extraction through `Declaration::Field`.
- Added enum-level attribute extraction through `EnumDecl::attributes()`.
- Added source-backed enum member value extraction without semantic value evaluation.
- Added raw leading doc-comment attachment for declarations and class members without parsing documentation tags.
- Added class-context method classification for regular methods, constructors, and destructors.
- Added source-backed parameter name, type text, default text, and modifier extraction.
- Added `ParameterKind` and callable-fragment extraction so future language features do not treat preserved literal fragments as declaration parameters.

## Future Improvements

- Add richer typed wrappers as parser coverage expands into statements and expressions.
- Add a normalized type-shape API separately; current parameter and field type text remains source-faithful.
- Add a semantic model layer separately when declaration extraction is stable.
- Add workspace indexing separately; AST wrappers should remain file-local.
