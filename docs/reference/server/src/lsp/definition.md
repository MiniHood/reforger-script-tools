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
workspace and game-data indexes when matching analysis is available. Local
targets use cached source analysis; external targets read source only to
project stored byte spans into LSP ranges. While semantic analysis is pending
after foreground installation, the worker-built current snapshot may return
only a cursor already on a lexically proven
top-level class, enum, or typedef declaration, linking to that same current
declaration. References, members, locals, and recovery-shaped declarations
return no result in that state. This never joins current text to former local
semantic facts or re-lexes the document on the request loop. URI generation handles local, drive-letter, UNC, and extended
UNC paths.

## Dependencies and Boundaries

Depends on `ReferenceResolver`, `FileIndexAnalysis`, `SymbolIndex`, external
index snapshots, and shared UTF-16 helpers from `lsp.rs`. All future resolution
rules belong in the resolver, not in this projection layer.

## Verification

Run focused definition tests and `cargo test` from `server/`. Cover local and
external targets, workspace-over-game-data precedence, pending current-snapshot
declaration targets and unresolved references, null results, Unicode ranges,
and Windows UNC URI forms.

## Future Direction

Support deliberate multi-target lookup only when resolver policy can select and
explain multiple declarations.
