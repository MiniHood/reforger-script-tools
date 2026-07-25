# Game Data corpus report

Research date: 2026-07-25.

This is a live-corpus check for the configured downloaded Reforger Game Data
catalogue after the focused hardening-and-topic-expansion ticket. It does not
describe a new search architecture.

## Method

The bundled `dist/server/win32-x64/reforger_language_server.exe` was started
in `mcp` mode with the extension's configured scripts root, metadata file, and
v12 cache. The session initialized with MCP `2025-11-25`, then called
`game_data_status`, `search_game_data_examples`,
`search_game_data_symbols`, and `read_game_data_source`. Results below are
from `structuredContent`; logical paths and the revision are copied from the
tool, rather than inferred from the filesystem.

The reproducible command shape is:

```powershell
$requests | & dist/server/win32-x64/reforger_language_server.exe mcp `
  --game-data-scripts <configured-scripts-root> `
  --game-data-metadata <configured-metadata.json> `
  --index-cache <configured-v12-cache>
```

## Corpus availability and coverage

`game_data_status` returned `available: true` and cache outcome `loaded` for
revision
`gd1:6f5e8485b33db55032a643c21edec743ead288e7d0a9a4cbd848616c04920c55`.
Its source metadata identifies the downloaded
`BohemiaInteractive/Arma-Reforger-Script-Diff` `main` revision
`2735631ce1400eaf9f1761c66cdee10c46921d37` (commit date 2026-06-24,
message `1.7.0.54`).

| Measure | Live result |
| --- | ---: |
| Source files / bytes | 6,495 / 27,640,746 |
| Indexed symbols | 143,144 |
| Lossless / lossy source files | 6,494 / 1 |
| Parse diagnostics | 0 |
| Cache bytes | 19,802,335 |
| Ready status load | 1,391 ms total (47 ms decode, 6 ms read, 1 ms validation) |

The largest source category is handwritten `game`: 4,522 files and 112,773
symbols. The catalogue also reports 1,637 generated files / 19,974 symbols,
177 Workbench files / 5,817 symbols, and 14 Doxygen files / 351 symbols.
The one warning is `lossy_files_present`; no search result should imply that
every source byte is lossless.

## What is useful now

The established curated call is immediately useful for an agent that needs a
source-backed prefab-spawn pattern:

```json
{"topic":"resource-loading","subtopic":"spawn-prefab","limit":10}
```

It returned 94 matches: 93 handwritten and one generated. The first ten were
handwritten `game` results, all carrying evidence terms, an exact line range,
and a copy-ready source read input. Representative returned paths include
`Game/Components/SCR_VehicleSpawner.c:70`, `Game/game.c:95`,
`Game/AI/ScriptedNodes/Inventory/SCR_AISpawnMagazines.c:43`, and
`Game/Destruction/SCR_PrefabSpawnable.c:123`. The only generated result was
`GameLib/generated/Game.c:128`.

The source-read handoff for `Game/Components/SCR_VehicleSpawner.c:70-94`
contains the complete local pattern: `Resource.Load`, a new
`EntitySpawnParams`, world transform setup, and
`game.SpawnEntityPrefab(resource, m_owner.GetWorld(), params)`. That is much
more actionable than a declaration alone. The result also correctly warns that
resource paths, prefab dependencies, authority, and runtime world/server
context still need Workbench/runtime verification.

The semantic side is similarly usable for progressive lookup. For example,
`search_game_data_symbols` found the exact `SCR_BaseGameMode` class at
`Game/GameMode/SCR_BaseGameMode.c:138` with signature
`class SCR_BaseGameMode : BaseGameMode`; a five-result page reported 332 total
matches and a cursor. It found the `RplRpc` declaration at
`Core/proto/EnNetwork.c:88` and its constructor at line 95, plus Doxygen
examples in `GameLib/replication/RplDocs.c`. It found both `SetEventMask`
declarations (`GameLib/generated/Components/GenericComponent.c:40` and
`Core/generated/Entities/IEntity.c:453`). Those results supply exact symbols,
signatures, source category, ranges, and inspect/read handoffs instead of
asking an agent to guess API spelling.

## Public curated example topics

The focused expansion is now live. Fresh bounded calls returned the following
handwritten-first results, each with exact source ranges, evidence terms,
revision-bound pagination, and a copy-ready source-read handoff.

| Topic / subtopic | Fresh result count | First result | Evidence boundary and guidance |
| --- | ---: | --- | --- |
| `resource-loading` / `spawn-prefab` | 94 | `Game/Editor/Components/Editor/SCR_PlacingEditorComponent.c` | Resource/prefab loading plus spawn evidence. Verify paths, dependencies, world, authority, and server context. |
| `replication` / `rpc-authority` | 150 | `Game/Editor/Components/Editor/SCR_WorldEntityDelegateEditorComponent.c` | `RplRpc` plus RPC or replication identity/use. Verify authority, receiver targeting, ownership, and dedicated-server behavior. |
| `entity-lifecycle` / `event-mask` | 207 | `Game/Components/HybridPhysicsComponent.c` | Lifecycle callback plus event-mask/entity-event evidence. Verify component wiring and event order. |
| `ui` / `widget-creation` | 214 | `Game/Editor/UI/InfoDisplays/SCR_EditorPingInfoDisplay.c` | Widget/layout creation plus `Widget` evidence. Verify layout resources and hierarchy. |

These are useful AI starting points because the terms encode a complete local
pattern rather than a lone API mention. The public MCP tool still keeps
generated examples available through its existing filter, while handwritten
results rank first when evidence quality is comparable.

`callqueue` remains a credible follow-up, not an automatic next topic:
`CallLater` is broad enough that it needs a narrowly specified use case (for
example, delayed UI refresh) before a high-signal evidence boundary is clear.

## Limitations and ticket guardrails

- The public example tool accepts the four topic/subtopic pairs listed above.
  Unsupported values name the supported choices; this is not a generic source
  query or semantic/vector search surface.
- Search results are catalogue evidence, not proof of current Workbench state,
  compiler acceptance, prefab wiring, multiplayer authority, or runtime
  behavior.
- Keep generated and handwritten filtering visible. The existing prefab topic
  demonstrates why: its useful corpus is overwhelmingly handwritten (93 of
  94), while the generated result supplies API-facing context.
- Keep the existing bounded result, exact logical range, revision-bound cursor,
  and copy-ready source-read handoff contract. Topic expansion should not
  weaken progressive inspection or make semantic symbol ranking fuzzy.

## Recommendation

The focused ticket achieved its goal. Do not redesign or broaden search again
immediately: move to the next product area, retaining this corpus report as a
baseline for future evidence-topic proposals.
