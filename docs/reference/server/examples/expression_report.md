# server/examples/expression_report.rs

## Purpose

Generates a dev-only Markdown report for reviewing statement and expression syntax in committed fixtures and selected game-data-derived files.

## Behavior

The report runs the parser over committed parser fixtures plus selected game-code examples when the local game-data scripts folder is available. It records diagnostics, token preservation, statement/expression kind counts, max expression depth, and a bounded body syntax outline.

This is review tooling only. It does not affect runtime LSP behavior, resolver hover, cache format, or Workbench validation.

## Usage

```powershell
cargo run --manifest-path server/Cargo.toml --example expression_report
node tools/expression-report.mjs
```

Optional flags:

```powershell
--scripts <path>
--out <path>
```
