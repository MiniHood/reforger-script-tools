# MCP Runtime

This guide explains how one Reforger Script Tools MCP server process starts and
answers requests. It distinguishes Enfusion parsing, the semantic index, the
parser-owned on-disk cache, and the separate live Workbench route.

## The short version

```text
Game Data script files
  -> language-engine parser and semantic indexer
  -> published Game Data index cache on disk

MCP process starts
  -> loads that published cache only
  -> holds the loaded immutable index in its own memory
  -> answers MCP searches from that in-memory index
```

The MCP runtime does not create a second Enfusion parser, semantic model, or
cache-maintenance path. The language engine owns building and refreshing the
index; MCP is its consumer. MCP and the language server are separate processes,
so they cannot share the language server's live memory, but they use the same
cache format.

## The three things that are easy to confuse

| Term | What it is | Owner |
| --- | --- | --- |
| Parsing and semantic indexing | Reading Enfusion source into declarations, expressions, scopes, diagnostics, and searchable language facts. | Language engine / language server. |
| Index cache | A serialized snapshot of those semantic facts on disk. It avoids reparsing all Game Data for every consumer process. | Language engine publishes it. |
| MCP in-memory catalogue | One immutable decoded cache snapshot used for searches, inspection, relationships, and source reads during one MCP process lifetime. | MCP process. |

The cache is parsed-and-indexed output, not a second source of language truth.
When the parser/indexer rebuilds the Game Data index, it replaces the published
snapshot. MCP must then be restarted to consume the new snapshot.

## MCP startup contract

An MCP client launches `reforger_language_server.exe mcp` with only an
`--index-cache` path. The extension's copied configuration intentionally does
not pass Game Data source paths or acquisition metadata to MCP.

On the first Game Data operation, MCP:

1. Checks that the cache file exists and is a compatible, internally valid
   language-engine cache.
2. Decodes the semantic index into its own process memory.
3. Loads parser-published source-line maps with that snapshot so semantic
   results retain correct source locations.
4. Uses the immutable snapshot for later Game Data operations.

MCP does not walk the Game Data tree, open or decode Game Data files, calculate
a source fingerprint, compare a source digest, parse source, choose whether an
index is stale, rebuild an index, or write the cache. If the cache is missing,
incompatible, or malformed,
`game_data_status` returns `available: false` and directs the caller to activate
the language server so the parser/indexer can publish a fresh cache. Restart
MCP after that happens.

Previously copied configurations may still include `--game-data-scripts` and
`--game-data-metadata`. They remain accepted so those configurations do not
break, but MCP ignores them; they are not part of its runtime contract.

## How a Game Data search works

```text
MCP client: search_game_data_symbols
  -> validate request and acquire bounded request admission
  -> use the already-loaded immutable semantic index
  -> filter and rank matching symbols
  -> project a bounded page with opaque follow-up references
  -> serialize the MCP result
```

`search_game_data_symbols` is semantic search, not a fresh file-system grep.
Its follow-up tools reuse an opaque, revision-bound reference so inspection,
member listing, relationship queries, and source reads remain tied to the same
catalogue generation.

Broad symbol and example searches may still scan the loaded index to find and
rank candidates. That is a search-performance concern after startup; it does
not imply parsing or cache rebuild work.

## The LSP relationship

```text
VS Code -> LSP process -> parses/indexes Game Data -> published cache
MCP client -> MCP process -> reads published cache -> private in-memory index
```

The LSP and MCP share Rust analysis code and the on-disk cache format, not a
live heap. This keeps the MCP client independent from VS Code while preserving
one authority for source-to-index work. A separate long-lived service or IPC
protocol would be needed to share an in-memory index; this project deliberately
does not introduce that additional runtime owner.

## Official Wiki and Workbench are different paths

Official Wiki search uses a separately validated packaged Markdown corpus. It
does not need the Enfusion semantic index.

Live Workbench tools use a different route:

```text
MCP live tool
  -> typed Rust Workbench Gateway
  -> loopback Workbench NET API
  -> running Workbench handler
```

Their latency includes Workbench dispatch and editor work. Improving Game Data
cache loading or symbol-search performance does not make Workbench compile,
reload, or world operations faster.

## What to measure before optimizing

Keep these timings distinct:

1. Process launch to MCP initialization response.
2. First `game_data_status`: cache read, decode, compatibility validation, and
   in-memory map reconstruction.
3. Warm Game Data search latency: exact-name, prefix, broad, filtered, and
   paginated queries, reported as p50 and p95.
4. Official Wiki search latency separately from Game Data search.
5. Workbench queue wait and NET API execution separately from local MCP work.

Start with these measurements before adding a new lookup structure. Source
freshness, source-line maps, and cache publication belong to the parser/indexer,
not the MCP process. Raw source evidence must likewise be published by the
parser before MCP can offer a source-evidence operation; MCP does not fall back
to source-file I/O.
