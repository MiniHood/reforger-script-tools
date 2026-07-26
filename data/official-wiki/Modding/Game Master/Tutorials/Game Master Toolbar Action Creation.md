# [Game Master: Toolbar Action Creation](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master:_Toolbar_Action_Creation)

We will be creating a specific General Toolbar action.
The Faction and Command toolbars are quite similar to the General Toolbar and we will touch upon them as well though these also interact with either the faction or the AI group.

## Script Creation

As we will be creating a general toolbar action we will need to inherit from one of the Toolbar classes:

| Class | Description |
| --- | --- |
| [SCR\_EditorToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_EditorToolbarAction.c;2) | Basic functionality for toolbar actions. You click the button and something happens, very similar to Context actions (see also [Context Action Creation tutorial](/wiki/Arma_Reforger:Game_Master:_Context_Action_Creation "Arma Reforger:Game Master: Context Action Creation")) |
| [SCR\_BaseToggleToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_BaseToggleToolbarAction.c;2) | A toggle action which can be toggled and untoggled; though in general the functionality is quite similar to SCR\_EditorToolbarAction |
| [SCR\_BaseCommandAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/CommandActions/SCR_BaseCommandAction.c;1) | Used for Faction and Command bar actions that take entity targets and have a built-in function to spawn a prefab. |

### Methods

There are a few functions to take into account:

| Method | Description |
| --- | --- |
| cCanBeShown() | Whether or not the action will be shown in the toolbar. The action cannot be performed (not even with shortcut) if it is not shown, but you can set it up in such a way that the action is shown/hidden depending on events. |
| cCanBePerformed() | Whether or not the action can be performed. The action cannot be selected or executed if false but the button is still shown. |
| cPerform() | Here you actually execute the action. Write the code to bed performed here. |
| cIsServer() | Set the return value to true/false if you want the action to be executed on server/locally |
| cTrack() | [SCR\_BaseToggleToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_BaseToggleToolbarAction.c;2) only. Track events which influence the action. |
| cUntrack() | [SCR\_BaseToggleToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_BaseToggleToolbarAction.c;2) only. Untrack events which influence the action. |
| cToggle() | [SCR\_BaseToggleToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_BaseToggleToolbarAction.c;2) only. Toggle action state. To be called by inherited classes. |

## Config Modding

### Add to Config

Locate the wanted config in which you want to create your Toolbar Action, and press the **+** button to add a new item to its array.

| Folder | Config | Description |
| --- | --- | --- |
| Data\Configs\Editor\ActionLists\Toolbar | [EditToolbar.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Toolbar/EditToolbar.conf) | General Toolbar for Edit mode |
| [PhotoToolbar.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Toolbar/PhotoToolbar.conf) | General Toolbar for Photo mode |
| [AdminToolbar.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Toolbar/AdminToolbar.conf) | General Toolbar for Admin mode |
| [SharedToolbar.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Toolbar/SharedToolbar.conf) | Toolbar actions that are shared between all the above modes |
| Data\Configs\Editor\ActionLists\Command | [Command.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/ActionLists/Command/Command.conf) | Toolbar for both Faction as well as Command |

### UI Info

Now that the action is added you will have to fill in the variables. We will start with the UI Info. Press the Set Class button and select [SCR\_UIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_UIInfo.c;6) from the list.

This will get you the following UI Info variables to fill in:

| Variable | Description |
| --- | --- |
| Name | Action name |
| Description | Description when focusing on action |
| Icon | Icon of action |

### Other Variables

| Variable | Description |
| --- | --- |
| Action Group | Makes sure actions are group together. Each enum is their own group divided by a divider. |
| Action Type | Purely visible and decides which widget the action has.  * ACTION: One click executes the action. * TOGGLE: The action can be toggled on and off (Though ACTION can also be used for this) * DYNAMIC: The action is only shown when certain conditions are met. Like placing a character as Player. |
| Command Prefab | [SCR\_BaseCommandAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/CommandActions/SCR_BaseCommandAction.c;1) only. The prefab that will be spawned when toggling the button. |
| Effects | Array of effects when action is performed, things such as Sound and Particle effects. |
| Enable Shortcut Logics | Will allow the "Shortcut" to work if true. You might want to execute the shortcut action from a different location. So "Shortcut" is purely informative. |
| Info Toggled | [SCR\_BaseToggleToolbarAction](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Actions/ToolbarActions/SCR_BaseToggleToolbarAction.c;2) only. The Name, Description and Icon of a toggled button. |
| Order | This dictates the order of the action within their respected Action Group (They can have no action group). The higher the number the closer the action is in the left compaired to other actions. Actions with the same order are ordered the same as in the config array. |
| Shortcut | Add an Input action name if you want the action also to be executed via short cut. Note that cCanBePerformed() will still be checked. |
