# EnfusionMCP Workbench handler review

This is an external source review, not Reforger implementation authority. It
records `EnfusionMCP/EnfusionMCP` at commit
`282393978cbe00c143f0872cf334c8432741c8e4`, reviewed on 2026-07-25.

The project injects 19 `NetApiHandler` scripts into a target mod and reaches
them through Workbench's TCP NET API. It is not a registered
`WorkbenchPlugin` or `WorkbenchTool`. Its TypeScript tests mock TCP; they do
not prove the scripts compile or work in a live Workbench session.

## enfusion-mcp-BK feature inventory

This separate snapshot records the public `main` branch of
[`steffenbk/enfusion-mcp-BK`](https://github.com/steffenbk/enfusion-mcp-BK)
at commit
[`3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739`](https://github.com/steffenbk/enfusion-mcp-BK/commit/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739),
reviewed on 2026-07-26. It distinguishes the local Node MCP host from the
Enforce Script handlers it injects into a mod. This is feature discovery from
external source, not Reforger implementation authority or an adoption list.

### Local MCP host features

The host is a separate Node process using MCP stdio. It owns tool registration,
local data/file access, generated artifacts, and Workbench process/TCP
orchestration. The entries below are host capabilities; they do not by
themselves run inside Workbench.

| Host feature group | Exposed tools or behavior | Effect boundary |
| --- | --- | --- |
| API and documentation research | `api_search`, `component_search`, `wiki_search`, `wiki_read`, and `wb_knowledge` search its packaged indexes, wiki corpus, and knowledge base. | Read-only local packaged data; source freshness and authority are external-project concerns. |
| Base-game inspection | `game_browse`, `game_read`, `asset_search`, and `prefab_inspect` inspect loose game files and `.pak` archives. | Read-only game-data access. |
| Project-file access | `project_browse`/`project_read` and `project_write` read and write the configured project directory. | Direct filesystem mutation is possible through the write tool. |
| Addon and artifact generation | `mod_create`, `script_create`, `prefab_create`, `layout_create`, `config_create`, `scenario_create`, `animation_graph`, and `server_config` generate or scaffold mod content. | Creates or overwrites project files. |
| Local project checks and build | `mod_validate`, `mod_build`, and `workshop_info` validate project content, invoke the Workbench build executable, or read `.gproj` metadata. | Build starts an external local process; validation/metadata reading are local. |
| Guided workflows and MCP resources | `/create-mod`, `/modify-mod`, and `enfusion://class`, `enfusion://pattern`, and `enfusion://group` provide workflow prompts and read-only resources. | No Workbench connection required. |
| Workbench bridge lifecycle | `wb_launch`, `wb_connect`, `wb_diagnose`, `wb_reload`, and `wb_cleanup` install/remove handlers, start Workbench, and manage its NET API connection. | Writes/removes handler scripts in the target mod and starts or contacts a separate Workbench process. |

The package configuration defaults its Workbench connection to
`127.0.0.1:5775`, while allowing the host and port to be configured. Its
optional remote scraper fetches API documentation from a third-party Doxygen
mirror during index generation; ordinary MCP stdio serving is local.

### Injected Workbench-handler features

The following is the separate feature set implemented by the `EMCP_WB_*`
Enforce Script handlers. The local MCP host wraps these handlers as `wb_*`
tools, but Workbench owns their execution and live-editor facts.

## Design lessons

The source demonstrates that custom handlers can cover useful project,
resource, world, editor, and compiler-adjacent workflows. It also demonstrates
why our MCP surface must use narrow versioned DTOs and verified outcomes.

The reviewed client often treats a successful TCP transaction as success even
when the handler JSON has `status: "error"`. It also contains schema mismatches,
name-based entity targeting, guessed menu paths, and unverified built-ins.

Do not copy its generic capability claims, raw handler dispatch, menu-label
fallbacks, or unchecked success mapping. Use this record only to discover
candidate capability groups for the typed Gateway and handler package.

## Workbench-handler inventory

| Handler | Request fields | Source-implemented feature | Material limit or mismatch |
| --- | --- | --- | --- |
| `EMCP_WB_Ping` | none | Reports bridge presence and `WorldEditorAPI` availability. | Returns `game`, while its client recognizes `play`; cached mode is unreliable. |
| `EMCP_WB_GetState` | none | Edit-mode entity/selection counts, up to 50 selected names, subscene, prefab-edit flag, and terrain bounds. | Game mode returns only `game` and optional bounds; no project, resource, runtime-game, or active-layer state. |
| `EMCP_WB_EditorControl` | `action`, `debugMode`, `fullScreen`, `path` | Starts/stops play mode; saves; runs undo/redo; opens a resource. | `saveAs` ignores path and calls `Save`; Undo/redo use hard-coded menu paths; resource opening is not project loading. |
| `EMCP_WB_ExecuteAction` | `menuPath` | Invokes a comma-separated World Editor menu path. | UI-label/version-dependent dispatch; no allowlist, confirmation, or reliable completion fact. |
| `EMCP_WB_Reload` | `target` | Attempts script compilation and plugin reload through menu paths. | Best effort only; no compiler diagnostics or proof that reload/compile completed. |
| `EMCP_WB_Resources` | `action`, `path`, `buildRuntime` | Registers, rebuilds, or opens one resource. | Rebuild is asynchronous and unverified; no resource graph, metadata, or whole-database operation. |
| `EMCP_WB_ScriptEditor` | `action`, `line`, `text`, `path` | Reads/edits individual open-document lines and opens a resource. | No whole-file read/save/diagnostics; response names differ from wrapper expectations. |
| `EMCP_WB_Localization` | `action`, `itemId`, `property`, `value` | Inserts/deletes an item, modifies one property, or counts table children. | Insert ignores `property` and `value`; table read returns a count, not entries. |
| `EMCP_WB_Terrain` | `action`, `x`, `z` | Reads surface Y or terrain bounds. | Read-only; no terrain edit, material/layer query, water, road, or heightmap workflow. |
| `EMCP_WB_ListEntities` | `offset`, `limit`, `nameFilter` | Paged entity list with name, class, and runtime position. | Linear scan; no stable identity, prefab, hierarchy, GUID, or actual property values. |
| `EMCP_WB_GetEntity` | `name` or `index` | Basic entity data, components, layer/subscene, and up to 50 variables. | First duplicate exact-name match; `GetDefaultAsString` can return defaults rather than overrides. |
| `EMCP_WB_CreateEntity` | `prefab`, `position`, `rotation`, `name`, `layerID` | Creates a root entity from a prefab. | Defaults to layer `0`; no parent/subscene/layer path; wrapper sends ignored `layerPath`. |
| `EMCP_WB_DeleteEntity` | `name` | Deletes the first exact-name entity in an editor action. | No stable-ID targeting, batch behavior, dependency safety, or confirmation. |
| `EMCP_WB_ModifyEntity` | `name`, `action`, `value`, `propertyPath`, `propertyKey`, `memberIndex` | Transforms, renames, reparents, edits properties/object arrays, and changes object class. | Stringly typed targeting; assumes `coords`/`angleX/Y/Z`; nested/current values are partly observable. |
| `EMCP_WB_Components` | `entityName`, `action`, `componentClass`, `componentIndex` | Lists, adds, and removes components. | Name targeting; remove-by-class removes first match; wrapper blocks index-only removal. |
| `EMCP_WB_SelectEntity` | `action`, `name` | Deselects, clears, and lists up to 100 selected entities. | `select` clears selection but does not select the target; wrapper claims it did. |
| `EMCP_WB_Clipboard` | `action` | Copies/cuts/duplicates selected entities; pastes or checks clipboard. | Depends on GUI state; no affected-entity list or deterministic paste target. |
| `EMCP_WB_Layers` | `action`, `subScene`, `entityName`, `visible` | Lists numeric layer IDs, current subscene, and an entity layer ID. | Cannot mutate layers; wrapper advertises unsupported operations. |
| `EMCP_WB_Prefabs` | `action`, `entityName`, `templatePath` | Creates/saves templates and gets a direct ancestor resource path. | No prefab search/GUID/override inspection; wrapper does not expose `getAncestor`. |

## Reforger implications

Our plugin should identify entities and resources canonically, declare
capabilities and versions, bound result sizes, return typed errors, and verify
every mutation. A named domain operation replaces a stringly typed generic one.

World mutation needs a native Undo transaction, a post-action result, and live
acceptance on supported Workbench versions. Feature claims remain unavailable
until those facts are established by the Workbench evidence hierarchy.
