# src/extensionConfig/gameData.ts

## Purpose

Centralizes game-data command IDs, VS Code setting keys, global state keys, storage names, repository identity, and thresholds.

## Ownership

This file is TypeScript extension-shell configuration. It is part of the generalized extension config area, keeping extension-facing names out of runtime feature logic.

## Current Behavior

Exports typed constant objects for game-data repository metadata, commands, user-facing config keys, internal state keys, global-storage names, and the low script-count threshold.

## Dependencies and Boundaries

Has no runtime dependencies. Do not put mutable state, VS Code API calls, filesystem access, network calls, or language intelligence here.

## Verification

Run `npm test` after changing a constant or its consumer. Confirm a development host reads the expected setting, command, or storage key when the change affects editor integration.
