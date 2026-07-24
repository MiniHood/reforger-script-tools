# Reforger Script Tools

Reforger Script Tools is a VS Code extension with a bundled Rust language
server for Enfusion Script. It aims for accurate language understanding and
reliable editor behavior without requiring users to install a separate
toolchain.

## Project Guide

- [System overview](docs/overview.md): purpose and evidence hierarchy.
- [Architecture](docs/architecture.md): runtime flow and ownership boundaries.
- [Language engine](docs/language-engine.md): analysis model and server contract.
- [Development](docs/development.md): build, test, and local extension workflow.

The [documentation index](docs/README.md) explains when a new document is
worth adding. Code and tests remain authoritative for implementation details.

## Settings

Open **Preferences: Open Settings (UI)** and search for `Reforger Script
Tools`, or add these keys to `settings.json`.

| Setting | Default | Purpose |
| --- | --- | --- |
| `reforgerScriptTools.gameData.manualFolder` | `""` | Use a local Reforger game-data folder instead of checking or downloading the GitHub game-data source. The value may be the folder containing `scripts/` or the `scripts/` folder itself. |
| `reforgerScriptTools.diagnostics.enabled` | `true` | Write local extension and language-server performance logs. Source text and LSP payloads are excluded. |
| `reforgerScriptTools.experimentalAutoFormatting` | `true` | Apply experimental source edits such as typing assists and preprocessor directive separators. |
| `reforgerScriptTools.bracketColoring` | `"semantic"` | Choose `"semantic"` owner colors, `"punctuation"` Reforger punctuation color, or native `"vscode"` bracket coloring and matching. This is an application-wide setting. |
| `reforgerScriptTools.workbench.enabled` | `true` | Enable the configured Workbench NET API integration, status checks, and compiler validation. |
| `reforgerScriptTools.workbench.host` | `"127.0.0.1"` | Set the Workbench NET API loopback host. Only IPv4 loopback addresses and `::1` are accepted. |
| `reforgerScriptTools.workbench.port` | `5775` | Set the Workbench NET API port from `1` through `65535`. The extension does not probe other ports. |
| `reforgerScriptTools.workbench.compilerValidationDelaySeconds` | `3` | Set the idle delay before saving and validating the active Enfusion Script, from `0` through `60` seconds. `0` disables edit/save automation but keeps session-start and manual validation. |
| `reforgerScriptTools.workbench.compilerValidationProfile` | `"WORKBENCH"` | Select the Workbench script configuration used for compiler validation. `WORKBENCH` is currently the only supported profile. |

## Reforger Semantic Palette

The extension applies a dark-oriented Enforce palette over the user's chosen
VS Code theme. It does not install or select a complete color theme. The
palette changes foreground colors only, so font styles and every non-Enforce
theme color remain owned by the selected theme.

| Enforce role | Semantic selector | Default foreground |
| --- | --- | --- |
| Class, enum, type, type parameter | `class:enforce`, `enum:enforce`, `type:enforce`, `typeParameter:enforce` | `#40b5ac` |
| Function, including global and class functions | `function:enforce` | `#f3ad58` |
| Field | `reforgerField:enforce` | `#cfcfcf` |
| Variable, parameter, enum value, number | `variable:enforce`, `parameter:enforce`, `enumMember:enforce`, `number:enforce` | `#cfcfcf` |
| Operator and punctuation | `operator:enforce`, `reforgerPunctuation:enforce` | `#cfcfcf` |
| Keyword | `keyword:enforce` | `#59A6E9` |
| Comment | `comment:enforce` | `#59aa59` |
| String | `string:enforce` | `#c178dd` |
| Preprocessor syntax | `reforgerPreprocessor:enforce` | `#d4fd95` |

To override one role while keeping the rest of the palette, add its selector
to the native VS Code customization setting:

```json
{
  "editor.semanticTokenColorCustomizations": {
    "rules": {
      "function:enforce": "#ffcc66",
      "reforgerField:enforce": "#d8dee9"
    }
  }
}
```

The shipped palette is intentionally dark-oriented and does not switch when a
light theme is selected. Light-theme users can override only the selectors
that need more contrast, or disable the palette for Enforce while keeping
their chosen theme:

```json
{
  "[enforce]": {
    "editor.semanticHighlighting.enabled": false
  }
}
```

Use **Developer: Inspect Editor Tokens and Scopes** on Enforce source to see
the semantic token type under the cursor. User and workspace semantic-token
rules take precedence over the extension defaults.
