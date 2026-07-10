# server/examples/lsp_hover_corpus_report.rs

## Purpose

Generates a dev-only corpus report for the resolver-first LSP hover path across downloaded or explicitly provided Reforger script data.

## Architecture Role

This example sits above `server/src/lsp.rs` and exercises the same hover projection used by `textDocument/hover`. It builds the game-data index once as external resolver context, then samples identifier-token positions and runs parser, AST, model, file-local index, resolver, query, and display logic once per sampled file.

It is review tooling only. It is not runtime VS Code behavior, not Workbench validation, not semantic workspace lookup, and not a persisted cache.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-hover-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for labeling the timing profile.
- `--samples-per-file <count>` for deterministic bounded identifier sampling.

The companion wrapper is `tools/lsp-hover-corpus-report.mjs`. It accepts `--release` and runs `cargo run --release` while passing `--profile-label release` to the Rust example. Debug mode remains the default.

The report includes hover hit/miss totals, file-local hit counts, external hit counts, resolver reason frequency, identifier context frequency, receiver owner/failure frequency, selected source frequency, selected symbol kind frequency, top files by hover misses, bounded miss samples, bounded hit samples, and timing. It intentionally samples identifiers instead of dumping every possible hover token.

## Dependencies and Boundaries

Uses only Rust standard library APIs and existing crate LSP helpers. It must not register VS Code commands, package scripts, or runtime extension behavior.

The report uses the game-data index as external context. Remaining misses are unresolved after file-local and external top-level/member lookup. Receiver/member-call resolution is shallow and source-backed; it is not full expression typing, overload resolution, Workbench validation, or workspace indexing.

## Change Notes

- Added the first corpus-scale hover report to review resolver-first hover behavior beyond targeted fixtures.
- Added identifier context reporting so type-position behavior can be reviewed across sampled corpus hovers.
- Added external game-data index context, selected-source frequency, and file-local/external hit counts.
- Added receiver owner/failure frequencies and receiver details in hit/miss samples for member-access review.

## Future Improvements

- Add focused miss classification if unresolved samples reveal repeated resolver gaps.
- Add release timing comparisons when hover performance becomes a runtime concern.
