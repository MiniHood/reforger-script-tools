# server/src/bin/reforger_language_server.rs

## Purpose

Provides the bundled Rust language-server executable entrypoint.

## Ownership

The binary owns only command-line argument handling and stdio process startup. `server/src/lsp.rs` owns protocol runtime behavior; lower language-engine modules own language features and caching.

## Current Behavior

It accepts `--log`, `--game-data-scripts`, `--game-data-metadata`, and `--index-cache` paths, starts the stdio LSP loop, and returns a nonzero status when the server returns an error.

## Dependencies and Boundaries

This entrypoint stays thin. It does not own parser, index, Workbench, VS Code, cache, or feature logic.

## Verification

Server integration tests exercise the LSP runtime; packaging/startup checks verify the binary receives its extension-owned paths.

## Future Direction

Add explicit diagnostic or version flags only for a concrete packaging or debugging requirement.
