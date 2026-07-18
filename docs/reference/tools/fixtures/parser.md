# tools/fixtures/parser/

## Purpose

Provides small Enfusion Script examples for parser tests and parser-structure reports.

## Ownership

Parser fixtures are repo-only language-tooling inputs under `tools/`. They ground the Rust parser scaffold in real Reforger declaration shapes without making the fixtures part of the packaged VS Code extension.

## Current Behavior

The fixtures cover Core type declarations, contextual keyword names, generic classes, typedefs, tuple-style `extends`, attributes, RPC/Rpl declarations, Workbench plugin attributes, modded game-mode member shapes, preprocessor directives, optional semicolons after method bodies, optional semicolons after attribute lists, field call initializers with nested brace lists, local/block symbol shapes, and larger game-code class excerpts. The larger excerpts include editor preview transform data and Workbench formatter declarations to keep parser behavior grounded in real static arrays, named attribute arguments, preprocessor guards, declaration-level initializer lists, local declarations, `foreach` variables, and `for` initializer declarations.

## Dependencies and Boundaries

Each fixture must include a truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. These files must not become runtime extension dependencies or source truth for compiler behavior.

## Verification

Run the parser tests or report that uses the changed fixture. Preserve its truth-status comment and validate a language claim with Workbench before labeling it `Workbench-confirmed`.
