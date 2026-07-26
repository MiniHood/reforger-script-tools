# Workbench play-session detection research

Research date: 2026-07-26. This note reviews the `feat/ui-gamemode-mcp-expansion`
branch of `steffenbk/enfusion-mcp-BK` at commit
[`3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739`](https://github.com/steffenbk/enfusion-mcp-BK/tree/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739).
MCPBK is an external implementation and establishes a candidate technique, not
an Enfusion contract by itself.

## What MCPBK implements

MCPBK does not expose a separate `isPlaying` boolean. Its handler returns a
string `mode`, and its TypeScript client maps that into cached `"edit"`,
`"play"`, or `"unknown"` state.

| Handler/class | Test | Response shape | Client interpretation |
| --- | --- | --- | --- |
| [`EMCP_WB_Ping`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Ping.c) | `Workbench.GetModule(WorldEditor)` is absent | `{ status: "ok", mode: "no_world_editor", message }` | leaves mode unknown |
| same | module exists and `worldEditor.GetApi()` is non-null | `{ status: "ok", mode: "edit", message }` | caches edit |
| same | module exists and `worldEditor.GetApi()` is null | `{ status: "ok", mode: "game", message }` | caches play |
| [`EMCP_WB_GetState`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_GetState.c) | same three-way branch | `mode` plus edit-only selection/entity fields | same mapping |

The client normalizes either handler value `"game"` or `"play"` to its
internal `"play"` state in
[`extractMode`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/src/workbench/client.ts#L360-L373).
Its `wb_play` and `wb_stop` commands call
[`EMCP_WB_EditorControl`](https://github.com/steffenbk/enfusion-mcp-BK/blob/3eecd8f8a35e44d8c8ff055fae0b1f6c8ee31739/mod/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_EditorControl.c),
which invokes `WorldEditor.SwitchToGameMode(debugMode, fullScreen)` and
`WorldEditor.SwitchToEditMode()` respectively. Those commands use the cached
mode only as a guard; they do not independently verify a transition completed.

## Evidence and reliability limits

The official World Editor Plugin tutorial establishes the first half of the
test: obtain `WorldEditor` from `Workbench.GetModule(WorldEditor)`, then obtain
`WorldEditorAPI` through `worldEditor.GetApi()` for editor operations
([official tutorial](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor_Plugin)).
It does **not** state that a null `GetApi()` is a stable, exclusive contract
for a running play session. MCPBK's assertion that it means game mode is its
own inference, expressed in the handler comments/messages rather than backed
by a cited engine contract.

Consequently, the useful result is a bounded World Editor state, not a general
game-session truth:

```json
{
  "worldEditorActive": true,
  "worldEditorApiAvailable": false,
  "playSession": "likely-running"
}
```

`"likely-running"` is appropriate only when the World Editor module exists
but its API is unavailable. The bridge should report `"editing"` when the API
is available, and `"unavailable"` when the module is absent. It should not
silently turn this into `isPlaying: false`: absence of a World Editor module
does not prove that no scenario/game process is running.

The same rule makes the signal safe for AI use: operations requiring editing
must require `worldEditorApiAvailable == true`; any operation that stops or
starts play needs an explicit user-authorized MCP action and a post-action
state refresh. It must never cause automatic stop/restart/close behaviour.

## Recommended next implementation

Extend `RST_WorkbenchState` with the two direct observations
`worldEditorActive` and `worldEditorApiAvailable`, then derive the small enum
`playSession` as above in the same handler. Retain `mode: "workbench"` as the
existing broad editor-context label: it should not be overloaded to claim a
play session. Test all three branches live:

1. no World Editor module;
2. World Editor loaded in editing mode;
3. World Editor loaded while the user runs the world.

Only promote `playSession: "likely-running"` to a definitive `"running"` or
add a bare `isPlaying` boolean if an official/generated API provides a direct
play-session fact and it is verified against the running Workbench version.

## Live activation boundary

On 2026-07-26, writing the extended handler package followed by native
`ValidateScripts` and the current bridge `reload` acknowledgement left the
running `RST_WorkbenchState` response on its previous JSON shape. This confirms
that this NET API compilation path does not hot-reload newly registered handler
classes. Live acceptance of the new fields therefore requires a real Workbench
script-host reload or restart; no MCP operation should close or restart it as a
side effect.
