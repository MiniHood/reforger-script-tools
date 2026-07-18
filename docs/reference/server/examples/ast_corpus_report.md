# server/examples/ast_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report by running parser plus AST declaration extraction across a Reforger scripts folder.

## Architecture Role

This is developer review tooling for AST extraction quality. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic indexing, not Workbench validation, and not compiler truth.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds an `AstSourceFile`, and writes `tools/reports/ast-corpus.report.md` by default. It reports corpus totals, parse diagnostics, extracted declaration/member counts including global fields, class fields, enum members, enum members with explicit values, regular methods, constructors, destructors, parameters with defaults, non-declaration callable fragments, attached doc comments, unknown extraction counters, parser-vs-AST attribute coverage including enum-level attributes, base/type/modifier/attribute/doc-comment-kind/enum-value/parameter frequencies, parameter type/modifier frequencies, top files with unknown extraction, bounded snippets, unmatched attribute snippets, and files that required lossy UTF-8 decoding. Constructor and destructor methods are excluded from the regular method/function return-type frequency.

Literal-only argument fragments preserved inside declaration-shaped source, such as inactive preprocessor-branch `void Name(false);` forms, come from the AST `parameter_fragments()` API and are reported separately as non-declaration callable fragments. They should stay visible for human review but should not count as unknown parameter names or types.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs and the crate parser/AST modules. It must not duplicate AST extraction behavior, add syntax rules, resolve symbols, index declarations, call Workbench, become VS Code runtime code, or become a package command.

## Change Notes

- Added corpus-scale AST extraction reporting for real downloaded/manual Reforger script data.
- Added bounded snippets around representative unknown extraction cases so AST gaps can be reviewed without dumping full files.
- Added a destructor count so destructor methods are visible without polluting method return-type frequencies as `void ~`.
- Added separate global-field and class-field counts while keeping the total field count.
- Added parser-vs-AST attribute coverage so unmatched parser attributes are visible without changing AST attachment behavior.
- Added enum attributes to AST corpus counts and attribute coverage.
- Added enum member totals, explicit value totals, and raw enum member value frequencies.
- Added attached doc-comment totals and line/block doc-comment kind frequency.
- Added constructor counts and regular-method counts using AST class-context method classification.
- Added parameter extraction quality counters plus parameter type and modifier frequencies.
- Switched non-declaration callable-fragment reporting to the AST `parameter_fragments()` API instead of local corpus-report classification.

## Future Improvements

- Add targeted sections for common unknown extraction patterns after the first report is reviewed.
- Keep corpus findings as planning evidence only; Workbench remains compiler truth.
