# Reforger Script Tools

Reforger Script Tools brings Enfusion Script language support and Arma Reforger
Workbench compiler feedback to Visual Studio Code. Everything needed by the
extension is included; no additional tools or runtimes are required.

## Unofficial Project and Product Terms

Reforger Script Tools is an independent, unofficial project. It is not
affiliated with, authorized by, endorsed by, or supported by Bohemia
Interactive a.s.

Use of this extension with Bohemia Interactive games, tools, services, or
content remains subject to all applicable end-user license agreements, terms of
use, and content licenses, including but not limited to the
[Arma Reforger EULA](https://reforger.armaplatform.com/eula) and
[Arma Reforger Workshop Terms of Use](https://reforger.armaplatform.com/workshop-terms). This extension is not designed or intended to circumvent
those agreements, violate license restrictions, or enable others to do so.
Users are responsible for ensuring that their use complies with the agreements
and licenses applicable to the Bohemia Interactive products and content they
use.

Bohemia Interactive, Arma, Arma Reforger, and associated logos and designs are
trademarks or registered trademarks of Bohemia Interactive a.s.

## Features

- Enfusion Script syntax highlighting and semantic colors.
- Context-aware completion, snippets, signature help, hover information, and
  go to definition.
- Document symbols and indexing of Reforger base-game data.
- Editor errors and authoritative Workbench compiler results shown separately.
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

## Known Limitations

Additional addons outside the Reforger base-game data are not currently
supported as language-feature reference data. Support may be added in the
future after Bohemia Interactive releases its official VS Code extension.

## Settings

Open **Preferences: Open Settings (UI)** and search for `Reforger Script
Tools`, or add the keys to `settings.json`.

| Setting | Default | Description |
| --- | --- | --- |
| `reforgerScriptTools.gameData.manualFolder` | `""` | Optional local Reforger game-data folder. Select either the folder containing `scripts/` or the `scripts/` folder itself. This disables GitHub game-data checks and downloads. |
| `reforgerScriptTools.experimentalAutoFormatting` | `true` | Apply experimental automatic source edits, including typing assists and preprocessor directive separators. |
| `reforgerScriptTools.bracketColoring` | `"semantic"` | Use `"semantic"` owner colors, `"punctuation"` palette color, or native `"vscode"` bracket coloring and matching. This setting applies across VS Code windows. |
| `reforgerScriptTools.workbench.enabled` | `true` | Enable Workbench NET API status checks and compiler validation. |
| `reforgerScriptTools.workbench.host` | `"127.0.0.1"` | Workbench NET API loopback host. IPv4 loopback addresses and `::1` are accepted. |
| `reforgerScriptTools.workbench.port` | `5775` | Workbench NET API port, from `1` through `65535`. The extension does not scan other ports. |

## Customize Semantic Colors

The extension supplies default Enfusion Script colors through VS Code's native
semantic-token settings. VS Code applies these defaults automatically, so they
do not appear in **User Settings (JSON)** until you add your own overrides.

To change a color:

1. Open the Command Palette with `Ctrl+Shift+P`.
2. Run **Preferences: Open User Settings (JSON)**.
3. Add the selectors you want to override under
   `editor.semanticTokenColorCustomizations.rules`.

For example:

```json
{
  "editor.semanticTokenColorCustomizations": {
    "rules": {
      "class:enforce": "#4EC9B0",
      "function:enforce": "#DCDCAA",
      "reforgerField:enforce": "#9CDCFE",
      "keyword:enforce": "#569CD6",
      "comment:enforce": "#6A9955",
      "string:enforce": "#CE9178",
      "reforgerPunctuation:enforce": "#D4D4D4"
    }
  }
}
```

Only the selectors included in the user's settings are changed; all others keep
the extension defaults. The `:enforce` suffix limits each rule to Enfusion
Script. Available selectors are:

`class:enforce`, `enum:enforce`, `type:enforce`, `typeParameter:enforce`,
`function:enforce`, `reforgerField:enforce`, `variable:enforce`,
`parameter:enforce`, `enumMember:enforce`, `number:enforce`,
`operator:enforce`, `reforgerPunctuation:enforce`, `keyword:enforce`,
`comment:enforce`, `string:enforce`, and `reforgerPreprocessor:enforce`.
