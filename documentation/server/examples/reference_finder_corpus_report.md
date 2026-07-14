# server/examples/reference_finder_corpus_report.rs

## Purpose

Generates a dev-only corpus report for file-local reference finding across downloaded game-data scripts.

## Architecture Role

This example sits above `server/src/reference_finder.rs` and exercises the same resolver-confirmed reference path that future references and rename should use. It proves reference grouping, scope behavior, shadowing behavior, and unresolved identifier classification without adding LSP behavior.

## Current Behavior

The report defaults to the downloaded game-data scripts folder under VS Code global storage and writes `tools/reports/reference-finder-corpus.report.md`. It supports:

- `--scripts <path>`
- `--out <path>`
- `--max-files <n>`
- `--no-external-index`

The report builds one optional game-data external index by default so normal cross-file references are counted as external selections instead of unresolved misses. Each sampled file is parsed, indexed, scoped, and scanned once. The report groups references by exact file-local symbol id, counts reference coverage by symbol kind, lists target samples, and classifies unresolved identifiers into source-noise and actionable buckets.

## Dependencies and Boundaries

Uses the lexer, parser, AST, model, index, scope, resolver, and reference finder. It must remain dev-only. It does not perform workspace-wide reference search, rename edits, text-only matching, Workbench validation, or LSP request handling.

## Change Notes

Added after the fixture reference report so reference/rename readiness can be reviewed against real game-data source shapes.

## Future Improvements

Add workspace-overlay reference corpus coverage once workspace-wide search exists. Add richer rename-safety metadata after file-local reference quality is proven.
