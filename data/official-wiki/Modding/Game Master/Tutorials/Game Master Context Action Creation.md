# [Game Master: Context Action Creation](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master:_Context_Action_Creation)

Context actions are actions the [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master") can do by opening the context menu such as Cast lighting, Delete entity, etc.
Context actions change depending on if the Game Master is focusing on an entity and on the type of entities.
The following is a guide about how to create new context actions.

## Script Creation

To create a context action you will need to create a new class inheriting from [SCR\_GeneralContextAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ContextActions/SCR_GeneralContextAction.c;2).

There are three main methods you will be using:

| Method | Description |
| --- | --- |
| cCanBeShown() | Whether or not the context action will be shown in the menu. Check here if the hovered/selected entities are the ones your action requires. The action cannot be performed (not even with the shortcut) if it is not shown. |
| cCanBePerformed() | Whether or not the action can be performed. The action will be shown but locked. By the time of writing only gamepad shows locked actions. This will also be checked for Shortcuts. |
| cPerform() | Here you actually execute the action. Write the code to perform the action here |

## Config Modding

Context actions are (generally) mode-specific so decide in which mode(s) the player can execute the action.
Context actions configs are located in \Data\Configs\Editor\ActionLists\Context.

| Config | Description |
| --- | --- |
| [TempEdit.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Context/TempEdit.conf) | Edit mode config |
| [TempAdmin.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Context/TempAdmin.conf) | Admin mode config |
| [TempPhoto.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Context/TempPhoto.conf) | Photo mode config |
| [Shared.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Context/Shared.conf) | All modes config |
| [Tasks.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Context/Tasks.conf) | Player tasks config |

Next, locate the config you want and press the **+** icon to add your own context action.

### UI Info

Now that the action is added you will have to fill in the variables. We will start with the Info. Press the Set Class button and select SCR\_UIInfo from the list.

This will get you the following UI Info variables to fill in:

| Variable | Description |
| --- | --- |
| Name | Action's display name |
| Description | Tooltip description when focusing on the action |
| Icon | Action's icon |

### Additional Variables

The base class has some additional variables.

ⓘ

More advanced classes, located in Data\scripts\Game\Editor\Containers\Actions\ContextActions\ (like [SCR\_DoubleClickAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ContextActions/SCR_DoubleClickAction.c;1)) exist that may offer more options than the default ones listed here but these will not be covered.

| Variable | Description |
| --- | --- |
| Enabled | Simply if this config action is to be considered or not. Useful for debug/development purpose. |
| Info | See [UI Info](#UI_Info) above. |
| Action Type | Purely visible and only used in tool bars. Keep on ACTION for context actions. |
| Action Group | Makes sure actions are group together. Keep NONE for context actions. |
| Order | This dictates the order of the action. The higher the number the higher the action is in the list. Actions with the same order are ordered the same as in the config array. |
| Shortcut | Add an Input action name (defined in [chimeraInputCommon.conf](enfusion://ResourceManager/~ArmaReforger:Configs/System/chimeraInputCommon.conf), e.g ManualCameraTeleportPlayer) if you want the action also to be executed via shortcut. Note that cCanBePerformed() will still be checked. |
| Enable Shortcut Logics | Will allow the "Shortcut" to work if true. You might want to execute the shortcut action from a different location. So "Shortcut" is purely informative. |
| Effects | Array of effects when the action is performed such as sound, particle, UI effects. |
| Cooldown Time | Cooldown to prevent spamming of action. Leave zero or less to ignore, duration is in seconds. |
| Show On Cooldown Notification | If true will show notification if trying to perform the action if the cooldown is active. |
| IsServer | The action is performed on the server if this is true, otherwise it is performed locally. Note that cCanBeShown() and cCanBePerformed() are still checked locally. |
| Hide On Hover | The action is hidden when the cursor is hovering above an entity. |
| Hide On Selected | The action is hidden if any entity is selected. |

ⓘ

It is possible to make shortcut-only context actions (that do not show up in the context menu) by not filling in the UIInfo but still filling in the Shortcut and having Enable Shortcut Logics ticked.
