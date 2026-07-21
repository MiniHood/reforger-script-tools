# System Overview

Reforger Script Tools is a VS Code extension and bundled Rust language server
for Enfusion Script. Its purpose is high-fidelity language understanding and
reliable editor behavior without making users install a separate toolchain.

## Runtime Flow

```text
VS Code editor
  -> TypeScript extension shell
  -> bundled Rust language server
  -> language results back to VS Code
```

The extension resolves user settings, game-data locations, storage paths, and
editor events. The language server receives documents and external source
locations, then owns language analysis and LSP results.

## Ownership

| Owner | Responsibility | Must not own |
| --- | --- | --- |
| `src/extension.ts` | Activation and top-level wiring | Language behavior or game-data logic |
| `src/extensionConfig/` | Extension-facing names, defaults, and limits | Runtime behavior |
| `src/gameData/` | Game-data acquisition and source resolution | Parsing or semantic analysis |
| `src/languageClient/` | Server lifecycle, transport, and thin editor bridges | Language decisions |
| `server/` | Parsing, semantic analysis, indexes, diagnostics, formatting, and LSP features | VS Code UI or game-data acquisition |
| `tools/` | Development and investigation tooling | Extension runtime behavior |

The Rust engine is the single language authority. TypeScript may adapt editor
events and render Rust results, but it must not reimplement syntax, lookup,
completion, or type reasoning.

## Sources of Truth

For Enfusion Script behavior, evidence is ordered as follows:

1. Workbench/compiler behavior.
2. Official Reforger documentation.
3. Verified extracted game data.
4. Source examples and fixtures, labelled by confidence.

Source code is authoritative for implementation. Tests prove covered behavior.
Generated reports and investigations are supporting evidence, never the
architecture or language authority.

## Documentation Rule

This overview records cross-module facts. Future documentation should explain
only a stable module contract, a consequential decision, or a reusable evidence
format. It should not restate volatile implementation details or mirror every
source file.
