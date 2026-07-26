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

The documentation does not establish authentication, TLS, concurrency,
request-size limits, or cancellation semantics. A live Workbench `1.7.0.54`
probe on 2026-07-23 confirmed the configured `127.0.0.1:5775` endpoint, the
documented framing, and the exact successful response error code `Ok`. The
extension still contacts only its user-configured loopback endpoint; it does
not discover or probe alternate ports.

## Placement and ownership

NET API is a local protocol between the MCP host's private adapter and an
external running Workbench process. It is neither MCP itself nor a public
listener owned by our MCP server. The custom plugin is code loaded inside
Workbench and is reached through NET API handler dispatch.

| Component | Runs where | Owns |
| --- | --- | --- |
| MCP client | Agent/editor client process | Tool selection and final consent UI. |
| Local MCP host | Extension-owned local process | Public MCP schemas/resources, file and language-engine adapters, tool policy, Workbench discovery, NET API transport/retries, capability cache, and MCP result mapping. |
| NET API adapter | Inside the local MCP host | Dedicated NET API codec and calls to the typed allowlist; never arbitrary endpoint proxying. |
| Reforger Workbench | Separate external editor process | Running editor, compiler, resource database, current world/editor state, and Undo history. |
| Project Workbench plugin | Loaded inside Workbench | Versioned handler DTOs and the engine-native resource/world/editor operations behind them. |

An operation therefore travels from a named MCP tool to the host policy, then
through the NET API adapter to a named plugin handler, and returns as a typed
DTO that the host maps into a structured MCP result. Workbench closure or a
missing/incompatible plugin disables only this route; it must not disable or
be emulated by the direct-file, language-engine, or evidence-catalogue routes.

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

These are the complete built-in endpoints established by the reviewed
`NetApiDocs.c`; they are not a general Workbench automation or scene-editing
surface. In particular, the built-ins do not establish generic entity search or
mutation, current selection inspection, resource search/metadata, asset
import/rebuild, tests, viewport screenshots/previews, generic logging, or an
arbitrary command/script execution endpoint. Those are either custom-plugin
opportunities requiring separate proof, or responsibilities of the direct-file
and language-engine path. A successful TCP connection must never be treated as
evidence that any of those richer capabilities exist.

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
  identity, a monotonic capability revision, named capability groups and
  operations, effect/read-only classifications, unavailable reasons, paging
  limits, and maximum supported payload size;
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

## Intended full plugin capability portfolio

The optional plugin is not limited to health checks, compiler validation, or
read-only asset inspection. Its intended destination is the Workbench half of a
full editor co-pilot: a typed capability layer through which MCP can inspect,
visually understand, and deliberately operate a live Reforger project. The
following groups should be reserved in the capability manifest now, even where
their operations remain future research and validation work:

| Capability group | Desired future operations | Contract requirements |
| --- | --- | --- |
| `project` | Active project/addon identity, mounts/content roots, editor state, logs, and context needed to relate files to Workbench resources. | Canonical identities; no broad filesystem escape. |
| `resource` | Search, resolve, inspect, preview, create, register, import, rebuild, repair, and validate supported prefabs, materials, textures, terrain, animations, and dependencies. | Typed resource kinds, bounded search/results, project containment, affected-resource report. |
| `world` | Inspect selection, find entities, read hierarchy/transforms/components, create/place/duplicate/configure/remove entities, and run domain operations such as composition or spawn configuration. | Stable entity IDs, preview, explicit consent, one named undo transaction, verification. |
| `editor` | Open/focus resources and modules, enter/leave relevant editor modes where supported, and expose an explicit current-editor context. | User-visible effect classification and actionable error results. |
| `compiler` | Query readiness, validate scripts, run explicitly selected build/test/autotest operations, and return normalized diagnostics/artifacts. | Explicit invocation/configuration, timeouts, structured locations and results. |
| `visual` | Capture a viewport screenshot; return asset thumbnails, prefab/material previews, and before/after captures for an operation. | Proven image transfer, dimensions/byte limits, capture source/time, no arbitrary file transfer. |
| `workflow` | Project-specific high-level tools built on the above groups, such as audit/repair flows or a named world-authoring operation. | Stable public name/description/schema; composed from approved domain commands, never arbitrary script execution. |

The group names are an architectural reservation, not a promise that an
unimplemented endpoint exists. A `capabilities` response must distinguish
`available`, `unavailable`, and `unsupported` operations with a reason. This
lets clients discover the full design vocabulary without treating a missing
plugin version as permission to guess or emulate editor behavior.

### External feature discovery

The following non-authoritative review consolidates two external projects:
`EnfusionMCP/EnfusionMCP` at `282393978cbe00c143f0872cf334c8432741c8e4`, and
[`steffenbk/enfusion-mcp-BK`](https://github.com/steffenbk/enfusion-mcp-BK) at
[`3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739`](https://github.com/steffenbk/enfusion-mcp-BK/commit/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739),
reviewed 2026-07-25 and 2026-07-26 respectively. They discover possible
capability groups; they do not establish Reforger behaviour, implementation
quality, or an adoption decision.

`enfusion-mcp-BK` separates a local Node MCP host from Enforce Script handlers
injected into a mod. Its host features are useful scope comparisons, but only
the last row belongs at the Workbench NET API boundary:

| External local-host feature group | Potential use here | NET API disposition |
| --- | --- | --- |
| API/wiki/knowledge and base-game inspection | Evidence retrieval and Game Data browsing | Existing local evidence authority; not a Workbench handler. |
| Project browse/read/write | Bounded workspace orientation or staged edits | Separate project-file boundary; never a handler proxy. |
| Addon, script, prefab, layout, config, scenario, animation, and server-config generation | Intent-level authoring workflows | Separate previewable file-creation workflow; not a NET API capability by itself. |
| Local validation/build and Workshop metadata | Explicit local development actions | Keep process invocation and metadata outside the Workbench handler surface. |
| Guided prompts and MCP resources | Client-facing guidance | MCP-host concern, not Workbench state. |
| Workbench launch/connect/diagnose/reload/cleanup | Optional live-editor integration | The typed Gateway may contact the configured endpoint; it must not install handlers, start Workbench, or manage arbitrary process lifecycle. |

The same review found 19 `EMCP_WB_*` injected handlers. Their source-backed
ideas are potential NET API capability candidates only after a versioned DTO,
canonical identities, bounded responses, and live acceptance. The stated
dispositions are deliberately conservative:

| External handler | Source-implemented feature | Potential typed capability / disposition |
| --- | --- | --- |
| `EMCP_WB_Ping` | Bridge presence and `WorldEditorAPI` availability. | Feed `capabilities`/availability only; its cached mode is unreliable. |
| `EMCP_WB_GetState` | Edit-mode entity/selection counts, subscene, prefab-edit state, terrain bounds. | Candidate for a bounded state summary after authoritative fields are defined. |
| `EMCP_WB_EditorControl` | Play mode, save, undo/redo, and resource opening. | Split into named editor commands; never use its hard-coded menu paths. |
| `EMCP_WB_ExecuteAction` | World Editor menu-path invocation. | Exclude: UI-label dispatch has no allowlist, confirmation, or completion proof. |
| `EMCP_WB_Reload` | Script compilation and plugin reload through menus. | Consider only a proven named operation with a verified result. |
| `EMCP_WB_Resources` | Register, rebuild, or open one resource. | Candidate for typed resource-lifecycle operations. |
| `EMCP_WB_ScriptEditor` | Individual open-document line reads/edits. | Prefer bounded project-file tools plus compiler validation; do not mirror editor lines by default. |
| `EMCP_WB_Localization` | Table-item/property edits and counts. | Future typed localization/resource capability. |
| `EMCP_WB_Terrain` | Surface height and terrain bounds. | Future read-only world query when a real workflow requires it. |
| `EMCP_WB_ListEntities` | Paged entities with name, class, and position. | High-value read candidate, but require stable entity IDs and filters. |
| `EMCP_WB_GetEntity` | Entity components, layer/subscene, and variables. | High-value read candidate, but use hierarchy IDs and actual override values. |
| `EMCP_WB_CreateEntity` | Root entity creation from a prefab. | Future mutation with preview, one Undo transaction, and verification. |
| `EMCP_WB_DeleteEntity` | Deletes the first exact-name entity. | Future mutation only with stable identity, confirmation, Undo, and verification. |
| `EMCP_WB_ModifyEntity` | Transform, rename, reparent, and generic property edits. | Split into narrow domain commands; exclude generic stringly typed editing. |
| `EMCP_WB_Components` | List, add, and remove components. | Future typed entity capability with component/resource validation. |
| `EMCP_WB_SelectEntity` | Selection list and mutations. | Start with a read summary; mutate only through a proven stable selection API. |
| `EMCP_WB_Clipboard` | Copy, cut, duplicate, and paste selected entities. | Exclude clipboard-state tools; prefer explicit composition/duplication commands. |
| `EMCP_WB_Layers` | Numeric layers, current subscene, and entity layer ID. | Future only after stable layer identities and operations are proven. |
| `EMCP_WB_Prefabs` | Template creation/save and ancestor resource path. | Candidate for typed prefab/resource inspection and lifecycle operations. |

The reviewed clients sometimes treated a successful TCP transaction as success
despite a handler-level error. They also show schema mismatches, name-based
entity targeting, guessed menu paths, and unsupported wrapper claims. Do not
copy their generic handler dispatch, menu fallbacks, unchecked success mapping,
or process/handler installation behaviour. Every candidate above still requires
primary evidence, a versioned DTO, canonical identity, bounds, and live
acceptance.

Each future operation should also carry the parts of the contract that make an
editor MCP usable by an agent: stable name and description; effect
classification; input and output DTO versions; declared default/max paging or
image limits; stable entity/resource identities; and an error/recovery shape.
For operations that access live editor state, the plugin performs the engine
call in its Workbench context and the host only transports/maps the result.
This is how the s&box-style strengths—live tool availability, rich typed
results, visual feedback, and direct editor control—translate safely across
Reforger's external NET API boundary.

For future retrieval, the manifest groups map to the product design index as
follows: `world` covers **live scene editing**; `resource` covers **asset
management**; `editor` and `compiler` cover **editor control**; `visual` covers
**visual feedback**; `workflow` covers **custom tools**; versioned DTOs and
stable IDs provide **rich typing**; and the capability manifest, descriptions,
limits, effect classifications, and recovery errors provide **AI-friendly
design**. These categories are intentional long-term scope, including when a
given operation is currently unavailable.

## Validation backlog

1. Repeat the confirmed `127.0.0.1:5775` byte-level probe on every supported
   Workbench version and record latency, connection-close behavior, and errors.
2. Verify `ValidateScripts` on a clean fixture and a warning fixture. A live
   deliberate-error fixture already confirmed `Ok`, `Success: false`, four
   errors, and one-based unpacked-addon source lines.
3. Reconcile clean, warning, packed-resource, and cross-addon locations with
   project paths and the Rust language engine.
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
| Built-in status succeeds | `$(plug) Workbench Connected` | Built-in `validate_scripts`; show `ScriptsCompiled` separately as compiler state. |
| Custom capabilities succeed | `$(plug) Reforger Workbench ready` | Built-in `validate_scripts` plus only handlers named in the capability response. |
| Connection lost | `$(circle-slash) Reforger Workbench unavailable — retrying` | None until re-established. |

The documented `IsWorkbenchRunning` response supplies both `IsRunning` and
`ScriptsCompiled`. A successfully decoded response proves that the configured
Workbench API is connected. Live Workbench also reports
`ScriptsCompiled: false` after a completed compiler failure, so that field is
compiler state rather than an availability gate. The built-in
`ValidateScripts` capability remains available without a project plugin. Call
the project's custom `capabilities` handler separately and cache only that
connection's typed plugin allowlist. This ensures a Workbench instance with an
absent, stale, or incompatible plugin still supports the proven compiler route
but does not expose speculative custom MCP tools.

Start with one immediate probe, retry once per second while unavailable, and
use a five-second heartbeat while ready. Do not poll every 500 ms: each NET API
transaction creates a TCP connection, and two connection attempts per second
has no meaningful user-facing advantage. On a failed heartbeat, transition back
to one-second discovery. Log state transitions and a sanitized failure category,
not every failed retry; show no recurring warning popup. A status-bar command
may offer immediate reconnect and reveal the last error/category in a tooltip.

`ValidateScripts` is compiler-backed verification, not a health check. Offer it
as a named command/tool with a declared configuration (`WORKBENCH`, `PC`, and
so on) and return structured diagnostics. Do not run it unconditionally at
activation, after every reconnect, or for every save. A user-controlled
automatic mode may schedule one debounced, coalesced validation after a saved
edit burst; it must not validate an unsaved buffer or block a save. The detailed
contract, trigger modes, diagnostics freshness policy, and live acceptance
experiments are in [Workbench compiler-validation research](workbench-compiler-validation-research.md).

When custom capability negotiation first succeeds, the host should cache the
capability manifest and its revision. Re-read it after a reconnect and whenever
a Workbench/plugin reload changes the revision; remove unavailable operations
from the host's effective allowlist immediately. This is the NET API equivalent
of a live editor tool catalogue: it prevents a stale MCP session from calling a
handler whose scripts were unloaded or whose contract changed. The host may
surface this catalogue through its read-only `search_tools` and
`describe_toolset` tools, but it must never turn it into arbitrary handler
dispatch.

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
| Workbench/compiler health and script validation | `NetApiDocs.c` | Initial built-in capability. Use status for readiness; validation is manual by default and may use an explicitly enabled saved-idle policy. |
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
   Our DTOs should add one shared envelope: `ok`, typed error code/message and
   recovery hint, API/plugin/capability revision, operation ID, and affected
   resource identities. The MCP host maps those DTOs into named MCP structured
   results; it must not flatten them into agent-directed prose.
2. Result limits are real. `PrefabImporter.c` limits/paginates hierarchy output
   and `ResourceInfo.c` comments on large prefab JSON exceeding transport
   limits. Every collection endpoint needs a limit, cursor, and declared
   maximum response size from its first version.

### Recommended custom plugin v1

Expose only these typed handlers in the first custom plugin: `capabilities`,
`project_context`, `resolve_resource`, `inspect_resource`,
`inspect_prefab_child`, `list_resources` with constrained type/filter/cursor,
`world_selection_summary`, and `validate_scripts`. The `capabilities` response
groups them under stable names such as `resource`, `world`, and `compiler`,
and declares each handler's input/output DTO version and effect
classification. The MCP host maps them to small named MCP tools, retains
transport/retry/status-bar ownership, and uses the manifest for progressive
tool discovery rather than exposing every possible operation up front.

Keep v1 JSON-only and bounded. A future viewport/screenshot tool may be useful
for visual world or resource inspection, but neither the reviewed NET API nor
the Blender sample proves a safe image-transfer contract. Research an explicit
size-limited image DTO or host-managed temporary artifact with project-safe
cleanup before advertising MCP image content.

Do not expose arbitrary handler dispatch. Put any future mutator behind a
separate domain endpoint with canonical project containment, resource/type
validation, a dry-run response, explicit confirmation at the MCP client,
atomic Workbench/World Editor transaction or undo group where supported, and a
post-operation verification result. This keeps the useful engine authority
while avoiding the breadth and implicit authority of the Blender example.

## Accepted first extension framework (2026-07-23)

The first delivered consumer is the VS Code extension, not an MCP host. It
hosts a reusable, host-neutral Workbench Gateway for exactly the built-in
`IsWorkbenchRunning` and `ValidateScripts` capabilities; it does not add a
custom handler, capability manifest, MCP server, or generic handler dispatch.
Any MCP Workbench adapter must consume this same Gateway boundary.

For this initial extension contract, endpoint selection is explicitly owned by
VS Code settings rather than automatic discovery: NET API enablement defaults
on, the loopback-only host defaults to `127.0.0.1`, and the port defaults to
`5775`. The Gateway contacts only the configured endpoint and never discovers,
scans, changes, or repairs it. Its normal status requests remain necessary to
assess whether that exact configured endpoint is connected; they are not
endpoint discovery.

The compiler-validation-specific scheduling, diagnostic provenance, and live
acceptance contract are owned by
[Workbench compiler-validation research](workbench-compiler-validation-research.md).
