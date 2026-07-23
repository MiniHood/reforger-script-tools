# Workbench NET API exploration journal

Research date: 2026-07-23. This is the evidence journal for using the Enfusion
Workbench NET API as one adapter behind a local MCP server. It describes the
observed protocol and feasible capabilities; it does not assert that a custom
handler is safe, authenticated, or stable across every Reforger version until
it is tested in the target Workbench.

## Primary evidence

The current extracted Reforger game data contains the engine-owned NET API
documentation in `WorkbenchGameCommon/NetApiDocs.c` (lines 11-220) and the
generated handler base in `WorkbenchGameCommon/generated/NetApi/NetApiHandler.c`
(lines 15-35). This is verified extracted game data, not a live Workbench test.
The corresponding official [Workbench NET API reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/Page_NetApi.html)
and [NetApiHandler reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/interfaceNetApiHandler.html)
corroborate the protocol and custom-handler model.
The official [Resource Manager options](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Options)
documents the user control that enables the NET API for external applications.
The official [Workbench Plugin tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Workbench_Plugin_Tutorial)
confirms that plugins can automate asset work and that World Editor actions can
participate in the history stack when bounded by `BeginEntityAction` and
`EndEntityAction`.

## Observed connection and wire contract

The extracted `NetApiDocs.c` states that an external client initiates a new
TCP/IP connection for each transaction. It sends and receives 32-bit
little-endian integers and UTF-8 Pascal strings (a 32-bit length followed by
the string). The request is, in order:

1. protocol version integer (`1` is the documented supported version);
2. client ID string;
3. content type string (`JsonRPC` is the documented supported value); and
4. a UTF-8 JSON payload string.

The JSON payload chooses an endpoint with `APIFunc`; the response contains an
error-code string and a JSON-payload string. Despite its `JsonRPC` label, the
documented payload is an `APIFunc` object rather than MCP or ordinary JSON-RPC
2.0 envelopes. The MCP host therefore needs a dedicated NET API codec/adapter,
never a transport pass-through.

The documentation does not establish the endpoint address/port, authentication,
TLS, concurrency, request-size limits, or cancellation semantics. Discover
those from the installed Workbench configuration/source and a loopback probe;
do not guess or publish the socket.

## Built-in endpoints with direct MCP value

`NetApiDocs.c` documents these built-ins:

| Workbench endpoint | MCP-facing capability | Effect |
| --- | --- | --- |
| `IsWorkbenchRunning` | `workbench_status` | Read-only status including successful script compilation state. |
| `IsWorldEditorRunning` | `world_editor_status` | Read-only status. |
| `OpenResource` | `open_resource_in_workbench` | Opens a named resource; a user-visible Workbench action. |
| `BringModuleWindowToFront` | `focus_workbench_module` | Brings a module window forward; user-visible. |
| `ValidateScripts` | `validate_scripts` | Compiler-backed errors/warnings for an explicit configuration. |

These should remain individual MCP tools, rather than an endpoint-name
parameter. `validate_scripts` is the highest-value first integration: its
documented response includes success, errors/warnings, resource-relative path,
and sometimes absolute path/addon/line information.

## Custom handler model and existing evidence

`NetApiHandler` dispatches by the derived Enforce Script class name: the
framework finds the matching subclass, obtains a request `JsonApiStruct`,
deserializes it, invokes `GetResponse`, and serializes the response. This
allows a custom Workbench plugin script to offer a deliberately small set of
MCP-required operations without changing the generic MCP host.

Existing extracted handlers demonstrate practical read and resource operations:

| Handler in extracted data | Evidence | Candidate lesson |
| --- | --- | --- |
| `GetResourceInfo` | `WorkbenchGameCommon/ResourceInfo.c:650-678` | Resolve a resource by absolute path or resource name using Workbench knowledge. |
| `GetPrefabChildInfo` | `ResourceInfo.c:682-755` | Inspect a selected child container with an explicit child-index path. |
| `GetGameMaterials` | `ResourceInfo.c:759-786` | Enumerate a constrained resource kind through `ResourceDatabase`. |
| `RegisterResource` | `ResourceInfo.c:1203-1274` | Registration/rebuild is a real side effect and must be surfaced as such. |
| `ExportMaterialResource` | `ResourceInfo.c:792-1007` | Resource/container mutation can be precise, but it writes and may register resources. |

These examples prove that the handler pattern can inspect and mutate
Workbench-managed assets. They do not establish a general safe edit API; some
existing handlers include broad paths and their own TODOs about project-boundary
checks. Our adapter must be stricter.

## Recommended plugin boundary

Create one optional Workbench-side plugin/handler package whose only public
surface is a versioned, typed command set owned by this project. It should:

- expose a `capabilities` handler with plugin/API version, loaded project
  identity, available operations, and maximum supported payload size;
- expose narrow read handlers first: resolved project/content roots, resource
  metadata, prefab/container inspection, and current World Editor selection;
- expose named action handlers only after their preview/result/undo contracts
  are specified; and
- return structured errors and resource IDs/paths, never prose intended for an
  agent to execute.

Keep TCP framing, retries, timeouts, and translation to MCP result objects in
the local MCP host. Keep engine calls, current editor state, resource database
resolution, and World Editor undo grouping inside Workbench. A handler must not
open a second listener, invoke arbitrary OS commands, accept arbitrary script,
or act as a raw endpoint proxy.

For World Editor mutation, the handler should take a domain command (for
example, `set_entity_visibility` for an explicit entity list), validate its
resource/project identity, start one named entity action, perform the change,
end the action in all paths, and return the resulting entity identities. The
official tutorial documents the begin/end pairing as the route into the editor's
Undo history ([World Editor plugin tutorial](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor_Plugin)).

## Validation backlog

1. Enable the NET API in a disposable Workbench project and identify the
   configured loopback endpoint/port without scanning unrelated interfaces.
2. Implement a small external probe that performs only `IsWorkbenchRunning`
   and records byte-level framing, connection close, latency, and errors.
3. Verify `ValidateScripts` on both a clean fixture and a deliberate compiler
   error; reconcile locations with project paths and the Rust language engine.
4. Add only a `capabilities` custom handler, reload scripts, and test missing,
   malformed, oversized, and unsupported-version requests.
5. Add one read handler for a known prefab, then one World Editor mutation and
   prove exactly one Undo restores it.
6. Repeat the probe against each supported Workbench/Reforger version and
   publish compatibility only for versions actually tested.

Until this backlog is complete, the general MCP server should present Workbench
features as optional/unavailable rather than falling back to guessed filesystem
semantics.
