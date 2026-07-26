# [General Game Mode Setup](https://community.bistudio.com/wiki/Arma_Reforger:General_Game_Mode_Setup)

The **SCR\_BaseGameMode** entity is the lowest usable game mode implementation that allows you to set up a custom game mode.

The core idea behind [SCR\_BaseGameMode](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_BaseGameMode.c;138) is the expansion via specialised components of [SCR\_BaseGameModeComponent](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_BaseGameModeComponent.c;8) type.

These provide the user with API that allows hooking onto certain game events as necessary. Many provided components can work standalone and can be mixed and matched.

## Example Scenarios

To easily get familiar with how game modes can be set up now, please see the following scenarios:

| Mode | Link | Description |
| --- | --- | --- |
| **Plain** | [MpTest.ent](enfusion://ResourceManager/~ArmaReforger:worlds/MP/MpTest.ent) | * Simplest scenario * Automatic respawn * Free for all |

### Basic Setup

To setup the most basic scenario the user must add the following. For the simplest set-up add all items from the **"Pre-made"** column.

#### Free for All

| Type | Pre-made Prefab | Base Prefab | Description |
| --- | --- | --- | --- |
| **SCR\_BaseGameMode** | [GameMode\_Plain.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Modes/Plain/GameMode_Plain.et) | [GameMode\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Modes/GameMode_Base.et) | Specialises game mode by using components and their combinations. Must always have a *SCR\_RespawnSystemComponent* and specialised *SCR\_RespawnHandlerComponent* to allow proper spawning. See [Requirements](#Requirements). |
| **SCR\_FactionManager** | [FactionManager\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_FFA.et) | [FactionManager\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_Base.et) | Provides list of available factions and their properties. For free for all modes, use the FFA faction. |
| **SCR\_LoadoutManager** | [LoadoutManager\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_FFA.et) | [LoadoutManager\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_Base.et) | Provides list of available loadouts and their properties. For free for all modes, use the FFA faction. |
| **SCR\_SpawnPoint** | [SpawnPoint\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_FFA.et) | [SpawnPoint\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_Base.et) | Provides information to the respawn system, marking a location available for spawning at. Set per faction. For free for all modes, use the FFA faction. |

#### Faction Based

| Type | Pre-made Prefab | Base Prefab | Description |
| --- | --- | --- | --- |
| **SCR\_BaseGameMode** | [GameMode\_Plain.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Modes/Plain/GameMode_Plain.et) | [GameMode\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Modes/GameMode_Base.et) | Specialises game mode by using components and their combinations. Must always have a *SCR\_RespawnSystemComponent* and specialised *SCR\_RespawnHandlerComponent* to allow proper spawning. See [Requirements](#Requirements). |
| **SCR\_FactionManager** | [FactionManager\_USxUSSR.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_USxUSSR.et) | [FactionManager\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_Base.et) | Provides list of available factions and their properties. |
| **SCR\_LoadoutManager** | [LoadoutManager\_USxUSSR.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_USxUSSR.et) | [LoadoutManager\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_Base.et) | Provides list of available loadouts and their properties. |
| **SCR\_SpawnPoint** | [SpawnPoint\_USSR.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_USSR.et) (USSR) [SpawnPoint\_US.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_US.et) (US) | [SpawnPoint\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_Base.et) | Provides information to the respawn system, marking a location available for spawning at. Set per faction. |

## Requirements

ⓘ

See [Respawn Setup](/wiki/Arma_Reforger:Respawn_Setup "Arma Reforger:Respawn Setup") in order to setup a game mode's respawn properly.

## Instances

**SCR\_BaseGameMode**: single instance in the world - mandatory.

Ties game mode together, manages attached **SCR\_BaseGameModeComponent**(s), calls necessary methods.

```enforce
BaseGameMode baseGameMode = GetGame().GetGameMode();
SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(baseGameMode);
```

**SCR\_FactionManager**: single instance in the world - mandatory.

Provides available factions and API for retrieving and finding these by keys or indices.

```enforce
FactionManager baseFactionManager = GetGame().GetFactionManager();
SCR_FactionManager factionManager = SCR_FactionManager.Cast(baseFactionManager);
```

**SCR\_LoadoutManager**: single instance in the world - mandatory.

Provides available loadouts and API for retrieving and finding these by indices.

```enforce
SCR_LoadoutManager loadoutManager = GetGame().GetLoadoutManager();
```

**SCR\_RespawnSystemComponent:** single instance in the world, attached to game mode. Mandatory.

Provides means of setting and request/response player faction, loadout and spawn points.

```enforce
SCR_GameModeBase gameMode; /* = yourInstance; */
SCR_RespawnSystemComponent respawnSystemComponent = gameMode.GetRespawnSystemComponent();
```

**SCR\_RespawnComponent:** Not to confuse with **SCR\_RespawnSystemComponent**!

This component is and should always be attached to a player controller. That means that remote client will always be able to retrieve their **local** SCR\_RespawnComponent only!
The authority will be able to retrieve and iterate over all SCR\_RespawnComponent.

```enforce
int myId; /* = my_local_id */
RespawnComponent baseRespawnComponent = GetGame().GetPlayerManager().GetPlayerRespawnComponent(myId);
SCR_RespawnComponent respawnComponent = SCR_RespawnComponent.Cast(baseRespawnComponent);
// Client:		if myId is local playerId, respawn component is returned. Null otherwise. (Owned PlayerController component only)
// Authority:	if myId is valid playerId, respawn component is returned. Null otherwise. (Any PlayerController component)
```

## Game State

The game has three pre-defined states that can be set by the server and are automatically replicated with appropriate callback to the clients. This can be omitted, leaving the game always in GAME state.

* **PREGAME** (e.g. wait for *n* players?)
* **GAME** (core game loop)
* **POSTGAME** (e.g. show scoreboard, next scenario voting)

For particular implementation and details, see [SCR\_EGameModeState](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_EGameModeState.c;4), [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4) and c[SCR\_BaseGameMode](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_BaseGameMode.c;138).OnGameStateChanged().

### Gamemode End

The game mode transitions into POSTGAME state once the authority deems so by calling the **EndGameMode()** method.

This method accepts an instance of [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4), which is a small serialisable structure that is replicated to all clients and contains somewhat arbitrary data.
This can be used to synchronise the end state (*winner player, winner faction, ...*) to show e.g. the end-game screen.

## Troubleshooting

**My game mode is not working as it was before, errors are logged into the console regarding missing game mode, what do I do?**

Navigate to the [Basic Setup](#Basic_Setup) section.

Drag prefabs from the [Faction Based](#Faction_Based) table if you want to have US versus USSR game mode or Free for All table if FFA is what you are looking for.
