# Reforger Script Tools

Reforger Script Tools adds parser-first Enforce Script support for Arma Reforger modding in VS Code. It focuses on fast code intelligence, Workbench validation, base-game API indexing, and editor assists that match Reforger script patterns.

## Features

- Enforce Script language support for `.c` files.
- Parser-backed completions for keywords, locals, parameters, current class members, inherited members, receiver members, indexed classes, enums, and Reforger API symbols.
- Callable-aware completions for callback APIs such as invokers, RPC methods, and script call queues.
- Hover, Go to Definition, references, rename, document highlights, document symbols, workspace symbols, and semantic highlighting from a shared language model.
- Workbench validation through the Reforger Workbench Net API, with current-file issues grouped first.
- Downloaded or user-provided base-game script data indexing for API lookup.
- Reforger-themed semantic colors and parser-backed bracket coloring.
- Optional typed assists for safe brackets, comments, class inheritance colons, declaration initializers, and semicolons.

## Requirements

- Visual Studio Code `1.125.0` or newer.
- Arma Reforger Tools / Workbench for validation features.
- Workbench Net API must be reachable for `Reforger: Validate Scripts`. The default endpoint is `127.0.0.1:5775`.

The extension can use downloaded BI script data by default. You can also point it at local exported script data with `reforgerScriptTools.gameScriptDataPath`.

## Commands

- `Reforger: Validate Scripts`
- `Reforger: Check Workbench Status`
- `Reforger: Refresh Game Data`
- `Reforger: Capture AC Debug Log`

## Extension Settings

This extension contributes these settings:

- `reforgerScriptTools.validateOnSave`: Validate Enforce scripts on save.
- `reforgerScriptTools.netApiHost`: Workbench Net API host.
- `reforgerScriptTools.netApiPort`: Workbench Net API port.
- `reforgerScriptTools.netApiTimeoutMs`: Workbench Net API timeout.
- `reforgerScriptTools.showBaseGameWarnings`: Include base-game warnings in validation output.
- `reforgerScriptTools.showSuccessMessage`: Show a notification when validation succeeds.
- `reforgerScriptTools.gameScriptDataPath`: Optional local folder containing Reforger script data.
- `reforgerScriptTools.checkGameScriptUpdates`: Check for updated BI script data during startup.
- `reforgerScriptTools.formatting.semicolons.enabled`: Enable semicolon typed assists. Default: `true`.
- `reforgerScriptTools.formatting.autoBrackets.enabled`: Enable safe bracket/body typed assists. Default: `true`.
- `reforgerScriptTools.formatting.classInheritanceColon.enabled`: Enable expanding `class Name ` to `class Name : `. Default: `true`.
- `reforgerScriptTools.formatting.comments.enabled`: Enable standalone block comment opener expansion. Default: `true`.
- `reforgerScriptTools.formatting.equalSigns.enabled`: Enable simple declaration initializer expansion. Default: `true`.

## Known Issues

- This extension is in a parser-first rework. Some language features may be conservative while the shared parser/model/index pipeline is expanded.
- Workbench validation depends on the Workbench Net API being enabled and reachable.
- Indexed base-game API completions depend on downloaded or configured script data.

## Release Notes

### 0.0.1

Initial parser-first preview release.
