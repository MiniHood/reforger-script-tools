# ArmoryForger MCP useful-feature backlog

Research date: 2026-07-28.

This is a prioritised capability backlog, not a promise to add every item. It
compares the shipped **Reforger Script Tools / ArmoryForger** MCP surface with
the current public `enfusion-mcp-BK` repository, official Reforger material,
and the existing s&box source review. It does not change the ownership model:
the Rust language engine owns language facts, Workbench owns live facts, and
the MCP host exposes small, typed operations rather than a command or file
proxy. See [MCP server research](mcp-server-research.md) for that contract.

## Baseline and conclusion

The local API has 56 published typed tools as of this research date
([generated catalogue](../mcp-api.md)). It already covers Game Data and
Official Wiki evidence, compiler validation, Workbench lifecycle, resource
inspection/listing, terrain/trace/viewport queries, selection/entity/component
inspection and editing, prefab editing, and play start/stop. Therefore the
next valuable features are not another generic `wb_entity_modify` equivalent.

The highest-value missing loop is:

```text
find the right project fact -> make one reversible change -> save/build ->
inspect diagnostics or a visual result -> undo or refine
```

The first four candidates below complete that loop. They should be delivered
as individual named operations, with project-bound opaque identities, explicit
effect annotations, bounded output, and post-action facts. No candidate
justifies raw handler invocation, menu-path dispatch, arbitrary console/script
execution, arbitrary file access, or dynamic reflection-based tool exposure.

## Evidence comparison

| Source | Useful lesson | What not to copy |
| --- | --- | --- |
| Local catalogue | Current direct entity/prefab/world operations make basic scene CRUD a solved problem; remaining operations must add a new authoritative fact or a whole verification step. | A second route to the same mutation. |
| Official World Editor guide | Save, undo, redo, scale, resource browsing, prefab-library metadata, World Editor play options, and the log console are real user workflows. [World Editor](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor) | UI shortcut simulation or menu labels as an API. |
| Official Resource Manager plugin tutorial | Resource Manager exposes resource registration/rebuild and metadata/selection to plugins. [Resource Manager Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager_Plugin) | Unbounded paths or a generic resource-manager passthrough. |
| Official WorldEditorAPI material | Prefab writes must use `WorldEditorAPI`, have begin/end entity-action boundaries, and save explicitly. [WorldEditorAPI Usage](https://community.bistudio.com/wiki/Arma_Reforger:WorldEditorAPI_Usage) | Direct `BaseContainer.Set*` writes or mutations outside an editor transaction. |
| `enfusion-mcp-BK` | Its current README has resource registration/rebuild, save/undo/redo, localization, project operations, material/texture validation, and many broad editor calls. [README](https://github.com/steffenbk/enfusion-mcp-BK/blob/main/README.md#tools) | Its broad `wb_execute_action`, name-based deletion, clipboard, editor-line writes, and generic stringly modifications; they do not meet this repo's safety contract. |
| s&box source review | Asset dependencies, asset compilation, diagnostics, visual thumbnails/screenshots, save/undo/redo, strong IDs, prevalidation, and batch operations close useful AI loops. [s&box review](sbox-mcp-research.md) | Its dynamic `call_tool` and in-editor HTTP/reflection architecture; static Rust MCP contracts remain the correct local boundary. |

The official documentation corpus also shows real high-value workflows not
represented by a generic entity API: incremental navmesh generation can take
substantial time and must be scoped to a changed area
([Navmesh tutorial](../../data/official-wiki/Modding/Tutorials/Navmesh%20Tutorial.md)),
and Game Master preview generation uses a prepared world to render selected
prefabs ([Image-generation tutorial](../../data/official-wiki/Modding/Game%20Master/Tutorials/Game%20Master%20Image%20Generation%20Tutorial.md)).
Those are useful later, but require a more specific evidence and cancellation
contract than the first slices.

## Prioritised backlog

| Priority | Candidate public capability | AI value and minimum contract | Evidence / prerequisite |
| --- | --- | --- | --- |
| P0 | `workbench_save_world`, `workbench_undo`, `workbench_redo` | Complete the existing entity/prefab mutation loop. Each returns before/after dirty or revision facts where obtainable; undo/redo report whether an operation was actually available. Never synthesize keyboard input. | Official World Editor lists all three operations. Prove native handler methods and one live acceptance for each; save must target the current resolved world only. |
| P0 | `workbench_register_resources` and `workbench_rebuild_resources` | Let the AI move a bounded, identified changed-resource set from source files into Workbench's resource database, then return per-resource outcomes and diagnostics. `rebuild` must be separately named, cancellable, and explicitly effectful. | The official Resource Manager API says plugins can register and rebuild resources; existing extracted handler evidence records `RegisterResource` ([NET API research](workbench-net-api-research.md)). Require containment, typed resource IDs, limits, and no arbitrary paths. |
| P0 | Project/workspace semantic queries: `search_workspace_symbols`, `inspect_workspace_symbol`, `query_workspace_symbol_relationships`, and later `find_workspace_references` | Gives the AI the same authoritative answer for the addon it is editing that it already gets for base-game symbols: definition, member/override/caller facts, exact source ranges, revision-bound opaque handoffs. This is much more useful than asking it to repeatedly read whole files. | Extend the existing language-engine Game Data query model, not Workbench. [Base-game search research](base-game-search-research.md) already requires one shared Rust query owner and distinguishes semantic from text search. Prove workspace indexing correctness before exposing mutations that rely on it. |
| P0 | Unified bounded diagnostics result and a `workbench_build_addon` only if a documented native build invocation exists | An agent needs a single post-change answer: script diagnostics plus resource/import/build diagnostics, phase, severity, logical locations, and a continuation cursor. Make build a named project-bound operation, not shell/CLI access. | `workbench_validate_scripts` already covers native script validation. s&box's compile-status/console pattern shows the loop; MCPBK's build claim is only a third-party candidate. First establish the official Workbench command/API and live result semantics. |
| P1 | `workbench_resource_dependencies` / `workbench_resource_dependants` and `workbench_resource_metadata` | Before changing a prefab, material, config, or texture, show bounded direct/transitive impact, provenance, GUID/meta identity, and whether it is imported/compiled. This enables safe impact review and targeted rebuilds. | Resource Manager's `GetMetaFile` and registered-resource model are official. s&box's `asset_dependencies` is a useful UX precedent, not Enfusion evidence. Start direct-only, resource-type-filtered, and paginated. |
| P1 | Typed localization-table inspection and mutation | List tables/keys/locales, detect missing targets and duplicate keys, then upsert one key with expected content revision and a post-write readback. This is a concrete common mod workflow, unlike generic text editing. | Official localisation instructions describe `.st` resources, locale targets, and Workbench registration ([Mod Localisation](../../data/official-wiki/Modding/Tutorials/Mod%20Localisation.md)). MCPBK demonstrates demand but not a safety contract. Add only once the responsible LocalizationEditor API is verified. |
| P1 | World transform completeness: inspect/set scale; optional `workbench_set_entity_transform` | Current movement and rotation leave scale as a manual gap. A single fully typed transform input (position/rotation/scale, exact entity ID, optional expected revision) is safer and more useful than serial per-field mutations. | The World Editor exposes coordinates, angles, and scale. Require one native undo action, prevalidation, and readback. Do not replace the current narrow operations until migration is explicitly planned. |
| P1 | Structured world save-state / dirty-state and content-aware log/diagnostic correlation | Let the agent distinguish unsaved world changes, compiler errors, integration failures, and prior log noise. Return bounded severity-filtered events plus a correlation ID from the originating operation. | The official Log Console has severity filters; s&box's bounded console history provides the agent-facing precedent. Existing `workbench_read_logs` remains the raw bounded tail, so this is a projection rather than a second log store. |
| P2 | Prefab or resource visual preview / thumbnail | A bounded PNG lets an AI verify a material/prefab appearance, editor framing, or generated Game Master preview instead of inferring from properties. Return capture provenance, dimensions, timestamp, and resource/world identity. | s&box returns editor-camera and asset images; Reforger's official image-generation workflow proves the domain demand. First prove one native capture/transfer method and strict byte/dimension limits; never scrape the desktop. |
| P2 | Batch composition placement from a declarative plan | One typed request creates a bounded list of prefab placements under one native Undo action, returns every resolved ID, and rolls back/declares partial failure exactly. It makes authored outposts/sets practical without exposing clipboard semantics. | s&box's multi-spawn plus one undo scope is a strong design precedent. Require Reforger live proof, all-resource prevalidation, position bounds, and a small initial item cap. |
| P2 | Navmesh change-impact inspection, then explicitly confirmed regional generation | Inspect configured navmesh resources and overlap between changed entities/terrain and tiles; only later generate a user-confirmed bounded area with progress, cancellation, output resource identity, and no implicit overwrite. | Official navmesh guidance explicitly recommends partial regeneration and warns full generation may be expensive. This is not a background convenience operation. |
| P3 | Named test/autotest run and result retrieval | Run one declared test class/suite against an explicit selected world and return structured pass/fail, timing, logs, and artifacts. | The official World Editor lists an Autotest Tool, but the callable plugin API, isolation, and result format must be independently established. Do not introduce arbitrary test-script execution. |
| P3 | Curated, versioned workflow prompts/resources | Supply concise `/create-mod`-style planning/checklist guidance for approved tasks—e.g. prefab change, localisation, terrain/navmesh—linking to the existing typed tools and evidence. | MCPBK shows strong demand for guided recipes; expose only documentation-owned, non-executing prompts/resources rather than a second scaffolding system. |

## Deliberate exclusions

- No generic `execute_action`, console command, handler dispatch, shell/CLI, or
  arbitrary project-file read/write tool. The extension and user already
  provide appropriate file control; these shortcuts erase the authority and
  consent boundaries.
- No name-based delete/select, clipboard operations, or generic property
  strings. Continue using stable entity IDs and descriptors returned by prior
  inspection.
- No claim that `GetApi() == null` definitively means a play session. The
  documented review establishes only a bounded availability signal
  ([play-session research](workbench-play-session-research.md)).
- No dynamic tool registry or a generic `call_tool` facade. The 56 static
  schemas are already machine-readable; a capability change should be a
  deliberate versioned MCP addition.

## Recommended delivery order

1. Prove native save/undo/redo and ship them with live postcondition tests.
2. Add project-bound resource registration/rebuild plus correlated diagnostics.
3. Extract workspace semantic queries from the Rust language engine and test
   them against project and Game Data identities.
4. Add resource impact/dependency inspection, then typed localisation.
5. Choose **one** visual or batch-world vertical slice only after its native
   Workbench API and result-transfer contract are proven.

Every live slice must follow the repository rule: after changes under
`server/`, run `npm run compile` to replace the bundled server and relaunch the
language server/Workbench before treating editor behaviour as verified.
