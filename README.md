# Reforger Script Tools

Reforger Script Tools brings Enfusion Script language support and Arma Reforger
Workbench compiler feedback to Visual Studio Code. The extension includes its
Rust language server, so users do not need to install Node.js, Rust, or a
separate server.

## Features

- Enfusion Script syntax highlighting and semantic colors.
- Context-aware completion, snippets, signature help, hover information, and
  go to definition.
- Document symbols and workspace-aware indexing of addon scripts and Reforger
  game data.
- Parser diagnostics and Workbench compiler diagnostics shown separately.
- Range formatting plus experimental automatic formatting while typing,
  including indentation, comment pairs, and preprocessor separators.
- Semantic, punctuation-colored, or native VS Code bracket presentation.
- Automatic game-data updates, with an optional local game-data folder.
- Automatic and manual script validation through the Workbench NET API.

The extension recognizes `.c` files under `Scripts` or `scripts` directories as
Enfusion Script.

## Enable the Workbench NET API

Workbench compiler validation requires Arma Reforger Tools and a running
Workbench instance with your addon project open.

1. In Workbench, select **Workbench > Options**.
2. Open the **Workbench** tab.
3. Enable **Enable net API**.
4. Select **OK** to save the setting.
5. In VS Code, leave the extension endpoint at its default
   `127.0.0.1:5775`, or set it to the loopback endpoint used by Workbench.

The extension reconnects automatically. The Workbench status item shows
availability, and **Reforger Script Tools: Validate Scripts in Workbench** runs
validation manually. Workbench validation is also requested at session start,
after an eligible save, and after the active dirty script has been idle for
three seconds.

These steps follow Bohemia Interactive's official
[Resource Manager options documentation](https://community.bistudio.com/wiki/Arma_Reforger%3AResource_Manager%3A_Options#Enable_net_API).

## Settings

Open **Preferences: Open Settings (UI)** and search for `Reforger Script
Tools`, or add the keys to `settings.json`.

| Setting | Default | Description |
| --- | --- | --- |
| `reforgerScriptTools.gameData.manualFolder` | `""` | Optional local Reforger game-data folder. Select either the folder containing `scripts/` or the `scripts/` folder itself. This disables GitHub game-data checks and downloads. |
| `reforgerScriptTools.diagnostics.enabled` | `false` | Write detailed local extension and language-server support logs. Enable this only while investigating a problem. |
| `reforgerScriptTools.experimentalAutoFormatting` | `true` | Apply experimental automatic source edits, including typing assists and preprocessor directive separators. |
| `reforgerScriptTools.bracketColoring` | `"semantic"` | Use `"semantic"` owner colors, `"punctuation"` palette color, or native `"vscode"` bracket coloring and matching. This setting applies across VS Code windows. |
| `reforgerScriptTools.workbench.enabled` | `true` | Enable Workbench NET API status checks and compiler validation. |
| `reforgerScriptTools.workbench.host` | `"127.0.0.1"` | Workbench NET API loopback host. IPv4 loopback addresses and `::1` are accepted. |
| `reforgerScriptTools.workbench.port` | `5775` | Workbench NET API port, from `1` through `65535`. The extension does not scan other ports. |

## Diagnostics

Support logging is disabled by default so normal editor requests do not incur
diagnostic disk I/O. When troubleshooting, enable
`reforgerScriptTools.diagnostics.enabled`, reproduce the problem, and then
disable it again. Logs are stored in the extension's VS Code global storage
area and omit source text and LSP payloads.

## Development

See the
[development guide](https://github.com/burn0ut7/reforger-script-tools/blob/main/docs/development.md)
for fresh-checkout setup, builds, tests, and local Extension Development Host
workflows. The
[documentation index](https://github.com/burn0ut7/reforger-script-tools/blob/main/docs/README.md)
links the architecture, language-engine, decision, and research records.
