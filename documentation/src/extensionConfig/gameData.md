# src/extensionConfig/gameData.ts

## Purpose

Centralizes game-data command IDs, VS Code setting keys, global state keys, storage names, repository identity, and thresholds.

## Architecture Role

This file is TypeScript extension-shell configuration. It is part of the generalized extension config area, keeping extension-facing names out of runtime feature logic.

## Current Behavior

Exports typed constant objects for game-data repository metadata, commands, user-facing config keys, internal state keys, global-storage names, and the low script-count threshold.

## Dependencies and Boundaries

Has no runtime dependencies. Do not put mutable state, VS Code API calls, filesystem access, network calls, or language intelligence here.

## Change Notes

Game-data runtime state remains in `context.globalState`, downloaded files remain in `context.globalStorageUri`, and the manual folder remains the only user-facing setting.

## Future Improvements

Add new game-data settings/state/storage keys here before use in feature code. Add sibling files under `src/extensionConfig/` for future subsystems instead of creating local constants files inside runtime feature folders.
