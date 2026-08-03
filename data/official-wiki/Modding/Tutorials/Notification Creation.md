# [Notification Creation](https://community.bistudio.com/wiki/Arma_Reforger:Notification_Creation)

This page explains how to create and declare a new notification for it to be used anywhere.

## Enum Declaration

Open the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) enum and add a new entry to the list.
Make sure that it has a unique int assigned to it! Also please add a description to it so people know what the notification is about and what variables it needs.

## Config Modding

Notifications are configured in [Notifications.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Notifications/Notifications.conf).
This config holds all the notifications and is added to the [PlayerController](enfusion://ScriptEditor/scripts/Game/generated/Player/PlayerController.c;16).

### Add a Notification

Press the **+** button to add a new notification to the config.

Below is a list of *most* of the notification classes (some may be missing and classes for specific notifications will not be included).
Use this table to choose the correct Class and add it to the list. Outside of [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master") you would most likely need one of the following:

* [SCR\_NotificationDisplayData](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationDisplayData.c;6)
* [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9)
* [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9)

| Class | Description | Parameters | Format | Code Example |
| --- | --- | --- | --- | --- |
| [SCR\_NotificationDisplayData](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationDisplayData.c;6) | A simple notification targeting nothing and displaying text | None | N/A | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification); ``` |
| [SCR\_NotificationEditableEntity](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationEditableEntity.c;6) | A notification that can hold the data of an EditableEntity | * Param1: EditableEntityID | * %1: EditableEntity Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, editableEntityID); ``` |
| [SCR\_NotificationEditableEntityEditableEntityTarget](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationEditableEntityEditableEntityTarget.c;7) | A notification that can hold the data of two EditableEntities | * Param1: EditableEntityID * Param2: Target EditableEntityID | * %1: EditableEntity Name * %2: Target EditableEntity Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, editableEntityID, targetEditableEntityID); ``` |
| [SCR\_NotificationEditableEntityTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationEditableEntityTargetPlayer.c;7) | A notification that can hold the data of an EditableEntity and target Player | * Param1: EditableEntityID * Param2: Target PlayerID | * %1: EditableEntity Name * %2: Target Player Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, editableEntityID, targetPlayerID); ``` |
| [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9) | A notification that can hold the data of a Player | * Param1: PlayerID | * %1: Player Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, playerID); ``` |
| [SCR\_NotificationPlayerAndFaction](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerAndFaction.c;7) | A notification that can hold the data of a Player and its faction | * Param1: PlayerID | * %1 Player Name * %2 Faction Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, playerID); ``` |
| [SCR\_NotificationPlayerTargetEditableEntity](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetEditableEntity.c;7) | A notification that can hold the data of an Player and target EditableEntity | * Param1: PlayerID * Param2: Target EditableEntityID | * %1 Player Name * %2 Target EditableEntity Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, playerID, targetEditableEntityID); ``` |
| [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9) | A notification that includes a Player and target Player | * Param1: PlayerID * Param2: Target PlayerID | * %1: Player Name * %2: Target Player Name | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, playerID, targetPlayerID); ``` |
| [SCR\_NotificationNumber](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationNumber.c;7) | A notification that includes a number (int). ⓘ  The number it takes is an Int. You can however multiply a float to a max of 1000 (10 means float: 0.0, 100: 0.00 and 1000: 0.000) and set the Number Divider in the Notification to the same as you multiply the number to display it correctly. | * Param1: Number * Param2: Number (Optional) * Param3: Number (Optional) * Param4: Number (Optional) * Param5: Number (Optional) | * %1: Number | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, number); // up to SCR_NotificationsComponent.SendToEveryone(ENotification, number1, number2, number3, number4, number5); ``` |
| [SCR\_NotificationPlayerAndNumber](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerAndNumber.c;8) | A notification that includes the player name and a number (int). ⓘ  The number it takes is an Int. You can however multiply a float to a max of 1000 (10 means float: 0.0, 100: 0.00 and 1000: 0.000) and set the Number Divider in the Notification to the same as you multiply the number to display it correctly. | * Param1: PlayerID * Param2: Number * Param3: Number (Optional) * Param4: Number (Optional) * Param5: Number (Optional) | * %1: Player Name * %2: Number | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, PlayerID, number); // up to SCR_NotificationsComponent.SendToEveryone(ENotification, PlayerID, number1, number2, number3, number4); ``` |

ⓘ

See [#New Notification System Class](#New_Notification_System_Class) to find out how to create a new class.

### Set the Key

Now scroll to the newly created notification and set the Notification Key to the enum created earlier.
Each notification needs to have its own unique key as it is used to get the Notification information from a database.

### Fill the Info Class

Next up click the button to create the Info class. There are currently three classes from which to choose.

| Class | Description |
| --- | --- |
| [SCR\_UINotificationInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_NotificationUIInfo.c;2) | The default notification |
| [SCR\_ColoredTextNotificationUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_ColoredTextNotificationUIInfo.c;1) | Similar to the default notification but the text can be colored |
| [SCR\_SplitNotificationUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_SplitNotificationUIInfo.c;1) | A split notification. It has text left and right of the icon. Think of a "player kills player" notification, e.g: "PlayerKillerName" [Icon] "PlayerKilledName". This one has some additional logic to it explained in [Notification Color](#Notification_Color). |

The **Name** variable is the actual notification that you want to display. What format you can use depends on the class chosen earlier.

For example:

* [SCR\_NotificationDisplayData](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationDisplayData.c;6) - only takes plain text
* [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9) - displays %1 as player name
* [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9) - displays %1 as player name and %2 as target player name

[SCR\_ColoredTextNotificationUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_ColoredTextNotificationUIInfo.c;1) additional variables

| Variable | Description |
| --- | --- |
| Notification text color | Works the same as the notification colour. Takes a [ENotificationColor](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotificationColor.c;4) and instead of colouring the widgets it will color the text. |

[SCR\_SplitNotificationUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_SplitNotificationUIInfo.c;1) additional variables

| Variable | Description |
| --- | --- |
| Name | Unlike the default notification the Name in the UI info is the left side of the split message. The center is the icon and right is the right message. It still works similar to the normal message and can take params. |
| Split Notification Right Message | The right side of the split notification message. It still works similar to the normal message and can take params. |
| Left notification Text color | Works very similar to Notification text color in the ColoredTextNotificationUIInfo as it will color the text but only the **left** part. If using any faction colored it will use Param1 to check faction color instead of using param2 (target). Only relevant for targeted notifications. |
| Right notification Text color | Works very similar to Notification text color in the ColloredTextNotificationUIInfo as it will color the text but only the **right** part. If using any faction colored it will use param2 (target) to check the color. Only relevant for targeted notifications. |
| Replace Left Color With Right Color If Factions Are Ally | A mouthful but as descriptive as they get. If any of the FACTION specific [ENotificationColor](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotificationColor.c;4) enums are used it AND there are two entity targets (player or other entity) then it will check. Is param1 entity an ally of param2 entity. If they are allied then the color of param1 becomes the same as param2. This is to show for example teamkills. |

#### Notification Colour

Notifications colours are listed in an enum found in [ENotificationColor](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotificationColor.c;4).
The enum is a key used to get the information data.
In general the colors are straightforward though the FACTION\_RELATED\_FRIENDLY\_IS\_NEGATIVE and FACTION\_RELATED\_FRIENDLY\_IS\_POSITIVE have some additional logic attached to them - see below.

| Enum Value | Description |
| --- | --- |
| NEUTRAL | A neutral-coloured notification |
| WARNING | A warning-coloured notification |
| POSITIVE | A positive-coloured notification |
| NEGATIVE | A negative-coloured notification |
| GM | A Game Master-colored notification which all players can see |
| GM\_ONLY | A Game Master-only coloured notification which all Game Masters can see (and only them) |
| FACTION\_FRIENDLY\_IS\_NEGATIVE | If using Notification class that has a player or EditableEntity then it will check is the Entity faction (or if there are two entities then the target entity) is friendly to the player that receives the notification. If friendly the notification will get the NEGATIVE color else it will get POSITIVE. If either the player that receives the notification or the entity that is checked has no faction then it will display NEUTRAL.  Think of "player died".   * If friendly it is negative * If enemy it is positive |
| FACTION\_FRIENDLY\_IS\_POSITIVE | If using Notification class that has a player or EditableEntity then it will check is the Entity faction (or if there are two entities then the target entity) is friendly to the player that receives the notification. If friendly the notification will get the POSITIVE color else it will get NEGATIVE. If either the player that receives the notification or the entity that is checked has no faction then it will display NEUTRAL.  Think of new player joined the same faction as the player that receives the notification.   * If new friendly player joined it is positive * If new enemy player joined it is negative |
| FACTION\_ENEMY\_IS\_NEGATIVE\_ONLY | Similar to FACTION\_FRIENDLY\_IS\_NEGATIVE but will set color to NEUTRAL if faction is not friendly |
| FACTION\_FRIENDLY\_IS\_POSITIVE\_ONLY | Similar to FACTION\_FRIENDLY\_IS\_POSITIVE but will set color to NEUTRAL if faction is not friendly |

#### Editor Set Position Data

This is an editor only setting and will essentially allow the Game Master to move to the notification's target.
This is really only relative if the notification has a position attached to it or an entity (Player or EditableEntity).
In most cases it is fine to keep it on default: AUTO\_SET\_AND\_UPDATE\_POSITION. The table below explains what each means.

| Enum | Description |
| --- | --- |
| AUTO\_SET\_AND\_UPDATE\_POSITION | If the notification has a target then the position is set and updated; if the target moves, so does the location attached to the notification. If the Game Master clicks on the notification the camera will go to the current location of the target. |
| AUTO\_SET\_POSITION\_ONCE | The position of the notification is only set once  when the notification is created and will never update. This is used for notifications such as "Player died". The Game Master does not care about the current location of that player but would want to see where the player died. |
| NEVER\_AUTO\_SET\_POSITION | Never set the notification position. This is used for positionless events, e.g if a player joins or leaves - the position is irrelevant and does not need to be set. |

## Send Notification

The only thing left is sending out the notification now that the notification data is created.

To do this call one of the following [SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76) methods:

| Method | Server Only | Description | Parameters | Example |
| --- | --- | --- | --- | --- |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendToEveryone(); | Checked | Sends to notification to every player | * First variable is the ENotification key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToEveryone(ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendToPlayer(); | Checked | Send the notification to specific players | * First variable is the specific Player ID the notification needs to be send to * Second variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToPlayer(PlayerID, ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendLocal(); | Unchecked | Sends the notification locally | * First variable is the ENotification key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendLocal(ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendToGameMasters(); | Checked | Sends the notification only to all game masters | * First variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToGameMasters(ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendToGameMastersAndPlayer() | Checked | Sends the notification to all game masters as well as the given player ID regardless if it is GM or not | * First variable is the specific Player ID the notification needs to be send to along with the GMs * Second variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToGameMastersAndPlayer(PlayerID, ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendLocalGameMaster(); | Unchecked | A combination of cSendLocal() and cSendToGameMasters(). Sends the notification local but player will only receive it has GM rights | * First variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendLocalGameMaster(ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendLocalNonGameMaster(); | Unchecked | Similar to cSendLocalGameMaster() but the player will only receive the notification if they are NOT game master | * First variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendLocalNonGameMaster(ENotification, NotificationParameters); ``` |
| c[SCR\_NotificationsComponent](enfusion://ScriptEditor/scripts/Game/Network/Notifications/SCR_NotificationsComponent.c;76).SendToGroup(); | Checked | Sends a notification to all players in a given group | * First variable is the specific group ID the players need to be a part of to receive the notification * Second variable is the [ENotification](enfusion://ScriptEditor/scripts/Game/Network/Notifications/ENotification.c;4) key * Other variables depend on the SCR\_Notification\* class chosen earlier | ENFORCECODEMARKER   ``` SCR_NotificationsComponent.SendToGroup(groupID, ENotification, NotificationParameters); ``` |

## Create a New SCR\_Notification

It is possible to create a new notification class by creating a class that inherits from [SCR\_NotificationDisplayData](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationDisplayData.c;6).
This class already has a lot of function most of which you do not have to worry. Below in the table you find three of the most used.
In general however you can simply inherit from [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9), [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9) or [SCR\_NotificationPlayerAndFaction](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerAndFaction.c;7) and override the cGetText() method.

| Method | Description |
| --- | --- |
| cGetText() | This returns the string that is actually displayed and sets the correct formatting such as getting the player name and adding it. Simple example of [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9):  ENFORCECODEMARKER   ``` int playerID, playerTargetID; data.GetParams(playerID, playerTargetID); return string.Format(m_info.GetName(), GetPlayerName(playerID), GetPlayerName(playerTargetID)); ```  This sets %1 to the name of the player with *playerID* and %2 to the name of the player with *playerTargetID*. |
| cSetFactionRelatedColor() | If the FACTION\_RELATED\_FRIENDLY\_IS\_NEGATIVE or FACTION\_RELATED\_FRIENDLY\_IS\_POSITIVE colors are used then the notification needs to know what they need to check against the notification receiving player faction.   ENFORCECODEMARKER   ``` int playerID; data.GetParams(playerID); data.SetFactionRelatedColor(GetFactionRelatedColorPlayer(playerID, m_info.GetNotificationColor())); ```     ColoredTextNotifications and SplitNotifications have additional logics for these where they can set the faction color of texts. Check [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9) for an example. |
| cSetPosition() | Editor only, to help out the editor team make sure that the entity position is set or inherit for example from either [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9) or [SCR\_NotificationPlayerTargetPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayerTargetPlayer.c;9) which sets the position of the player if the first (or second) parameter is a *playerID*. Here is the example found in [SCR\_NotificationPlayer](enfusion://ScriptEditor/scripts/Game/Network/Notifications/NotificationDisplayData/SCR_NotificationPlayer.c;9):    ENFORCECODEMARKER   ``` int playerID; data.GetParams(playerID); SetPositionDataPlayer(playerID, data); ``` |

ⓘ

* There are three notifications message prefab: BaseNotificationMessage, NotificationMessage and NotificationMessageSplit;
* Both NotificationMessage and NotificationMessageSplit inherit from BaseNotificationMessage.

## Localisation

Select [Notifications.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Notifications/Notifications.conf) in the Resource Browser and open the **Localisation Parser plugin** by pressing `Ctrl` + `Alt` + `L` (or click on it in [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager")'s **Plugins** dropdown menu).

This opens a window where the **Config Path** field should aim at [Editor.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/LocParser/Editor.conf).

ⓘ

* It is possible to set the target stringtable with the **String Table Override** field.
* It is possible to override the config's AR- translation key prefix using the **Prefix Override** field.

Clicking **Logs** simulates the localisation process and logs results and warnings in the Log Console (press `F1` to display it if hidden).

Clicking **Localize' *will edit the translation, taking the values originally set in the field to the* EN-US** translation field with a new localisation key (e.g PREFIX-Notification\_ENUM\_NAME).

⚠

While the new stringtable translations are automatically saved in [String Editor](/wiki/Arma_Reforger:String_Editor "Arma Reforger:String Editor"), be sure to apply the newly created translation keys to the config as this is not done automatically.
