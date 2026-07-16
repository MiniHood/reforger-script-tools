# tools/lsp-startup-trace.mjs

## Purpose

Creates a single human-reviewable startup trace for the Rust LSP server by merging the TypeScript startup log, Rust runtime log, and VS Code language-client output.

## Architecture Role

This is repo-only diagnostic tooling. It does not start the extension, change runtime behavior, parse source, or register a VS Code command. It exists to make startup stalls and crashes easier to reason about from one report.

## Current Behavior

The script reads logs from the extension global storage folder and the latest VS Code output log that appears to belong to Reforger Script Tools. It writes `tools/reports/lsp-startup-trace.report.md` by default.

The report includes the latest TypeScript client session, latest Rust server startup session, latest VS Code output lines, and a small interpretation section that flags common startup states such as initialize missing, semantic-token response missing, cache map rebuild without ready, memory allocation pressure, and development binary restarts.

## Dependencies and Boundaries

Uses only Node built-in modules. It must remain dev-only and must not become runtime extension code. It reads logs and writes a report; it must not mutate global storage, cache files, game data, workspace files, or packaged extension files.

The report is diagnostic evidence only. It does not replace targeted Rust logs, hover debug output, semantic-token reports, cache baselines, or manual Extension Development Host validation.

## Change Notes

- Added after startup issues became hard to diagnose from separate TypeScript, Rust, and VS Code output logs.

## Future Improvements

- Add optional process memory sampling around a launched Extension Development Host if we need to catch memory growth live.
- Add structured Rust session IDs if the server log starts emitting them directly.
