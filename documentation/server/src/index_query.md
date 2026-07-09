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
- editor-facing class completion through `completion_members_for_preferred_class`
- explicit raw/debug escape hatches for all-symbol lookup, top-level lookup, and owner-name aggregate completion

Editor completion candidates include symbol identity, name, kind, detail/signature text, source kind, priority, paths, spans, and a best-effort origin of direct, overlay, inherited, or unknown. The origin is derived from the current preferred-class completion ordering and owner class names; it is not semantic inheritance proof.

## Dependencies and Boundaries

This file depends on the index and model types only. It must not parse source, build catalogs, mutate index state, resolve symbols semantically, merge `modded` classes, evaluate types/defaults, call Workbench, persist caches, or handle LSP protocol requests.

Raw methods on `IndexQuery` are debug escape hatches. Future editor features should use the editor-facing APIs unless they are intentionally building debug/report output.

## Change Notes

- Added the first query facade so future LSP/editor code has a clear safe path over `SymbolIndex`.
- Kept raw owner-name aggregate completion available but separated it from editor-facing preferred-class completion.
- Added source and origin metadata to completion candidates for human review and future completion-item construction.

## Future Improvements

- Add typed completion item shaping when the LSP layer exists.
- Add structured type-shape access when completion needs type-aware filtering.
- Replace best-effort origin classification if a future semantic model can distinguish true direct, modded overlay, and inherited members.
