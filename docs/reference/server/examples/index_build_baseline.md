# server/examples/index_build_baseline.rs

## Purpose

Generates a compact dev-only performance baseline for building the in-memory symbol index without expensive corpus-analysis Markdown rendering.

## Architecture Role

This example sits above `server/src/index_build.rs` and uses the shared source-root indexing pipeline. It is review tooling for deciding whether runtime/LSP startup will need a persisted game-data index cache. It is not VS Code runtime behavior, not LSP wiring, not a cache implementation, and not Workbench validation.

## Current Behavior

The Rust example accepts `--scripts <path>`, optional `--workspace <path>`, `--out <path>`, and `--profile-label <label>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder. It builds the index, reports source counts, indexed file/symbol counts, fine-grained build timings, throughput, and a lower-bound index-shape memory estimate.

The Node wrapper `tools/index-build-baseline.mjs` runs the example once in debug mode and once in release mode, then combines both sections into `tools/reports/index-build-baseline.report.md`. Cache recommendation text is based on release build time only.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate index builder and index modules. It must not duplicate scan/read/parse/catalog/index behavior, run Workbench, persist a cache, register a VS Code command, or become package runtime code.

The memory estimate is intentionally conservative. It uses public index counts, copied text lengths, and Rust record sizes as a lower bound; it is not process RSS and excludes allocator overhead and private map/vector capacity.

## Change Notes

- Added the runtime index build baseline report so game-data indexing performance can be measured separately from corpus report rendering.
- Added debug and release comparison through the dev-only Node wrapper.
- Added a release-time threshold for recommending future disposable global-storage game-data index caching.

## Future Improvements

- Add persisted cache design only if release baseline and future LSP startup measurements justify it.
- Add platform-specific RSS measurement only if lower-bound index-shape estimates are not enough for startup decisions.
