# s&box MCP server source review

Research date: 2026-07-25.

This review uses Facepunch's open-source s&box repository at the exact commit
[`724eaae0875d6203d230a4056e471c9e8072a9ed`](https://github.com/Facepunch/sbox-public/tree/724eaae0875d6203d230a4056e471c9e8072a9ed).
It covers every file in the requested
[`game/addons/tools/Code/Mcp`](https://github.com/Facepunch/sbox-public/tree/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp)
directory, the complete supporting runtime in
[`engine/Sandbox.Tools/Mcp`](https://github.com/Facepunch/sbox-public/tree/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp),
and every other MCP reference found by searching the pinned repository.

This is a comparative example bank, not a Reforger implementation plan. The
[MCP server research](mcp-server-research.md) and Workbench NET API research
own Reforger architecture and delivery decisions. Do not infer an adopted
transport, discovery model, tool shape, or roadmap from this source.

The source review corrects an important ambiguity in the earlier documentation
review. At this commit, the server has **seven permanent host tools** and
**45 live first-party domain tools**. The host advertises only the seven stable
entry points through MCP `tools/list`; the other 45 are discovered through the
live registry. Addons may contribute more tools, so 45 is a source snapshot,
not a permanent protocol promise.

## Executive findings

s&box embeds a Streamable HTTP MCP server directly in the editor process. The
server owns transport and protocol handling, while a reflection-backed registry
discovers callable editor methods. The editor's type library is re-read whenever
the registry is queried, so hotloaded tools appear and disappear without a
separate registration step or persisted index
([server](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpServer.cs),
[registry](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs)).

The design is notably AI-oriented:

- The initial context contains a small, stable discovery and feedback surface.
- Tools and toolsets carry agent-facing descriptions and generated schemas.
- Results are structured, bounded, paginated, and optionally visual.
- Common harmless model mistakes are tolerated, while unknown arguments fail
  loudly.
- Tool failures are returned in-band with recovery guidance so an agent can
  correct its next call.
- Status, compiler diagnostics, screenshots, console history, undo, and redo
  close the observe-act-verify loop.
- Mutations use editor-native undo scopes rather than bypassing editor state.

For Reforger Script Tools, the best ideas to borrow are the progressive
discovery surface, strong descriptions, bounded results, status and diagnostic
feedback, and direct access to authoritative sources. The reflection and
in-editor HTTP architecture should not be copied literally: our TypeScript
extension and Rust server must retain their existing ownership boundaries.

## Source topology and runtime architecture

```text
MCP client
  -> loopback POST /mcp
  -> McpServer JSON-RPC dispatch
       -> initialize / ping
       -> tools/list: seven permanent entry points
       -> tools/call
  -> TopLevelTools discovery or invocation
  -> ToolRegistry live EditorTypeLibrary scan
  -> attributed static editor/addon method
  -> editor main-thread queue
  -> text / structuredContent / image result
```

The implementation is divided into four useful layers:

| Layer | Source | Responsibility |
| --- | --- | --- |
| Transport and protocol | [`McpServer.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpServer.cs), [`JsonRpc.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/JsonRpc.cs) | Loopback listener, request limits, MCP negotiation, JSON-RPC dispatch, and protocol errors. |
| Stable AI front door | [`TopLevelTools.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/TopLevelTools.cs) | Discovery, dynamic invocation, batching, editor status, and console feedback. |
| Tool framework | [`ToolRegistry.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs), [`McpAttributes.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpAttributes.cs), [`McpResult.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpResult.cs) | Live discovery, schemas, argument binding, main-thread invocation, result shaping, and tool annotations. |
| Domain providers | [Seven addon MCP files](https://github.com/Facepunch/sbox-public/tree/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp) | Asset, component, editor, log, package, play, and scene capabilities. |

This is a deep-module design. Domain providers do not implement MCP framing,
JSON-RPC, schema generation, error envelopes, or thread dispatch. The registry
does not know asset or scene semantics. The server does not contain a switch
statement for all domain commands.

### Editor lifecycle and preferences

The MCP server is enabled by default, uses port `7269` by default, and restarts
when either preference changes. It starts after editor initialization and stops
on editor exit
([preferences](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/EditorPreferences.cs#L38-L62),
[lifecycle](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/ToolsDll.cs#L46-L49)).
The preferences page shows status, exposes the URL, copies it, and gives users a
ready-to-run client setup command
([preferences UI](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Editor/EditorPreferences/PageMcp.cs)).

The whole-repository search found no additional MCP tool collection and no
dedicated MCP test files at this commit. References outside the two main MCP
directories are lifecycle and preferences integrations.

## Protocol and network behavior

The server supports MCP protocol revisions `2025-11-25`, `2025-06-18`,
`2025-03-26`, and `2024-11-05`. Its protocol handlers are:

| JSON-RPC/MCP method | Behavior |
| --- | --- |
| `initialize` | Negotiates a supported protocol revision, identifies the server, advertises tools capability, and injects operating instructions. |
| `ping` | Returns an empty successful result. |
| `tools/list` | Returns only tools carrying the internal `[McpListed]` marker. |
| `tools/call` | Validates the requested name and arguments, then routes through the registry. |

Notifications return HTTP `202` without a response body. Unsupported JSON-RPC
methods return the standard method-not-found error. JSON-RPC IDs retain string
or numeric form, and protocol batches are not supported
([server dispatch](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpServer.cs#L172-L293),
[JSON-RPC types](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/JsonRpc.cs)).

The HTTP boundary is deliberately small:

- It binds both `127.0.0.1` and `localhost`, never a non-loopback address.
- Only `POST /mcp` is accepted. Unknown paths return `404`; other methods return
  `405`.
- An `Origin` header is accepted only when its host is loopback, preventing a
  browser page from reaching the local editor through DNS rebinding.
- Requests are capped at 8 MiB. Both declared `Content-Length` and bytes
  actually read from a chunked request are bounded.
- The listener accepts requests continuously and gives each request an
  asynchronous task.
- It uses plain JSON Streamable HTTP and does not provide server-initiated
  streams or session deletion.

These safeguards are visible directly in
[`McpServer.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpServer.cs#L16-L144).

## The seven permanent direct tools

These are the exact tools advertised through `tools/list` at the pinned commit
([definitions](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/TopLevelTools.cs),
[list filtering](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs#L41-L57)):

| Direct tool | Intent | Behavior |
| --- | --- | --- |
| `list_toolsets` | Read-only | Lists current tool groups, descriptions, and tool names. Classes sharing a toolset name are merged case-insensitively and results are ordered. |
| `describe_toolset` | Read-only | Returns full schemas and descriptions for every tool in one live group. An unknown group returns available names and close suggestions. |
| `search_tools` | Read-only | Searches the live registry. Every space-separated query term must match the combined tool name, toolset, title, description, or parameter descriptions. An empty query returns all tools. |
| `call_tool` | Potentially mutating | Invokes one live tool by name with its argument object. Whether the delegated operation reads or writes is determined by the selected tool. |
| `call_tools` | Potentially mutating | Runs an ordered batch to reduce round trips. It validates every call name and shape before beginning, executes serially, and stops after the first runtime failure while reporting skipped calls. |
| `editor_status` | Read-only | Returns engine version, project, active scene, dirty state, play/pause state, live tool count, and useful filesystem paths. |
| `read_console` | Read-only | Reads a bounded, filterable console history with severity, timestamps, repetition counts, and short error stacks. |

The seven-tool front door is the core context-management strategy. The server
declares `listChanged: false` because the permanent list does not change.
Agents explicitly query the live registry when they need domain capabilities;
the dynamic inventory can therefore hotload without invalidating MCP's
advertised list.

## Complete first-party live tool snapshot

The requested addon directory contributes 45 domain tools across seven
toolsets: 21 read-only operations and 24 operations that may change editor,
project, package, asset, play, or scene state. These tools are concrete
first-party features at the pinned commit, but they are intentionally not the
permanent protocol list.

### Asset toolset: 11 tools

Source:
[`AssetSystem.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/AssetSystem.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `asset_search` | Read-only | Searches by keywords and optional asset type; can restrict to project, unreferenced, compile-failed, or uncompiled assets; supports offset and bounded limit. |
| `asset_info` | Read-only | Returns detailed metadata for a resolved asset. |
| `asset_compile` | Mutating | Compiles an asset and reports failure with a `read_console` recovery hint. |
| `asset_dependencies` | Read-only | Traverses references, dependants, or parents; supports deep traversal, type filtering, and a result cap. |
| `asset_read` | Read-only | Reads a `GameResource` asset as JSON. |
| `asset_write` | Mutating | Validates JSON, writes a `GameResource`, and compiles it. |
| `asset_thumbnail` | Read-only | Generates an inline PNG thumbnail with the asset path as text context. |
| `asset_files` | Read-only | Reports source, compiled, input, additional, and unresolved files associated with an asset. |
| `asset_find_by_file` | Read-only | Resolves an asset from an associated filesystem path. |
| `asset_types` | Read-only | Enumerates searchable/creatable asset types. |
| `create_asset` | Mutating | Validates type, path, and JSON, creates the resource, and compiles it. |

Search results are deterministically sorted by path, return total/showing/offset
facts, and clamp limits to `1..500`. Resolution errors can include up to five
nearby paths. The read/write operations deliberately use the engine resource
model instead of editing arbitrary files.

### Component toolset: one tool

Source:
[`Components.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/Components.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `get_component_type` | Read-only | Resolves a component type by exact or substring name and describes the inspector-visible `[Property]` surface. Ambiguity returns candidate types instead of guessing. |

This is a good example of exposing the same authoritative property contract the
editor uses rather than reflecting every public field indiscriminately.

### Editor toolset: two tools

Source:
[`EditorTools.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/EditorTools.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `console_command` | Mutating | Runs a command through the editor's console system. |
| `compile_status` | Read-only | Reports every compiler's status and bounded warning/error diagnostics. |

`compile_status` sorts errors before warnings, includes one-based source
locations, and caps diagnostics at 50 per compiler. Together with
`read_console`, it gives an agent a direct validation loop after code or asset
changes.

### Log toolset: three tools

Source:
[`Log.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/Log.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `log_info` | Mutating | Writes one or more informational lines to the editor log. |
| `log_warning` | Mutating | Writes one or more warning lines. |
| `log_error` | Mutating | Writes one or more error lines. |

These are classified as effectful because even diagnostic output changes
editor state. Splitting multiline input preserves the console's line model.

### Package toolset: three tools

Source:
[`Packages.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/Packages.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `find_packages` | Read-only | Searches the package backend asynchronously with offset and a limit clamped to `1..100`. |
| `get_package` | Read-only | Retrieves package details. |
| `install_package` | Mutating | Downloads and installs a package into the active tools context. |

### Play toolset: three tools

Source:
[`Play.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/Play.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `play_start` | Mutating | Starts play mode after checking current state. |
| `play_stop` | Mutating | Stops play mode after checking current state. |
| `play_pause` | Mutating | Pauses or resumes play mode and reports the resulting state. |

### Scene toolset: 22 tools

Source:
[`Scene.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/game/addons/tools/Code/Mcp/Scene.cs).

| Tool | Intent | Feature |
| --- | --- | --- |
| `list_scenes` | Read-only | Lists editor scenes and identifying state. |
| `scene_tree` | Read-only | Returns a bounded hierarchy with optional maximum depth and explicit omitted-child counts. |
| `get_game_object` | Read-only | Resolves a game object and can include serialized component properties. |
| `find_game_objects` | Read-only | Searches scene objects with bounded traversal, pagination-like limit, total count, and a truncation flag. |
| `get_editor_camera` | Read-only | Returns current editor camera state. |
| `set_editor_camera` | Mutating | Changes editor camera state. |
| `editor_camera_screenshot` | Read-only | Captures the editor camera as an MCP image. |
| `create_game_object` | Mutating | Creates a scene object under an editor undo scope. |
| `spawn_model` | Mutating | Creates one model-backed object. |
| `spawn_models` | Mutating | Creates multiple model-backed objects in one call and one undo step. |
| `delete_game_object` | Mutating | Deletes a resolved object with undo support. |
| `set_game_object` | Mutating | Updates object state/properties with undo support. |
| `add_component` | Mutating | Adds a resolved component type with undo support. |
| `remove_component` | Mutating | Removes a resolved component with undo support. |
| `set_component` | Mutating | Validates and updates inspector-visible component properties with undo support. |
| `get_selection` | Read-only | Returns current editor selection. |
| `set_selection` | Mutating | Resolves all requested targets before replacing selection. |
| `save_scene` | Mutating | Saves a resolved scene. |
| `undo` | Mutating | Invokes editor undo. |
| `redo` | Mutating | Invokes editor redo. |
| `camera_screenshot` | Read-only | Renders a scene screenshot from an explicit camera definition. |
| `scene_trace` | Read-only | Performs an engine physics trace through scene geometry. |

Scene traversal has explicit work bounds: `scene_tree` has a 5,000-node output
budget, while `find_game_objects` has a 100,000-node visit budget and a result
limit clamped to `1..500`. Screenshots clamp dimensions to `16..4096`. Game
objects and components are addressed with GUIDs, not names that may collide.

Scene mutations use specialized editor undo captures for object/component
creation, destruction, and changes. Several tools prevalidate a complete
request before making changes: component property updates validate names,
writability, and JSON conversion before setting anything, and selection
replacement resolves every target before clearing the old selection. Batch
model spawning exists because one semantically coherent operation is cheaper
and easier to undo than many protocol round trips.

## Tool discovery and authoring contract

A static method becomes available by carrying `[McpTool]` or
`[McpTool.ReadOnly]`. A class-level `[McpToolset]` gives a group its stable name
and description. If a tool name is omitted it is derived as snake case. The
registry scans `EditorTypeLibrary` on demand, sorts by name, and warns while
skipping duplicate names
([authoring README](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/README.md),
[attributes](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpAttributes.cs)).

The source treats tool names and descriptions as public API:

- A stable name matters because agent prompts and learned workflows refer to it.
- A description should explain the result, the identifier it returns, and the
  next tool that consumes that identifier.
- Addon XML documentation is code-generated into descriptions; engine
  assemblies use `[Description]`.
- Toolsets let an agent browse a domain without loading unrelated schemas.
- Only permanent infrastructure tools receive the internal `[McpListed]`
  marker.

This achieves live discovery without a persisted tool index. The trade-off is
that every registry access enumerates attributed methods. Facepunch accepts
that reflection cost in exchange for hotload correctness, while keeping the
initial MCP list fixed and small.

## Schema generation and argument binding

The registry generates `inputSchema` from method signatures
([schema and binding implementation](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs)):

- Required parameters are those without defaults.
- Parameter descriptions, ranges, and meaningful defaults are included.
- Strings, booleans, integers, numbers, enums, arrays, lists, and
  string-keyed dictionaries get specific JSON Schema shapes.
- Engine vectors and angles are represented honestly as comma-separated
  strings because that is how the engine converters serialize them.
- Plain DTO classes/structs expose public settable properties, respect
  `JsonPropertyName` and `JsonIgnore`, and recognize C# `required` members.
- Schema reflection stops after depth four to avoid recursive type expansion.
- Framework and engine types with custom converters use an open schema rather
  than a guessed, misleading structure.

Binding is forgiving where correction is unambiguous:

- Tool, argument, and enum names are matched case-insensitively.
- JSON accidentally encoded inside a string is unwrapped and retried.
- Numeric and boolean strings can bind through JSON conversion.
- Any JSON value can become a string parameter.
- Values with `[Range]` are clamped rather than rejected when an agent
  overshoots a documented limit.

It is strict where silent acceptance could perform the wrong action:

- Unknown arguments are rejected and the valid parameter vocabulary is shown.
- Missing required arguments are rejected.
- Nullability and conversion failures name the expected type, received value,
  and complete parameter contract.
- Unknown tools can suggest up to five close names and direct the agent back to
  `search_tools`.

The combination is useful: tolerate representational noise, reject semantic
ambiguity.

## Results, images, and errors

Return shaping is centralized in
[`McpResult.cs`](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpResult.cs)
and the registry:

| Tool return | MCP behavior |
| --- | --- |
| Named DTO | Generates `outputSchema` and returns `structuredContent`, plus JSON text for older clients. |
| `string` or other ordinary value | Returns a text content block. |
| `GameObject` or `Component` | Uses compact custom identity JSON rather than serializing a full engine graph. |
| `Resource` | Uses an authoritative resource path. |
| `Bitmap` | Encodes an inline PNG image block. |
| `McpResult` | Allows composed text, image, and structured blocks. |
| `Task` / `Task<T>` | Is awaited; the eventual value is shaped normally. |
| `null` / `void` | Returns `ok`. |

Named DTOs are preferred for recurring contracts because the promised output
shape becomes machine-readable. Anonymous objects remain plain text because
the registry cannot advertise a stable output schema for them.

Lookup, validation, and execution failures are returned as normal MCP tool
results with `isError: true`. They do not become opaque transport failures.
Messages are deliberately short and actionable; ordinary exceptions include
only a few useful stack frames, while registry-authored validation errors omit
stack noise.

## Performance and responsiveness

The implementation contains several concrete performance controls:

| Control | Source behavior | Design value |
| --- | --- | --- |
| Small permanent tool list | Only seven schemas occupy the client's initial tool context. | Reduces model context cost even when addons add many tools. |
| Live, unindexed registry | Reflection/type-library discovery runs on demand. | Avoids stale caches and index rebuilds after hotload. Suitable while registry size remains modest. |
| `call_tools` batching | Multiple ordered operations share one protocol call. | Reduces HTTP/model round trips and groups related work. |
| Bounded input | HTTP bodies stop at 8 MiB. | Prevents accidental or hostile memory growth. |
| Bounded traversal | Scene queries cap output and visits. | Prevents an exploratory query from freezing a large scene. |
| Bounded paging | Asset, package, object, and console limits are clamped. | Keeps latency and output predictable while returning totals/truncation facts. |
| Deterministic ordering | Tools and asset paths are sorted. | Produces reproducible agent results and stable paging. |
| Async support | Package and other asynchronous operations are awaited off the main dispatch path. | Avoids pretending inherently asynchronous work is synchronous. |
| Main-thread pickup deadline | A queued editor call must start within 30 seconds. | Detects a modal dialog or blocked editor instead of silently timing out at the client. |
| Console ring buffer | At most 2,000 events are retained under a lock. | Gives bounded, thread-safe observability independent of the visible console window. |
| Schema depth cap | DTO reflection stops at depth four. | Avoids recursive or excessively large schemas. |

There is a subtle main-thread behavior to understand. The 30-second deadline
only covers pickup. Once a tool begins, it is never forcibly cancelled. If the
editor is blocked and picks the queued call up later, that call still runs even
though its original result is no longer returned. A late failure is written to
the console so `read_console` can recover it
([main-thread invocation](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs#L305-L362)).

For our sub-five-second official-wiki search goal, the relevant lesson is not
to copy reflection. It is to benchmark direct authoritative-source search
first, bound every query, return totals/truncation/source paths, and introduce a
derived index only if measured corpus size makes the direct path miss the
latency budget.

## Safety and mutation practices

Read-only tools advertise MCP `readOnlyHint`. Tools without a hint are left
unannotated because the MCP default already tells clients to assume they may
write or destroy state. This is a conservative and useful default
([annotations](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/ToolRegistry.cs#L84-L131)).

Additional safety properties include:

- Loopback binding and Origin validation at the network boundary.
- Exact route and method enforcement.
- Request, result, traversal, screenshot, and log bounds.
- Resolution by stable GUID or authoritative asset path.
- Ambiguity errors instead of first-match guesses.
- Validation of whole property/selection requests before mutation.
- Native editor undo scopes for scene changes.
- Explicit `undo` and `redo` tools.
- State checks before play-mode transitions.
- Error messages that identify the recovery action.

`console_command`, package installation, asset writes, and scene mutations are
powerful capabilities. s&box exposes them because the MCP server is a
loopback-only editor feature, not an unauthenticated remote service. A Reforger
implementation must still apply its own consent and capability policy; local
transport alone does not make every operation safe to auto-run.

## Global AI instructions

During `initialize`, s&box supplies a compact operating guide to every client
([instruction source](https://github.com/Facepunch/sbox-public/blob/724eaae0875d6203d230a4056e471c9e8072a9ed/engine/Sandbox.Tools/Mcp/McpServer.cs#L151-L170)).
It tells the agent to:

- begin with `editor_status`;
- discover tools with `search_tools`, `list_toolsets`, and
  `describe_toolset`;
- invoke them through `call_tool` or batch them with `call_tools`;
- inspect failures and compilation through `read_console`;
- honor `limit`/`offset` ranges and expect server-side clamping;
- use the engine's vector, angle, coordinate, and units conventions;
- retain GUIDs and asset paths returned by authoritative lookup tools; and
- expect every scene edit to push an undo step.

Within s&box, this is architecture rather than prompt decoration. Cross-cutting
facts live once at initialization instead of being repeated inconsistently in
every tool description. Tool-specific facts remain with the tool schema.

## Optional ideas to evaluate

These observations are prompts for future evaluation, not requirements. Keep
only an idea that matches Reforger evidence, the existing ownership boundaries,
and a demonstrated user workflow.

| s&box observation | Possible question for Reforger | Not an adoption decision |
| --- | --- | --- |
| Strong descriptions and bounded structured results | Does a proposed named tool give an agent enough identity, paging, and recovery information? | Do not copy its dynamic registry. |
| Status, diagnostics, and Undo support an observe-act-verify loop | Can a typed Workbench operation return authoritative status and native Undo evidence? | Do not infer that an endpoint or handler already exists. |
| A small stable front door limits context | Does a real Reforger tool surface require progressive discovery? | Do not add toolset search, generic invocation, or reflection without measurement. |
| Optional image results help visual editor tasks | Is there a proven bounded Workbench image-transfer contract? | Do not copy Streamable HTTP, image behavior, or its editor process model. |

The current Reforger architecture remains independent: Rust owns offline
semantic and documentation operations; the private NET API Gateway and a
versioned Workbench handler package own live editor operations.

## Practices not to copy blindly

- A dynamic `call_tool` dispatcher is appropriate for s&box's hotloaded editor
  ecosystem. Our initial reference repository has a known, narrow contract, so
  direct typed MCP tools may be clearer and safer.
- Reflection on every discovery request is acceptable for s&box's current
  tool count, but it says nothing about full-text search performance over our
  Markdown corpus. Measure the actual corpus and query path.
- A 30-second main-thread pickup timeout is not cancellation. Reforger
  operations need explicit cancellation/idempotency rules before adopting
  delayed effectful calls.
- `call_tools` prevalidates call names and shapes, but it is not a transaction:
  earlier calls remain applied when a later runtime call fails. Do not describe
  a similar batch as atomic without rollback.
- Loopback-only HTTP reduces exposure but does not replace operation-level
  consent or path validation.
- Generic console command execution is powerful but weakly typed. Prefer named
  Workbench capabilities with schemas when a durable API exists.

## Bottom line

The pinned s&box source demonstrates one mature AI-facing editor design:
seven stable front-door commands, live discovery, generated schemas, bounded
results, actionable errors, editor-native Undo, and verification channels.

Use it as a comparison point when a Reforger feature needs ideas. Reforger
requirements remain governed by its own authority boundaries, Workbench
evidence, packaging model, and MCP research; no s&box mechanism is presumed
portable or required.
