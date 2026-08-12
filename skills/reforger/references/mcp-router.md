# MCP router

Use this reference before the first Reforger MCP call. Choose a route by the claim that must be established, follow returned handoffs exactly, and stop only at that route's completion criterion.

## Response protocol

1. Treat the live MCP catalogue and call schemas as authoritative.
2. Read JSON fields from `MCP.structuredContent`; use `MCP.content` for text or image payloads.
3. When `MCP.isError` is true, read `MCP.code`, `MCP.message`, `MCP.recovery`, and `MCP.retryable`. Follow the recovery instead of guessing another tool or argument.
4. Copy revisions, add-on GUIDs, cursors, symbol references, entity/resource/component IDs, descriptors, confirmation tokens, and ready-made input objects unchanged.
5. Continue an opaque cursor when proof requires more results. Start over from the originating search after a stale cursor or reference.
6. Treat an exact engine identifier as fact only after Game Data verification. Until then, preserve supplied names only as labeled search terms and describe other candidates generically.

## Route table

| Needed evidence | Start | Continue |
| --- | --- | --- |
| Reforger concept or workflow | `official_wiki_status` when uncertain, then `search_official_wiki` | `read_official_wiki` |
| Engine declaration from a natural-language need or suspected name | `research_game_data` | Stop on a resolved sufficient result; use one targeted follow-up only for a missing fact |
| Exact symbol filtering or enumeration | `search_game_data_symbols` | `inspect_game_data_symbol` only when the search result lacks a required declaration fact |
| Offline prefab/config/layout/world/resource identity | `search_game_data_resources` | Live resource inspection when registration or effective state matters |
| Add-on declaration or usage | `search_workspace_symbols` | `inspect_workspace_symbol`, members, relationships, source |
| Cross-source override, inheritance, or modded-class fact | Semantic search for the exact anchor | `query_source_symbol_relationships` |
| Literal string, comment, expression, or regex evidence | `search_game_data_text` or `search_workspace_text` | Returned source-read input |
| Native compilation | `workbench_status` when uncertain | `workbench_project_context`, then `workbench_validate_scripts` |
| Reloaded runtime | Successful native validation | `workbench_reload`, state, logs, targeted live test |

Semantic search owns declarations. Literal search owns textual occurrences. Workbench owns current live state.

## Official Wiki route

1. Call `official_wiki_status` on first use, uncertainty, or corpus failure; require `official_wiki_status.available` to be true.
2. Search narrow terms with `search_official_wiki`; use path scope only when the domain subtree is known.
3. Compare titles, paths, headings, match kinds, and excerpts instead of accepting rank one automatically.
4. Copy `search_official_wiki.readInput` from `search_official_wiki.results` unchanged to `read_official_wiki`.
5. Preserve `read_official_wiki.content`, `read_official_wiki.sourceUrl`, and `read_official_wiki.relativePath`; follow continuation unchanged until the required section is complete.

Complete the route with a canonical source, corpus revision, and exact line range. Wiki examples inform design but do not satisfy the engine API gate.

## Game Data declaration route

1. Start with `research_game_data` for a natural-language need, a suspected identifier, or an exact identifier. Pass the user's need as one concise query and add `addonGuids` only when the task already establishes an add-on scope.
2. Treat `status: resolved` plus `followUp: none` as a stop signal when `primary` contains the declaration facts needed for the current claim. Do not call status, search, inspect, members, relationships, examples, or source merely to collect more context.
3. On `status: ambiguous` or `notFound`, use `matchedTerms` and at most two `alternatives` to refine the query once. Call `game_data_status` only when availability, coverage, version, cache health, or scope is genuinely uncertain.
4. Use `search_game_data_symbols` instead when the task requires exact filtering or enumeration across multiple symbols. Use `inspect_game_data_symbol` only when the compact primary result lacks a declaration fact required by the API ledger.
5. Use `list_game_data_symbol_members` only when a specifically required member is absent from the primary result's `relevantMembers`. Query relationships only when inheritance, overrides, callers, references, or another edge is itself a required claim.
6. Read source only when declaration metadata cannot establish the required behavior or usage. Copy the returned `readSourceInput` unchanged and continue a bounded read only while the needed evidence crosses the boundary.

Complete the route as soon as the exact declaration facts and only the relationships or usage evidence required by the current task are proven. The compact resolved result is verification evidence; ambiguity is discovery evidence.

### API ledger

Inventory every engine-facing class, member, callback, attribute, enum value, helper, and inherited assumption in changed lines:

| Identifier | Exact owner/declaration | Signature/attributes | Relationship or usage evidence | Status |
| --- | --- | --- | --- | --- |

Set status to `verified`, `workspace-owned`, `language construct`, or `blocked`. Only the first three may reach exact emitted code.

## Resource route

1. Use `game_data_status` when offline resource scope or catalogue revision is uncertain.
2. Call `search_game_data_resources` with basename or path terms and an exact resource kind when useful.
3. Preserve canonical resource identity, add-on provenance, logical path, registration or stale flags, and Workbench link.
4. Use `workbench_search_resources` and exact resource/prefab inspection when current registration, effective values, ancestry, component wiring, or editor state matters.

Complete the route when canonical identity and owning add-on are known, and any effective live-state claim has Workbench inspection evidence.

## Workspace route

1. Use `search_workspace_symbols` for declarations. Copy `search_workspace_symbols.inspectInput` or `search_workspace_symbols.symbolRef` unchanged to `inspect_workspace_symbol` and member tools.
2. Use `query_workspace_symbol_relationships` for add-on definitions, inheritance, references, and callers.
3. Use `query_source_symbol_relationships` for exact edges across workspace and selected Game Data scope.
4. Copy `search_workspace_symbols.readSourceInput` unchanged to `read_workspace_source` and continue bounded reads when needed.
5. Use workspace text search only for literal or regex evidence.

Complete the route when relevant add-on declarations, usage, and relationships are proven from one workspace snapshot. Restart the MCP process after editing before treating semantic reinspection as fresh.

## Compiler route

1. Call `workbench_status` when availability is uncertain; this establishes availability only.
2. Call `workbench_project_context` and verify the intended add-on.
3. Call `workbench_validate_scripts` without a cursor to run the native compiler.
4. Preserve and continue every `workbench_validate_scripts.nextCursor`; cursored calls page the same compilation without recompiling.
5. Account for every error and warning. Fix failures, then begin a fresh uncursored validation.

Complete the route when `workbench_validate_scripts.success` is true for the latest validation, all diagnostic pages are consumed, and remaining warnings are reported. Any code edit reopens the gate.

## Reload and runtime route

Use this only after the compiler route completes for an authorized implementation.

1. Read `workbench_state` and `workbench_project_context`; verify project/editor context and play state.
2. Call `workbench_reload`, accounting for Save All and active-world persistence.
3. Require `workbench_reload.reloadDispatched` true and a replacement `workbench_reload.runtimeGeneration`. Record `workbench_reload.worldSavedBeforeReload` and `workbench_reload.worldSaveStatus`.
4. Read fresh `workbench_status` and `workbench_state`.
5. Read `workbench_read_logs` with the narrowest source and range. Logs are diagnostic history, not proof of current state or behavior.
6. Run the smallest live test. Command acceptance proves only dispatch; observe state, behavior, and fresh logs before claiming success.
7. Stop any play session started for the test, confirm edit mode, and read final relevant logs.

Complete the route when reload is structurally confirmed, behavior is observed in feasible required roles, fresh logs are reviewed, and the editor is left in the intended state.

## Freshness and recovery

- Follow structured errors exactly. A missing tool or unsupported request is blocked, not permission to substitute an unrelated route.
- On stale symbols, relationships, revisions, or cursors, repeat the originating semantic search.
- On `game_data_unavailable`, call `game_data_status` and follow recovery. Use generic architecture or placeholder pseudocode without exact engine identifiers.
- On unavailable Wiki data, follow status recovery and stop before Reforger-specific design.
- On `game_data_changed`, refresh the language index through its owning client, then restart MCP.
- Treat workspace semantic evidence as a per-process snapshot. Restart MCP after workspace edits.
- Treat parser analysis, Workbench status, native validation, reload, logs, editor state, and runtime observation as separate evidence surfaces.
