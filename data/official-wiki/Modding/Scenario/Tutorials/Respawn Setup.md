# [Respawn Setup](https://community.bistudio.com/wiki/Arma_Reforger:Respawn_Setup)

## Game Mode Setup

[![](/wikidata/images/thumb/e/ec/armaR_respawn-system-RespawnSystemComponent.png/300px-armaR_respawn-system-RespawnSystemComponent.png)](/wiki/File:armaR_respawn-system-RespawnSystemComponent.png)

The Spawn Logic can be found and set under Game Mode components\SCR\_RespawnSystemComponent\**Respawn System**.

1. Make sure the game mode's [SCR\_RespawnSystemComponent](enfusion://ScriptEditor/scripts/Game/GameMode/Respawn/SCR_RespawnSystemComponent.c;7) component has Spawn Logic set (under Respawn System).  
   Spawn Logic is an object responsible for defining respawn logic.
2. You can override [SCR\_SpawnLogic](enfusion://ScriptEditor/scripts/Game/Respawn/Logic/SCR_SpawnLogic.c;17) to define your custom respawn behaviour.  
   Right now, there are two Spawn Logic objects defined:
   * [SCR\_AutoSpawnLogic](enfusion://ScriptEditor/scripts/Game/Respawn/Logic/SCR_AutoSpawnLogic.c;5) - spawns the player automatically after assigning random faction, loadout and spawn point. You can also define Forced Faction and Forced Loadout, which will assign the player with a specific faction and loadout.
   * [SCR\_MenuSpawnLogic](enfusion://ScriptEditor/scripts/Game/Respawn/Logic/SCR_MenuSpawnLogic.c;1) - spawning is done via respawn menu. This object notifies the player when they're ready for spawn and opens respawn menu.
3. You can notice that SCR\_RespawnSystemComponent has multiple [SCR\_SpawnHandlerComponent](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Base/SCR_SpawnHandlerComponent.c;13) components as its children. This signifies what types of respawn are possible with the game mode to which it is attached.
   * [SCR\_FreeSpawnHandlerComponent](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Implementation/FreeSpawn/SCR_FreeSpawnHandlerComponent.c;6) - allows for spawning player at a certain position.
   * [SCR\_PossessSpawnHandlerComponent](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Implementation/PossessSpawn/SCR_PossessSpawnHandlerComponent.c;6) - allows for possessing a character by passing its [RplId](enfusion://ScriptEditor/scripts/Core/generated/Replication/RplId.c;13).
   * [SCR\_SpawnPointSpawnHandlerComponent](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Implementation/SpawnPointSpawn/SCR_SpawnPointSpawnHandlerComponent.c;6) - allows for spawning player at a spawn point.
4. The Deploy menu got a overhaul too. Instead of using a [SCR\_SuperMenuBase](enfusion://ScriptEditor/scripts/Game/UI/Menu/SubMenu/SCR_SuperMenuBase.c;6) and [SCR\_SubMenuBase](enfusion://ScriptEditor/scripts/Game/UI/Menu/SubMenu/SCR_SubMenuBase.c;6), the deploy menu now consists of 3 separate menus, all inheriting from [SCR\_DeployMenuBase](enfusion://ScriptEditor/scripts/Game/UI/Menu/DeployMenu/SCR_DeployMenuBase.c;6).  
   Opening/closing logic of respawn menu screens is handled in [SCR\_PlayerDeployMenuHandlerComponent](enfusion://ScriptEditor/scripts/Game/Respawn/Menu/SCR_PlayerDeployMenuHandlerComponent.c;14), which is attached to the player controller.
   * [SCR\_WelcomeScreenMenu](enfusion://ScriptEditor/scripts/Game/UI/Menu/DeployMenu/SCR_WelcomeScreenMenu.c;3) - serves as an overview of the current game mode, objectives and rules, similar to what briefing submenu used to be. Opens only once when player joins the game.
   * [SCR\_RoleSelectionMenu](enfusion://ScriptEditor/scripts/Game/UI/Menu/DeployMenu/SCR_RoleSelectionMenu.c;2) - here you can select your faction, group and a loadout. Opens only for the initial setup when the player does not yet have an assigned faction.
   * [SCR\_DeployMenuMain](enfusion://ScriptEditor/scripts/Game/UI/Menu/DeployMenu/SCR_DeployMenuBase.c;107) - main deploy menu with map for selecting spawning points, but you are able to change your loadout and group as well. Opens after player's death.

## Requesting Respawn

If you need to request respawn, you can do easily by creating a new [SCR\_SpawnData](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Base/SCR_SpawnData.c;9) class and passing it to the c[SCR\_RespawnComponent](enfusion://ScriptEditor/scripts/Game/GameMode/Respawn/SCR_RespawnComponent.c;17).RequestSpawn([SCR\_SpawnData](enfusion://ScriptEditor/scripts/Game/Respawn/RequestHandling/Base/SCR_SpawnData.c;9) data) method.

Here are examples for requesting respawn for aforementioned spawn handlers:

SCR\_FreeSpawnHandlerComponent
:   ENFORCECODEMARKER

    ```
    void RequestFreeSpawn()
    {
    	ResourceName prefab = "{84B40583F4D1B7A3}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_Rifleman.et";
    	SCR_FreeSpawnData data = new SCR_FreeSpawnData(prefab, "0 0 0");
    	m_RespawnComponent.RequestSpawn(data);
    }
    ```

SCR\_PossessSpawnHandlerComponent
:   ENFORCECODEMARKER

    ```
    void RequestPossessEntity(IEntity ent)
    {
    	SCR_PossessSpawnData data = SCR_PossessSpawnData.FromEntity(ent);
    	m_RespawnComponent.RequestSpawn(data);
    }
    ```

SCR\_SpawnPointSpawnHandlerComponent
:   ENFORCECODEMARKER

    ```
    void RequestSpawnPointSpawn(SCR_BasePlayerLoadout loadout, SCR_SpawnPoint spawnPoint)
    {
    	SCR_SpawnPointSpawnData data = new SCR_SpawnPointSpawnData(loadout.GetLoadoutResource(), spawnPoint.GetRplId());
    	m_RespawnComponent.RequestSpawn(data);
    }
    ```
