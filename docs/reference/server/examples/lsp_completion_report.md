# server/examples/lsp_completion_report.rs

## Purpose

Provides a dev-only fixture report for LSP completion.

## Current Behavior

The report builds small source-backed game-data and workspace indexes, runs the same completion paths used by `textDocument/completion`, and writes `tools/reports/lsp-completion-fixtures.report.md`.

The report includes completion context, receiver text, inferred owner type, typed prefix, candidate count, failure reason, bounded sample labels, first inserted text, and first sort key for:

- collection member completion
- call-result receiver completion
- callable insertion snippets with required/optional parameter counts
- overload label details
- attribute and RPC constructor-call completion
- bare attribute shorthand completion such as `attribut` to `[Attribute($0)]`
- named-argument label completion inside attribute, function, and method call argument lists
- inherited override method skeleton completion
- type-position prefix completion
- top-level value/callable prefix completion
- committed game-derived fixture completion samples
- workspace-over-game-data member completion
- workspace delete behavior
- unresolved receivers
- non-member cursor positions

## Commands

```powershell
node tools/lsp-completion-report.mjs
```

## Boundaries

This report covers source-backed member, type-prefix, top-level-prefix, and inherited override skeleton completion, including callable snippet insert text, required/optional parameter presentation, callable label details, and source-aware sort text. It does not cover `completionItem/resolve`, full overload UI, fuzzy matching, diagnostics, Workbench validation, or TypeScript-side language analysis.
