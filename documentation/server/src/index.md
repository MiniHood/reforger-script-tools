# server/src/index.rs

## Purpose

Owns the first in-memory workspace/game-data symbol index over file-local model catalogs.

## Architecture Role

This file sits above the model layer and below future semantic resolution, diagnostics, and LSP features. It aggregates many `SymbolCatalog` values into global symbol handles and lookup maps for names, kinds, classes, typedefs, owner methods, and parent-child relationships.

## Current Behavior

The index exposes `SymbolIndex::from_catalogs()` and `add_catalog()` for building an in-memory index from model catalogs. It assigns each catalog a `SourceFileId` and represents global symbols as `{ file_id, symbol_id }`, keeping `SymbolId` file-local. It stores compact copied lookup and presentation facts, including names, detail text, modifier text, raw attribute text and names, raw doc comments, conditional context summaries, and callable form metadata, so lookup and display results remain usable without re-slicing source text.

The index can answer all-symbol name lookup, top-level name lookup, all-symbol preferred-name lookup sorted by source priority, top-level-only preferred-name lookup for declaration conflict review, preferred ordering for an explicit symbol ID slice, symbol-kind lookup, class lookup by name, typedef lookup by name, function lookup by name, kind-specific preferred class/typedef/function lookup, method lookup by owner/name, field lookup by owner/name, direct class-member lookup by owner, best-effort inherited member lookup by exact base class name, raw owner-name aggregate completion lookup, preferred-class overlay completion lookup, callable signature display, method owner/name group iteration for report tooling, child lookup, duplicate top-level-name review, source-kind counts, and map-size counts. Local variables are indexed for all-symbol lookup and hover/debug display, but they are not top-level declarations, class members, inherited members, or completion members.

Source-root scanning, file reading, metadata creation, parser/AST/model catalog construction, and index population are owned by `server/src/index_build.rs`. Future tools and runtime code should use that builder instead of duplicating the pipeline around `SymbolIndex::add_catalog`.

Future editor/LSP features should prefer `server/src/index_query.rs` and `server/src/symbol_display.rs` over calling raw `SymbolIndex` APIs directly. `IndexQuery` exposes the intended editor-facing path for kind-specific preferred lookup, top-level conflict review, callable signatures, and preferred-class completion while keeping raw aggregate lookup available only as an explicit debug escape hatch. `SymbolDisplay` owns shared presentation formatting for labels, details, signatures, docs previews, attributes, modifiers, and provenance.

Dev-only overlay tooling can build an index from game data and an explicit workspace folder by assigning game-data catalogs priority `100` and workspace catalogs priority `200`. This uses existing source-priority ordering only; it does not merge declarations or interpret `modded` semantics.

## Dependencies and Boundaries

This file depends on lexer spans and the model layer. It must not parse source, extract AST declarations, resolve symbols semantically, evaluate typedefs/defaults/enum values, infer inheritance, merge partial classes, call Workbench, perform file watching, write binary caches, or handle LSP requests. Persistence and cache formats are future work and must remain optional cache behavior, not source truth.

## Change Notes

- Added the first in-memory index layer with `SourceFileId`, `GlobalSymbolId`, indexed files, indexed symbols, and lookup maps.
- Kept global IDs separate from file-local `SymbolId`.
- Added a separate top-level name map so conflict review is not polluted by repeated parameter/local member names.
- Added source-priority ordering for all-symbol preferred lookup and top-level-only preferred lookup without treating either as semantic resolution.
- Added kind-specific preferred lookup for classes, typedefs, and functions. Generic preferred top-level lookup remains a cross-kind conflict/debug view and should not be used as the authoritative answer when the desired declaration kind is known.
- Added read-only method owner/name group access so reports can show overload groups without duplicating index state.
- Added preferred ordering for explicit symbol ID slices so debug tooling can reuse index preference rules for class, typedef, and method queries.
- Added direct class field/member lookup maps and method signature display for future completion/signature-help groundwork. These are direct-owner lookups only and do not perform inherited member resolution.
- Added a best-effort inherited member lookup scaffold that walks `base_type` by exact class name with cycle protection. It preserves direct-then-base order and does not merge overrides, resolve generics, or apply modded-class semantics.
- Added `completion_members_for_class()` as a completion-facing view over the raw inherited member scaffold. It keeps raw candidates for debug review, returns direct members before inherited members, and hides later candidates with the same kind/name/signature key. Method keys use method name, parameter type shape, and return type while excluding owner names, parameter names, and defaults. Field keys use kind and name. Same owner/depth duplicate keys are resolved by source priority, so workspace overlay members can replace game-data members without allowing inherited/base members to beat direct members. This is still non-semantic and source-backed; it does not prove compiler override behavior.
- Added `completion_members_for_preferred_class()` as the future editor-facing class completion path. It starts from preferred class declarations, intentionally includes lower-priority same-owner overlay members, then appends exact-name base-chain members. The raw owner-name aggregate completion API remains available for debugging and report review.
- Added dev-only overlay report/debug usage that validates workspace priority over game-data priority without changing index semantics.
- Added `callable_signature()` for source-backed function, method, constructor, and destructor display. Parameter display includes source-backed parameter modifiers such as `out`, `inout`, and `notnull`. Kept `method_signature()` as a method-only compatibility API.
- Added `IndexBuild` in `server/src/index_build.rs` as the shared builder for explicit source roots and report/debug summaries.
- Added `IndexQuery` in `server/src/index_query.rs` as the future editor-facing facade over these raw lookup maps.
- Copied source category, conditional context, and callable form facts from model catalogs into indexed symbols for debug/report/query policy. `SymbolIndex` remains raw and policy-free; filtering belongs in `IndexQuery`.
- Copied modifiers, attributes, and doc comments into indexed symbols so future editor display does not need source text to show hover/completion/document-symbol facts.
- Indexed local variables by name and kind while keeping them out of top-level and class-member lookup maps.

## Future Improvements

- Add incremental update behavior for changed workspace files.
- Add local-scope-aware lookup only after a real scope model exists.
- Add explicit workspace-over-game-data override reporting after real workspace indexing exists.
- Replace best-effort inherited member lookup with semantic class/inheritance resolution when that layer exists.
- Add optional persisted cache only if startup measurements justify it.
- Add semantic resolution as a separate model/index layer after lookup behavior is validated.
