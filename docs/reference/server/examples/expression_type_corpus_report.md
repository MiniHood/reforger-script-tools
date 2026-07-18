# server/examples/expression_type_corpus_report.rs

## Purpose

Generates a dev-only corpus report for source-backed expression type inference.

## Ownership

This report exercises `server/src/expression_type.rs` across real parser expression nodes. It sits above parser, AST expression wrappers, model/index, lexical scope, and optional game-data index context. It validates the expression type environment that resolver, hover, definition, completion, and semantic tokens rely on.

## Current Behavior

The report scans `.c` scripts from the downloaded game-data folder by default, builds a per-file parser/AST/model/index/scope analysis, optionally builds one external game-data index, then asks `ExpressionTypeEnvironment` to infer an owner/type for every AST expression wrapper. It writes `tools/reports/expression-type-corpus.report.md`.

The report includes expression kind frequency, inferred/unresolved expression kind frequency, expression role frequency, top inferred owner types, unresolved reason frequency, unresolved classification, unresolved review buckets, category-specific actionable/review samples, bounded inferred samples, unresolved samples, deep member/call/index chain samples, and generic/index/cast samples.

Expression samples include the expression role and parent syntax kind. Roles distinguish standalone values from callees, member receivers, member names, named argument labels, call arguments, declaration defaults, and other child-expression positions. This keeps the report from treating every child `Name` wrapper as an independent type-environment failure.

The summary includes both raw unresolved expressions and actionable unresolved expressions. Raw unresolved keeps every expression wrapper visible for audit. Actionable unresolved excludes expected/source-noise buckets such as named argument labels, container/assignment wrappers, and child names already typed by a parent expression. The `typed by parent expression` classification means the child wrapper has no standalone type fact, but the enclosing call/member/index expression did infer successfully.

`Actionable / Review Unresolved Samples By Classification` exists so important buckets are not hidden by whichever files are scanned first. It prints bounded samples for probable expression-type defects, unresolved name/type facts, receiver/member chain issues, source/API unavailable facts, and declaration/type syntax.

`declaration/type syntax` is an expected/noise classification for expression wrappers that come from declaration signatures or type syntax rather than standalone runtime values. This keeps parameter, return-type, class, typedef, and enum declaration syntax from being confused with expression type-environment failures.

Supported flags:

-…2028 tokens truncated… but it does not change language-server startup behavior or cache policy.

## Current Behavior

The report measures an existing cache path, a temporary cache-miss rebuild/write, and a direct rebuild without cache. Cache measurements use the v9 runtime-pruned binary game-data cache, while direct rebuild measures the full source index. The report therefore checks that cache symbols equal direct rebuild symbols minus local variables and that parameter symbols remain preserved. It also compares a full-map JSON estimate against the v9 actual binary cache file, including string-table storage, detail-span stripping, and lookup-map rebuild visibility. Cache hit timing is split into file read, binary decode, validation, and lookup-map rebuild. The Node wrapper runs debug and release profiles and combines them into `tools/reports/index-cache-baseline.report.md`.

The runtime cache structural section also reports lookup-map shape: key counts and symbol-id entry counts for all rebuilt maps. This makes lookup-map rebuild time reviewable without persisting the maps in the cache.

Release timing is the only timing used for the cache usefulness decision. Debug timing is informational.

## Dependencies and Boundaries

Depends on `index_cache`, `index_build`, and public `SymbolIndex` data for counts and lower-bound memory estimates. It must not add runtime extension commands, mutate source files, change cache invalidation policy, or treat the cache as source truth.

## Verification

Run `cargo run --example index_cache_baseline` from `server/` and inspect the generated report for the documented fixture or corpus checks.
