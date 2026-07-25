# Game Data corpus report

Research date: 2026-07-25.

This is a live-corpus check for the configured downloaded Reforger Game Data
catalogue. It records evidence for a focused hardening-and-topic-expansion
ticket; it does not describe a new search architecture.

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

## Best next example topics

The next expansion should stay small and evidence-defined. These candidates
have abundant handwritten source and exact declarations, but need deliberately
chosen co-occurrence terms and a verification warning before becoming public
topics.

| Candidate topic | Corpus evidence | Suggested evidence boundary |
| --- | --- | --- |
| `replication` / `rpc-authority` | `RplRpc` occurs in 168 Game Data files; `RplProp` in 81. A bounded read of `Game/Character/SCR_CharacterControllerComponent.c:445-496` shows `RplRpc` server and broadcast handlers, `Rpc(...)`, `RplId` validation, and `Replication.FindItem`. | Require `RplRpc` plus an `Rpc` call or `RplId`/`Replication` use. Warn that authority, receiver, ownership, and dedicated-server behavior require runtime validation. |
| `entity-lifecycle` / `event-mask` | `SetEventMask` occurs in 238 source files and `OnPostInit` in 244. Semantic search gives the exact generated declarations above. | Require a lifecycle callback (`EOnInit`/`EOnFrame`/`OnPostInit`) plus `SetEventMask` or an entity event token. Warn that prefab component wiring and runtime event order need Workbench/runtime validation. |
| `ui` / `widget-creation` | `CreateWidgets` occurs in 212 source files. Semantic search returned seven symbols, including `SCR_GalleryComponent.CreateWidgets(int count)` at `Game/UI/Components/WidgetLibrary/GalleryView/SCR_GalleryComponent.c:167`. The bounded source read at lines 160-194 shows cleanup, widget creation loop, and post-create selection. | Require a workspace/widget creation term and a `Widget`/layout operation. Warn that layout resource availability and UI hierarchy need Workbench validation. |

`callqueue` is a credible follow-up rather than part of the first ticket:
`CallLater` occurs in 414 files. Its breadth makes it especially important to
define a narrow use case (for example, delayed UI update) rather than expose a
generic, noisy topic.

## Limitations and ticket guardrails

- The public example tool currently accepts only `resource-loading` and its
  optional `spawn-prefab` subtopic. The candidates above are corpus evidence,
  not currently supported queries.
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

Proceed with one focused ticket: harden the existing example-search topic
definition/ranking/cancellation tests and add the three evidence-bounded
topics above, beginning with replication. Do not redesign search or merge
example search into semantic symbol search. After that slice is verified
against this corpus and focused fixtures, move to the next product area.
