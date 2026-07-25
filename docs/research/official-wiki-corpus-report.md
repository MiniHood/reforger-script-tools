# Official Wiki Corpus Report

## Scope and method

This report records a direct MCP evaluation of the packaged Official Wiki
Corpus.  It is evidence about the shipped copy of Reforger documentation, not
about the current public wiki, Workbench, or compiler behaviour.

The checks used the bundled server with the repository corpus explicitly
selected:

```powershell
server/target/release/reforger_language_server.exe mcp `
  --official-wiki-root resources/official-wiki
```

The MCP client initialized the server, then called `official_wiki_status`,
`search_official_wiki`, and `read_official_wiki`.  The status revision below
binds every reported search and read result.

## Validated corpus

`official_wiki_status` reported a healthy, usable corpus:

| Fact | Observed value |
| --- | --- |
| Source | `evidence-catalogue` |
| Revision | `ow1:d40df5e4830cab07dbc5a9c1c06beadd521c4214315d43176954e54c27f682e7` |
| Valid pages | 310 |
| Valid bytes | 3,308,362 |
| Invalid pages | 0 |
| Excluded files | `wiki-index.md` |
| Cold-search target | 5,000 ms |
| Maximum accepted page size | 4,194,304 bytes |

The corpus contains 48 category `index.md` pages.  Its top-level content is
principally `Modding/` (272 pages), with `Content/` (16), `Support/` (10), and
12 root-level documentation pages.  This is strong coverage for modding and
Workbench workflows, but it is not a complete game-data or API reference.

The logical paths are copied Markdown whose headings retain a canonical source
URL.  For example, the multiplayer guide is
[`Modding/Scripting/Tutorials/Multiplayer Scripting.md`](../../resources/official-wiki/Modding/Scripting/Tutorials/Multiplayer%20Scripting.md),
whose canonical source is
[Arma Reforger: Multiplayer Scripting](https://community.bistudio.com/wiki/Arma_Reforger:Multiplayer_Scripting).

## What an AI can retrieve

The interface returned deterministic, section-local results with citation and
read handoffs rather than only titles or unbounded pages.

| Query | Result | Why it is useful |
| --- | --- | --- |
| `script profiling` | 6 matching sections; the first is the exact-title page [`Modding/Script Profiling.md`](../../resources/official-wiki/Modding/Script%20Profiling.md) at lines 1-4 | Finds a focused operational guide and separates its profiler and allocation sections. |
| `Game Master` with `pathPrefix: "Modding/"` | 134 matching sections; the leading result is the Game Master composition tutorial | Constrains a broad topic to the modding subtree and returns actionable prefab/configuration passages. |
| `replication` | 31 matching sections; first hit is **Replication**, lines 53-62 of the multiplayer guide | Finds networking concepts at the relevant heading, plus the exact source range. |

For the last query, the returned `readInput` was copied unchanged into
`read_official_wiki`:

```json
{
  "corpusRevision": "ow1:d40df5e4830cab07dbc5a9c1c06beadd521c4214315d43176954e54c27f682e7",
  "relativePath": "Modding/Scripting/Tutorials/Multiplayer Scripting.md",
  "startLine": 53,
  "lineCount": 10
}
```

The read returned lines 53-62, the canonical Multiplayer Scripting URL, and a
copy-ready continuation beginning at line 63.  That is the desired AI loop:
search for a passage, preserve the revision-bound handoff, read a bounded
primary-source excerpt, and continue only when needed.

## Assessment

The corpus is useful for an AI working on Reforger because it supplies
officially sourced, local, reproducible guidance for workflows that symbol
data cannot explain: Game Master composition setup, profiling, replication
concepts, Workbench tools, assets, terrain, and tutorials.  Ranking appears
especially helpful for precise topic queries: `script profiling` produced its
page and its relevant sections before incidental category/index references.

It should complement, not replace, Game Data tools.  The corpus is a copied
documentation snapshot; its own MCP instructions correctly state that it does
not prove live Workbench or compiler behaviour.  It also cannot answer an
exact Enfusion declaration, inheritance question, caller query, or source
usage question as reliably as the semantic Game Data and source-read tools.

Search is lexical and deliberately strict: every normalized query term must
occur in one heading section plus the title/path context.  Consequently, an
AI should try the documentation's terminology, use `pathPrefix` to narrow
broad concepts, and not treat an empty result as proof that a topic is absent.
The 272/310-page Modding concentration also means player-facing and support
coverage is relatively smaller.  Finally, the retrieved Markdown preserves
some source encoding artefacts, so an AI should cite the returned canonical
URL and inspect the bounded read rather than relying on an excerpt alone.

## Conclusion

The packaged corpus is ready for normal AI use as a bounded documentation
authority.  Its status, revision, section search, canonical URL, and
continuation handoff form a complete evidence workflow.  The most valuable
future enhancement is content freshness/coverage acquisition, not a redesign
of the MCP search contract.
