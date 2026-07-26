# [Game Master: Entity Tooltip Creation](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master:_Entity_Tooltip_Creation)

[![](/wikidata/images/thumb/e/ea/armareforger_entity-tooltip-example.png/300px-armareforger_entity-tooltip-example.png)](/wiki/File:armareforger_entity-tooltip-example.png)

An entity tooltip in action.

This is a step by step guide on how to create tooltips for Entities that can be viewed when focusing on Editable Entities in [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master").

The entity name as well as the icon and image are set within the [SCR\_EditableEntityComponent](enfusion://ScriptEditor/scripts/Game/Editor/Components/EditableEntity/SCR_EditableEntityComponent.c;13) on the (editable) entity itself.

## Script Creation

Create a new class inheriting from [SCR\_EntityTooltipDetail](enfusion://ScriptEditor/scripts/Game/Editor/UI/Components/Tooltips/Tooltips/Details/SCR_EntityTooltipDetail.c;1) in Data\scripts\Game\Editor\UI\Components\Tooltips\Tooltips\Details.

There are a few methods that are important for a basic tooltip detail:

| Method | Description |
| --- | --- |
| cInitDetail() | This method checks if the tooltip can be displayed as well as getting any references to the (content) layout part of the tooltip detail using the given widget. Return true if the tooltip is to be displayed. Note that once the condition is met the tooltip detail cannot be hidden until the player stops focusing on the entity, so take that into account. |
| cNeedUpdate() | It will update the tooltip detail if you return it true. |
| cUpdateDetail() | This is called after init and if cNeedUpdate() returns true. Here you can update the content part of the tooltip detail by changing the widgets. |

## Config Modding

Open the Entity Tooltip config [EntityTooltips.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/Tooltips/EntityTooltips.conf) located in \Data\Configs\Editor\Tooltips; all entity tooltips share the same config.

The Entity Tooltip config holds an array of all Editable Entity types. Each Entity Type has their own tooltips that are displayed when hovering over it.
Each of the Entity Type elements holds once again an array of tooltip Details. Generally each line you see in the tooltip is its own detail that can be shown, hidden and changed depending on the entity conditions.  
The layout of a tooltip detail is divided into two parts: the **label** layout and the **content** layout.

| Layout | Description |
| --- | --- |
| Label | The label is spawned by default and holds the Display Name of the tooltip detail as well as a widget which will hold the content layout. |
| Content | This is the more unique part of the tooltip which you can change. In general this is a Text widget but it can be anything you like such as a slider. |

### Add to Config

Locate the correct entity type with which to display the tooltip detail. Add the tooltip detail within their array - note that the order in the array is the same as displayed in the actual tooltip.

Make sure to set the following default variables:

| Variable | Description |
| --- | --- |
| Display Name | This is the display name which will be shown on the label of the tooltip detail |
| Layout | This is the actual content of the tooltip detail. (cInitDetail() has a reference to this Widget) You can use any of the existing Layouts (\Data\UI\layouts\Editor\Tooltips\TooltipPrefabs\) or create your own. |
| Show Label | If false it will not show a label. Display Name in this case is not needed as it is hidden. |

ⓘ

Most tooltip details have more variables than the default, be sure to check them for examples.
