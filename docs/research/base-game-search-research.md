# Base-game source search research

Research date: 2026-07-24. This note answers one design question: where search
over extracted Reforger base-game source belongs, how it should appear in VS
Code, and how an MCP server should reuse it. It is a design recommendation, not
an implementation commitment.

## Recommendation

Build search once in the Rust language/index engine, then expose it through two
thin adapters:

```text
                         immutable Rust index/query service
                       / semantic symbols | source text \
                      /                               \
        LSP + VS Code presentation                 MCP adapter
        Ctrl+T / dedicated picker          typed search tools + resources
```

The MCP server should expose base-game search, but it should not own the search
index, parse Enfusion, rank symbols independently, or read the storage tree as
a competing normal path. That conclusion matches the existing repository
boundary: `server/` is the language authority, and its external-index snapshot
already carries distinct immutable `workspace` and `game_data` layers
(`server/src/lsp/external_overlay.rs`). The current server advertises document
symbols but not `workspaceSymbolProvider` (`server/src/lsp/mod.rs`), while
`SymbolIndex` already owns symbols, source metadata, by-name maps, and a folded
top-level prefix map (`server/src/index.rs`).

Treat semantic symbol search and arbitrary source-text search as related but
different products. Implement semantic search first because "find the base
class `SCR_Foo`" is an indexed language query. Add full-text search for
questions such as "where does this literal, comment, or call shape occur" as a
separate deterministic source query.

## The editor experiences are not the same search

| User intent | Native VS Code analogue | Recommended Reforger surface |
| --- | --- | --- |
| Find text in the current open file | `Ctrl+F` searches the current editor. | Keep native behavior. |
| Find arbitrary text across ordinary project files | `Ctrl+Shift+F` searches files in the opened folder. | Keep native search for addon files; provide **Reforger: Search Base Game Source** for extracted game data outside the workspace. |
| Find a class, enum, function, or other declaration by name | `Ctrl+T` opens a cross-file symbol picker for languages that provide it. | Implement LSP `workspace/symbol`; optionally include base-game results at lower priority, and provide a dedicated base-game-only picker when the user wants that scope explicitly. |

VS Code documents those three behaviors separately: `Ctrl+F` is current-file
text search, `Ctrl+Shift+F` searches the currently opened folder, and `Ctrl+T`
is language-backed cross-file symbol navigation
([basic editing](https://code.visualstudio.com/docs/editing/codebasics#_find-and-replace),
[search across files](https://code.visualstudio.com/docs/editing/codebasics#_search-across-files),
[open symbol by name](https://code.visualstudio.com/docs/editing/editingevolved#_open-symbol-by-name)).
Therefore pointing users at the extension's `globalStorageUri` would be a
storage leak, not a good search experience.

For navigation, expose game-data files under stable logical identities and open
them as read-only documents. VS Code's `TextDocumentContentProvider` is
specifically designed to create read-only documents from arbitrary sources
under a custom URI scheme
([VS Code virtual documents](https://code.visualstudio.com/api/extension-guides/virtual-documents)).
Use that only as a presentation bridge: the TypeScript client maps a logical
game-data URI to content supplied by the existing source owner. It must not
become a TypeScript parser or index. Do not mount the whole storage directory as
a virtual workspace merely to gain search; VS Code still describes LSP
filesystem-provider support for virtual workspaces as work in progress
([VS Code virtual workspaces](https://code.visualstudio.com/api/extension-guides/virtual-workspaces#_what-about-support-in-the-language-server-protocol-lsp-for-accessing-virtual-resources)).

## What established language toolchains do

### LSP defines the symbol-navigation surface

LSP 3.18 defines `workspace/symbol` as the request for project-wide symbols
matching a query. It asks servers to use relaxed matching rather than strict
prefix or substring matching, permits an empty query, supports partial-result
streaming, and since 3.17 permits lazy `workspaceSymbol/resolve` so the initial
result can omit a range when the client supports it. The standard request has
no dependency, standard-library, or "base only" scope field
([LSP workspace symbols](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#workspace_symbol)).

That makes `workspace/symbol` the right interoperable surface for ordinary
`Ctrl+T` navigation, but a dedicated base-game-only picker needs either a
server configuration/convention or a small custom request. A custom
`reforger/searchSymbols` request should still call the same core Rust query; it
is a scope-rich projection, not another search implementation.

### rust-analyzer: dependencies and stdlib are source domains in one analyzer

rust-analyzer's workspace-symbol feature fuzzy-searches project dependencies,
including the standard library. Its documented query modifiers distinguish
workspace types, all symbols, and dependency/stdlib scope; configuration also
provides scope, kind, and a default result limit of 128
([rust-analyzer workspace symbol](https://rust-analyzer.github.io/book/features.html#workspace-symbol),
[rust-analyzer configuration](https://rust-analyzer.github.io/book/configuration#rust-analyzer.workspace.symbol.search.limit)).
Its Rust-specific LSP extension makes the scope distinction explicit as
`Workspace` versus `WorkspaceAndDependencies`
([rust-analyzer LSP extensions](https://rust-analyzer.github.io/book/contributing/lsp-extensions.html#workspace-symbols-filtering)).

The precedent is not the punctuation used in rust-analyzer's UI. The useful
precedent is that standard-library/dependency symbols stay in the authoritative
language engine, while scope and result caps are query inputs.

### clangd: one index interface, several source/cache layers

clangd's index stores symbols, references, relations, declaration locations,
documentation, and completion attributes; symbols can be looked up by ID or
fuzzy-searched by name. `MergedIndex` presents open-file, background, static,
and optional remote indexes to features as one combined index. Background
index data is cached on disk, including a separate cache location for headers
without a compilation database such as the standard library
([clangd index design](https://clangd.llvm.org/design/indexing)).
clangd also enables eager standard-library indexing by default so those symbols
can participate in completion even in an empty file
([clangd `Index.StandardLibrary`](https://clangd.llvm.org/config#standardlibrary)).

This is very close to Reforger's desired shape: base-game acquisition and cache
policy may differ from workspace policy, but consuming features query one
language-owned abstraction.

### Eclipse JDT LS: workspace symbol can include libraries and the JDK

Eclipse JDT LS implements workspace-symbol search through JDT's `SearchEngine`.
Its scope always includes workspace sources and referenced projects; when the
client can display class-file content, it also includes application and system
libraries. It rejects blank queries, accepts a maximum result count, and uses
the language model to map matches to source/class-file locations
([JDT LS `WorkspaceSymbolHandler`](https://github.com/eclipse-jdtls/eclipse.jdt.ls/blob/main/org.eclipse.jdt.ls.core/src/org/eclipse/jdt/ls/core/internal/handlers/WorkspaceSymbolHandler.java#L56-L147)).
The server also advertises type search and automatic source resolution for
classes in dependency jars
([JDT LS features](https://github.com/eclipse-jdtls/eclipse.jdt.ls#features)).

This establishes that including a language's base libraries in
`workspace/symbol` is a legitimate server choice. It also shows why navigation
and readable source identities matter: a search hit is not useful if the
client cannot open it.

### TypeScript: the editor chooses library noise policy, the server still searches

TypeScript's language service owns "navigate to" symbol search over program
source files, supports a maximum result count, and can exclude declaration
files, external libraries, and the default library. tsserver coordinates the
query across projects and deduplicates results
([TypeScript `navigateTo`](https://github.com/microsoft/TypeScript/blob/main/src/services/navigateTo.ts#L42-L74),
[tsserver session](https://github.com/microsoft/TypeScript/blob/main/src/server/session.ts#L2803-L2861)).
VS Code currently defaults its TypeScript
`workspaceSymbols.excludeLibrarySymbols` preference to `true`
([VS Code TypeScript configuration](https://github.com/microsoft/vscode/blob/main/extensions/typescript-language-features/src/configuration/configuration.ts#L347-L349)).

This is the counterweight to JDT LS: library symbols are semantically searchable,
but an editor may hide them by default to control noise. For Reforger, a
dedicated base-game picker is therefore safer than forcing every base-game
symbol into the default workspace picker from day one.

## Proposed Rust query boundary

Add a protocol-independent engine query that both LSP and MCP can call. The
exact Rust types can follow existing conventions, but the stable concepts
should be:

- query text and matching mode;
- source scope: `workspace`, `game-data`, or both;
- symbol-kind filters;
- deterministic limit and generation-bound cursor;
- result identity, name, kind, containing symbol, signature/summary where
  available, logical source path, selection range, source layer, and game-data
  version/fingerprint.

Do not implement this by calling the editor-completion projection. Completion
has context-specific kind/source filtering and insertion concerns. Factor or
add a general indexed-name query over `SymbolIndex`, then let completion and
symbol search project the shared facts according to their own contracts.

For arbitrary text, add a separate bounded lexical query over the immutable
game-data corpus. It may use a purpose-built inverted/trigram index or a bounded
scan based on measured corpus performance, but it remains in the Rust
language/evidence layer. Results need a logical path, exact range, bounded
excerpt, version/fingerprint, total where known, and cursor. Do not mix
embedding-only ranking into the authoritative exact-search path.

## LSP and VS Code presentation

1. Implement `workspace/symbol` for relaxed symbol-name search and
   `workspaceSymbol/resolve` only if deferred location work is measurably useful.
2. Initially default `Ctrl+T` to workspace symbols, or admit exact/high-quality
   base-game matches at lower priority after measuring result noise.
3. Add **Reforger: Go to Base Game Symbol** for explicit `game-data` scope and
   **Reforger: Search Base Game Source** for arbitrary text. Both should query
   Rust incrementally, cancel stale searches, and enforce server-side caps.
4. Open hits through a logical read-only URI such as
   `reforger-game-data:/<version>/<logical-path>`. Never display or require the
   physical `globalStorageUri` path.

## MCP presentation

MCP resources are application-controlled, URI-addressed context, while tools
are model-controlled functions with input schemas. `resources/list` supports
pagination but not a free-form search query; resource templates parameterize
URIs rather than replacing a ranked search API
([MCP resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)).
Therefore use:

| MCP capability | Role |
| --- | --- |
| `search_symbols` tool | Semantic search with `query`, source scope, kinds, limit, and cursor. |
| `search_reference` tool | Exact/full-text search across game source and, later, other evidence kinds. |
| `read_reference` or resource read | Retrieve a bounded passage/document identified by a returned logical URI and version. |
| `reference_catalogue` resource | Report installed corpus version, fingerprint, coverage, and availability; do not enumerate every file as initial model context. |

MCP tools can return typed `structuredContent` validated by an output schema and
may also return resource links that clients can fetch for more context
([MCP tool results](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#tool-result)).
Each result should therefore carry the same engine-produced symbol/source
identity and provenance as the editor result, plus a logical resource link.
The MCP adapter owns MCP schemas, limits, error mapping, and resource URI
validation; it does not own Enfusion matching or ranking.

This refines, rather than contradicts, the existing
[MCP exploration journal](mcp-server-research.md): `search_symbols` is the
semantic engine projection, `search_reference` is bounded corpus text search,
and `reference_catalogue` is stable context. None requires Workbench to be
running.

### One federated reference-search contract

Use one public `search_reference` tool across extracted game source and the
official Reforger wiki/document corpus. Keep `search_symbols` separate because
it is a semantic language query, but do not add parallel
`search_game_source` and `search_wiki` tools for corpora that share the same
read-only research intent and result shape. The input should require `query`
and accept `evidenceKinds`, filters, `limit`, and `cursor`. Use the stable
evidence kinds `game-data` and `official-wiki`; here `official-wiki` means
official Reforger documentation/wiki pages, never Wikidata.org.

This keeps the public contract small while preserving separate internal
retrievers. Split the public tool later only if a source proves to require a
materially different authorization, availability, query language, latency, or
result schema. `search_reference` is a valid durable MCP name: current MCP
guidance says tool names should be unique within a server and use only ASCII
letters, digits, underscores, hyphens, or dots
([MCP tools and names](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#tool-names)).

Publish object-shaped `inputSchema` and `outputSchema` definitions and return
the result in `structuredContent`; when an output schema is declared, the
server must conform to it. Also return the serialized JSON as text for
backwards compatibility, then add `resource_link` content blocks for hits that
can be read through MCP
([MCP structured tool results](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#structured-content),
[MCP resource links](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#resource-links)).

Use one shared result envelope:

| Field group | Stable contract |
| --- | --- |
| Page | `results`, `returned`, optional `total`, `nextCursor`, and `catalogueRevision`. |
| Identity | Stable hit ID, `evidenceKind`, evidence authority, document/symbol kind, logical resource URI, and title or logical path. |
| Match | Bounded excerpt, exact range or document anchor, matched fields, global rank, and within-kind rank. |
| Provenance | For game data: Reforger/game-data version and content hash. For official docs: canonical official URL, source revision or retrieval timestamp, and content hash. |

Do not rely on generic MCP resource annotations as the provenance record:
resources expose useful display hints such as `audience`, `priority`, and
`lastModified`, but version, authority, retrieval identity, and content hash
must remain explicit fields in the shared DTO
([MCP resource annotations](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#annotations)).
The logical resource URI should use a custom server-mediated scheme and retain
the corpus version; MCP permits custom RFC 3986 schemes and recommends `https`
only when the client can fetch the resource directly
([MCP resource URI schemes](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#common-uri-schemes)).

Rank inside each evidence kind first, then fuse deterministically. Exact symbol,
API, path, or documentation-title matches should precede exact body-text
matches, which should precede fuzzy or later semantic matches. Combine
source-local ranks through a documented interleave or rank-fusion rule; never
compare raw scores from unlike retrievers or erase source identity. A source
preference may break ties for a proven intent—game source for declaration facts,
official documentation for documented workflows—but every hit retains its
authority so ranking is never mistaken for evidence strength.

MCP's standard pagination contract applies to list operations rather than
arbitrary `tools/call` results. Define `cursor` and `nextCursor` explicitly in
the search tool schemas while following the same opaque-cursor rules: bind a
cursor to the normalized query, filters, and catalogue revision; let the server
choose page size; reject stale or modified cursors instead of silently
continuing against another corpus generation
([MCP pagination](https://modelcontextprotocol.io/specification/2025-11-25/server/utilities/pagination)).

For an immutable locally installed catalogue, annotate `search_reference` and
`read_reference` with `readOnlyHint: true` and `openWorldHint: false`. Omit
`destructiveHint` and `idempotentHint`, which the schema defines as meaningful
only for non-read-only tools. These annotations are client hints, not
enforcement; the server must still validate URIs and maintain its read-only
boundary
([MCP `ToolAnnotations`](https://modelcontextprotocol.io/specification/2025-11-25/schema#toolannotations)).
If official pages are queried live rather than acquired into the immutable
catalogue, `openWorldHint` must become `true` for the federated tool or the live
operation must be split into a separately named tool.

Deliver the framework without changing its public shape:

1. Ship deterministic lexical/path/symbol search for `game-data` with the
   shared DTO, provenance, cursors, and resource reads.
2. Add licensed, versioned acquisition and indexing for `official-wiki`, then
   admit it through the same `evidenceKinds` filter and result envelope.
3. Enable default cross-kind fusion only after relevance tests; add semantic or
   hybrid ranking later as an optional signal without weakening exact search or
   citation fidelity.

## Suggested delivery order

1. Prove the core Rust symbol query against the existing workspace and
   game-data index layers, including deterministic ordering, kind/scope filters,
   cancellation, limits, and stable logical locations.
2. Expose it through LSP `workspace/symbol` and a dedicated base-game picker;
   verify navigation into read-only logical documents without exposing storage.
3. Add bounded base-game full-text search only after measuring direct search
   over a representative corpus.
4. Expose both core queries through MCP tools and resource links. Verify that
   editor and MCP requests return the same symbol identities, locations, and
   game-data version for equivalent queries.

The key acceptance condition is one authoritative answer. If completion,
definition, `Ctrl+T`, the base-game picker, and MCP search disagree about the
same class because they use different indexes or source identities, the design
has failed.
