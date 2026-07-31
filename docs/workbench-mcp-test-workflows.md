# Workbench MCP test workflows

This document defines the acceptance model for automated testing of every
published `workbench_*` MCP capability. The generated
[MCP API reference](mcp-api.md) and the live MCP `tools/list` response own the
endpoint inventory. This document owns the dependency graph, acceptance rules,
workflow order, and endpoint status vocabulary.

The current public inventory contains 63 Workbench endpoints. A new test
campaign starts with all 63 endpoints at `not-run`; results from an older
harness are historical evidence and never seed a new campaign's status.

## Acceptance seam

Live acceptance crosses one seam: a real MCP Runtime receives standard
`tools/list` and `tools/call` requests and controls a real Workbench process
through the published `workbench_*` capabilities. A bridge handler unit test,
private Workbench process mode, direct NET API request, filesystem inspection,
or operating-system process command can diagnose an implementation, but none
of them can approve a public MCP endpoint. The no-consent profile snapshot is a
separate safety oracle: it can fail the guard for an unexpected write, but the
guard is accepted only from the public MCP structured consent error.

The corpus runner exposes one conceptual operation:

```text
run Workbench MCP corpus
    -> compare tools/list with the generated contract inventory
    -> establish reusable Workbench facts through named workflows
    -> evaluate every endpoint's required acceptance cases
    -> clean up disposable state
    -> emit one versioned endpoint corpus report
```

The repository runner implements this operation as `runWorkbenchCorpus`. Its
endpoint plan is generated from the published Workbench catalogue, validated
for exact one-to-one parity and dependency producers, and included in every
report. Scenario calls are grouped and executed as named workflows in
dependency order, then normalized into explicit `test`, `setup`, `readback`,
or `teardown` invocations with an acceptance-case identity; a successful call
without its required case or readback cannot approve an endpoint. One
invocation may carry multiple roles when one public read also proves a
preceding mutation or performs disposable teardown.

The runner should hide session management, fact capture, confirmation tokens,
readback, cleanup, and reporting behind that interface. The executable plan is
materialized by `buildWorkbenchEndpointPlan`; `runWorkbenchWorkflows` executes
its named workflow groups while sharing only facts captured from public MCP
responses. A step whose required fact is unavailable is recorded as a blocked
synthetic observation and is not counted as live endpoint evidence; independent
later workflows continue when their own facts are available. Teardown evidence
also carries the materialized opaque target identity so a different disposable
entity cannot satisfy cleanup for the mutation under test.

## Status model

Each endpoint has one final status for one complete corpus run:

| Status | Meaning |
| --- | --- |
| `passed` | Every required acceptance case completed through the public endpoint and matched its success, readback, and safety oracles. |
| `failed` | A required case reached the endpoint, but the response, readback, cleanup, or safety behavior contradicted its contract. |
| `blocked` | A required dependency could not be established through the public MCP surface, so the endpoint's behavior could not be evaluated. |
| `not-run` | No required case was attempted in this campaign, including when an earlier workflow failure prevented scheduling it. |

`blocked` and `failed` are deliberately different. For example, the absence of
a public operation that enters prefab-edit mode blocks a prefab-edit write; it
does not prove that the write endpoint failed.

An expected structured error can pass an explicit guard case, but it never
substitutes for a required successful-behavior case. An endpoint with a passing
guard and a blocked success case is `blocked`, not `passed`.

## Cases, invocation roles, and non-repetition

Every endpoint declares one or more required acceptance cases. Each observed
call is labeled with one of these roles:

| Role | Meaning |
| --- | --- |
| `test` | Owns an explicit acceptance case for this endpoint. |
| `setup` | Establishes a fact consumed by a later case. It can also own its endpoint's test case when its own oracle is asserted. |
| `readback` | Observes the effect of a mutation. It counts for the read endpoint only when it also satisfies a declared case for that endpoint. |
| `teardown` | Removes disposable state or closes a session. It counts for its endpoint only when it also satisfies that endpoint's declared case. |

This allows dependencies to be tests without pretending every invocation is a
new test. For example, one `workbench_create_entity` call can both pass the
create-entity case and produce an entity identity for component, transform,
selection, duplication, prefab, and deletion cases. A later create call needed
for a shape fixture is labeled `setup`; it does not retest entity creation.

Repeated calls are valid when the public contract requires a transition or
readback. Preview and confirmation calls form one safety-sensitive acceptance
case. A mutation followed by inspection is one mutation case plus, when
declared, one inspection case. The report preserves every invocation and names
the case, fact, and role it served.

## Reusable facts

The executable workflows produce named facts rather than relying on hidden
step order:

| Fact | Meaning |
| --- | --- |
| `catalogue` | Live `tools/list` exactly matches the generated Workbench contract inventory. |
| `ownedProcess` | The MCP session launched an exact project and owns its observed process ID. |
| `connectedWorkbench` | The configured loopback endpoint reports native NET API readiness. |
| `managedBridge` | A compatible, already-consented handler package is active. |
| `projectContext` | Expected add-ons and project identity are loaded. |
| `worldEditor` | The discovered World Editor module is open. |
| `activeWorld` | A disposable world, subscene, and unlocked layer are observed through `workbench_state`. |
| `canonicalResource` | Resource discovery returned a canonical identity; no path or GUID was guessed. |
| `entity` | A disposable exact live entity identity exists. |
| `component` | The entity has an exact component identity and a typed writable descriptor. |
| `relatedEntities` | Exact parent and child identities exist for hierarchy tests. |
| `shape` | A disposable polyline has a readable authored point set. |
| `prefabResource` | A disposable canonical prefab resource and resource write descriptors exist. |
| `prefabEditEntity` | Workbench reports `prefabEditMode:true` for an exact open prefab entity and exposes writable descriptors. |
| `savedWorld` | All intended mutations were acknowledged by the public save operation. |
| `playSession` | Workbench accepted play start for the active saved world. |
| `reloadedRuntime` | Reload returned a changed compatible runtime generation and matching log evidence. |
| `replacementProcess` | Restart replaced the owned process with a newly observed connected process. |

Facts are valid only inside the MCP session and Workbench context that produced
them. Opaque entity IDs, component IDs, member IDs, confirmation tokens,
window IDs, cursors, and runtime generations are never hardcoded or reused
across runs.

## Workflow graph

The suite executes named workflows in dependency order. A workflow may continue
past a blocked endpoint when its remaining cases do not consume the missing
fact.

```text
contract catalogue
    -> owned process launch
        -> bridge and project readiness
            -> resource discovery + editor/world opening
                -> world read baseline
                -> entity/component/hierarchy/selection chain
                -> shape chain
                -> prefab resource chain
                    -> prefab-edit chain
                -> save -> play start -> play stop -> reload -> logs
        -> restart owned process -> verify replacement -> stop replacement
```

### Workflow 0: contract catalogue

Initialize one MCP Runtime, call `tools/list`, and compare the complete live
Workbench catalogue against the generated MCP API reference. Fail the corpus
before live mutation if an endpoint is missing, duplicated, undocumented,
unexpected, or lacks its schemas and annotations. Also verify that every
published endpoint appears exactly once in the endpoint plan and every named
dependency has a producer.

### Workflow 1: owned process and visible windows

Launch the exact disposable `.gproj` with no reusable Workbench process. Capture
the returned process ID, prove status readiness, list only that process's visible
windows, and capture one returned window ID. Process ownership is mandatory for
later restart and stop acceptance.

### Workflow 2: bridge, project, and editor readiness

Maintain an already-consented managed bridge, validate scripts, inspect loaded
add-ons, discover editor IDs, and open the World Editor by its returned ID. A
separate disposable profile is required for the no-consent bridge guard case;
the guard passes only when no profile files are written.

### Workflow 3: resources, world, and read baseline

Search for the fixture world, use the returned canonical resource identity for
listing and inspection, open it through native resource routing, and verify the
active world through `workbench_state`. Then exercise terrain, trace, layer,
viewport, entity search, radius search, and the initial empty selection against
that one world.

### Workflow 4: entity, component, hierarchy, and selection

Create one disposable entity and reuse its identity. Inspect it, add and inspect
a writable component, change entity and component properties to distinct test
values, and read both back. Reuse the entity for rename, move, rotate,
duplication, reparenting, search, selection, hierarchy, component removal, and
confirmed deletion. Every mutation has a direct public readback.

### Workflow 5: shape editing

Create or use one disposable `PolylineShapeEntity`, read its baseline points,
and reuse it for direct editing, regular polygon generation, local/world
conversion, named transforms, and resampling. Read points after every mutation.
Restore the baseline when practical, then delete the disposable shape. Calls
that create the shape are setup unless they own an otherwise unexecuted
create-entity case.

### Workflow 6: prefab resources

Create one prefab from the disposable scene entity and one generic prefab at
unique project-relative destinations. Use only returned canonical resource
identities, component IDs, write descriptors, and confirmation tokens. Inspect,
add a component, set a property to a distinct value, save, reopen, and verify
persisted readback before confirmed component removal and cleanup.

### Workflow 7: prefab editor

Open the disposable prefab through a public operation and require Workbench to
report `prefabEditMode:true`. Inspect the root and component to obtain editor
write descriptors, change root and component properties, inspect readback, save
the exact editor target, reopen, and verify persistence.

If no public MCP workflow can establish `prefabEditEntity`, only the two
prefab-edit write endpoints and editor-target save case are blocked. Resource
prefab tests and outside-edit-mode guard cases continue and retain their own
evidence.

### Workflow 8: save, play, reload, and logs

Save the world after mutations. Start play only here, where it is an explicit
target and prerequisite for stop-play. Stop play immediately, verify edit mode
through state, reload scripts, require a changed compatible runtime generation,
and read logs scoped to that reload. Play is never used as incidental setup for
an unrelated endpoint.

### Workflow 9: restart and stop

Restart the exact owned process after save. Adopt only the returned replacement
process ID, wait for native readiness, and then stop that replacement. Restart
passes when a new connected process is observed; it does not require the
replacement to already be exited. Stop passes only when the exact replacement
process is confirmed exited.

## Endpoint dependency and acceptance matrix

The `Fresh status` column is the baseline for the new corpus campaign. Runtime
reports, not manual edits to this document, become authoritative after the new
runner executes.

### Process, integration, resources, and editor state

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_launch` | 1 | `catalogue`, exact project, no existing fixture process | Returns `alreadyRunning:false`, positive owned process ID, and native readiness. | `not-run` |
| `workbench_status` | 1 | `ownedProcess` | Returns Workbench-authored running and compiled facts without side effects. | `not-run` |
| `workbench_list_windows` | 1 | `ownedProcess` | Returns only visible top-level windows owned by the exact process and at least one usable window ID. | `not-run` |
| `workbench_capture_window` | 1 | window ID from `workbench_list_windows` | Returns bounded in-memory PNG image content for that exact window and writes no file. | `not-run` |
| `workbench_install_bridge` | 2 | `connectedWorkbench`, preconsented profile | Maintenance returns compatible version/managed files; separate no-consent case returns the stable consent error without writes. | `not-run` |
| `workbench_project_context` | 2 | `managedBridge` | Returns the expected loaded add-on identities from live Workbench context. | `not-run` |
| `workbench_validate_scripts` | 2 | `managedBridge`, `projectContext` | Fixed `WORKBENCH` validation succeeds and paging, if present, preserves one compilation result. | `not-run` |
| `workbench_list_editors` | 2 | `managedBridge` | Returns native editor IDs including World Editor without opening one. | `not-run` |
| `workbench_open_editor` | 2 | editor ID from `workbench_list_editors` | Opens the returned World Editor ID and subsequent state proves availability. | `not-run` |
| `workbench_search_resources` | 3 | `managedBridge`, `projectContext` | Finds the fixture by supported kind/query and returns canonical resource identity and add-on facts. | `not-run` |
| `workbench_list_resources` | 3 | `managedBridge`, fixture query | Returns a bounded compatibility page containing the fixture; canonical search remains the source of target identity. | `not-run` |
| `workbench_inspect_resource` | 3 | `canonicalResource` | Returns `found:true`, the same canonical identity, class, and source add-on metadata. | `not-run` |
| `workbench_open_resource` | 3 | `canonicalResource`, `worldEditor` | Native routing opens the fixture and `workbench_state` reads back the expected active world. | `not-run` |
| `workbench_state` | 3, 8 | `managedBridge`; later `activeWorld` and stopped play | Reports loaded context and active world; after stop-play reports edit mode again. | `not-run` |

### World inspection

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_world_selection_summary` | 3, 4 | `activeWorld` | Proves the initial empty selection and later returns exact selected entity identity. | `not-run` |
| `workbench_selected_entity_hierarchy` | 4 | selected child from `relatedEntities` | Returns the exact child plus expected parent and bounded direct children without changing selection. | `not-run` |
| `workbench_list_entities` | 3 | `activeWorld` | Returns a bounded page with stable IDs and correct subscene/layer filtering. | `not-run` |
| `workbench_search_world_entities` | 3, 4 | `activeWorld`; later named `entity` and relation | Finds known fixture/entity facts with ANDed filters and returns exact relation evidence. | `not-run` |
| `workbench_layer_state` | 3 | `activeWorld` subscene/layer IDs | Returns canonical layer path, visibility, and effective unlocked state. | `not-run` |
| `workbench_find_entities_by_radius` | 3 or 4 | `activeWorld`, known entity position | Bounded radius query includes the known entity under the documented scope. | `not-run` |
| `workbench_sample_terrain` | 3 | `activeWorld`, known terrain coordinate | Returns an available bounded grid with coherent dimensions, spacing, and finite heights. | `not-run` |
| `workbench_get_viewport_context` | 3 | `activeWorld`, visible World Editor | Returns coherent camera/viewport facts; cursor-outside status is accepted only as its explicit environment case, not as a substitute for camera proof. | `not-run` |
| `workbench_trace` | 3 | `activeWorld`, deterministic fixture geometry/terrain | A bounded explicit trace returns the expected nearest hit or successful miss and target kind. | `not-run` |
| `workbench_inspect_entity` | 4 | `entity` | Returns the same exact identity, class, transform, hierarchy, and component summaries; later confirms deletion is no longer addressable. | `not-run` |
| `workbench_list_components` | 4 | `entity` | Reads baseline, added component, and post-removal states using opaque component IDs. | `not-run` |
| `workbench_inspect_component` | 4 | `component` | Returns the exact component and complete typed properties including a writable descriptor; later reads changed value. | `not-run` |
| `workbench_list_entity_properties` | 4 | `entity` | Returns typed direct properties and writable descriptor; later reads changed transform/property value. | `not-run` |

### Entity, component, hierarchy, and selection mutation

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_create_entity` | 4 | `activeWorld`, unlocked layer | Creates one exact disposable entity and inspection reads back class, position, subscene, and layer. | `not-run` |
| `workbench_add_component` | 4 | `entity`, supported component class | Adds one component; list and inspection return a new exact component ID. | `not-run` |
| `workbench_set_component_properties` | 4 | `component`, descriptor from inspection | Writes a value distinct from baseline and component inspection reads back that exact value. | `not-run` |
| `workbench_set_entity_properties` | 4 | `entity`, descriptor from property listing | Writes a value distinct from baseline and property listing reads it back. | `not-run` |
| `workbench_rename_entity` | 4 | `entity` | Renames once; entity inspection and exact search read back the new name and same ID. | `not-run` |
| `workbench_move_entity` | 4 | `entity` | Moves to a distinct explicit position; inspection reads back the coordinates. | `not-run` |
| `workbench_rotate_entity` | 4 | `entity` | Rotates to distinct explicit angles; entity properties read back those angles. | `not-run` |
| `workbench_duplicate_entity` | 4 | `entity` | Returns a distinct disposable entity ID whose class and requested position are inspectable. | `not-run` |
| `workbench_reparent_entity` | 4 | distinct parent and child in `relatedEntities` | Parents the child once; hierarchy and entity inspection read back the exact relationship. | `not-run` |
| `workbench_set_selection` | 4 | `entity` | Replaces visible selection; selection summary returns exactly that entity ID. | `not-run` |
| `workbench_clear_selection` | 4 | selected `entity` | Clears selection; selection summary returns zero selected entities. | `not-run` |
| `workbench_remove_component` | 4 | `component` | Preview returns a token without mutation; confirmation consumes it once and component listing proves absence. | `not-run` |
| `workbench_delete_entity` | 4, 5 | disposable entity | Preview returns a token without deletion; confirmation consumes it once and inspection proves absence. Shape cleanup calls are teardown, not duplicate delete tests. | `not-run` |

### Shape inspection and mutation

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_get_shape_points` | 5 | `shape` | Returns ordered finite local authored points and keeps entity position separate; later reads every mutation. | `not-run` |
| `workbench_edit_shape_points` | 5 | `shape`, baseline points | Set/insert/delete case changes the intended ordered points and immediate readback matches. | `not-run` |
| `workbench_set_polyline_regular_polygon` | 5 | polyline `shape` | Valid sides/radius produce the deterministic point count and representative local coordinates. | `not-run` |
| `workbench_convert_shape_points` | 5 | transformed or parented `shape`, known points | Local-to-world-to-local round trip preserves points within the declared numeric tolerance. | `not-run` |
| `workbench_transform_shape_points` | 5 | `shape`, baseline points | One named transform changes all points as specified; readback matches and restore is verified. | `not-run` |
| `workbench_resample_polyline` | 5 | polyline `shape` with valid path | Valid local/world spacing returns coherent original/result counts and readback preserves required endpoints/closure. | `not-run` |

### Prefab inspection and mutation

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_create_prefab` | 6 | `entity`, unused project-relative destination | Preview does not create; confirmation consumes its token and canonical resource inspection proves creation. | `not-run` |
| `workbench_create_generic_prefab` | 6 | `activeWorld`, unlocked layer, unused destination | Preview does not create; confirmation creates a discoverable GenericEntity prefab and removes its temporary source. | `not-run` |
| `workbench_inspect_prefab_context` | 6, 7 | `prefabResource`; later `prefabEditEntity` | Resource case returns ancestry/members/effective values; editor case reports edit mode and writable descriptors. | `not-run` |
| `workbench_inspect_prefab_component` | 6, 7 | component ID from prefab context | Resource case returns effective typed values; editor case returns writable descriptors and changed-value readback. | `not-run` |
| `workbench_add_prefab_resource_component` | 6 | local `prefabResource`, supported class | Preview is non-mutating; confirmation saves and fresh inspection returns the added opaque component ID. | `not-run` |
| `workbench_set_prefab_resource_property` | 6 | resource descriptor and baseline value | Preview is non-mutating; confirmation writes a distinct value, saves, and fresh/reopened inspection reads it back. | `not-run` |
| `workbench_remove_prefab_resource_component` | 6 | component added to `prefabResource` | Preview is non-mutating; confirmation saves and fresh inspection proves exact component absence. | `not-run` |
| `workbench_save_prefab` | 6, 7 | `prefabResource`; editor case needs `prefabEditEntity` | Resource-target preview/confirm proves persisted inspection; editor-target case saves and survives reopen. | `not-run` |
| `workbench_set_prefab_property` | 7 | `prefabEditEntity`, descriptor from editor context | Writes a distinct root value only in edit mode; context inspection reads it back before and after save/reopen. Outside-mode guard is an additional case. | `not-run` |
| `workbench_set_prefab_component_property` | 7 | `prefabEditEntity`, exact component and editor descriptor | Writes a distinct component value only in edit mode; component inspection reads it back before and after save/reopen. Outside-mode guard is an additional case. | `not-run` |

### Save, play, reload, logs, and process teardown

| Endpoint | Workflow | Required dependencies | Required acceptance proof | Fresh status |
| --- | --- | --- | --- | --- |
| `workbench_save` | 8 | dirty `activeWorld` with existing path | Save All and world save are acknowledged without opening Save As; readback remains available. | `not-run` |
| `workbench_start_play_session` | 8 | `savedWorld`, World Editor in edit mode | Returns `accepted:true` and play-started status; used only as the explicit target/prerequisite for stop-play. | `not-run` |
| `workbench_stop_play_session` | 8 | `playSession` | Returns `accepted:true` and play-stopped status; `workbench_state` confirms edit mode. | `not-run` |
| `workbench_reload` | 8 | `savedWorld`, compatible active bridge | Saves first, dispatches reload, and returns only after a changed compatible runtime generation is observed within the contract timeout. | `not-run` |
| `workbench_read_logs` | 8 | `reloadedRuntime` | Latest mode begins at the matching reload marker and exposes bounded diagnostic evidence without arbitrary paths. | `not-run` |
| `workbench_restart` | 9 | `ownedProcess`, saved project identity | Saves, replaces the exact observed process, and returns a different connected replacement process ID with `alreadyRunning:false`. | `not-run` |
| `workbench_stop` | 9 | `replacementProcess` | Saves, closes the exact replacement process, uses any force fallback only within its public contract, and returns `exited:true`. | `not-run` |

## Fixture contract

The corpus uses disposable, explicit inputs:

- one exact `.gproj` with required base add-ons and managed bridge consent;
- one world with an existing persistence path, known terrain/trace coordinates,
  and an unlocked disposable layer;
- supported entity and component classes with writable scalar properties;
- unique per-run entity names and prefab destinations;
- a disposable profile for first-install consent guard testing;
- no pre-existing Workbench process for owned lifecycle acceptance.

Fixture data may establish hard-to-author environmental facts, but it must not
pre-bake the behavior being tested. The runner discovers every canonical
resource, entity, component, window, editor, layer, and descriptor through the
public MCP surface.

## Report contract

One JSON report is emitted per corpus run. It records:

- generated and live catalogue revisions or fingerprints;
- server, bridge, protocol, Workbench, project, add-on, and fixture identity;
- run start/end time and workflow timing;
- one record for every published endpoint;
- required cases, dependencies, invocation roles, structured request outcome,
  readback evidence, cleanup evidence, and blockers;
- endpoint counts and exact lists for `passed`, `failed`, `blocked`, and
  `not-run`;
- unexpected, missing, duplicated, or undocumented endpoints.

An endpoint passes only when all of its required cases pass. The corpus passes
only when the catalogue matches, every endpoint is `passed`, cleanup succeeds,
and no unknown mutation remains. A run containing `blocked`, `failed`, or
`not-run` endpoints is useful evidence but is not a complete corpus.

Reports are generated artifacts under an ignored report directory. They are
not release inputs and are never bundled into the extension or language server.
The executable report uses `kind: "workbench-mcp-corpus"`, `corpusVersion: 1`,
and the status vocabulary above; historical `approved` or `incomplete` fields
are not used for current endpoint acceptance.

## Harness verification

The runner itself needs deterministic tests that do not claim live endpoint
approval:

- catalogue drift, duplicate, and uncategorized endpoint detection;
- plan completeness and dependency-producer validation;
- invocation-role and case ownership validation;
- status aggregation, especially guard-pass plus success-blocked behavior;
- preview/confirmation token flow and one-time consumption;
- fact capture and opaque identity propagation;
- readback and cleanup failure propagation;
- continuation past independent blocked workflows;
- report schema, ordering, and reproducibility;
- exact owned-process adoption across restart and stop.

Live acceptance then runs the same public corpus interface against Workbench.
Unit tests of the harness and bridge remain supporting evidence only.
