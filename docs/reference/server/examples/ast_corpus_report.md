# server/examples/ast_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report by running parser plus AST declaration extraction across a Reforger scripts folder.

## Ownership

This is developer review tooling for AST extraction quality. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds an `AstSourceFile`, and writes `tools/reports/ast-corpus.report.md` by default. It reports corpus totals, parse diagnostics, extracted declaration/member counts including global fields, class fields, enum members, enum members with explicit values, regular methods, constructors, destructors, parameters with defaults, non-declaration callable fragments, attached doc comments, unknown extraction counters, parser-vs-AST attribute coverage including enum-level attributes, base/type/modifier/attribute/doc-comment-kind/enum-value/parameter frequencies, parameter type/modifier frequencies, top files with unknown extraction, bounded snippets, unmatched attribute snippets, and files that required lossy UTF-8 decoding. Constructor and destructor methods are excluded from the regular method/function return-type frequency.

Literal-only argument fragments preserved inside declaration-shaped source, such as inactive preprocessor-branch `void Name(false);` forms, come from the AST `parameter_fragments()` API and are reported separately as non-declaration callable fragments. They should stay visible for human review but should not count as unknown parameter names or types.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate parser/AST modules. It must not duplicate AST extraction behavior, add syntax rules, resolve symbols, index declarations, call Workbench, become VS Code runtime code, or become a package command.

## Verification

Run `cargo run --example ast_corpus_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
