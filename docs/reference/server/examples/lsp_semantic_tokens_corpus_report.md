# server/examples/lsp_semantic_tokens_corpus_report.rs

## Purpose

Generates a dev-only corpus report for the LSP semantic-token projection that drives Enforce coloring.

## Architecture Role

This example sits above `server/src/lsp.rs` and exercises the same semantic-token conversion used by `textDocument/semanticTokens/full`. It validates the Rust semantic-token coloring path after the TextMate grammar removal.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/lsp-semantic-tokens-corpus.report.md`. It supports:

- `--scripts <path>` for an explicit scripts folder.
- `--out <path>` for an explicit report path.
- `--profile-label <label>` for labeling timing output.
- `--max-files <n>` for bounded investigation runs.
- `--file <path>` for targeting one specific source file while still using `--scripts` as the external game-data index root.
- `--no-external-index` to measure file-local semantic coloring without a game-data index.
- `--runtime-only` to measure the token-only path used by live `textDocument/semanticTokens/full` without decoded debug rows.

By default the report builds a game-data index once and passes it as external semantic context for each file, matching the live server's ability to color references through the external overlay. It reports token type frequency, modifier frequency, identifier coloring coverage, declaration/reference/lexical token split, weakest coverage files, uncolored identifier classification, uncolored identifier samples, projection phase timing, and overall timing.

The default mode is coverage/debug oriented and builds decoded semantic-token rows for human review. `--runtime-only` skips decoded rows and coverage classification so it can measure the same lower-allocation projection path used by the live LSP semantic-token request.

Uncolored identifiers are classified with their actual token offset and local source context. The report separates named call argument labels, attribute named arguments, attribute enum/static values, preprocessor directive/macro tokens, enum/static member values, unresolved member fields/methods, and genuinely unresolved identifiers so semantic-token coverage does not overstate language-engine misses. Per-classification sample tables include line, column, token, and the trimmed source line so remaining gaps are actionable.

## Dependencies and Boundaries

Uses only Rust standard library APIs, the existing index builder, lexer, and LSP semantic-token helper. It must remain dev-only review tooling. It must not register VS Code commands, mutate source, add runtime logging, or create a separate coloring implementation.

## Change Notes

- Added after removing the TextMate grammar so semantic-token coverage can be reviewed across real game data, not only fixture examples.
- Added uncolored-identifier classification after corpus coverage became high enough that named labels and attribute/source-noise positions dominated the remaining samples.
- Added source-context sample tables by classification so unresolved semantic-token gaps can be reviewed as member-chain/type-inference issues instead of a single broad unresolved bucket.
- Added `--runtime-only` after large-file investigation showed that live semantic-token performance must be measured separately from decoded debug row generation.
- Added internal semantic-token timing so large-file reports show lexing, resolver, declaration overlay, sort/filter/split, encoding, and report-only decode cost separately.

## Future Improvements

- Add release-mode timing comparison only if semantic-token generation becomes a visible editor performance problem.
- Add source-kind/origin splits if workspace overlay coloring needs corpus-scale review.
