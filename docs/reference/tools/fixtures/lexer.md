# tools/fixtures/lexer/

## Purpose

Provides small Enfusion Script examples for lexer tests and future language-tooling planning.

## Ownership

Fixtures are repo test and review inputs under `tools/`, not extension runtime files. They help keep lexer and future parser behavior grounded in real Reforger script shapes without treating speculative examples as compiler truth.

## Current Behavior

The lexer fixtures cover class declarations, inheritance, `modded class`, attributes, generic type syntax, method signatures, normal comments, documentation comments, preprocessor directives, string escapes, RPC/Rpl-looking declarations, Core generated/proto language shapes, game-mode component/modded-class shapes, and documented/game-data-observed keywords such as `auto`, `event`, `thread`, `debug`, `vanilla`, and `func`. They include larger game-data-derived fixtures copied or composed from `Core/proto/Types.c`, `Game/Commanding/SCR_PlayerCommandsConfig.c`, `Game/GameMode/SCR_BaseGameMode.c`, `Game/Editor/Containers/Rpc/SCR_EditorPreviewParams.c`, and `WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c` patterns.

## Dependencies and Boundaries

Each fixture must include a short truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. Use `official-sample-derived` or `game-data-derived` for examples based on official docs, samples, extracted API, or downloaded game data.

Fixtures must not be packaged with the VS Code extension unless a future runtime feature explicitly needs them. Keep them under `tools/fixtures/` while they are used only for tests, reports, and language-tooling research.

## Verification

Run the lexer tests or report that uses the changed fixture. Preserve its truth-status comment and validate a language claim with Workbench before labeling it `Workbench-confirmed`.
