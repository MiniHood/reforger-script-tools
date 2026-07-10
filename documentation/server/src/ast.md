# server/src/ast.rs

## Purpose

Owns typed, source-backed AST wrapper views over the parser syntax tree.

## Architecture Role

This file is the first Rust AST layer. It sits above syntax parsing and below future semantic model, indexing, diagnostics, formatting, and LSP behavior. It converts parser `SyntaxNode` shapes into ergonomic declaration views without changing parser output.

## Current Behavior

The AST layer exposes `AstSourceFile` for one parsed source file and best-effort declaration wrappers for classes, enums, typedefs, functions, top-level fields/globals, class fields, class methods, constructors, destructors, parameters, attributes, modifiers, doc comments, and empty semicolon declarations. Extracted names and text are returned as span-backed slices into the original source text. Uncertain names or type text return `Option` instead of inventing semantic facts. Class, enum, function, field, and method attributes are exposed through declaration wrappers. Leading `//!` and `/*! ... */` documentation comments are exposed as raw `DocComment` values attached from immediate leading trivia; tags and comment text are not parsed or normalized. Top-level fields are returned as `Declaration::Field`, while class fields remain `ClassMember::Field`. Field extraction ignores preserved semicolons that belong to leading attribute lists before scanning the actual declaration tokens. Typedef declarations expose the alias name and raw aliased type text, such as `typedef string FactionKey;` producing name `FactionKey` and type text `string`, without resolving the alias. Enum members expose source-backed explicit value text when present, but values are not evaluated or resolved. Destructor methods such as `void ~Example()` are detected through `MethodDecl::is_destructor()`, keep `name()` as `Example`, and return `void` from `return_type_text()` rather than `void ~`. Constructor classification is exposed through `ClassDecl::classify_method()` because it requires comparing a method name with the containing class name. Parameters expose raw text plus best-effort name, leading type text, default text, and top-level `out`/`inout`/`notnull` modifiers. `ref` remains part of parameter type text because it is strong-reference/type ownership syntax in Reforger sources. `ParameterKind` distinguishes real declaration parameters from preserved non-declaration callable fragments such as literal-only `false` inside declaration-shaped conditional source; `parameters()` returns declaration parameters and `parameter_fragments()` returns those preserved fragments.

Static-array field declarations keep the field identifier before the array suffix as the field name. For example, `string TAGS[COUNT];` exposes name `TAGS` and type text `string`; the `[COUNT]` suffix is preserved by source span for the model type-shape layer rather than folded into `type_text`.

Comma-separated field declarations expose `FieldDeclarator` views from one parser `FieldDecl`. For example, `Widget a, b, c;` exposes three declarators with shared type text `Widget` and local declarator spans. The parser node remains full-fidelity and unchanged; AST is responsible for splitting the field-list view for model/index consumers.

Method/function body blocks expose a narrow `LocalVariable` view for local declarations, `foreach` variables, and `for` initializer declarations. Local discovery is syntax-backed: AST walks parsed `LocalDeclStatement`, `ForInitializer`, and `ForeachHeader` nodes, then uses token-slice declarator helpers only inside those selected syntax nodes. It preserves names, source spans, raw type text, default text, and local modifiers without semantic scope resolution. Static-array locals with brace initializer defaults, such as `vector value[4] = {...};`, keep the array-suffixed local span and expose the complete brace initializer default text, including nested initializer lists. It is intended for hover and later local completion groundwork, not semantic scope resolution.

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
- Added source-backed typedef aliased type text extraction.
- Fixed static-array field extraction so array bound identifiers do not replace the actual field name.
- Added `FieldDeclarator` extraction so comma-separated field declarations expose every declared field with the correct shared type text.
- Added local/block symbol extraction for local variables, `foreach` variables, and `for` initializer declarations.
- Fixed local default extraction for static-array locals with brace initializer lists so hover/display details do not truncate the closing initializer braces.
- Moved local discovery from broad block token scanning to parsed statement/header syntax nodes.

## Future Improvements

- Add richer typed wrappers as parser coverage expands into statements and expressions.
- Add real statement/expression AST and lexical scope modeling in separate verified slices.
- Add a normalized type-shape API separately; current parameter and field type text remains source-faithful.
- Add a semantic model layer separately when declaration extraction is stable.
- Add workspace indexing separately; AST wrappers should remain file-local.
