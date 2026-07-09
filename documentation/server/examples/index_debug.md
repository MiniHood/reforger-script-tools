# server/examples/index_debug.rs

## Purpose

Prints compact index lookup debug output for the downloaded or manually selected game-data script corpus.

## Architecture Role

This is developer/Codex inspection tooling for the in-memory symbol index. It is not VS Code runtime behavior, not an LSP command, not semantic resolution, not a persisted cache, and not Workbench validation.

## Current Behavior

The example uses `server/src/index_build.rs` to build the same parser, AST, model, and index pipeline used by index corpus reporting. It accepts `--scripts <path>`, optional `--workspace <path>`, and exactly one exact lookup mode: `--name`, `--top-level`, `--class`, `--typedef`, `--function`, or `--method <owner> <name>`. Output includes corpus totals, parse diagnostics, all matches, the preferred match, source kind, priority, path, symbol kind, spans, details, callable signatures, owner-name aggregate class-member summaries, raw best-effort inherited/base-chain member summaries, raw aggregate completion summaries, preferred-class overlay completion summaries, shadowed member groups, and immediate children when useful.

For `--top-level`, the tool shows generic cross-kind preferred ordering for conflict/debug review and a separate kind-specific preferred section for class, typedef, and function lookups. Use the kind-specific rows when the expected declaration kind is known.

When `--workspace` is supplied, the debug index includes both game-data scripts and the workspace folder. Game-data files use priority `100`; workspace files use priority `200`, so preferred lookup output should show workspace symbols first when names overlap.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not resolve symbols semantically, infer inheritance, evaluate typedefs/defaults/enum values, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added dev-only index debugging for exact name, top-level, class, typedef, and method owner/name lookups.
- The tool rebuilds the in-memory game-data index per invocation; persisted index cache behavior remains future work.
- Method lookup now prints source-backed overload signatures, and class lookup shows direct member summaries from the index.
- Class lookup now also shows inherited/base-chain member counts and bounded inherited member samples from the index's exact-name inherited member scaffold.
- Class lookup now separates the raw all-candidates inherited member view from the completion-ready de-duplicated view, including bounded shadow groups that explain which base members were hidden by kind/name/signature keys.
- Added optional `--workspace <path>` overlay input for debugging workspace-vs-game-data preferred lookup behavior.
- Detail output now uses the general callable signature API, so functions, methods, constructors, and destructors can display source-backed signatures consistently.
- Class lookup headings now clarify that direct, inherited, and completion member sections are owner-name aggregate views. In overlay mode they can include members from multiple source files/source kinds and are not limited to the preferred class declaration.
- Completion member debug output now reflects priority-aware same-owner/depth de-duplication: workspace overlay members should be kept over matching game-data members with the same completion key, while inherited/base members still remain lower priority than direct members.
- Added exact `--function` lookup and kind-specific preferred top-level output so class/typedef/function conflicts are not reduced to one ambiguous generic preferred declaration.
- Class lookup now also prints preferred-class overlay completion. This is the future editor-facing view; raw owner-name aggregate completion remains visible for debugging.
- Switched debug indexing to the shared `index_build` module.

## Future Improvements

- Add optional workspace script roots after real workspace indexing exists.
- Add optional JSON output only if a future tool genuinely needs machine-readable index debug records.
