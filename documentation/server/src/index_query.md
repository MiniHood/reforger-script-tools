# server/src/index_query.rs

## Purpose

Owns the editor-facing query facade over the raw symbol index.

## Architecture Role

This file sits above `server/src/index.rs` and below future LSP request handlers. It provides the preferred lookup surface that editor features should call for class lookup, typedef lookup, function lookup, callable signatures, top-level conflict review, and class member completion.

## Current Behavior

`IndexQuery` wraps a read-only `SymbolIndex`. It delegates source-priority and lookup ordering to the index while exposing safer feature-level APIs:

- kind-specific preferred lookup for classes, typedefs, and functions
- top-level conflict lookup without treating cross-kind results as semantic truth
- callable signature lookup for functions, methods, constructors, and destructors
- symbol display lookup through `SymbolDisplay`
- editor-facing class completion through `completion_members_for_preferred_class`
- explicit raw/debug escape hatches for all-symbol lookup, top-level lookup, and owner-name aggregate completion

Editor completion candidates include symbol identity, name, kind, detail/signature text, source kind, source category, priority, paths, spans, conditional context, callable form, a shared `SymbolDisplayInfo`, and a best-effort origin of direct, overlay, inherited, or unknown. The display record exposes raw `doc_comments` plus a bounded `documentation_preview`; future editor code should not rebuild documentation presentation from raw index internals. The origin is derived from the current preferred-class completion ordering and owner class names; it is not semantic inheritance proof.

Editor completion applies the current source-category policy: include workspace, Game, GameCode, GameLib, Core, and generated runtime symbols by default; exclude docs/Doxygen, test/autotest, Workbench, and unknown categories. The preferred class anchor for completion must also come from an included source category; docs-only or test-only classes produce no editor candidates even though raw/debug lookup still exposes them.

When duplicate completion keys remain after source-category filtering, `IndexQuery` prefers direct/overlay/inherited ordering, then higher source priority, then callable form quality: implementation, declaration, prototype. This keeps source facts broad in `SymbolIndex` while giving future editor features a safer default view.

## Dependencies and Boundaries

This file depends on the index and model types only. It must not parse source, build catalogs, mutate index state, resolve symbols semantically, merge `modded` classes, evaluate types/defaults, call Workbench, persist caches, or handle LSP protocol requests.

Raw methods on `IndexQuery` are debug escape hatches. Future editor features should use the editor-facing APIs unless they are intentionally building debug/report output.

## Change Notes

- Added the first query facade so future LSP/editor code has a clear safe path over `SymbolIndex`.
- Kept raw owner-name aggregate completion available but separated it from editor-facing preferred-class completion.
- Added source and origin metadata to completion candidates for human review and future completion-item construction.
- Added source-category filtering, included-source preferred class anchoring, conditional context exposure, callable form exposure, and implementation-over-declaration/prototype preference for editor completion candidates.
- Added `symbol_display()` and embedded display info in completion candidates so future editor code has one canonical presentation shape.

## Future Improvements

- Add typed completion item shaping when the LSP layer exists.
- Add structured type-shape access when completion needs type-aware filtering.
- Replace best-effort origin classification if a future semantic model can distinguish true direct, modded overlay, and inherited members.
