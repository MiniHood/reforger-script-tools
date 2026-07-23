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
  |-- Workbench adapter: optional, capability-negotiated NET API client
  `-- operation policy: workspace root, dry-run, confirmation, audit result
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

## MCP presentation choices

Use resources for stable, read-only context: a project manifest, resolved
content-root map, language-engine diagnostics snapshot, and (when connected) a
Workbench capability snapshot. Use tools for queries requiring arguments or
actions: `search_symbols`, `inspect_resource`, `validate_scripts`, and
`apply_workspace_edit`. MCP tools are model-controlled and the specification
recommends a human be able to deny invocations, especially for operations
([MCP tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)).

Prompts are a good fit for user-selected workflows rather than hidden policy:
"understand this addon," "plan a prefab change," "validate before packaging,"
and "explain these compiler errors." They should guide use of the small tool
set, not smuggle write permission into a prompt.

Avoid a universal `run_command`, arbitrary path read/write, or raw
`call_workbench_api` tool. Those are indistinct authority escalations, make
review difficult, and would turn an MCP schema into an undocumented second API.
Add one named, typed tool per enduring capability instead.

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

## Decisions to validate before implementation

1. Determine whether this server belongs beside the existing extension as a
   developer tool/package or as a separately launched local executable. Its
   lifecycle must not add a marketplace dependency on user-installed runtimes.
2. Define a shared project identity/content-root resolver so direct files and
   Workbench resource names cannot silently refer to different projects.
3. Prototype one read path (`inspect_resource`) and one verification path
   (`validate_scripts`) through the Workbench adapter.
4. Prototype one custom world action with preview plus `BeginEntityAction` /
   `EndEntityAction`, then prove Undo in Workbench before surfacing a mutating
   world tool.
5. Establish exact limits: response-size paging, operation timeouts,
   cancellation, and connection/retry behaviour when Workbench is unavailable.

The initial design is successful if it makes the existing language engine and
Workbench compiler/resource facts available under clear, narrow MCP contracts;
not if it exposes every possible engine call.
