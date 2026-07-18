# server/examples/expression_report.rs

## Purpose

Generates a dev-only Markdown report for reviewing statement and expression syntax in committed fixtures and selected game-data-derived files.


## Ownership

This dev-only Rust example owns a repeatable report or debug projection for its named language-engine subsystem. It must not become extension runtime behavior.

## Current Behavior

The report runs the parser over committed parser fixtures plus selected game-code examples when the local game-data scripts folder is available. It records diagnostics, token preservation, statement/expression kind counts, max expression depth, and a bounded body syntax outline.

This is review tooling only. It does not affect runtime LSP behavior, resolver hover, cache format, or Workbench validation.

## Verification

```powershell
cargo run --manifest-path server/Cargo.toml --example expression_report
node tools/expression-report.mjs
```

Optional flags:

```powershell
--scripts <path>
--out <path>
```

## Dependencies and Boundaries

Depends on the Rust language-engine owners and repository fixtures named by the example. It may write a developer report but must not become an LSP runtime path or source of language truth.
