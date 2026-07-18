# server/examples/index_overlay_report.rs

## Purpose

Generates a dev-only Markdown report for indexing game-data scripts together with an explicit workspace script folder.

## Ownership

This is developer review tooling for the future workspace/game-data overlay path. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic resolution, not Workbench validation, and not compiler truth.

## Current Behavior

The example accepts `--workspace <path>` as a required workspace script root, optional `--scripts <path>` for game data, and optional `--out <path>`. It uses `server/src/index_build.rs` to recursively scan `.c` files from both roots, build parser/AST/model catalogs, assign game-data metadata with priority `100`, assign workspace metadata with priority `200`, aggregate everything into `SymbolIndex`, and write `tools/reports/index-overlay.report.md` by default.

The report shows source counts, parse diagnostics by source kind, indexed symbols by source kind, bounded parse diagnostic snippets, workspace-involved duplicate classification buckets, workspace-involved duplicate top-level declarations, kind-specific preferred top-level declarations for classes/typedefs/functions, workspace preferred-failure audits, generic preferred top-level conflict/debug samples, workspace-only top-level samples, method owner/name overlays where workspace and game-data methods share the same owner/name key, and workspace method groups including workspace-only overload groups. Declaration rows use shared symbol display detail text, including callable signatures when available. Duplicate classification distinguishes true workspace/game-data overlays from workspace-local duplicate patterns such as typedef/function delegate-style pairs.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not resolve symbols semantically, merge `modded` declarations, evaluate compiler validity, watch files, write caches, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example index_overlay_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
