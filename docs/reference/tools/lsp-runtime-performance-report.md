# tools/lsp-runtime-performance-report.mjs

## Purpose

Builds a dev-only Markdown report from the Rust language-server runtime log so high CPU or typing-latency sessions can be reviewed after the fact.

## Ownership

This tool is outside the packaged runtime path. It reads `globalStorageUri/logs/language-server.log`, groups multi-line Rust log records, parses key timing fields, and writes `tools/reports/lsp-runtime-performance.report.md`.

## Current Behavior

The report summarizes logged elapsed work by operation, file/URI, one-second time window, completion latency, edit-analysis latency, and semantic-token worker health. It specifically separates:

- foreground request/notification time
- queue wait time for live stdio requests when available
- background rich semantic-token time
- stale/skipped rich semantic-token work
- cancelled rich semantic-token work
- completion candidate lookup time
- `didChange` parse/catalog/index/scope timing
- lazy document-symbol projection spikes

It accepts:

```powershell
node tools/lsp-runtime-performance-report.mjs
node tools/lsp-runtime-performance-report.mjs --since-minutes 10
node tools/lsp-runtime-performance-report.mjs --log <path> --out <path>
```

The report does not sample OS CPU directly. It uses logged elapsed timings as the first debugging pass for identifying which LSP subsystem likely caused CPU use or visible delay. When logs include `queue_ms`, completion sections separate execution time from perceived latency caused by waiting behind earlier LSP work.

## Dependencies and Boundaries

Uses only Node built-ins. It must not register a package script or VS Code command unless runtime performance reporting becomes a user-facing feature. It must not add logging overhead to the LSP server; it consumes the existing log.

## Verification

Run the script against a captured language-server log and inspect that malformed records are reported without changing the log or extension runtime.
