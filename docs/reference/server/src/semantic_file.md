# `server/src/semantic_file.rs`

## Purpose

Owns compiler-produced semantic declaration facts for one parsed source file.
It is the successor ingress representation for workspace and game-data index
contributions; it is not an LSP feature cache or a serialized parser tree.

## Ownership

`SemanticFile` consumes parser-owned typed CST declarations directly through
`Parse::declarations` and records declaration identity, hierarchy, source/selection spans,
modifiers, attributes, comments, signature details, callable form, parameters,
and local bindings. File-private callable regions group each callable's
parameters and locals for bounded future cursor queries. Its
`FileContribution` projection exposes external declarations and callable
signature parameters with dense snapshot-local IDs, remapped parents, spans, details,
modifiers, attributes, documentation, directive context, and callable form.
This permits lossless `SymbolIndex` reconstruction without source text or a
legacy catalog. It carries schema and source-manifest versions and validates
both plus its required public names, dense IDs, and retained parent references
before publication; file-private locals
remain available only to the file-local semantic/query path.

It does not resolve names, decide editor presentation, manage open-document
revisions, read files, or own workspace snapshot replacement.

## Current Behavior

`index_build`, workspace external-overlay ingestion, and open-document analysis
construct a `SemanticFile` directly from parser output, project and validate a versioned
`FileContribution`, then reconstruct their `SymbolIndex` through the validated
contribution boundary without constructing `SymbolCatalog`. The projection
preserves declaration and parameter signature facts, modifiers, documentation,
directive context, spans, and callable form. Conditional-directive context
remains an explicit follow-up semantic fact before file-local LSP analysis can
cut over completely.

`SemanticBuildStats` records source-free directive-line, typed-CST declaration
visitor, declaration-record, and macro-scan operation counts. Directive branch
stacks are interned once per file and declarations retain compact context IDs,
rather than cloned stacks. The committed
`tools/fixtures/semantic/semantic_scale_declaration_unit.c` fixture is repeated
at 1x, 2x, and 4x by the semantic-file scale test; each observed counter must
grow by the same exact ratio. This avoids hardware-sensitive wall-clock gates
while guarding against repeated whole-source conditional scans or extra
whole-file declaration walks.

## Verification

Run `cargo test semantic_file --lib`, focused direct-index tests, and the full
`cargo test` suite from `server/` for changes affecting its consumers.

## Future Direction

Complete the file-local fact set—conditional context and syntax/scope query
anchors—then replace legacy catalog construction in the open-document analysis
path. Do not create a second TypeScript or LSP-specific semantic model.
