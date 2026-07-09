# server/examples/index_overlay_report.rs

## Purpose

Generates a dev-only Markdown report for indexing game-data scripts together with an explicit workspace script folder.

## Architecture Role

This is developer review tooling for the future workspace/game-data overlay path. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic resolution, not Workbench validation, and not compiler truth.

## Current Behavior

The example accepts `--workspace <path>` as a required workspace script root, optional `--scripts <path>` for game data, and optional `--out <path>`. It uses `server/src/index_build.rs` to recursively scan `.c` files from both roots, build parser/AST/model catalogs, assign game-data metadata with priority `100`, assign workspace metadata with priority `200`, aggregate everything into `SymbolIndex`, and write `tools/reports/index-overlay.report.md` by default.

The report shows source counts, parse diagnostics by source kind, indexed symbols by source kind, bounded parse diagnostic snippets, workspace-involved duplicate classification buckets, workspace-involved duplicate top-level declarations, kind-specific preferred top-level declarations for classes/typedefs/functions, workspace preferred-failure audits, generic preferred top-level conflict/debug samples, workspace-only top-level samples, method owner/name overlays where workspace and game-data methods share the same owner/name key, and workspace method groups including workspace-only overload groups. Declaration rows use shared symbol display detail text, including callable signatures when available. Duplicate classification distinguishes true workspace/game-data overlays from workspace-local duplicate patterns such as typedef/function delegate-style pairs.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not resolve symbols semantically, merge `modded` declarations, evaluate compiler validity, watch files, write caches, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added the first explicit workspace plus game-data overlay report path.
- Kept overlay priority as source metadata and existing index preference behavior rather than adding semantic override logic.
- Overlay declaration details now use the general callable signature API for source-backed callable display.
- Added a `Workspace Preferred Failures` section so overlay reports show if any workspace-involved duplicate or method overlay fails to prefer a workspace symbol.
- Added kind-specific preferred top-level reporting and failure auditing for classes, typedefs, and functions. Generic top-level preferred ordering remains visible only as cross-kind conflict/debug review because names can legitimately contain unrelated declaration kinds.
- Switched overlay indexing to the shared `index_build` module.
- Added workspace duplicate classification buckets so overlay duplicates are explicitly labeled as workspace overlays instead of relying on readers to infer that from source kind.
- Refined duplicate classification so workspace-local typedef/function duplicates can still be labeled by pattern, while true workspace/game-data collisions remain workspace overlays.
- Added workspace-only top-level and workspace method-group sections so stronger overlay fixtures are visible in the report, not only through targeted debug output.
- Added parse diagnostic snippet rendering from builder-owned details so malformed workspace files are actionable in overlay reports.
- Switched declaration detail rows to shared `SymbolDisplay` output so overlay presentation matches query/debug display.

## Future Improvements

- Add workspace-root discovery through the future language server or VS Code integration.
- Add persisted cache only after real startup measurements justify it.
- Add semantic overlay/merge behavior only after Workbench-backed language behavior is validated.
