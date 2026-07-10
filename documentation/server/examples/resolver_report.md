# server/examples/resolver_report.rs

## Purpose

Provides a dev-only fixture report for the reference resolver scaffold.

## Architecture Role

This example sits above `server/src/resolver.rs` and exercises identifier resolution without starting VS Code or changing LSP runtime behavior.

## Current Behavior

The report writes `tools/reports/resolver-fixtures.report.md`. When downloaded game data is available, it first runs targeted real-source checks against files such as `Game/GameMode/SCR_BaseGameMode.c` and `Game/Sandbox/Resources/SCR_ResourceComponent.c`. It then runs inline game-data-shaped fallback checks plus a committed parser fixture.

Each check shows the cursor token, selected symbol, candidate list, reason, source snippet, and definition path/position when that position is available from the file-local source. External candidates show path and span but not line/column because the runtime index intentionally does not retain full source text.

## Dependencies and Boundaries

The report uses only Rust standard library code and existing parser, AST, model, index, and resolver APIs. It accepts `--scripts <path>` to override the downloaded game-data scripts folder and `--out <path>` to choose the report path. It must remain dev-only review tooling. It must not register VS Code commands, add package scripts, call Workbench, perform full semantic resolution, or mutate source files.

## Change Notes

- Added the first resolver fixture report for human/Codex review before hover or definition integration.
- Added real game-data resolver checks and source snippets so hover/definition behavior can be reviewed against current Reforger source shapes.

## Future Improvements

- Add receiver/member-access cases when resolver behavior expands beyond identifier-only lookup.
- Add a corpus-oriented resolver report only after deterministic target selection exists.
