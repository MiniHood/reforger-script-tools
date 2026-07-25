# EnfusionMCP Workbench handler review

This is an external source review, not Reforger implementation authority. It
records `EnfusionMCP/EnfusionMCP` at commit
`282393978cbe00c143f0872cf334c8432741c8e4`, reviewed on 2026-07-25.

The project injects 19 `NetApiHandler` scripts into a target mod and reaches
them through Workbench's TCP NET API. It is not a registered
`WorkbenchPlugin` or `WorkbenchTool`. Its TypeScript tests mock TCP; they do
not prove the scripts compile or work in a live Workbench session.

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

## Handler inventory

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
