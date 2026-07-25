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

## Primary product goal: AI-friendly Reforger work

AI-friendly behavior is one of the MCP server's main product goals, not a
presentation detail to add later. A capable client must be able to discover a
small relevant capability, ask a bounded question, receive structured and
citable evidence, and follow a stable identity for more context without being
given an opaque storage path or an entire corpus. Tool descriptions, schemas,
result identities, excerpts, ranges, source URLs, limits, cursors, effect
metadata, and recovery hints are therefore product-facing contracts.

The server must make the authoritative path easy for an AI to use. In
particular, official wiki Markdown is directly searchable source material, not
an AI-prepared summary or a hidden retrieval database. Search may rank and
excerpt it, but it must preserve the title, canonical source URL, logical
relative path, and exact passage that support the answer. This lets an AI
distinguish documented workflow guidance from language-engine, filesystem, or
live Workbench facts.

## Proposed shape

```
MCP client
  | MCP over local stdio / JSON-RPC
  v
local MCP host (the public AI-facing server)
  |-- project gateway: bounded filesystem reads and staged writes
  |-- language-engine adapter: parser, symbols, references, diagnostics
  |-- evidence-catalogue adapter: bundled game data and wiki documents
  |-- tool catalogue and operation policy: discovery, workspace root, dry-run,
  |   confirmation, audit result
  `-- Workbench NET API adapter (private typed client)
        | local NET API protocol; not MCP and never exposed as a pass-through
        v
     running Reforger Workbench (external editor process)
        `-- this project's optional Workbench plugin
              `-- versioned typed handlers: engine/resource/world/editor calls
```

The host is an adapter, not a new semantic engine or a general shell. Each tool
should declare a small schema, return structured results with stable paths and
diagnostics, and say which authority supplied the answer: `filesystem`,
`language-engine`, or `workbench`.

The NET API is therefore outside the MCP server's public protocol boundary but
is reached through an adapter inside the MCP host. The Workbench plugin is
separate Enfusion Script loaded by the external Workbench process, not a module
running in the MCP host. The host owns MCP schemas, policy, discovery, retries,
and result mapping; the plugin owns live engine/editor calls. If Workbench is
unavailable, the host remains available for its file, language-engine, and
evidence-catalogue capabilities, while only Workbench-backed capability groups
are unavailable.

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
| Verification | Run compiler-backed script validation and normalize its errors/warnings into file locations. | Workbench NET API | Manual or explicitly user-enabled saved-idle policy |
| World editing | Inspect selection; run a narrowly named, undo-grouped custom plugin operation. | Custom NET API handler | Preview/confirmation/undo contract |
| Testing | Run an explicit Workbench/autotest target and return structured report artifacts. | Workbench plugin/API | Explicit invocation |

### Authority boundary: files versus Workbench

Direct files and the Rust language engine are the authoritative path for
durable workspace content: project tree and metadata, raw source/prefab/config
content, text and symbol search, syntax/semantic analysis, bundled game-data
and wiki research, version-checked edits, and diffs. They remain useful when
Workbench is closed and must not be displaced by an editor RPC layer.

They cannot establish live engine/editor facts: whether Workbench or World
Editor is running, compiler readiness or compiler diagnostics, active mounted
project/resource resolution, current selection or unsaved world state, editor
UI state, Undo history, imported/registered resource state, or what a viewport
currently renders. Do not fabricate those facts from paths or source text.

The built-in NET API supplies only status, opening/focusing editor UI, and
compiler validation. Engine-resolved resource inspection, live scene
inspection/editing, asset import/rebuild/registration, tests, and visual
captures require named custom Workbench-plugin handlers and validation in a
live supported Workbench version. A result that combines both worlds must label
each fact with its authority (`filesystem`, `language-engine`,
`evidence-catalogue`, or `workbench`) rather than silently merging potentially
different states.

The first three rows are the strongest initial slice: they are useful even when
Workbench is closed, operate on a known workspace, and complement the existing
language engine. They are foundations, not the intended limit of the product.
The destination is a capable local Reforger editor co-pilot that can understand,
see, and deliberately operate a live Workbench project. Compiler validation is
the next useful proof point because it has a clear, inspectable result; it is
not a statement that asset or world capabilities are less desired.

## Intended full editor capability set

The long-term MCP surface should support the following product capabilities.
This is a desired feature portfolio, not a claim that every underlying NET API
operation is already proven. Each capability needs a named, versioned plugin
contract and live-Workbench validation before it is advertised.

| Product capability | Intended user outcome | Required foundation |
| --- | --- | --- |
| Live scene inspection | Read current World Editor selection; find entities by name, prefab, class, tag, or area; inspect transforms, hierarchy, components, and relevant world state. | Stable entity/resource identities, bounded queries, and a World Editor context handler. |
| Live scene editing | Create, duplicate, place, move, rotate, configure, and delete entities; apply prefab/component changes; execute domain operations such as composition placement or spawn-point setup. | Typed domain commands, preview, explicit approval, one undo group, and post-action verification. |
| Asset discovery and inspection | Search and understand prefabs, materials, textures, terrain assets, animations, dependencies, metadata, and validation findings. | Canonical resource/path resolution, typed result DTOs, cursors, and provenance. |
| Asset lifecycle management | Create, register, import, rebuild, repair, or update supported engine-managed assets and return the affected resources/files. | Project containment, deterministic request inputs, preview/diff where meaningful, and outcome verification. |
| Editor control and verification | Open/focus resources or modules, inspect editor/compiler state, validate scripts, run explicit tests/builds, and return usable diagnostics. | Capability-aware editor actions and compiler/test result normalization. |
| Visual feedback | Return viewport screenshots, asset thumbnails, prefab/material previews, and optionally before/after captures for a requested action. | A proven bounded Workbench-to-host image contract, dimensions/size limits, and clear capture provenance. |
| Custom workflow tools | Offer project-specific, high-level operations whose names match a creator's intent rather than raw engine calls. | Plugin-owned toolsets, stable names/schemas, and narrowly scoped authority. |
| Explainable planning | Combine workspace, language-engine, bundled evidence, and live Workbench facts into plans and previews before an effectful operation. | Shared project/resource identity and explicit authority/provenance on every result. |

This breadth is intentional. The architecture should preserve one path from MCP
tool to a typed Workbench plugin command so later scene, visual, and asset
features extend the same capability manifest and result contract rather than
adding a parallel automation system. Safety requirements shape the contracts;
they do not narrow the future feature set.

### Future design index

Keep these exact categories in scope whenever the MCP server or Workbench
plugin is extended. They are the short rediscovery index for the intended
product, informed by the s&box MCP design:

| Category | What to look for when revisiting it |
| --- | --- |
| **Live scene editing** | World selection, entity search/inspection, hierarchy/component/transform reads, placement, creation, duplication, configuration, deletion, previews, Undo, and verification. |
| **Asset management** | Resource search, metadata/dependency inspection, previews, creation, import, registration, rebuilding, repair, validation, and affected-resource reporting. |
| **Editor control** | Open/focus resources and modules, inspect editor/console/compiler state, drive explicit validation/build/test workflows, and report actionable results. |
| **Visual feedback** | Viewport screenshots, thumbnails, prefab/material previews, and before/after operation captures with bounded image transfer. |
| **Custom tools** | Project-specific high-level workflow operations with durable names, descriptions, schemas, intent-level inputs, and narrow authority. |
| **Rich typing** | Versioned input/output DTOs, structured content, stable entity/resource identities, typed diagnostics, result counts/cursors, and output schemas. |
| **AI-friendly design** | Progressive discovery, excellent descriptions, stable tool names, linked follow-up IDs, bounded responses, effect metadata, and recovery-oriented errors. |

The categories are mutually reinforcing: live scene editing needs rich typing
and visual feedback; asset management and editor control need AI-friendly
discovery; custom tools compose the other capabilities into creator-oriented
workflows.

## Bundled evidence catalogue and search

Bundled game data and official wiki documents should be a first-class,
read-only evidence catalogue. This makes answers such as "what is this API?",
"show examples of this attribute," and "where is this Workbench workflow
documented?" available even when Workbench is closed or its NET API is
disabled. It is not a replacement for compiler validation, live World Editor
state, or the resource database.

One public result contract must not force one storage strategy. Game-data
semantic search may use its Rust-owned index, while the extension's packaged
`resources/official-wiki` Markdown is the authoritative official-wiki corpus
and is searched directly from those files. The MCP server must not require a
prebuilt wiki index, copied text store, or per-document manifest to answer a
wiki query. It extracts the title and canonical source URL from the matched
Markdown, returns the logical relative path and exact range, and reads the same
file again when the client follows the result.

Direct wiki search has a hard performance acceptance target: a cold search of
the complete packaged corpus must finish within five seconds, honor
cancellation, and return server-capped excerpts and result counts. The optional
`wiki-index.md` navigation aid is not source evidence and must not participate
in normal `search_reference` ranking or results. It may remain available as a
separate AI-oriented discovery resource, but no tool may depend on it.

Every evidence result must identify its evidence kind (`game-data` or
`official-wiki`), source identity (logical path or source URL), bounded excerpt,
and cursor/range sufficient to retrieve more context. This prevents a wiki
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

### s&box techniques adopted for Reforger

The [s&box MCP review](sbox-mcp-research.md) is a design input for this server,
not merely background reading. Adopt its advanced editor-MCP techniques where
they fit the different Reforger/NET API boundary:

| s&box technique | Reforger design commitment |
| --- | --- |
| Live progressive tool discovery | The host publishes a small discovery surface and derives currently available Workbench capabilities from the plugin manifest/revision. It refreshes after reconnect or plugin reload, so a client never plans against stale editor tools. |
| Toolsets with stable public names | Capability groups and their named operations are durable API. Descriptions state what is returned, limits, effects, and the next operation/identity to use. |
| Schema-first parameters and outputs | Inputs and repeated result shapes use versioned DTOs. The MCP tool publishes its input schema and structured output schema, rather than requiring the caller to parse prose or invent JSON. |
| Rich visual results | The `visual` group is a first-class future capability for viewport screenshots, previews, thumbnails, and before/after captures—not an optional UI embellishment. |
| Small linked results | Search/list tools return totals, limits/cursors, and stable entity/resource identities that feed detail and mutation tools; they never serialize an entire world or resource graph by default. |
| Clear effect signals | Read-only, navigation/editor-visible, and mutating operations are distinguished in metadata and descriptions; unknown operations are treated as effectful. |
| Actionable errors | Failures include a stable error code, a concise cause, and the safe next action, such as searching first or opening a required editor context. |
| Native editor execution | The Workbench plugin owns calls that need editor/world/resource state. The host never attempts to emulate those calls from raw files or run a general external command channel. |

The one deliberate non-adoption is s&box's generic `call_tool` entry point.
It solves s&box's in-process hotload discovery problem. Reforger's external
adapter should retain named typed MCP tools and a manifest-derived allowlist,
so client consent, schemas, audits, and compatibility remain visible at the
MCP boundary.

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
