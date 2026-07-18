# server/examples/lsp_workspace_overlay_report.rs

## Purpose

Provides a dev-only proof report for the runtime workspace/game-data overlay used by LSP hover and definition.

## Current Behavior

The example builds small source-backed game-data and workspace indexes, merges them as the runtime external overlay would, and writes `tools/reports/lsp-workspace-overlay.report.md`.

The report checks:

- workspace member hover wins through the external overlay
- updating the workspace source changes later hover results
- deleting the workspace source removes stale workspace symbols
- definition targets the workspace source file when the workspace declaration is selected
- synthetic larger-workspace overlay update cost, including changed-file reindex timing and full overlay recompute timing

This report does not use VS Code APIs and does not persist a workspace cache.

## Commands

```powershell
node tools/lsp-workspace-overlay-report.mjs
```

The Rust example also accepts stress-shape flags:

```powershell
cargo run --manifest-path server/Cargo.toml --example lsp_workspace_overlay_report -- --stress-files 500 --stress-members 12 --stress-updates 50
```

Stress timings are dev-machine wall-clock diagnostics, not benchmarks. They are intended to show whether the current full overlay recompute approach is still acceptable or whether a future incremental overlay-map update slice is justified.

## Boundaries

This is observability only. It does not add semantic `modded` merge rules, diagnostics, completion, references, Workbench validation, or a persisted workspace cache.
