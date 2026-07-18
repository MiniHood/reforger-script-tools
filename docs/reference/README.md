# Reference Documentation

This directory is the current-state documentation for source and subsystem
owners. Read [the architecture overview](architecture.md) first for a
cross-layer change, then use the nearest owner page before editing a non-trivial
source file.

## Entry points

- [Rust language engine](server.md): compiler-style layers, LSP, and report
  examples. Its child pages mirror `server/src/` and `server/examples/`.
- [Extension activation](src/extension.md): top-level TypeScript shell wiring.
- [Game-data service](src/gameData/gameData.md): game-data acquisition and
  source resolution.
- [Language client](src/languageClient/languageClient.md): bundled server,
  protocol transport, and editor bridge.
- [Extension configuration](src/extensionConfig/gameData.md) and
  [language-client configuration](src/extensionConfig/languageClient.md):
  extension-facing ids, defaults, and thresholds.
- [Developer tooling](tools/build-language-server.md): build, investigation,
  fixture, and verified-refactor tooling.
- [Theme](themes/reforger-enforce-dark-color-theme.md): Enforce presentation
  layer.
- [Extension manifest](package.md) and
  [language configuration](language-configuration.md): VS Code contributions.

Pages are path-mirrored where a file has independent ownership; a small file
may instead be covered by its nearest subsystem page. The required page contract
and update rules live in [the documentation procedure](../documentation.md).
