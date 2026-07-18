# server/examples/ast_report.rs

## Purpose

Generates a human-readable report of AST declaration extraction over committed parser fixtures.

## Architecture Role

This is developer review tooling for the Rust AST wrapper layer. It is not runtime extension behavior, not LSP wiring, not semantic indexing, and not Workbench validation.

## Current Behavior

The example parses each committed parser fixture, builds an `AstSourceFile`, and writes `tools/reports/ast-fixtures.report.md` by default. The report lists parse diagnostic counts, top-level declarations, global fields, typedef aliased type text, enum attributes, enum member values, doc comment counts/previews, class members, names, spans, attribute counts, modifier text, return/type text, parameter counts, parameter name/type/default/modifier details, non-declaration callable fragments, constructor members, and destructor members.

It accepts `--out <path>` for an explicit report destination.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser and AST modules. It must not duplicate AST extraction logic, inspect workspace/game-data corpora, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added the first AST fixture report for reviewing source-backed declaration extraction.
- Added destructor rendering so `void ~Name()` appears as a destructor instead of a method returning `void ~`.
- Added global-field rendering for top-level `Declaration::Field` values.
- Added enum attribute counts to declaration output.
- Added enum member value rendering for explicit source-backed values.
- Added doc comment counts and first-line previews for declarations and class members.
- Added constructor rendering through AST class-context method classification.
- Added parameter detail rendering from the AST parameter accessors.
- Added rendering for AST-classified non-declaration callable fragments.
- Added typedef aliased type text rendering.

## Future Improvements

- Add corpus-scale AST extraction reporting after the file-local AST API stabilizes.
- Add richer declaration details only when future AST/model work needs them.
