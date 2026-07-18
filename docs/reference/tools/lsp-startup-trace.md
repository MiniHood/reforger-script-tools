# tools/lsp-startup-trace.mjs

## Purpose

Creates a single human-reviewable startup trace for the Rust LSP server by merging the TypeScript startup log, Rust runtime log, and VS Code language-client output.

## Ownership

This is repo-only diagnostic tooling. It does not start the extension, change runtime behavior, parse source, or register a VS Code command. It exists to make startup stalls and crashes easier to reason about from one report.

## Current Behavior

The script reads logs from the extension global storage folder and the latest VS Code output log that appears to belong to Reforger Script Tools. It writes `tools/reports/lsp-startup-trace.report.md` by default.

The report includes the latest TypeScript client session, latest Rust server startup session, latest VS Code output lines, and a small interpretation section that flags common startup states such as initialize missing, semantic-token response missing, cache map rebuild without ready, memory allocation pressure, and development binary restarts.

## Dependencies and Boundaries

Uses only Node built-in modules. It must remain dev-only and must not become runtime extension code. It reads logs and writes a report; it must not mutate global storage, cache files, game data, workspace files, or packaged extension files.

The report is diagnostic evidence only. It does not replace targeted Rust logs, hover debug output, semantic-token reports, cache baselines, or manual Extension Development Host validation.

## Verification

Run the script against representative global-storage and VS Code logs; confirm it produces a trace without mutating any input log, cache, or workspace file.
