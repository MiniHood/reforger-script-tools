# server/examples/index_debug.rs

## Purpose

Prints compact index lookup debug output for the downloaded or manually selected game-data script corpus.

## Architecture Role

This is developer/Codex inspection tooling for the in-memory symbol index. It is not VS Code runtime behavior, not an LSP command, not semantic resolution, not a persisted cache, and not Workbench validation.

## Current Behavior

The example builds the same parser, AST, model, and index pipeline used by index corpus reporting. It accepts `--scripts <path>` and exactly one exact lookup mode: `--name`, `--top-level`, `--class`, `--typedef`, or `--method <owner> <name>`. Output includes corpus totals, parse diagnostics, all matches, the preferred match, source kind, priority, path, symbol kind, spans, details, method signatures, direct class-member summaries, best-effort inherited/base-chain member summaries, and immediate children when useful.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not resolve symbols semantically, infer inheritance, evaluate typedefs/defaults/enum values, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added dev-only index debugging for exact name, top-level, class, typedef, and method owner/name lookups.
- The tool rebuilds the in-memory game-data index per invocation; persisted index cache behavior remains future work.
- Method lookup now prints source-backed overload signatures, and class lookup shows direct member summaries from the index.
- Class lookup now also shows inherited/base-chain member counts and bounded inherited member samples from the index's exact-name inherited member scaffold.

## Future Improvements

- Add optional workspace script roots after real workspace indexing exists.
- Add optional JSON output only if a future tool genuinely needs machine-readable index debug records.
