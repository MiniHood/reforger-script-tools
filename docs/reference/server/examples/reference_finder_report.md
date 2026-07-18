# server/examples/reference_finder_report.rs

## Purpose

Generates a dev-only fixture report for the file-local reference finder foundation.

## Ownership

This example sits above `server/src/reference_finder.rs` and proves that reference search uses resolver-selected symbols instead of text-only matching. It is groundwork for future references and rename.

## Current Behavior

The report writes `tools/reports/reference-finder-fixtures.report.md` by default and supports `--out <path>`. It uses an inline game-data-shaped source fixture with a typedef, global field, enum member, class field, method, parameter, and local variable. For each target it reports declaration references, usage references, resolver reason, candidate count, and source line.

## Dependencies and Boundaries

Uses only existing Rust lexer, parser, AST, model, index, scope, resolver, and reference finder APIs. It is dev-only and must not add LSP behavior, workspace-wide search, rename edits, Workbench validation, or a second reference matching implementation.

## Verification

Run `cargo run --example reference_finder_report` from `server/` and inspect the generated report for the documented fixture or corpus checks.
