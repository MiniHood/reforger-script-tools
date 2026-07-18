# server/examples/lsp_completion_corpus_report.rs

## Purpose

Generates a dev-only corpus report for LSP completion behavior across real game-data member-access and identifier-prefix positions.

## Architecture Role

This example sits above `server/src/lsp.rs` and exercises the same `textDocument/completion` helper used by the Rust LSP. It samples real `receiver.` positions plus real identifier-prefix positions such as `SCR_`, `GetG`, `Widget`, and generic type prefixes, then validates receiver inference, type/top-level prefix context, owner lookup, candidate counts, and completion item shape against game data.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-completion-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for timing labels.
- `--max-files <n>` for bounded file scans.
- `--max-checks <n>` for bounded completion checks.

It builds a game-data index once, samples member-access dot positions and identifier-prefix positions evenly across the bounded file set, builds each sampled file's parser/AST/model/index analysis once, and runs all sampled completion positions in that file against the cached analysis. It reports completion context frequency, failure reason frequency, empty-result classification, candidate-count buckets, top inferred owner types, empty/failure samples, large-candidate samples, and timing.

Completion projection uses the same optimized LSP path as live completion: it queries the open-document file-local index and external workspace/game-data index separately, then combines just the returned candidates. It must not rebuild or merge a full external index per completion request. Member completion uses resolver-owned expression context, including full receiver-chain selection before the dot and direct `new Type(...)` receiver inference. The timing section includes the broad completion projection time plus reported phase totals for context/receiver detection, candidate lookup, item rendering, and reported completion total. External index build/load time remains a separate phase.

Empty completion results are classified so review can distinguish expected non-items from actionable defects:

- `enum/static owner` for enum/static-value style member access that still produced no items after static-owner completion policy.
- `static class owner` for static class access that still produced no copied static member items.
- `excluded source / non-completion-worthy` for docs, Workbench, Autotest/test, and similar source categories that editor completion excludes by policy.
- `unresolved receiver` when receiver type inference failed.
- `no members indexed for owner` when the owner is known but has no indexed members.
- `source-noise / non-completion-worthy` for attribute/named-label/preprocessor-style positions.
- `true completion defect` for member contexts that infer an owner, have indexed members, and still return no items.

## Dependencies and Boundaries

Uses only Rust standard library APIs, the lexer, reusable index builder, and existing LSP completion helper. It must stay dev-only. It must not register VS Code commands, perform Workbench validation, add diagnostics, or implement a second completion path.

## Change Notes

- Added after member/type/top-level completion existed so real game-code completion behavior can be reviewed beyond synthetic fixture checks.
- Updated to reuse cached per-file analysis for all sampled completion positions in a file.
- Added empty-result classification so `0`-item completions are not all treated as the same failure mode.
- Removed per-completion full-index merging from the report/live helper path and added phase timing for completion projection.
- Updated static/enum owner buckets after static-owner completion started returning enum members and copied static class members.
- Added excluded-source classification so Autotest/Workbench/docs samples do not look like editor completion defects.
- Updated after receiver-chain and direct-`new` receiver inference so the bounded corpus can distinguish source-noise empties from true completion defects.
- Added type/top-level identifier-prefix sampling so the report covers `SCR_`, `GetG`, and generic type-argument completion paths, not only member-dot completion.

## Future Improvements

- Add explicit source-kind and origin-mix tables if workspace overlay completion needs corpus-scale review.
