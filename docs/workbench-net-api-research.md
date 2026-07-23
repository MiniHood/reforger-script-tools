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

## Recommended extension connection lifecycle

Workbench availability is an optional capability, never a prerequisite for
extension activation, game-data acquisition, or language-engine startup. The
extension should begin discovery only after the language engine has published
its initial external game-data index. That requires a specific server-ready
signal in a future implementation; it must not infer readiness merely because
the server process was spawned.

Use a lower-right status-bar item instead of a perpetual VS Code progress
notification. A notification is appropriate for bounded work such as the
existing game-data download; discovering an independently started local
Workbench is ongoing availability state. Suggested wording and state are:

| State | Status-bar text | Enabled capability |
| --- | --- | --- |
| Initial index pending | No Workbench status yet. | None. |
| Discovering | `$(sync~spin) Checking for Reforger Workbench…` | None. |
| Socket reached; Workbench not ready | `$(sync~spin) Reforger Workbench is starting…` | None. |
| `ScriptsCompiled` is false | `$(sync~spin) Reforger Workbench is compiling scripts…` | Read-only health only. |
| Health plus custom capabilities succeed | `$(plug) Reforger Workbench ready` | Only handlers named in the capability response. |
| Connection lost | `$(circle-slash) Reforger Workbench unavailable — retrying` | None until re-established. |

The documented `IsWorkbenchRunning` response supplies both `IsRunning` and
`ScriptsCompiled`; use those facts rather than a successful TCP connection as
the readiness decision. After it reports ready, call the project's custom
`capabilities` handler once and cache only that connection's typed allowlist.
This ensures a Workbench instance with an absent, stale, or incompatible plugin
does not expose speculative MCP tools.

Start with one immediate probe, retry once per second while unavailable, and
use a five-second heartbeat while ready. Do not poll every 500 ms: each NET API
transaction creates a TCP connection, and two connection attempts per second
has no meaningful user-facing advantage. On a failed heartbeat, transition back
to one-second discovery. Log state transitions and a sanitized failure category,
not every failed retry; show no recurring warning popup. A status-bar command
may offer immediate reconnect and reveal the last error/category in a tooltip.

`ValidateScripts` is compiler-backed verification, not a health check. Offer it
as an explicit MCP tool/command with a declared configuration (`WORKBENCH`,
`PC`, and so on), return structured diagnostics, and require normal mutation/
operation consent. Do not run it automatically at activation, after every
reconnect, or on every save: it can be expensive and changes the meaning of an
availability indicator into an unsolicited build.

## Complete installed-source review — 2026-07-23

This review fully read every file, without content-search shortcuts, under the
installed extension data roots supplied for this investigation:

- `WorkbenchGameCommon` (16 files outside the Blender folder); and
- `WorkbenchGameCommon/EnfusionBlenderTools` (22 files, including
  `BlenderAPI`).

It is evidence from the installed extracted data, not proof that every handler
is registered or enabled in a particular live Workbench version. File paths
below are relative to `scripts/WorkbenchGameCommon` in that installed data.

### Files reviewed

| Area | Files fully read |
| --- | --- |
| Protocol/base | `NetApiDocs.c`; `generated/NetApi/NetApiHandler.c`; `generated/TxaExporter.c` |
| Resource/import | `ResourceInfo.c`; `TextureImportTool.c`; `ValidateFBXPlugin.c`; `ValidateMaterialPlugin.c` |
| Workbench/editor plugins | `PeerConfig.c`; `PeerTool.c`; `WorkbenchDialogs.c`; `WorldExporterPlugin.c` |
| Localization | `LocalizationEditor/CheckLocalizationPlugin.c`; `LocalizationEditor/TranslationPlugin.c`; `LocalizationEditor/TranslationPlugin/TranslationPluginMatchConfig.c`; `TranslationPluginRequest.c`; `TranslationPluginResponse.c` |
| Blender integration | `EnfusionBlenderTools/AnimExport.c`; `AnimExportProfiles.c`; `AssetLibraryUtils.c`; `BakeMLOD.c`; `CallBlenderFunction.c`; `CheckGUID.c`; `EBT_HTTPRequest.c`; `EBTConfig.c`; `EBTEmatUtils.c`; `EBTResponse.c`; `ExportTerrain.c`; `GetPortalMat.c`; `LayerPresets.c`; `LoadedProjects.c`; `OpenXOB.c`; `PrefabImporter.c`; `PrefabImporterBake.c`; `SendToBlender.c`; `TerrainToBlender.c`; `TextureValidation.c`; `BlenderAPI/BlenderEndpoints.c`; `BlenderAPI/BlenderRestAPI.c` |

### Confirmed API model

`NetApiDocs.c` defines a client-initiated, one-transaction-per-TCP-connection
protocol. A request contains protocol version, client ID, `JsonRPC` content
type, and an `APIFunc` JSON object; the response is an error-code string plus a
JSON payload. `NetApiHandler.c` documents dispatch by derived class name and
the `GetRequest` → deserialize → `GetResponse` lifecycle. The handler examples
consistently use a `JsonApiStruct` request/response with explicitly registered
fields; `AnimExport.c`, `PrefabImporterBake.c`, and the translation request
classes also show custom array/object serialization where needed.

The built-in endpoints remain the narrow, dependable base: status, World Editor
status, opening a resource, focusing a module, and `ValidateScripts`. The
review establishes a much broader *custom-handler* potential, but no generic
handler catalogue or handler-discovery endpoint is supplied by the platform.
Our plugin must therefore publish its own versioned `capabilities` response.

### Capability inventory

| Capability | Evidence in installed source | MCP recommendation |
| --- | --- | --- |
| Workbench/compiler health and script validation | `NetApiDocs.c` | Initial capability. Use status for readiness and `ValidateScripts` only on explicit request. |
| Resolve a resource, inspect its class/container/metadata, inspect prefab children, list game materials | `ResourceInfo.c` | Initial read-only capability, but bound recursive expansion and result size. |
| Loaded projects, addon/path ownership, prefab discovery, GUID/resource-ID/path mapping | `EnfusionBlenderTools/AssetLibraryUtils.c`, `LoadedProjects.c`, `CheckGUID.c`, `EBTEmatUtils.c` | Initial read-only capability after canonical project-root validation. |
| Query physics-layer presets and membership | `EnfusionBlenderTools/LayerPresets.c` | Suitable read-only capability. |
| Inspect portal materials, material assignments, and texture/material GUID issues | `GetPortalMat.c`, `TextureValidation.c`, `ValidateMaterialPlugin.c` | Good diagnostic/inspection capability; return structured findings rather than the sample's parallel prose/severity arrays. |
| Enumerate/export prefab hierarchy, transforms, FBX paths, sockets, and material overrides | `PrefabImporter.c`, `PrefabImporterBake.c` | Candidate read capability after pagination. The sample itself caps responses, demonstrating that output-size limits are required. |
| Read animation export profiles/channel metadata | `AnimExportProfiles.c`, `generated/TxaExporter.c` | Candidate read capability. |
| Texture-import policy and batch diagnostics/fixes | `TextureImportTool.c`, `ResourceInfo.c` | Read-only audit is useful; any repair/rebuild must be a separate previewable operation. |
| Open/focus Workbench resources/modules | `NetApiDocs.c`, `OpenXOB.c` | Explicit user-visible navigation tool. |
| World selection/entity/prefab/terrain operation | `WorldExporterPlugin.c`, `ExportTerrain.c`, `BakeMLOD.c` | Later only: narrowly named action, preview, confirmation, and a verified World Editor undo transaction. |
| Import/register/rebuild textures, FBX, materials and prefabs | `ResourceInfo.c`, `TextureImportTool.c`, `PrefabImporterBake.c`, `BakeMLOD.c` | Later deterministic-write capability. Return affected paths/resource IDs and a rebuild outcome. |
| Animation export to files | `AnimExport.c`, `generated/TxaExporter.c` | Later file-writing capability; do not expose raw target-path/file-data controls initially. |
| Launch Blender, start its HTTP bridge, or invoke arbitrary operators/processes | `CallBlenderFunction.c`, `SendToBlender.c`, `BlenderAPI/*`, `ValidateFBXPlugin.c` | Exclude from the initial MCP surface. These are external-process/network bridges, not Workbench facts. |
| Translation HTTP service and localization edits | `LocalizationEditor/TranslationPlugin*.c` | Exclude from this Workbench MCP; it is an unrelated external web-service workflow. Its request validation and explicit confirmation are useful design examples. |
| Peer-client process launch/control | `PeerConfig.c`, `PeerTool.c` | Exclude. It constructs and starts external processes. |
| Map export/copy/temporary-file cleanup | `WorldExporterPlugin.c` | Exclude initially. It performs broad entity mutations, writes/copies resources, saves the world, and deletes files. |

### Design lessons and safety findings

The Blender integration is valuable proof that Workbench can resolve projects,
resources, prefabs, materials, terrain and animation data through typed custom
handlers. It is not a safe MCP design to reuse unchanged. Its handlers use
inconsistent response shapes (`status`, `Output`, `Result`, null, or bespoke
objects), accept broad absolute paths, and mix inspection with registration,
resource rebuilding, world modification, file copying, dialogs, process launch,
or HTTP calls.

In particular, `ResourceInfo.c` has an explicit TODO that texture copying does
not ensure the destination is inside the project. `AssetLibraryUtils.c` uses a
substring-style project-path test rather than canonical containment.
`ExportTerrain.c`, `BakeMLOD.c`, and `WorldExporterPlugin.c` can make material,
terrain, prefab, entity, resource, world-save, copy, or delete changes without
the MCP-facing preview/confirmation contract. `CallBlenderFunction.c` composes
process command lines; `BlenderRestAPI.c` starts a localhost service. None
should become a raw `call_workbench_handler`, arbitrary path, shell, process,
or HTTP MCP tool.

The source also shows two reusable positive patterns:

1. Small request/response DTOs are the right Workbench-plugin boundary.
   Our DTOs should add one shared envelope: `ok`, typed error code/message,
   API/plugin version, operation ID, and affected resource identities.
2. Result limits are real. `PrefabImporter.c` limits/paginates hierarchy output
   and `ResourceInfo.c` comments on large prefab JSON exceeding transport
   limits. Every collection endpoint needs a limit, cursor, and declared
   maximum response size from its first version.

### Recommended custom plugin v1

Expose only these typed handlers in the first custom plugin: `capabilities`,
`project_context`, `resolve_resource`, `inspect_resource`,
`inspect_prefab_child`, `list_resources` with constrained type/filter/cursor,
`world_selection_summary`, and `validate_scripts`. The MCP host maps them to
small named MCP tools and retains transport/retry/status-bar ownership.

Do not expose arbitrary handler dispatch. Put any future mutator behind a
separate domain endpoint with canonical project containment, resource/type
validation, a dry-run response, explicit confirmation at the MCP client,
atomic Workbench/World Editor transaction or undo group where supported, and a
post-operation verification result. This keeps the useful engine authority
while avoiding the breadth and implicit authority of the Blender example.
