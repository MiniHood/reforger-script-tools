# server/examples/lsp_hover_report.rs

## Purpose

Provides a dev-only fixture report for the LSP hover path.

## Architecture Role

This example sits above the Rust LSP helper API. It exercises the same file-local hover projection used by `textDocument/hover` without starting VS Code or a stdio server.

## Current Behavior

The report reads targeted committed parser fixtures, downloaded game-data checks when available, plus small inline hover coverage sources for enum/global-field and usage cases. When game data exists, it builds a game-data index once and supplies it as external hover context. It writes `tools/reports/lsp-hover-fixtures.report.md` with target positions, hit/miss state, selection source, selected source, resolver reason, identifier context, receiver owner/failure, resolver candidate count, selected symbol kind/name, parse diagnostics, elapsed time, and a compact Markdown hover preview. Current checks include class, field, method, parameter, typedef, enum member, global field, local variable, `foreach` variable, `for` initializer, receiver/member access, resolver syntax-span hover on a non-identifier return type, whitespace miss behavior, `SCR_BaseGameMode` game-data hover cases, external type checks for `Widget`, `IEntity`, and `SCR_ScenarioFrameworkGet`, and `SCR_AutotestResult` constructor/type-name collision cases.

## Dependencies and Boundaries

The report uses only Rust standard library code and the existing LSP/index helper functions. It accepts `--scripts <path>` to override the downloaded game-data scripts folder. It must remain dev-only review tooling. It must not perform semantic lookup, Workbench validation, runtime logging, VS Code command registration, or source mutation.

## Change Notes

- Added the first hover fixture report alongside the LSP hover feature.
- Inline enum/global source is used because the current committed parser fixture set does not contain those hover shapes.
- Added local/block symbol hover checks from `tools/fixtures/parser/local_block_symbols.c`.
- Added resolver-first hover visibility through selection source, resolver reason, and candidate count columns.
- Added downloaded game-data hover checks for `Game/GameMode/SCR_BaseGameMode.c`, including a deliberate external-base miss that documents the current file-local hover boundary.
- Added identifier context and game-data checks for constructor/type-name collisions such as `SCR_AutotestResult`.
- Added external game-data index context and selected-source reporting for hover fixture checks.
- Added receiver/member-access hover checks and receiver owner/failure report columns.
- Added a resolver syntax-span hover check for non-identifier positions inside declarations.

## Future Improvements

- Add larger game-data-derived hover samples when needed.
- Add a corpus hover report only after there is a useful set of deterministic hover points to sample.
