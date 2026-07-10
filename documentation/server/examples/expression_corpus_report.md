# server/examples/expression_corpus_report.rs

## Purpose

Generates a dev-only corpus report for statement/expression parser coverage across downloaded or manually supplied Reforger `.c` scripts.

## Behavior

The report scans `.c` files, parses them, and summarizes statement/expression syntax kind frequencies, diagnostics, recovery/error nodes, deepest expression trees, named arguments, and initializer expressions. It is intended to guide the next resolver/hover slice after body parsing is stable.

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
