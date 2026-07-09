# src/extension.ts

## Purpose

Owns VS Code extension activation and deactivation. It wires top-level extension services and keeps command registration close to the VS Code API surface.

## Architecture Role

This file is TypeScript shell code. It should register editor-facing features and delegate subsystem behavior to focused modules. Serious language intelligence belongs behind the future Rust/LSP boundary, not here.

## Current Behavior

On activation, it registers game-data features. The game-data service performs startup checks, command registration, and global-storage management from its own module.

## Dependencies and Boundaries

Imports `vscode` and `registerGameDataFeatures`. Do not add parser, AST, indexing, or semantic-analysis logic here.

## Change Notes

Removed the starter hello-world command so activation only wires real extension behavior.

## Future Improvements

Keep activation limited to top-level feature registration as new subsystems are added.
