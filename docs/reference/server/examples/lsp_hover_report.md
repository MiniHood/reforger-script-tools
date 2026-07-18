# server/examples/lsp_hover_report.rs

## Purpose

Provides a dev-only fixture report for the LSP hover path.

## Ownership

This example sits above the Rust LSP helper API. It exercises the same file-local hover projection used by `textDocument/hover` without starting VS Code or a stdio server.

## Current Behavior

The report reads targeted committed parser fixtures, downloaded game-data checks when available, plus small inline hover coverage sources for enum/global-field and usage cases. When game data exists, it builds a game-data index once and supplies it as external hover context. It writes `tools/reports/lsp-hover-fixtures.report.md` with target positions, hit/miss state, selection source, selected source, resolver reason, identifier context, receiver owner/failure, receiver expression kind, receiver lookup path, resolver candidate count, selected symbol kind/name, parse diagnostics, elapsed time, and a compact Markdown hover preview. Current checks include class, field, method, parameter, typedef, enum member, global field, local variable, `foreach` variable, `for` initializer, receiver/member access, resolver syntax-span hover on a non-identifier return type, whitespace miss behavior, `SCR_BaseGameMode` game-data hover cases, external type checks for `Widget`, `IEntity`, and `SCR_ScenarioFrameworkGet`, `SCR_AutotestResult` constructor/type-name collision cases, and `GetGame()` external receiver-chain cases.

## Dependencies and Boundaries

The report uses only Rust standard library code and the existing LSP/index helper functions. It accepts `--scripts <path>` to override the downloaded game-data scripts folder. It must remain dev-only review tooling. It must not perform semantic lookup, Workbench validation, runtime logging, VS Code command registration, or source mutation.

## Verification

Run `cargo run --example lsp_hover_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
