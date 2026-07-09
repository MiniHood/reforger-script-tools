# tools/fixtures/index/

## Purpose

Provides small Enfusion Script source trees for index and overlay-index tests, reports, and debug tooling.

## Architecture Role

Index fixtures are repo-only language-tooling inputs under `tools/`. They help validate lookup behavior, source-kind priority, and workspace/game-data overlay reporting without making the fixtures part of the packaged VS Code extension.

## Current Behavior

The `overlay/` fixture is a small workspace-like source root used to validate dev-only overlay tooling. It intentionally contains names that also exist in game data, such as `SCR_BaseGameMode` and `FactionKey`, so reports and debug commands can show workspace priority over game-data priority.

The overlay fixture also includes workspace-only source shapes for stronger index review: a `modded` class, a workspace-only class, a workspace class extending another workspace class that extends a game-data class, overloaded methods with the same name and different signatures, direct field shadowing, and a typedef/function delegate-style duplicate.

## Dependencies and Boundaries

Each fixture file must include a truth-status comment. Use `Workbench-confirmed` only after actual Workbench/compiler validation. These fixtures must not become runtime extension dependencies or source truth for compiler behavior.

## Change Notes

- Added the initial overlay fixture root for workspace/game-data index review.
- Expanded the overlay fixture with workspace-only classes, inheritance, overloads, field shadowing, and a typedef/function duplicate so overlay reports exercise more than the happy path.

## Future Improvements

- Add additional index fixtures only when a specific lookup behavior needs repeatable validation.
- Split future semantic or diagnostics fixtures into their own folders only when those systems exist.
