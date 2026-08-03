# [Game Master](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master)

[![](/wikidata/images/thumb/e/ed/armareforger-game_master_everon.png/300px-armareforger-game_master_everon.png)](/wiki/File:armareforger-game_master_everon.png)

The Game Master tile.

**Game Master** is a game mode where nothing is planned, and where a player chooses what the next events will be.
It allows to create a real-time experience as a dedicated Game Master; curating events, placing assets, or just exploring by oneself.

It is equivalent to, and is the generic game mode name of [Arma 3 Zeus](/wiki/Arma_3_Zeus "Arma 3 Zeus").

"Game Master" is also used to designate the player in the role of the current curator.

## Description

* the player that is **Server Host** (or the declared **Server Admin**) *always* has access to the Game Master interface
* if no Game Master is present, the **first player to connect** obtains the Game Master role
* the role cannot be transferred unless that player disconnects

| Possible | Not Possible |
| --- | --- |
| * create, edit, move and kill the AI units and vehicles present in the world * edit, move and kill a player * provide groups with waypoints * create, move and delete prefabs on the fly * spawn at will into the game | * modify the base terrain |

## Controls

| Keyboard/Mouse | Controller | Description |
| --- | --- | --- |
| `W``A``S``D` | ⇱ | Move the camera horizontally |
| `Q``Z` | ↚↛ | Move the camera vertically |
| Right Mouse Button + Mouse | ⇲ | Rotate the camera |
| `V` | Hold ⇺ | Edit scenario properties |
| Hold `Y` | ⇻ + ↜ | Exit the interface |

## Interface

The interface is split into four panels:

* [Mode](#Mode)
* [Toolbar](#Toolbar)
* [Scenario Menu](#Scenario_Menu)
* [Entity Menu](#Entity_Menu)

## Mode

␼

The Mode interface is accessible using ↟.

The top interface allows to switch between Game Master and Armavision.

ⓘ

See [Armavision](/wiki/Arma_Reforger:Armavision "Arma Reforger:Armavision") for more information.

## Toolbar

␼

The Toolbar interface is accessible using ↡.

### Map

Shortcut: `M` / Hold ⇻

Toggle the in-game map.

### Flashlight

Shortcut: `L` / ↝ + ↻

Toggle flashlight for improved visibility in the dark. The light is only visible when it is dark and only to the Game Master, not to other players or AI.

### Toggle Interface

Shortcut: `I` / ↝ + ↟

Hide editor interface. When hidden, use the shortcut to reveal it again.

### Guided Tour

Step-by-step introduction of the current mode.

### Clear Destroyed Entities

Delete all killed soldiers and destroyed vehicles. This will free up budget they may still occupy.

## Scenario Menu

␼

The Scenario Menu interface is accessible using ↞.

Scenario properties shortcut: `V` / Hold ⇺

### Scenario Menu Interface

This interface allows to define playable factions when none are defined (by clicking the big "+") and to create faction spawn points as well as set *faction* **objectives** addressed to the **players** - the AI will ignore them.

␼

The objectives menu is accessible by selecting a faction and holding ↥.

#### Move

Shortcut: `Alt` + `2`

#### Seize

Shortcut: `Alt` + `3`

#### Defend

Shortcut: `Alt` + `4`

#### Custom

Shortcut: `Alt` + `5`

Can be one of:

* Objective
* Recon
* Suppress
* Stand down
* Wait
* Assemble
* Prepare
* Rescue captive
* Rescue captives
* Board vehicle
* Locate contact
* Take captive
* Target killing
* Locate vehicle
* Steal vehicle
* Destroy vehicle
* Locate intel
* Locate asset
* Ambush
* Locate base
* Prepare defenses

### Playable Factions

Set which factions can be joined by players. This interface is also accessible by clicking the big "+" when no factions are configured.

⚠

Be aware that removing a playable faction will kill any players already assigned to it.

### Game

* Enable respawn
  + Spawn near radio operators
  + Respawn time
* Server-wide ambient music

### Time and Date

* Time of the day (quick selection: 05:45, 08:52, 12:00, 20:12, 22:06, 00:00)
  + Time (slider, 00:00..24:00 by 15 minutes steps)
* Date (day, month, year, default 8 August 1989)
* Time progression (Y/N)
  + Time progression multiplier (default 1.0×, range 0.1×..12.0×, step 0.1×)

### Weather

* Automate weather (Y/N)
  + Weather: choose between Clear, Cloudy, Overcast, Rain and Fog
* Automate wind (Y/N)
  + Wind speed (range 0.0..15.0 m/s)
  + Wind direction (North, Northeast, East, Southeast, South, Southwest, West, Northwest)

## Entity Browser

Shortcut: `↹ Tab` / ⇺

Open the Entity Browser to place various characters, vehicles, structures and more.

### Controls

| Keyboard/Mouse | Controller | Description |
| --- | --- | --- |
| `Z` / `C` | ↚ / ↛ |
| `R` | ↻ | Reset filters |
| `Esc` | ↦ | Close |

### Filters

The Entity Browser allows to sort by:

* Faction
* Entity type
* Role
* Trait
* Content (if an entity is modded or not)

### Budgets

Budgets are the available "resources" that can be allocated on each Game Master aspects. They are shown at the bottom-right of the screen in the Entity Browser interface.

Once a budget reaches 100%, it is impossible to add more of its items unless previous ones are removed from the game.

For example, placing too many units will make the **AI Budget** reach 100% - more AI cannot be placed, and some previously placed ones (usually the unused ones) must be deleted in order to place new ones.

Other budget items (e.g vehicles) can still be placed.

Object Budget

Budget for all objects such as props and compositions.

AI Budget

Budget for computer-controlled entities such as non-player characters.

Vehicle Budget

Budget for cars, armored vehicles, etc.

System Budget

Budget for respawn points, objectives, arsenals, etc.

### Entity Placement

Once an entity has been selected in the Entity Browser, it can be placed in the world.

| Keyboard Controls | Gamepad Controls | Description |
| --- | --- | --- |
| Left Mouse Button | ↧ | Place |
| `Space` | Hold ↧ | Place as a player |
| `Ctrl` + Left Mouse Button | ↝ + ↧ | Place and keep placing |
| `⇧ Shift` | ↝ + ⇄ | Rotate |
| `Alt` Move vertically | ↝ + ⇅ | Move vertically |
| N/A | ↝ + ↺ | Snap to surface |
| Right Mouse Button | ↦ | Cancel placing |

## Gamepad-Specific Menu

Holding ↤ will open a radial menu which content depends on what is under the cursor.

Possible options:

| Action | Keyboard equivalent |
| --- | --- |
| Ping | `Y` |
| Move Camera | `F` |
| Teleport Player | `Space` |
| Take Control | `C` |
| Create new AI group | Right Mouse Button option (on AI unit(s)) |
| Snap to terrain | Right Mouse Button option |
| Play | `Space` |
| Heal | Right Mouse Button option |
| Resupply | Right Mouse Button option |
| Start Bleeding | Right Mouse Button option |
| Stop Bleeding | Right Mouse Button option |
| Cast Lightning | 2× `G` |
| Neutralize | `End` |
| Find in Entity Browser | N/A |
| Edit Properties | 2× Left Mouse Button |

## Waypoints

A waypoint is a move/action order provided to an AI **group**.

⚠

Not to be confused with **Objectives**, which are "waypoints" but for factions.

␼

The waypoints menu is accessible by selecting the group then holding ↥.

| Name | Keyboard Shortcut | Prefab | Description | Priority | Radius | Timeout | Behaviour Tree |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Move | `Alt` + `1` | [E\_AIWaypoint\_Move.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_Move.et) | * Simple move waypoint * Completes once the group leader reaches the waypoint radius. | 0.0 | 5.0 | N/A | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) |
| Forced Move | `Alt` + `2` | [E\_AIWaypoint\_ForcedMove.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_ForcedMove.et) | * Move waypoint ignoring autonomous behavior * Completes once the group leader reaches the waypoint radius. | 2000.0 | 5.0 | N/A | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) |
| Move Relaxed | `Alt` + `3` | [E\_AIWaypoint\_Patrol.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_Patrol.et) | * Like simple move only different move type, slower | 0.0 | 5.0 | N/A | [WP\_Patrol.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Patrol.bt) |
| Search and Destroy | `Alt` + `4` | [E\_AIWaypoint\_SearchAndDestroy.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_SearchAndDestroy.et) | * Timed waypoint, for which duration the waypoint radius must not contain any known enemies. When it does during that time, the timer resets. * AI investigate area inside given waypoint radius. | 0.0 | 20.0 | 600.0 | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) > [WP\_SearchAndDestroy.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_SearchAndDestroy.bt) |
| Defend | `Alt` + `5` | [E\_AIWaypoint\_Defend.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_Defend.et) | * Timed waypoint for which duration AI will stand guard inside given radius. * By default never completes | 0.0 | 30.0 | Never completes | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) > [WP\_Defend.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Defend.bt) |
| Get In | `Alt` + `6` | [E\_AIWaypoint\_GetInNearest.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_GetInNearest.et) | * Timed waypoint for which duration AI checks inside WP radius for available functional vehicle to mount * Completes once all units have found a position and mounted, or times out. * It is possible to adjust boarding parameters in prefab (driver/gunner/cargo allowance) | 0.0 | 20.0 | 30.0 | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) > [WP\_GetInNearest.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_GetInNearest.bt) |
| Get Out | `Alt` + `7` | [E\_AIWaypoint\_GetOut.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_GetOut.et) | * AI moves to waypoint radius and disembarks * It is possible to adjust boarding parameters in prefab (driver/gunner/cargo allowance) | 0.0 | 9.0 | N/A | [WP\_Move.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Move.bt) > [WP\_GetOut.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_GetOut.bt) |
| Suppressive Fire | `Alt` + `8` | [E\_AIWaypoint\_Suppress\_Editor.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/Auto/AI/Waypoints/E_AIWaypoint_Suppress_Editor.et) | * Units shoot at WP position * waypoint does not complete | 2000 | N/A | Never completes | [WP\_Suppress.bt](enfusion://BehaviorEditor/~ArmaReforger:AI/BehaviorTrees/Waypoints/WP_Suppress.bt) |
