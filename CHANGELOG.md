# Change Log

All notable changes to the "reforger-script-tools" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [2.0.1] - 2026-08-04

- Added automatic native VS Code discovery for the bundled MCP Runtime, with
  workspace-aware refreshes and no separate MCP configuration required.
- Kept MCP-only activation independent of Workbench consent and editor startup.
- Packaged portable `reforger`, `reforger-deep-dive`, and
  `reforger-workbench-edit` Agent Skills for native VS Code discovery, with
  validated references and generated MCP-contract checks.

## [2.0.0] - 2026-08-03

- Added automatic discovery and indexing for installed add-ons and base-game data.
- Added a Search browser for scripts, resources, text, and documentation; open it with `Ctrl+Alt+F`.
- Added a bundled MCP server for script, resource, add-on, and documentation search, plus Workbench inspection and editing.
- Improved indexing, search, source-preview, and startup performance.
- Added semantic, punctuation-colored, and native VS Code bracket presentation modes.
