# fixtures/lexer/

## Purpose

Provides small Enfusion Script examples for lexer tests and future language-tooling planning.

## Architecture Role

Fixtures are repo test inputs, not extension runtime files. They help keep lexer and future parser behavior grounded in real Reforger script shapes without treating speculative examples as compiler truth.

## Current Behavior

The lexer fixtures cover class declarations, inheritance, `modded class`, attributes, generic type syntax, method signatures, comments/trivia, preprocessor directives, string escapes, and RPC/Rpl-looking declarations. They also include a larger game-data-derived fixture copied from `Game/Commanding/SCR_PlayerCommandsConfig.c`.

## Dependencies and Boundaries

Each fixture must include a short truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. Use `official-sample-derived` or `game-data-derived` for examples based on official docs, samples, extracted API, or downloaded game data.

Fixtures must not be packaged with the VS Code extension unless a future runtime feature explicitly needs them.

## Change Notes

- Added initial lexer fixtures based on discovery-report patterns from downloaded Reforger game data.
- Added a larger game-data-derived player commands config fixture with multiple related classes and methods.

## Future Improvements

- Add Workbench-confirmed fixtures as syntax assumptions are validated.
- Split parser/model/index fixtures into their own folders only when those systems exist.
