# server/examples/lsp_hover_corpus_report.rs

## Purpose

Generates a dev-only corpus report for the resolver-first LSP hover path across downloaded or explicitly provided Reforger script data.

## Ownership

This example sits above `server/src/lsp.rs` and exercises the same hover projection used by `textDocument/hover`. It builds the game-data index once as external resolver context, then samples identifier-token positions and runs parser, AST, model, file-local index, resolver, query, and display logic once per sampled file.

It is review tooling only. It is not runtime VS Code behavior, not Workbench validation, not semantic workspace lookup, and not a persisted cache.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-hover-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for labeling the timing profile.
- `--samples-per-file <count>` for deterministic bounded identifier sampling.

The companion wrapper is `tools/lsp-hover-corpus-report.mjs`. It accepts `--release` and runs `cargo run --release` while passing `--profile-label release` to the Rust example. Debug mode remains the default.

The report includes hover hit/miss totals, file-local hit counts, external hit counts, raw hit rate, actionable hover hit rate, resolver reason frequency, identifier context frequency, receiver owner/failure frequency, receiver expression-kind samples, selected source frequency, selected symbol kind frequency, remaining miss classification, top files by actionable hover misses, top files by raw hover misses, bounded miss samples, bounded hit samples, and timing. It intentionally samples identifiers instead of dumping every possible hover token.

## Dependencies and Boundaries

Uses only Rust standard library APIs and existing crate LSP helpers. It must not register VS Code commands, package scripts, or runtime extension behavior.

The report uses the game-data index as external context. Remaining misses are unresolved after file-local and external top-level/member lookup. Receiver/member-call resolution is syntax-backed through AST expression views but remains shallow and source-backed; it is not full expression typing, overload resolution, Workbench validation, or workspace indexing. Named argument labels are suppressed by resolver and classified separately as call or attribute labels so they do not look like actionable unresolved symbol failures. Miss classification uses the sampled token offset in the original source line instead of searching for the first matching token text in a truncated snippet.

The raw hit rate keeps every sampled identifier in the denominator for continuity. The actionable hover hit rate excludes sampled misses classified as attribute named arguments, attribute enum/static values, preprocessor directive or macro tokens, named call argument labels, and Workbench/docs/test source noise. Attribute named arguments, preprocessor directives, preprocessor macro names, and named argument labels are classified from resolver-owned non-symbol reasons first; report-local source-line heuristics remain only for attribute value and source-policy buckets. This makes the corpus report better reflect resolver/editor quality without hiding the raw counts.

## Verification

Run `cargo run --example lsp_hover_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
