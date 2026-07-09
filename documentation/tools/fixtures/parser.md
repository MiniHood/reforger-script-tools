# tools/fixtures/parser/

## Purpose

Provides small Enfusion Script examples for parser tests and parser-structure reports.

## Architecture Role

Parser fixtures are repo-only language-tooling inputs under `tools/`. They ground the Rust parser scaffold in real Reforger declaration shapes without making the fixtures part of the packaged VS Code extension.

## Current Behavior

The fixtures cover Core type declarations, contextual keyword names, generic classes, typedefs, tuple-style `extends`, attributes, RPC/Rpl declarations, Workbench plugin attributes, modded game-mode member shapes, preprocessor directives, optional semicolons after method bodies, optional semicolons after attribute lists, field call initializers with nested brace lists, and larger game-code class excerpts. The larger excerpts include editor preview transform data and Workbench formatter declarations to keep parser behavior grounded in real static arrays, named attribute arguments, preprocessor guards, and declaration-level initializer lists.

## Dependencies and Boundaries

Each fixture must include a truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. These files must not become runtime extension dependencies or source truth for compiler behavior.

## Change Notes

- Added the initial parser fixture set for the declaration-level parser scaffold.
- Added larger game-data-derived class excerpts for building network RPC, building provider budget logic, and editable group behavior.
- Added larger game-data-derived class excerpts for editor preview transform data and the Workbench basic code formatter plugin.
- Added a game-data-derived fixture for optional semicolon forms from generated API declarations and handwritten attributed fields.
- Added a game-data-derived fixture for field initializers that call helper methods with nested brace-list arguments.

## Future Improvements

- Add Workbench-confirmed parser fixtures when specific syntax questions are validated.
- Split future expression/statement/model fixtures into separate folders only when those systems exist.
