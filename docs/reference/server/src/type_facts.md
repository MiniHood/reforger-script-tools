# server/src/type_facts.rs

## Purpose

Exposes source-backed type facts copied into the symbol index.

## Architecture Role

This file sits above `index` and below future semantic/type resolution. It is the first dedicated place to ask what raw type-related facts are known for a symbol: declared type text, callable return type text, class/enum base type text, typedef target text, defaults, enum values, and containing class names.

## Current Behavior

`TypeFacts` wraps a borrowed `SymbolIndex` and returns borrowed facts from indexed symbols. It does not allocate normalized type records, resolve aliases, instantiate generics, evaluate enum/default expressions, or merge declarations. It is usable over open-document indexes, workspace overlays, and runtime game-data caches because it only depends on copied index facts.

`SymbolTypeFacts` is a compact snapshot for one symbol. It includes symbol identity, kind, optional name, containing class name, and optional raw detail strings. Helper methods expose narrower views for value symbols, typedef targets, callable return types, class/enum bases, and enum member values. `server/src/expression_type.rs` builds on this layer for reusable receiver-owner and generic-substitution facts.

## Dependencies and Boundaries

This file depends on `index` and `model::SymbolKind`. It must not parse source, inspect AST syntax, walk files, call Workbench, implement hover/completion/definition policy, or perform semantic type inference. Resolver-owned receiver inference can migrate toward this layer in later slices, but this file should remain a fact-access layer until a richer semantic/type environment is designed.

## Change Notes

- Added the first read-only type-facts facade over copied indexed detail text.
- Kept facts borrowed from the index so the layer works with compact runtime cache data and does not introduce another storage path.

## Future Improvements

- Add source-backed type-shape access over indexed type text when resolver/completion need structured owner/generic facts.
- Keep raw fact access here and put receiver/type-text inference in `expression_type`.
- Add a real semantic type environment separately; do not turn this file into a type checker by incrementally adding ad hoc resolution rules.
