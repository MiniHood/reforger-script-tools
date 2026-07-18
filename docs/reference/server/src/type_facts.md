# server/src/type_facts.rs

## Purpose

Provides read-only source-backed type facts copied into the symbol index.

## Ownership

`TypeFacts` is the narrow fact-access boundary between indexed detail records and expression/resolver consumers. It owns no inference or storage.

## Current Behavior

The borrowed facade returns declared type text, callable returns, class/enum bases, typedef targets, defaults, enum values, container names, and compact `SymbolTypeFacts` snapshots. Narrow helpers expose value, typedef, callable, base, and enum-member views without allocation, normalization, alias resolution, generic instantiation, or expression evaluation.

It works over open-document indexes, workspace overlays, and cached game-data indexes because it only reads copied records. `expression_type` builds reusable receiver-owner inference on these facts.

## Dependencies and Boundaries

Depends on index and `model::SymbolKind`. It does not parse/inspect AST, walk files, handle LSP, choose feature policy, call Workbench, or perform semantic inference.

## Verification

Type-fact tests cover each detail view and indexed-symbol kind; expression-type tests exercise downstream use.

## Future Direction

Keep raw access here. Structured type shapes and real type checking belong in dedicated semantic layers.
