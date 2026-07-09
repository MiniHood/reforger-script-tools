# server/src/index.rs

## Purpose

Owns the first in-memory workspace/game-data symbol index over file-local model catalogs.

## Architecture Role

This file sits above the model layer and below future semantic resolution, diagnostics, and LSP features. It aggregates many `SymbolCatalog` values into global symbol handles and lookup maps for names, kinds, classes, typedefs, owner methods, and parent-child relationships.

## Current Behavior

The index exposes `SymbolIndex::from_catalogs()` and `add_catalog()` for building an in-memory index from model catalogs. It assigns each catalog a `SourceFileId` and represents global symbols as `{ file_id, symbol_id }`, keeping `SymbolId` file-local. It stores compact copied lookup facts, including names and detail text, so lookup results remain usable without re-slicing source text.

The index can answer all-symbol name lookup, top-level name lookup, all-symbol preferred-name lookup sorted by source priority, top-level-only preferred-name lookup for declaration conflict review, preferred ordering for an explicit symbol ID slice, symbol-kind lookup, class lookup by name, typedef lookup by name, method lookup by owner/name, field lookup by owner/name, direct class-member lookup by owner, best-effort inherited member lookup by exact base class name, method signature display, method owner/name group iteration for report tooling, child lookup, duplicate top-level-name review, source-kind counts, and map-size counts.

## Dependencies and Boundaries

This file depends on lexer spans and the model layer. It must not parse source, extract AST declarations, resolve symbols semantically, evaluate typedefs/defaults/enum values, infer inheritance, merge partial classes, call Workbench, perform file watching, write binary caches, or handle LSP requests. Persistence and cache formats are future work and must remain optional cache behavior, not source truth.

## Change Notes

- Added the first in-memory index layer with `SourceFileId`, `GlobalSymbolId`, indexed files, indexed symbols, and lookup maps.
- Kept global IDs separate from file-local `SymbolId`.
- Added a separate top-level name map so conflict review is not polluted by repeated parameter/local member names.
- Added source-priority ordering for all-symbol preferred lookup and top-level-only preferred lookup without treating either as semantic resolution.
- Added read-only method owner/name group access so reports can show overload groups without duplicating index state.
- Added preferred ordering for explicit symbol ID slices so debug tooling can reuse index preference rules for class, typedef, and method queries.
- Added direct class field/member lookup maps and method signature display for future completion/signature-help groundwork. These are direct-owner lookups only and do not perform inherited member resolution.
- Added a best-effort inherited member lookup scaffold that walks `base_type` by exact class name with cycle protection. It preserves direct-then-base order and does not merge overrides, resolve generics, or apply modded-class semantics.

## Future Improvements

- Add incremental update behavior for changed workspace files.
- Add explicit workspace-over-game-data override reporting after real workspace indexing exists.
- Replace best-effort inherited member lookup with semantic class/inheritance resolution when that layer exists.
- Add optional persisted cache only if startup measurements justify it.
- Add semantic resolution as a separate model/index layer after lookup behavior is validated.
