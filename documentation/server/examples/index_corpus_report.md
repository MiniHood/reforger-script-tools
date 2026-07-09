# server/examples/index_corpus_report.rs

## Purpose

Generates a corpus-scale Markdown report from the first in-memory symbol index.

## Architecture Role

This is developer review tooling for index lookup quality. It is not VS Code runtime behavior, not an LSP entrypoint, not semantic resolution, not Workbench validation, and not compiler truth.

## Current Behavior

The example scans `.c` files under a scripts folder, parses each file, builds AST/model catalogs with game-data metadata, aggregates them into `SymbolIndex`, and writes `tools/reports/index-corpus.report.md` by default. It reports indexed file/symbol totals, map counts, source-kind counts, symbol-kind frequency, bounded duplicate top-level name groups, top-level-only preferred duplicate samples, lookup samples for classes and typedefs, and grouped method owner/name samples with overload counts.

It accepts `--scripts <path>` and `--out <path>`. If no scripts path is provided, it uses the downloaded game-data global-storage scripts folder.

## Dependencies and Boundaries

Uses only Rust standard library APIs plus the crate parser, AST, model, and index modules. It must not duplicate index behavior, resolve symbols, call Workbench, become VS Code runtime code, or become a package command. The example leaks source strings for process-lifetime report generation only; production language-server ownership should use a real source storage layer.

## Change Notes

- Added corpus-scale index reporting for real downloaded/manual Reforger script data.
- Preferred duplicate samples use top-level-only preferred lookup so member or parameter symbols cannot affect declaration conflict review.
- Method owner/name samples render grouped owner-qualified method rows with overload counts, first path, and unique return types.

## Future Improvements

- Add workspace-vs-game-data override sections after the index can ingest workspace catalogs.
- Add timing and memory estimates after a real language-server startup path exists.
