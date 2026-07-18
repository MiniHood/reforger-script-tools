# `server/src/lsp/definition.rs`

## Purpose

Projects resolver-selected declarations into LSP definition links for
navigation.

## Ownership

Owns conversion of file-local and external candidates into `LocationLink[]`,
including file URI encoding and target/origin range projection. It does not own
symbol lookup policy, external-index lifecycle, workspace watching, or request
dispatch.

## Current Behavior

The resolver selects the candidate using file-local facts plus layered
workspace and game-data indexes. Local targets use cached source analysis;
external targets read source only to project stored byte spans into LSP ranges.
The response preserves origin selection ranges and returns no result for
ambiguous or non-symbol positions. URI generation handles local, drive-letter,
UNC, and extended UNC paths.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `FileIndexAnalysis`, `SymbolIndex`, external
index snapshots, and shared UTF-16 helpers from `lsp.rs`. All future resolution
rules belong in the resolver, not in this projection layer.

## Verification

Run focused definition tests and `cargo test` from `server/`. Cover local and
external targets, workspace-over-game-data precedence, null results, Unicode
ranges, and Windows UNC URI forms.

## Future Direction

Support deliberate multi-target lookup only when resolver policy can select and
explain multiple declarations.
