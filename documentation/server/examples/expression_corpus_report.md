# server/examples/expression_corpus_report.rs

## Purpose

Generates a dev-only corpus report for statement/expression parser coverage across downloaded or manually supplied Reforger `.c` scripts.

## Behavior

The report scans `.c` files, parses them, and summarizes statement/expression syntax kind frequencies, diagnostics, recovery/error nodes, deepest expression trees, named arguments, named-argument labels, initializer expressions, and member/call/index chain samples. It also includes body-structure quality sections for:

- `For Initializer Shape Coverage`: declaration-shaped initializers with nested `LocalDeclStatement` versus expression-form initializer lists.
- `Foreach Header Shape Coverage`: `ForeachVariableList`, `ForeachVariable`, and `ForeachIterable` coverage.
- `Switch Section Coverage`: `SwitchSection` grouping counts beside case/default label counts.
- `Expected Recovery Nodes`: known preserved preprocessor-test recovery versus unexplained recovery.
- `Expression Depth Samples With Snippets`: bounded examples of the deepest expression syntax.
- `Member / Call / Index Chain Samples With Snippets`: bounded examples of deep receiver/call/index shapes that feed resolver work.
- `Named Argument Label Frequency`: named argument labels such as `level`, `desc`, and `defvalue` so hover miss noise can be reviewed separately from unresolved identifiers.

It is intended to guide the next resolver/hover slice after body parsing is stable.
The expression wrappers are source-backed parser views; report counts are syntax coverage evidence, not semantic expression evaluation.

This is review tooling only. Workbench remains compiler truth.

## Usage

```powershell
cargo run --manifest-path server/Cargo.toml --example expression_corpus_report
node tools/expression-corpus-report.mjs
```

Optional flags:

```powershell
--scripts <path>
--out <path>
```
