# MCP Runtime

This guide explains how one Reforger Script Tools MCP server process starts and
answers requests. It distinguishes Enfusion parsing, the semantic index, its
on-disk cache, and the separate live Workbench route.

## The short version

```text
Game Data script files
  -> parser and semantic index builder
  -> saved Game Data index cache on disk

MCP process starts
  -> proves which Game Data source it was given
  -> reuses the saved cache when it matches that source
  -> holds the loaded immutable index in its own memory
  -> answers MCP searches from that in-memory index
```

The MCP runtime does not create a second Enfusion parser or semantic model. It
uses the Rust language-engine modules and the same Game Data cache format as
the language-server runtime. It is nevertheless a different process, so it
cannot share the language server's live memory.

## The four things that are easy to confuse

| Term | What it is | When it happens |
| --- | --- | --- |
| Parsing | Reading Enfusion source into language structure: declarations, expressions, scopes, and diagnostics. | When an index must be built or rebuilt. |
| Semantic index | The searchable language facts built from parsing: symbols, kinds, owners, relationships, source positions, and source metadata. | Built from Game Data, then kept immutable for one runtime generation. |
| Index cache | A serialized copy of that semantic index on disk. It avoids reparsing all Game Data for every new process. | Loaded at startup when its source identity matches. |
| Fingerprint | A compact identity derived from the configured source tree. It answers “is this cache still for these exact inputs?” | Before a cache is trusted. |

The cache contains parser/index output. It is not an alternative to parsing;
it is reusable parsed-and-indexed output from an earlier run.

## Why the fingerprint exists

Using an old cache after a Reforger update, a manual-folder change, or a partial
extraction update would make MCP searches return stale API facts. The runtime
therefore derives a fingerprint from the configured Game Data source before it
loads the cache.

```text
configured script tree
  -> collect stable source identity facts
  -> fingerprint
  -> matches cached fingerprint?
       yes: decode the cached semantic index
       no: parse and index the current source, then write a replacement cache
```

A matching fingerprint does not mean “the files look roughly similar.” It is
the prerequisite for treating the cached semantic facts as authoritative for
that source generation.

The current manual-folder fingerprinting work can be expensive because it must
walk a large tree. That cost is separate from parsing and from a later
in-memory search. `game_data_status` exposes its bounded timing breakdown so an
operator can tell fingerprinting, cache decoding, rebuilding, and cache writing
apart.

## Process startup, step by step

1. An MCP client launches `reforger_language_server.exe mcp` with stable Game
   Data and cache paths. The client owns this stdio process; VS Code is not
   required to remain open.
2. The runtime creates its owners: the Game Data catalogue, Official Wiki
   corpus, and typed Workbench gateway. This is cheap construction, not an
   eager full parse.
3. The first Game Data operation initializes the catalogue. It validates the
   configured source, derives its fingerprint, and attempts to load the
   on-disk cache.
4. If the cached fingerprint matches, the runtime decodes the cached semantic
   index. If it does not, it parses the current Game Data, builds the semantic
   index, and replaces the cache after successful construction.
5. The runtime retains that immutable index in MCP-process memory. Later Game
   Data search, inspection, member, relationship, and source-read tools reuse
   it; they do not parse the whole corpus again.
6. When the MCP process exits, its in-memory index disappears. The on-disk
   cache remains for the next process, subject to the next fingerprint check.

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

Current broad symbol and example searches may still scan the loaded index to
find and rank candidates. That is a search-performance concern, but it occurs
after startup and does not imply reparsing. It should be measured separately
from cold and warm cache startup before adding a new lookup structure.

## The LSP relationship

The VS Code language server and MCP runtime intentionally run as separate
processes:

```text
VS Code -> LSP process -> its in-memory index
MCP client -> MCP process -> its in-memory index
                         \-> shared on-disk cache format
```

They share Rust analysis code and the cache format, not a live heap. This keeps
an MCP client independent from VS Code and prevents one protocol's lifecycle
from invalidating the other's requests. Sharing one live in-memory index would
require an additional long-lived service or inter-process protocol, which would
replace this simple independent-process boundary with another runtime owner.

## Official Wiki and Workbench are different paths

Official Wiki search uses a separately validated packaged Markdown corpus. It
does not need the Enfusion semantic index.

Live Workbench tools are different again:

```text
MCP live tool
  -> typed Rust Workbench Gateway
  -> loopback Workbench NET API
  -> running Workbench handler
```

Their latency includes Workbench dispatch and editor work. Improving Game Data
cache or symbol-search performance does not make Workbench compile, reload, or
world operations faster.

## What to measure before optimizing

Keep these timings distinct:

1. Process launch to MCP initialization response.
2. First `game_data_status`: fingerprint, cache read/decode, index rebuild, and
   cache write timings.
3. Warm Game Data search latency: exact-name, prefix, broad, filtered, and
   paginated queries, reported as p50 and p95.
4. Official Wiki search latency separately from Game Data search.
5. Workbench queue wait and NET API execution separately from local MCP work.

The first likely optimization target is expensive source fingerprinting during
manual-folder startup. A search index should only be added after measurements
show that in-memory candidate filtering and ranking are a meaningful portion
of user-visible latency.
