# MCP server exploration journal

Research date: 2026-07-23. This is a design exploration, not an implementation
commitment. It records the useful feature surface and the architectural boundary
for a local MCP server that helps make Arma Reforger content. It deliberately
separates capabilities proven by ordinary project files from operations that
need a running Workbench instance; the companion [Workbench NET API journal](workbench-net-api-research.md)
owns the latter.

## Evidence and working premise

MCP servers expose three intentionally different primitives: model-controlled
tools, application-controlled read-only resources, and user-invoked prompts
([MCP server concepts](https://modelcontextprotocol.io/docs/learn/server-concepts)).
The protocol itself is JSON-RPC and supports local `stdio` as well as Streamable
HTTP; `stdio` is the appropriate first transport for a local developer tool
([MCP transports](https://modelcontextprotocol.io/specification/draft/basic/transports)).

The project already owns a Rust language engine for Enfusion understanding.
The MCP server must query that engine rather than parse Enfusion independently.
Likewise, it must use Workbench only for facts and operations that are actually
owned by the running Workbench. This preserves the repository's TypeScript-shell
/ Rust-engine ownership boundary and avoids a second language server.

## Proposed shape

```
MCP client
  | stdio / JSON-RPC
local MCP host
  |-- project gateway: bounded filesystem reads and staged writes
  |-- language-engine adapter: parser, symbols, references, diagnostics
  |-- evidence-catalogue adapter: bundled game data and wiki documents
  |-- Workbench adapter: optional, capability-negotiated NET API client
  `-- tool catalogue and operation policy: discovery, workspace root, dry-run,
      confirmation, audit result
```

The host is an adapter, not a new semantic engine or a general shell. Each tool
should declare a small schema, return structured results with stable paths and
diagnostics, and say which authority supplied the answer: `filesystem`,
`language-engine`, or `workbench`.

Start with a local, single-user `stdio` server. Do not expose the Workbench
socket through HTTP or bind it beyond loopback as part of this exploration.
Remote access changes authentication, authorization, audit, and network-exposure
requirements before it creates new editor value.

## Feature catalogue by authority

| Feature family | Useful MCP capability | Authority | Mutation policy |
| --- | --- | --- | --- |
| Project orientation | List mounted projects, addon metadata, content roots, file tree, and discovered resources. | Filesystem | Read-only |
| Source understanding | Search symbols/text, inspect declarations, references, call sites, syntax tree, diagnostics, and formatting preview. | Rust language engine + files | Read-only |
| Reference research | Search and read bundled game-data source and wiki-document passages, with source/version/provenance metadata. | Bundled evidence catalogue | Read-only |
| Change planning | Produce an edit plan/diff for a named file set; create a new script/prefab/config from an explicit template. | Filesystem + language engine | Preview first |
| Safe source edits | Apply an exact, version-checked set of text edits and return the resulting diagnostics. | Filesystem + language engine | Explicit confirmation |
| Asset inspection | Read a registered resource's engine-resolved metadata, container shape, child data, or material list. | Workbench NET API | Read-only |
| Asset lifecycle | Register/rebuild/import an asset; update an engine-managed resource/container. | Workbench NET API | Explicit confirmation; return affected files |
| Workbench navigation | Open a resource or bring a named module forward. | Workbench NET API | Explicit user-visible action |
| Verification | Run compiler-backed script validation and normalize its errors/warnings into file locations. | Workbench NET API | Explicit invocation |
| World editing | Inspect selection; run a narrowly named, undo-grouped custom plugin operation. | Custom NET API handler | Preview/confirmation/undo contract |
| Testing | Run an explicit Workbench/autotest target and return structured report artifacts. | Workbench plugin/API | Explicit invocation |

The first three rows are the strongest initial slice: they are useful even when
Workbench is closed, operate on a known workspace, and complement the existing
language engine. The next validation step should be compiler-backed script
validation, because it has a clear, inspectable result. Asset import and world
mutation are valuable, but they should follow only after their side-effect and
rollback contracts are demonstrated.

## Bundled evidence catalogue and search

Bundled game data and official wiki documents should be a first-class,
read-only evidence catalogue. This makes answers such as "what is this API?",
"show examples of this attribute," and "where is this Workbench workflow
documented?" available even when Workbench is closed or its NET API is
disabled. It is not a replacement for compiler validation, live World Editor
state, or the resource database.

Build one catalogue at extension packaging/acquisition time from immutable
documents. Every indexed document/chunk must retain a manifest record containing
at least its evidence kind (`game-data` or `official-wiki`), Reforger/extension
version, original logical path or source URL, retrieval/build timestamp, and
content hash. Search results must return that provenance, a bounded excerpt,
and a cursor/range sufficient to retrieve more context. This prevents a wiki
claim from being presented as compiler truth, or a result from one game-data
version being silently applied to another.

The useful initial MCP contract is deliberately small:

| Capability | Contract |
| --- | --- |
| `search_reference` | Full-text/structured search across selected evidence kinds; filters include source kind, version, path/API name, and result limit. Return ranked excerpts plus provenance, never an unbounded corpus dump. |
| `read_reference` | Read a bounded, cursor-addressed passage returned by search, preserving source identity and version. |
| `reference_catalogue` resource | Exposes installed corpus versions, coverage counts, update time, and unavailable/mismatched sources. |
| `find_api_examples` | A later structured projection over parsed game source: symbol/attribute/handler name to declarations and call sites. It must reuse Rust facts where it needs Enfusion syntax. |

Use deterministic lexical/path/symbol search first. Add semantic or embedding
ranking only if it can preserve exact source citations and deterministic
filters; it must not become the only way to find an API. Keep the original
documents out of model instructions: they are untrusted data returned through
bounded tools/resources, not authority to invoke tools or override policy.

Before distributing wiki content in the extension, record its source URL,
version/update strategy, attribution, and redistribution licence/terms in the
catalogue build contract. A stored web page is not automatically equivalent to
the game data distributed with the tools.

## MCP presentation choices

Use resources for stable, read-only context: a project manifest, resolved
content-root map, reference-catalogue manifest, language-engine diagnostics
snapshot, and (when connected) a Workbench capability snapshot. Use tools for
queries requiring arguments or actions: `search_reference`, `search_symbols`,
`inspect_resource`, `validate_scripts`, and `apply_workspace_edit`. MCP tools
are model-controlled and the specification recommends a human be able to deny
invocations, especially for operations
([MCP tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)).

Prompts are a good fit for user-selected workflows rather than hidden policy:
"understand this addon," "plan a prefab change," "validate before packaging,"
and "explain these compiler errors." They should guide use of the small tool
set, not smuggle write permission into a prompt.

Avoid a universal `run_command`, arbitrary path read/write, or raw
`call_workbench_api` tool. Those are indistinct authority escalations, make
review difficult, and would turn an MCP schema into an undocumented second API.
Add one named, typed tool per enduring capability instead.

## Progressive discovery and result design

Organize the public surface into small capability groups: `project`,
`reference`, `resource`, `compiler`, and `world`. Names and input/output
schemas within a group are a compatibility contract; a connected Workbench
plugin may make a group available, but must not silently redefine an existing
tool. File, language-engine, and evidence tools remain usable without
Workbench. Workbench-dependent groups must report their unavailable reason
instead of falling back to approximate filesystem behavior.

The initial tool set should stay compact. If the MCP client needs help finding
less-frequent operations, provide read-only `search_tools` and
`describe_toolset` discovery tools. They return a short description, stable
name, read-only/effect classification, input/output summary, availability, and
the capability revision that supplied it. This follows the useful progressive
discovery pattern seen in s&box without copying its generic invocation
indirection. Standard MCP already has a typed tool invocation primitive, so do
not add a `call_tool` or `call_workbench_handler` multiplexer: it hides schemas,
weakens permission signalling, and recreates the raw-dispatch problem above.

Every query result should be a named, structured DTO rather than formatted
prose. Collections need `returned`, `total` where known, a cursor, and a
server-enforced limit; evidence and resource answers need identity and
provenance; errors need a stable code, concise message, and an actionable
recovery hint. Return the smallest useful summary first and let the caller
follow stable IDs/cursors for detail. Future visual inspection may return MCP
image content, but only after a bounded, authenticated Workbench-to-host data
path is demonstrated; it is not a reason to add a generic binary or file API.

## Cross-cutting operation contract

Every mutating operation should accept a target rooted in the selected project,
offer a `dry_run`/preview where meaningful, and report: requested operation,
authority, concrete affected paths/resource IDs, Workbench action name, and
verification result. Filesystem edits also need expected document versions or
content hashes. World-editor actions need a Workbench-side undo action name.

The MCP client remains responsible for the final consent UI; the server must
still make effects legible enough for that UI and refuse targets outside the
configured project roots. Treat contents received from files, assets, logs, or
NET API responses as data, never as MCP-server instructions.

Classify a tool as read-only only when every supported invocation has no
external/editor effect. Navigation such as opening a resource is therefore a
separate user-visible action, even though it does not edit project data. A
missing classification is potentially effectful, not read-only.

## Decisions to validate before implementation

1. Specify the packaged evidence-catalogue manifest, source/update and
   attribution/licensing rules, and a bounded index format that remains
   self-contained in a Marketplace install.
2. Prototype `search_reference` over a representative game-data plus wiki
   corpus; verify version filtering, citation fidelity, pagination, and
   predictable results for symbol/path/text queries.
3. Determine whether this server belongs beside the existing extension as a
   developer tool/package or as a separately launched local executable. Its
   lifecycle must not add a marketplace dependency on user-installed runtimes.
4. Define a shared project identity/content-root resolver so direct files and
   Workbench resource names cannot silently refer to different projects.
5. Prototype one read path (`inspect_resource`) and one verification path
   (`validate_scripts`) through the Workbench adapter.
6. Prototype one custom world action with preview plus `BeginEntityAction` /
   `EndEntityAction`, then prove Undo in Workbench before surfacing a mutating
   world tool.
7. Establish exact limits: response-size paging, operation timeouts,
   cancellation, and connection/retry behaviour when Workbench is unavailable.

The initial design is successful if it makes the existing language engine and
Workbench compiler/resource facts available under clear, narrow MCP contracts;
not if it exposes every possible engine call.
