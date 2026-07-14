# server/examples/lsp_definition_report.rs

## Purpose

Provides a dev-only fixture report for the LSP definition path.

## Architecture Role

This example sits above the Rust LSP helper API. It exercises the same resolver-first location projection used by `textDocument/definition` without starting VS Code or a stdio server.

## Current Behavior

The report reads targeted committed parser fixtures, downloaded game-data checks when available, plus a small inline definition coverage source. When game data exists, it builds a game-data index once and supplies it as external definition context. It writes `tools/reports/lsp-definition-fixtures.report.md` with target positions, hit/miss state, selected source, resolver reason, identifier context, candidate count, selected symbol kind/name, target URI, target range, parse diagnostics, and elapsed time.

Current checks include class, method, field, parameter, local variable, typedef, enum member, global field, external type, receiver/member access, named-argument label miss, and unresolved miss behavior.

Definition currently returns standard LSP `Location[]` targets using the selected declaration name range. That is enough for the current Ctrl+click foundation. Future `LocationLink` support would add origin selection ranges and separate target declaration ranges, but should use the same resolver-owned target selection rather than adding a competing definition path.

## Dependencies and Boundaries

The report uses only Rust standard library code and existing LSP/index helper functions. It accepts `--scripts <path>` to override the downloaded game-data scripts folder. It must remain dev-only review tooling. It must not perform semantic lookup, Workbench validation, runtime logging, VS Code command registration, or source mutation.

## Change Notes

- Added the first definition fixture report alongside the LSP definition feature.
- Added external game-data index context so definition can review external type targets such as `BaseGameMode`.
- Added named-argument and unresolved miss rows to prove definition does not navigate misleadingly.

## Future Improvements

- Add `LocationLink` review output only when the runtime definition response moves to `LocationLink[]`.
