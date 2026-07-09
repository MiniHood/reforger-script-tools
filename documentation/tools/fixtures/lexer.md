# tools/fixtures/lexer/

## Purpose

Provides small Enfusion Script examples for lexer tests and future language-tooling planning.

## Architecture Role

Fixtures are repo test and review inputs under `tools/`, not extension runtime files. They help keep lexer and future parser behavior grounded in real Reforger script shapes without treating speculative examples as compiler truth.

## Current Behavior

The lexer fixtures cover class declarations, inheritance, `modded class`, attributes, generic type syntax, method signatures, normal comments, documentation comments, preprocessor directives, string escapes, RPC/Rpl-looking declarations, Core generated/proto language shapes, game-mode component/modded-class shapes, and documented/game-data-observed keywords such as `auto`, `event`, `thread`, `debug`, `vanilla`, and `func`. They include larger game-data-derived fixtures copied or composed from `Core/proto/Types.c`, `Game/Commanding/SCR_PlayerCommandsConfig.c`, `Game/GameMode/SCR_BaseGameMode.c`, `Game/Editor/Containers/Rpc/SCR_EditorPreviewParams.c`, and `WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c` patterns.

## Dependencies and Boundaries

Each fixture must include a short truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. Use `official-sample-derived` or `game-data-derived` for examples based on official docs, samples, extracted API, or downloaded game data.

Fixtures must not be packaged with the VS Code extension unless a future runtime feature explicitly needs them. Keep them under `tools/fixtures/` while they are used only for tests, reports, and language-tooling research.

## Change Notes

- Added initial lexer fixtures based on discovery-report patterns from downloaded Reforger game data.
- Added a larger game-data-derived player commands config fixture with multiple related classes and methods.
- Replaced the synthetic method/generic fixture with `core_array_class.c`, a full `array<Class T>` class copied from `Core/proto/Types.c`.
- Added a Core-derived fixture for generic class declarations, destructor declarations, static array declarators, proto modifier stacks, owned fields, event methods, and inline initializer lists.
- Extended Core fixture coverage with generic `set<T>`, container typedefs, and nested generic type names from `Core/proto/Types.c`.
- Added a game-mode-derived modded-class fixture composed from `SCR_BaseGameMode.c` patterns and real modded-class syntax shapes. It is not a copied Workbench-confirmed `modded SCR_BaseGameMode`.
- Added game-data-derived fixtures for editor preview transform/RPC-style data and the Workbench basic code formatter plugin. These stress doc comments, fixed arrays, out static-array parameters, named attribute arguments, preprocessor guards, and declaration-level initializer lists.
- Lexer tests now assert documentation comment classification while preserving Doxygen-style tags as raw comment text.
- Moved lexer fixtures from root `fixtures/lexer/` to `tools/fixtures/lexer/` so dev/test data stays in the tooling area.

## Future Improvements

- Add Workbench-confirmed fixtures as syntax assumptions are validated.
- Split parser/model/index fixtures into their own folders only when those systems exist.
