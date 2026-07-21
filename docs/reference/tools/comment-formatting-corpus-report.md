# tools/comment-formatting-corpus-report.mjs

## Purpose

Generates a development-only, discovery-level inventory of comment, Doxygen,
indentation, brace, declaration, and source-provenance shapes in a Reforger
script corpus.

## Ownership

This tool belongs under `tools/`, not the extension runtime. It does not parse
or validate Enfusion Script and must not be imported by `src/` or registered as
an extension command.

## Current Behavior

The script scans `.c` files from the downloaded global-storage corpus by
default, or from `--scripts <path>`, and writes
`tools/reports/comment-formatting-corpus.report.md` by default. It reports
text-pattern counts and bounded examples for comment forms, selected Doxygen
tags, indentation/brace signals, attribute and callable shapes, control headers
whose following nonblank line has no opening brace, possible missing
semicolons, and heuristic generated/proto/native/Workbench categories.

Its output is explicitly discovery-only. A result is not compiler truth,
documentation attachment, formatter eligibility, or authorization to mutate a
file.

## Dependencies and Boundaries

Uses only Node built-ins and reads no extension runtime state beyond the
default downloaded game-data location. It must preserve the distinction between
corpus frequency and Workbench/compiler validation.

## Verification

Run `node tools/comment-formatting-corpus-report.mjs --scripts <path>` and
inspect the ignored Markdown report. Confirm its file count, source path,
bounded examples, and explicit interpretation limits.
