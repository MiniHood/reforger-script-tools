# server/examples/index_cache_strings_report.rs

## Purpose

Measures repeated copied string values in the runtime game-data cache.

## Architecture Role

This is dev-only review tooling. It loads the current disposable game-data cache through `server/src/index_cache.rs`, inspects public `SymbolIndex` fields, and writes `tools/reports/index-cache-strings.report.md`.

## Current Behavior

The report counts string occurrences, unique strings, occurrence bytes, unique bytes, and duplicated string bytes. It groups strings by source, including file absolute paths, root paths, relative paths, symbol names, detail text, modifiers, attribute names/raw text, doc comments, and conditional expressions.

The path review section exists because v3 stores absolute path, root path, and relative path per file. That is useful for debug output, but repeated path prefixes are a likely future path-table optimization if cache size or memory pressure becomes a real problem.

## Boundaries

This report does not change cache format, runtime language-server behavior, lookup semantics, source-category policy, or LSP behavior. It is planning evidence for possible string interning, path tables, or binary cache work.

## Usage

Run:

```powershell
node tools/index-cache-strings-report.mjs
```

Optional inputs:

```powershell
node tools/index-cache-strings-report.mjs --scripts <path> --metadata <path|none> --cache <path> --out <path>
```

## Change Notes

- Added after v3 cache optimization to determine whether string interning or path-table storage would provide meaningful savings without removing editor-visible facts.

## Future Improvements

- Use this report before designing an interned-string cache format.
- Add real process memory measurements separately if runtime RSS becomes the concern.
