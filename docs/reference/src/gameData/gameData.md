# src/gameData/gameData.ts

## Purpose

Owns Reforger game-data source resolution and update checks for the VS Code extension layer.

## Ownership

This is TypeScript integration code for settings, global storage, GitHub download/update checks, VS Code prompts, and commands. It does not implement language parsing or analysis.

## Current Behavior

The service registers game-data commands and checks game data on activation. A configured manual folder overrides downloaded data and skips all GitHub checks. Without a manual folder, the service checks Bohemia's script-diff `main` commit, prompts once before first download through a bottom-right notification, then auto-updates stale global-storage data after consent.

The first-run prompt offers `Download` and `Set Manual Folder`. Dismissing the notification cancels the action. `Set Manual Folder` opens a folder picker and writes the selected folder to `reforgerScriptTools.gameData.manualFolder`.

Downloaded data is stored under `globalStorageUri/game-data/scripts`, with metadata in `globalStorageUri/game-data/metadata.json`. Updates extract into a temporary staging folder and replace the single live `scripts/` folder only after scripts are found. Staging folders are deleted after success or failure, and stale `staging-*` folders are cleaned before updates. The progress notification is shown only while data is downloading, extracting, or finalizing; checking and prompting do not show a progress notification.

## Dependencies and Boundaries

Uses Node filesystem/path APIs, VS Code APIs, GitHub HTTP endpoints, `fflate` for zip extraction, and `src/extensionConfig/gameData.ts` for subsystem keys. Keep Workbench out of this downloader. Keep parser/analyzer behavior out of this file.

## Verification

Run `npm test`. For acquisition or storage changes, exercise the affected manual-folder or downloaded-data path in an Extension Development Host and inspect `globalStorageUri` rather than the workspace.
