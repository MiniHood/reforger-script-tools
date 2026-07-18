# server/src/index_query.rs

## Purpose

Provides the editor-facing query facade over the raw symbol index.

## Ownership

`IndexQuery` owns feature-safe lookup and completion candidate shaping. It applies source-category filtering and ordering while `SymbolIndex` retains broad raw facts and `SymbolDisplay` supplies shared presentation data.

## Current Behavior

The facade offers kind-specific preferred class/typedef/function lookup, top-level conflict review, callable signatures, display records, class/static member completion, bounded top-level/type completion, and selected scoped-symbol shaping. Raw all-symbol, top-level, and owner-name APIs remain explicit debug escape hatches.

Completion includes source/provenance/spans/display data, constructor signatures, callable form, conditional context, and best-effort direct/overlay/inherited origin. Default editor policy includes workspace and runtime categories and excludes docs, tests, Workbench, and unknown sources. Matching is case-insensitive and ranked by exact case, exact insensitive, camel/word-boundary abbreviation, prefix, then conservative subsequence before limits apply.

Typedef owners can expose clear target-owner members; static owners expose enum members or static class members, including source-backed `Class.Cast`. These are query conveniences, not generic instantiation, semantic inheritance, or API validation.

## Dependencies and Boundaries

Depends on index/model types. It does not parse, build/mutate indexes, merge `modded` classes, evaluate values/types, persist caches, call Workbench, or handle LSP protocol requests.

## Verification

Query tests cover category policy, match ranking and limits, completion origins, static/typedef owner paths, signatures, and display shaping.

## Future Direction

Structured type filtering and semantic direct/overlay/inherited classification require later semantic work.
