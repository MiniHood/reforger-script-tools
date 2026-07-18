# src/extension.ts

## Purpose

Owns VS Code extension activation and deactivation. It wires top-level extension services and keeps command registration close to the VS Code API surface.

## Ownership

This file is TypeScript shell code. It should register editor-facing features and delegate subsystem behavior to focused modules. Serious language intelligence belongs behind the future Rust/LSP boundary, not here.

## Current Behavior

On activation, it writes TypeScript-side startup timing marks, registers game-data features, and starts the Rust language client. The game-data service performs startup checks, command registration, and global-storage management from its own module. The language-client module resolves and starts the bundled Rust LSP server.

## Dependencies and Boundaries

Imports `vscode`, `registerGameDataFeatures`, and language-client registration/deactivation/timing helpers. Do not add parser, AST, indexing, LSP request handling, or semantic-analysis logic here.

## Verification

Run `npm test` after changing this owner or a boundary it defines. Inspect the extension in a development host when activation or editor-facing behavior changes.
