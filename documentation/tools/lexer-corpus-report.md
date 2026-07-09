# tools/lexer-corpus-report.mjs

## Purpose

Provides the repo-level Node entrypoint for generating a lexer corpus report from downloaded or manually supplied Reforger script data.

## Architecture Role

This is dev-only tooling under `tools/`. It keeps corpus analysis out of `src/` and out of the packaged extension while still making the workflow easy to run from the repo root.

## Current Behavior

Running `node tools/lexer-corpus-report.mjs` delegates to the Rust `lexer_corpus_report` example and writes `tools/reports/lexer-corpus.report.md` by default. It forwards `--scripts <path>` and `--out <path>` to the Rust implementation.

## Dependencies and Boundaries

The wrapper uses Node standard library APIs only. It requires local development tooling because it is not runtime extension code. It must not implement its own lexer or become a package command unless intentionally promoted as developer workflow.

## Change Notes

- Added dev-only lexer corpus report wrapper.

## Future Improvements

- Keep this wrapper thin; expand report behavior in the Rust example so it always uses the real lexer.
