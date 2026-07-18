# server/examples/index_debug.rs

## Purpose

Prints compact index lookup debug output for the downloaded or manually selected game-data script corpus.

## Ownership

This is developer/Codex inspection tooling for the in-memory symbol index. It is not VS Code runtime behavior, not an LSP command, not semantic resolution, not a persisted cache, and not Workbench validation.

## Current Behavior

The example uses `server/src/index_build.rs` to build the same parser, AST, model, and index pipeline used by index corpus reporting. It accepts `--scripts <path>`, optional `--workspace <path>`, and exactly one exact lookup mode: `--name`, `--top-level`, `--class`, `--typedef`, `--function`, or `--method <owner> <name>`. Output includes corpus totals, parse diagnostics, all matches, the preferred match, source kind, source category, editor-completion inclusion/exclusion, priority, path, symbol kind, spans, display details, callable signatures, modifiers, attributes, doc previews, callable form, conditional context, owner-name aggregate class-member summaries, raw best-effort inherited/base-chain member summaries, raw aggregate completion summaries, raw preferred-class overlay completion summaries, true `IndexQuery` editor completion summaries, shadowed member groups with report-style likely-cause labels, and immediate children when useful.

Focused review flags keep large class output readable: `--limit <n>` caps repeated rows, `--member <name>` filters class member-heavy sections to an exact member name, `--symbol <name>` filters printed symbols/candidates by exact label, and `--show-docs` prints raw doc-comment text. By default, docs are shown as bounded previews only. Class lookups keep the preferred class anchor visible even when `--symbol` filters member/candidate rows, and member-heavy sections report filtered shown counts against total counts.

For `--top-level`, the tool shows generic cross-kind preferred ordering for conflict/debug review and a separate kind-specific preferred section for class, typedef, and function lookups. Use the kind-specific rows when the expected declaration kind is known.

When `--workspace` is supplied, the debug index includes both game-data scripts and the workspace folder. Game-data files use priority `100`; workspace files use priority `200`, so preferred lookup output should show workspace symbols first when names overlap.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not resolve symbols semantically, infer inheritance, evaluate typedefs/defaults/enum values, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example index_debug` from `server/` and inspect the generated report for the documented fixture or corpus checks.
