# tools/game-data-discovery.mjs

## Purpose

Scans a downloaded or manually supplied Reforger game script corpus and writes a Markdown discovery report for Codex and human planning.

## Ownership

This is repo-only developer tooling. It is not part of the VS Code extension runtime, does not use VS Code APIs, and must not be registered as an extension command. The tool helps choose future parser, semantic model, indexing, and fixture priorities from real script data while keeping discovery code out of `src/`.

## Current Behavior

The script defaults to the extension's local global-storage game-data path and accepts `--scripts <path>` for manual corpus locations. It writes to `tools/reports/game-data-discovery.report.md` by default and supports `--out <path>`.

The report includes source metadata, file counts, byte counts, top-level folder distribution, largest files, conservative declaration examples, RPC/Rpl usage, component/entity/plugin-like classes, and parser priority notes. Findings are regex-based discovery signals only and must not be treated as Workbench/compiler truth.

## Dependencies and Boundaries

The script uses only Node built-in modules. It may read extension global-storage game data and metadata as input, but it must not become a runtime dependency of the extension.

Do not move this tool under `src/`, do not import it from extension code, and do not add package command or VS Code command registration unless the behavior intentionally becomes user-facing.

## Verification

Run the script against a known script corpus and inspect the generated ignored report for paths, counts, and clearly labeled discovery-only findings.
