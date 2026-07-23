# s&box MCP server review

Research date: 2026-07-23. This review fully read the requested official
s&box [MCP Server documentation](https://sbox.game/dev/doc/editor/mcp-server)
and its rendered MCP API-search results, including every first-party
`Editor.Mcp` API entry surfaced by that search. It distinguishes the s&box
editor's MCP *host and extension framework* from third-party packages listed
by API search; package-advertised tool counts are not claims about the s&box
core.

## What s&box provides

s&box embeds an MCP server in its editor process. It is enabled by default,
listens only on loopback, and is configurable from Editor Preferences; the
documented default is `http://127.0.0.1:7269/mcp`
([editor documentation](https://sbox.game/dev/doc/editor/mcp-server)). The
server is tied to the editor lifecycle: the public
[McpServer API](https://sbox.game/api/Editor.Mcp.McpServer) exposes running,
port, and URL state, with no standalone server when the editor is closed.

The editor documentation describes the resulting agent experience as project
and scene inspection/editing, asset search, play-mode control, console access,
and screenshots. Crucially, those are not a fixed universal tool list. The
server starts clients with a small discovery surface, then agents use
`search_tools` to find tools and `call_tool` to invoke them. Tools are live
because code hotload can add or remove them during the session.

This is the most consequential design choice to borrow: a large editor surface
does not have to occupy the model's initial tool context, and a hotloaded tool
does not require client/server restart or a stale `tools/list` cache.

## Tool authoring contract

The complete first-party `Editor.Mcp` group is indexed by the
[s&box API index](https://sbox.game/api/i/alleditor) and consists of:

| API | s&box contract | Reforger design lesson |
| --- | --- | --- |
| [McpToolAttribute](https://sbox.game/api/Editor.Mcp.McpToolAttribute) | Marks a static method as a tool; name, XML summary, parameter descriptions, schema, execution, and result shaping are derived from the method. | Define Workbench capability DTOs once and project them to MCP schemas; do not maintain duplicate handwritten tool descriptions. |
| [McpToolsetAttribute](https://sbox.game/api/Editor.Mcp.McpToolsetAttribute) | Names and describes a coherent static-class group; agents can discover groups. | Publish a small, versioned set of capability groups such as `project`, `resource`, `compiler`, and `world`. |
| [McpTool](https://sbox.game/api/Editor.Mcp.McpTool) | Convenience forms for normal and read-only tools. | Make read-only vs. write intent explicit in every tool descriptor. |
| [McpToolHints](https://sbox.game/api/Editor.Mcp.McpToolHints) | Client-facing behavior metadata; absent hints are conservatively treated as potentially mutating/destructive. | Default new Workbench actions to mutation-capable until their safety contract is proven. |
| [McpTool.ReadOnlyAttribute](https://sbox.game/api/Editor.Mcp.McpTool.ReadOnlyAttribute) | A promise that a tool only reads project/scene/editor state, permitting lower-friction invocation. | Use this only for handlers that cannot register, rebuild, open/focus, modify, or otherwise affect Workbench state. |
| [McpResult](https://sbox.game/api/Editor.Mcp.McpResult) | Builds mixed text, image, and structured-content result blocks. | Design the MCP host to return structured JSON and, later, image blocks for editor screenshots/previews. |
| [McpServer](https://sbox.game/api/Editor.Mcp.McpServer) | Embedded loopback Streamable-HTTP server status/lifecycle. | Keep Workbench as an optional local adapter, but expose its status as a first-class capability. |

Tools and toolsets are public API in s&box: agents remember names and workflows
depend on them, so renaming breaks those workflows. The documentation similarly
treats descriptions as executable interface design: a description should say
what the tool returns, which identifier it returns, and the next tool that
consumes it. This is stronger guidance than merely naming a function.

## Behavior that makes the server agent-effective

| Feature | How s&box does it | Value for Reforger |
| --- | --- | --- |
| Progressive discovery | Exposes a small front door; agents search live tool names/descriptions instead of loading a giant tool list. | Essential if project, resource, compiler, and World Editor capabilities grow. Implement `search_tools`/`describe_toolset` over a capability manifest. |
| Hotload coherence | Tool visibility follows compiled code hotload. | Workbench's custom handler set needs a version/capabilities handshake on reconnect; emit a tool-list change or force a fresh search when it changes. |
| Permission signaling | Read-only tools are labelled; ordinary tools are assumed capable of writing/destruction. | Map inspection to read-only; require client confirmation for registration, rebuild, import, world actions, and navigation. |
| Forgiving but fail-closed binding | Case-insensitive names and common value coercions are accepted; an unknown argument errors rather than being ignored. | Accept friendly agent input only where unambiguous; reject unknown fields and canonicalize paths/resource IDs before calling Workbench. |
| Structured DTO outputs | Stable classes yield output schemas and `structuredContent`; anonymous shapes are reserved for one-offs. | Use named result DTOs for compiler diagnostics, resources, project context, and world selection. |
| Bounded results | Paging is conventionally `limit` + `offset`; tools return totals alongside truncated results. | Require limits/cursors and `total`/`returned` facts for resource and reference-catalogue queries. |
| Native visuals | `Bitmap` returns an inline PNG; `McpResult` combines image and caption. | A later Workbench plugin can provide explicitly requested screenshots/previews, which are often more useful than serialized scene prose. |
| Practical errors | Exceptions return actionable agent-readable messages naming the recovery tool. | Return structured error code plus concise next action, e.g. “run `project_context` to select a loaded project.” |
| Editor-thread ownership | Tools execute on the editor main thread; slow asynchronous work must respect that boundary. | Keep Workbench engine/UI calls inside the plugin and let the MCP host own only socket/framing/retry work. |

The docs also establish engine-coordinate and identity conventions: scene objects
and components use GUIDs, assets use paths returned by asset search, and paging
defaults/maxima belong in parameter descriptions. These are small conventions
that remove repeated agent ambiguity.

## Differences that should remain different

s&box can reflect static methods directly inside its editor process. Reforger's
MCP host is external and reaches Workbench through a TCP NET API plus custom
`NetApiHandler` classes. Reforger cannot copy the attribute/reflection mechanism
verbatim. Its equivalent should be a typed, versioned plugin-side capability
manifest and individual named handler DTOs; the MCP host converts that manifest
to `search_tools`, `describe_toolset`, and tool schemas.

Likewise, s&box's loopback HTTP server belongs to the editor. Reforger should
not publish Workbench's proprietary TCP socket directly or turn it into a
network-facing HTTP service. Keep MCP local (`stdio` first, with a separately
designed HTTP option) and keep the Workbench connection loopback-only.

## Recommended adoption order

1. Add capability groups and stable names: `project`, `reference`, `resource`,
   `compiler`, and later `world`.
2. Implement a minimal progressive-discovery front door: `search_tools`,
   `describe_toolset`, and named tool invocation only for capabilities the
   current Workbench plugin reports.
3. Define named structured result DTOs with pagination and source/resource
   identifiers; avoid anonymous text blobs for recurrent queries.
4. Classify tools conservatively: read-only inspection is automatic-eligible;
   navigation and all changes remain consent-gated.
5. Add preview/image support only after a Workbench-side capture path is
   verified, keeping image generation explicit and bounded.

The primary lesson is architectural, not the number of commands: discoverable,
well-described, stable, small tools paired with structured, bounded outputs let
an agent operate an advanced editor without receiving an overwhelming and stale
tool catalogue.
