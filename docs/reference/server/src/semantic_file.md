# `server/src/semantic_file.rs`

## Purpose

Owns compiler-produced semantic declaration facts for one parsed source file.
It is the successor ingress representation for workspace and game-data index
contributions; it is not an LSP feature cache or a serialized parser tree.

## Ownership

`SemanticFile` consumes the typed CST facade through its zero-copy declaration
iterator and records declaration identity, hierarchy, source/selection spans,
modifiers, attributes, comments, signature details, callable form, parameters,
and local bindings. File-private callable regions group each callable's
parameters and locals for bounded future cursor queries. Its
`FileContribution` projection exposes external declarations and callable
signature parameters with their snapshot-local IDs, parents, spans, details,
modifiers, attributes, documentation, directive context, and callable form.
This permits lossless `SymbolIndex` reconstruction without source text or a
legacy catalog. It carries schema and source-manifest versions and validates
both plus its required public names before publication; file-private locals
remain available only to the file-local semantic/query path.

It does not resolve names, decide editor presentation, manage open-document
revisions, read files, or own workspace snapshot replacement.

## Current Behavior

`index_build` and workspace external-overlay ingestion construct a
`SemanticFile` directly from `AstSourceFile`, project and validate a versioned
`FileContribution`, then reconstruct their `SymbolIndex` through the validated
contribution boundary without constructing `SymbolCatalog`. The projection
preserves declaration and parameter signature facts, modifiers, documentation,
directive context, spans, and callable form. Conditional-directive context
remains an explicit follow-up semantic fact before file-local LSP analysis can
cut over completely.

`SemanticBuildStats` records source-free directive-line, declaration-record,
and macro-scan operation counts. Directive branch stacks are interned once per
file and declarations retain compact context IDs, rather than cloned stacks.
Scale tests use these counters instead of hardware-sensitive timing to guard
against reintroducing repeated whole-source conditional scans.

## Verification

Run `cargo test semantic_file --lib`, focused direct-index tests, and the full
`cargo test` suite from `server/` for changes affecting its consumers.

## Future Direction

Complete the file-local fact set—conditional context and syntax/scope query
anchors—then replace legacy catalog construction in the open-document analysis
path. Do not create a second TypeScript or LSP-specific semantic model.
