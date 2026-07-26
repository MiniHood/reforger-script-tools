# [Scenario Framework](https://community.bistudio.com/wiki/Arma_Reforger:Scenario_Framework)

The goal of the **Scenario Framework** is to provide scenario creators simple way how to build their scenarios using World Editor without any scripting knowledge.

It is expected that scenario creators are already familiar with the basic usage of World Editor.

However, most of the work is done by drag and dropping components into the world and adjusting the attributes which is the least difficult and with using just that, you can build even more complex scenarios.

As always, people with scripting knowledge can modify and expand the framework to suit their needs.

|  |  |
| --- | --- |
| **Contents**   * 1. [Prerequisites](#Prerequisites) * 2. [Basics](#Basics) * 3. [Log Messages](#Log_Messages) * 4. [Components](#Components) * 5. [Plugins](#Plugins_2) * 6. [Logic](#Logic_2) * 7. [Getters](#Getters) * 8. [Actions](#Actions) * 9. [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn_2) * 10. [Samples](#Samples) * 11. [Compositions](#Compositions) * 12. [QRF System](#QRF_System) * 13. [1.1.0 Structural Changes](#1.1.0_Structural_Changes) | ⓘ  In this documentation, we will be providing links that lead directly to the tools to said items. It is advised to explore said links in the tools yourself so you can get familiar with them as we go. Before clicking on any of the links, you will need to register the Enfusion protocol on your computer - see the [Register enfusion:// protocol](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") section for more information. |

## Prerequisites

In order to properly work with the Scenario Framework, you need to have several things already set in your world that you are already working on.

For this, we have created a very simple way how to setup any functional world on its own using the World Editor Plugin called Game Mode Setup.

Here, you can choose from several Game Mode Templates, but for the Scenario Framework, you will want to use the [ScenarioFramework.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/GameModeSetup/ScenarioFramework.conf) variant.

However if you do not want to use this Prefab, note that it contains the following Prefabs that are needed for an optimal functionality of your scenario:

* [GameModeSF.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Scenario_Framework/GameModeSF.et)
* [SCR\_AIWorld.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/SCR_AIWorld.et)
* [FactionManager\_USxUSSR.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_USxUSSR.et)
* [LoadoutManager\_USxUSSR.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_USxUSSR.et)
* [RadioManager.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Radio/RadioManager.et)
* [PerceptionManager.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/World/Game/PerceptionManager.et)
* [ScriptedChatEntity.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/ScriptedChatEntity.et)
* [TaskManager.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Tasks/TaskManager.et)

and in the Game Mode's Mission Header, [SCR\_MissionHeaderCombatOps](enfusion://ScriptEditor/scripts/Game/Mission/SCR_MissionHeaderCombatOps.c;1) is used.

## Basics

Scenario Framework has a hierarchical setup which upon getting familiar with, you will be able to build your scenarios very quickly and do many interesting things.

Once you have your world properly setup with all the [requirements](#Prerequisites) listed above, you are ready to build.

### GameMode Manager Settings

The core entity of each Scenario is the GameMode entity.

In the case of Scenario Framework, it is recommended to use the [GameModeSF.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Scenario_Framework/GameModeSF.et) Prefab.

It has many components already added to it and they can be adjusted according to your needs.

It is recommended to go through them all and explore the possibilities that each component gives you.

The most important component on this GameMode is the [SCR\_GameModeSFManager](enfusion://ScriptEditor/scripts/Game/ScenarioFramework/GameMode/SCR_GameModeSFManager.c;6) which allows you to use the properties listed below.

#### Tasks

*Task Types Available* - Setup a random task generation on scenario start by setting which Task Types are available

*Max Number Of Tasks* - Maximal number of tasks that will get spawned using this system

*After Tasks Init Actions* - Actions that will be activated after task generation is finished (see [Actions](#Actions))

Here is the example from CombatOps Arland that has 4 possible Task Types set but only 3 of them will be randomly selected upon scenario start

#### Debug

Debug can easily allow you to pick only certain Area and Layer Tasks for you to debug in the midst of your scenario. Useful in a scenario with many elements and depending on randomisation.

Or you can activate predefined sets of Debug Actions to your liking and execute them on the fly.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") *Debug Areas* - List of Areas that will be spawned (Optionally with desired Layer Task) as opposed to leaving it to random generation

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") *Core Areas* - List of Core Areas that are essential for the Scenario to spawn alongside Debug Areas

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") *Debug Areas* - List of Areas that will be spawned (Optionally with desired Layer Task) as opposed to leaving it to random generation

#### Dynamic Spawn/Despawn

*Dynamic Despawn* - In default, it is set to false, but upon setting it to true, it will enable the Dynamic Despawn feature

*Update Rate* - Controls the update rate for the Dynamic Spawn/Despawn on how frequently it is being checked to perform Spawn/Despawn of Areas

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Voice Over Data Config - Config with voice over data that is global through the Scenario and can be used for all Voice Over actions

### Basic Hierarchy Components

After setting up the GameMode entity and possibly other entities (listed above) we can get creative with the main part of the Scenario Framework and that are all the pieces from which the scenarios are built from.

We will go very briefly over the basic hierarchy of how it works, and detailed information about each component are provided.

#### Area

At top of the hierarchy stands so called Area.

Idea behind this is that your scenario is divided into several areas (as you can for example see on CombatOps Arland) and each area is handling its own things and has its purpose.

Area has all the other components under it in its own hierarchy and it also serves with the Dynamic Spawn/Despawn as an enclosure.

#### Layer

Right under the Area is the so-called Layer.

Note that there is a "basic" Layer that serves as a hierarchical tool to further divide things and it can contain other layers or slots.

It is one of the most powerful entities that you will be using when building your scenario because it allows you to set the layout exactly how you will want it and knowing how it works will allow you to create all sorts of scenarios.

#### LayerTask

One of the types of the Layer is a LayerTask which has many subtypes depending on the task it is focused on and it serves to handle the Task creation and workflow.

#### Slot

At the bottom of the hierarchy, there is a Slot.

"Basic" Slot allows you to spawn any prefab which is the main purpose of it, but you do not have to spawn anything and attach some components onto it as well (for example smart action component for the AI).

It has other subtypes focused on the AI and also the SlotTask which is designed to work in sync with the corresponding LayerTask.

#### Logic

There are also Logic entities (LogicCounter, LogicOR and LogicSwitch) that are similar to Slots but their main purpose is to receive inputs and activate actions further down the line to allow you to design more sophisticated workflows

#### Example

For starters, here is one of the most simple task you can create within seconds using Scenario Framework.

Here it will be very briefly commented, all the examples are documented in more detail.

At the top, there is Area, right under it is LayerTaskMove intended to encapsulate the SlotMoveTo which spawns said task.

In this view, we have three columns.

In the left column, there is an unique name of this entity.

In the middle, there is a name of the Prefab and on the right column, it is the class.

#### Tutorial

Here is a step-by-step tutorial on how to setup a world from scratch and using functionalities such as a Task Move, going over all the basics: [Scenario Framework Setup Tutorial](/wiki/Arma_Reforger:Scenario_Framework_Setup_Tutorial "Arma Reforger:Scenario Framework Setup Tutorial").

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

## Debug Menu

ScenarioFramework gives you an extensive Debug Menu Suite that allows you to Inspect, Activate further debug lines in Script Editor and depending on what you are debugging, there are run-time options to manipulate with much of the ScenarioFramework.

In order to work with it, you need to enable the Debug in Debug Menu. First, go to the category Systems > Systems diag and enable it to basic. Then in System points, you need to change it to FixedFrame and here, you will tick the box for SCR\_ScenarioFrameworkSystem. Then you can close this Systems Diag and go back to the root of the debug menu. Unfortunately, you will have to do this everytime re-launch the game including the Play from Workbench mode. After that, it is very straightforward. In the root of the debug menu, there is ScenarioFramework category that then contains respective debug tools.

Some of these tools are interconnected and they have both standalone variant or certain tools can initialise specific instance of said tool that works the same way as the standalone variant. This was done for easier usage thanks to the hierarchical design of the ScenarioFramework. In some cases, you will have to know the entity name which is usually set in workbench, but it should be accessible via Registered Areas tool. Intended usage is for Scenario Creators to use it when playing the scenario from workbench as it is not made for multiplayer environment.

### Tasks

It shows you a list of all the active tasks in the Scenario. Each entry is selectable and begins with the Name of the Task, Name of the parent Area, LayerTask and from which SlotTask it is spawned.

It gives you option to Inspect related Area, LayerTask and Slot Task. It also shows the Task State and gives you option to Finish the Task. You can also Restore the Task To Default. Here, Randomization is turned off so it will restore exactly the same variant, but it could be toggled and if there are more options to randomly choose from, it will do so.

### Registered Areas

It shows you a list of all the registered Areas that are valid in this Scenario. You can then select and Inspect Area which will open instance of Layer Inspector for it.

### Debug Areas

This can closely work with the Debug Areas from the Component where it will show these, but here you can also add new Debug Area entries by their name and optionally specify Layer Task and Slot Task. If there are any Debug entries already present, you can select it and remove specific one or clear it all.

There is also an option to show possible Debug Area Presets that tries to group possible Task Combinations for given Scenario into a presets that you can add as well.

Lastly, you can Reinit ScenarioFramework in a sence that everything that was spawned via Areas will be reinitialised and if Debug Areas are set, it will use these to generate these tasks.

### Layer Inspector

It will show you relevant information for given layer. When used as a standalone variant, you have to input a Layer Name to inspect which is the unique name of desired entity. This can be also opened from variety of other places.

Depending on the type of given layer, there will be different attributes shown and certain buttons to perform runtime changes available.

* For all Layers, it will show it Layer is Initiated (aka it is spawned, active and not for example dynamically despawned).
* If it was terminated (This is mainly for AI slots but certain layers might be marked to be terminated via action).
* If Spawning of child layers is in progress, you can spot currently spawned children out of the supposed number. However if that number would not go away, it means that init of this layer failed and certain children have issues.
* Name of the Parent Area and Layer (If it exists)
* If it is not a slot, it can show Layer Hierarchy and ability to inspect these, where number of dashes is used for hierarchy level
* If it is not a slot, it can show Logic Hierarchy and ability to inspect these.
* If given layer has Plugins, there shall be option to Inspect it. Same goes for Actions and Conditions
* Ability to Teleport to the Layer Entity
* Initiate Dynamic Spawn/Despawn
* Restore To Default with respective options - Include Children will affect child layers, Reinit after restoration will once again reinitialise given layer and Affect Randomization will restore the layer but it can re-randomise itself again

For LayerTasks, there shall be shown a Task State, ability to Finish the Task and Teleport to Spawned entity.

For Slots, there might be shown Spawned Entity Display Name and if it is a Trigger, it will show Trigger Periodic Queries status (If said trigger is actively scanning for entities) and ability to turn it enable/disable it.

For SlotTasks, there shall be shown a Task State, ability to Finish the Task and Teleport to Spawned entity. Additionaly all the different Actions for Conditions and Finish/Create/Failed/Progress/Updated states will be shown here.

### Action Inspector

It will show you relevant information for given layer Actions. When used as a standalone variant, you have to input a Layer Name to inspect which is the unique name of desired entity. This can be also opened from variety of other places.

For Each action, it can show your other sub actions hierarchy and for selected ones, you can enable Action debug that can be used in Script Editor to break a break-point. You can also Init said action again and Activate it. For Basic Actions it will also show you Number of Activations for this action.

### Logic Inspector

It will show you relevant information for given Logic. When used as a standalone variant, you have to input a entity name of desired logic. This can be also opened from variety of other places.

For Each logic, you can enable Action debug that can be used in Script Editor to break a break-point. Termination status is shown. If said logic has Actions, it can show you the hiearchy of these and further debug it.

In Cases of Logic Counter, it shows the Counter Value and gives you ability to Increase/Decrease Counter.

### Plugin Inspector

It will show you relevant information for given layer plugins. When used as a standalone variant, you have to input a Layer Name to inspect which is the unique name of desired entity. This can be also opened from variety of other places.

For Each plugin, you can enable Plugin debug that can be used in Script Editor to break a break-point. and then depending on the variant of the plugin, it will show you relevant info/options.

* Trigger - Will have option to see and enable/disable Periodic Querries and Force Finish Trigger
* OnInventoryChange - Will show you respective actions and ability to further inspect/debug it
* SpawnPoint - Will show you respective actions and ability to further inspect/debug it
* OnDestroyEvent - Will show you respective actions and ability to further inspect/debug it

### Condition Inspector

It will show you relevant information for a given layer's Conditions. When used as a standalone variant, you have to input a Layer Name to inspect, which is the unique name of the desired entity. This can be also opened from a variety of other places.

For each Condition, you can enable Condition debug that can be used in Script Editor to break a break-point. You can also Init said action condition again and see if the Condition is True/False.

### Debug Actions

It will show you a list of predefined Debug Actions.

For Each Debug Action, you can select it from the list and activate it.

## Log Messages

Scenario Framework prints messages into the log.

Some of them are there to inform you about the most important things and usually, you do not need to worry about them as much.

They always have Scenario Framework word contained in them.

However, there are two other types of log messages that vary in severity:

* (W) - Warnings - These messages warn you about improper usage of the system and can give you a hint as to why is something not working as you might expect.
  + This can help you troubleshoot your scenarios
* (E) - Errors - These messages have high severity and mark that something is technically not working (not necessarily user fault).
  + They can help you understanding an issue in configuration

## Components

All Scenario Framework components can be found in Prefabs/Systems/Scenario Framework/Components.

### Shared Attributes

All of the listed components below contain a lot of shared attributes and we are going to quickly go through them.

Some of those categories are then further expanded by the specific components but that will be explained within them.

Attributes are divided into categories:

#### Children

This Category contains handling of the child layers that is affecting spawning of them

*Spawn Children* - Controls how many child layers will get spawned.

It provides 4 different options:

* ALL - Spawns all Children
* RANDOM\_ONE - Spawns random child
* RANDOM\_MULTIPLE - It spawns random number of children based on the Random Percent attribute setting and number of children according to this equation: NumberOfChildren / 100 \* RandomPercentAttribute; children are then picked randomly and results are rounded up to the closest whole number
* RANDOM\_BASED\_ON\_PLAYER\_COUNT - It spawns number of children based on how many children are there compared to the number of players according to this equation: NumberOfChildren / 100 \* (CurrentNumberOfPlayers / MaximumNumberOfPlayers) \* 100; children are then picked randomly and results are rounded up to the closest whole number

*Random Percent* - Sets the random chance for the Spawn Children

*Enable Repeated Spawn* - Enables repeated Spawn of children layers in hierarchy

*Repeated Spawn Number* - If Repeated Spawn is enabled, how many times can children be spawned? If set to -1, it is unlimited

*Repeated Spawn Timer* - If Repeated Spawn is enabled, how frequently it will spawn next wave of children? Value -1 means disabled, thus children will not be spawned by the elapsed time.

#### Asset

This Category handles the Asset properties.

More specific layers/slots have expanded attributes in this category, but here is the only one for the inheritance purposes

*Faction Key* - Faction key that corresponds with the SCR\_Faction set in FactionManager.

It also applies this attribute to all the child layers unless they override it there with different key.
If left empty, "US" Faction Key is used as default. For Layer Tasks, this dictates for which Faction is that task going to be assigned.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") *Can Be Garbage Collected* - If the spawned entity can be garbage-collected (Default setting is set to true so the Garbage Collector can delete this entity)

#### Debug

This Category handles debug visuals

*Show Debug Shapes During Runtime* - When enabled, there will be a colored sphere on the position of the layer. In some instances, size of a sphere is set by the trigger range or Dynamic Despawn range.

*Show Debug Shapes In Workbench* - When enabled, there will be a colored sphere on the position of the layer in World Editor. In some instances, size of a sphere is set by the trigger range or Dynamic Despawn range.

#### Activation

This Category handles activation type, conditions and Dynamic Spawn/Despawn.

##### Activation Type

Sets how the layer is activated and spawned.

It has the following options:

* SAME\_AS\_PARENT - It activates the same way how the parent layer gets activated
* ON\_TRIGGER\_ACTIVATION - It gets activated from the trigger or actions that are using this activation type to activate it
* ON\_AREA\_TRIGGER\_ACTIVATION - It gets activated when the parent area of this layer has trigger and that gets activated
* ON\_INIT - It gets activated right after said layer is created (which does not need to happen on scenario start, but for example when said layer is being spawned somehow else)
* ON\_TASKS\_INIT - It activates after scenario is started and the system for random task generation is triggered
* CUSTOM1, CUSTOM2, CUSTOM3, CUSTOM4 - These are for modders to activate this layer via custom ways and you will not use it in common scenario creating

*Exclude From Dynamic Despawn* - It excludes this layer from being despawned by Dynamic Spawn/Despawn system. Layer will get spawned, but it will be skipped and will not get despawned, including all the children layers of this layer

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

##### Activation Conditions

Conditions that will be checked upon init and based on the result it will let this to finish init or not. Conditions are evaluated in order which they are inserted here. Modders can easily add their own conditions

* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Character In Vehicle Condition
  + Getter - Entity to check
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") ADS Condition
  + Getter - Entity to check
  + Must Be ADS - Entity(s) must have this aim down sights state
* Day Time Condition
  + Only During Day - If true, this can be activated only during the day. If false, only during the night.
* Day Time Hour Condition - Allows you to limit activation to only a certain time window
  + Min Hour - Minimal day time hour
  + Max Hour - Maximal day time hour
* Weather Condition
  + Min Wind Speed - Minimal wind speed in meters per second
  + Max Wind Speed - Maximal wind speed in meters per second
  + Min Rain Intensity - Minimal rain intensity
  + Max Rain intensity - Maximal rain intensity
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Variable Value Condition
  + Variable Name - Name of the variable
  + Variable Value To Check - Check if the variable has set this value or not
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Entity Damage State Condition  
  Contains a mistake in class name, where it is Entityk.

  ⓘ

  Contains a mistake in class name, where it is Entityk.

  + Getter - Entity to check
  + Damage State - Damage state to check. Returns true when the entity state match. If entity doesn't have damage manager, returns true. If array of entities is passed on, return false when at least one entity doesn't fit selected damage state.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") AI Threat State Condition
  + Entity - AI Entity to evaluate. Leave empty to use entity spawned by this slot.
  + AI Threat State - What threat state should be checked by this condition. If used on a group, return true when at least one member of group has this threat state.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Entity Distance Condition
  + Getter A - Entity A
  + Getter B - Entity B
  + Min Distance - Minimum distance between entities in metres (Inclusive).
  + Max Distance - Maximum distance between entities in metres (Inclusive). -1 for infinity"
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Medical Condition
  + Getter - Entity to check
  + Requiered Medical Conditions - Character must have all of these medical conditions.
    - Bleeding - Checking the bleeding status according to further settings
      * Can Bleed Anywhere - If true, is satisfied by bleeding on any HitZone.
      * Hit Zone Names - Is satisfied by bleeding on any specified HitZone.
      * Hit Zones Groups - Is satisfied by bleeding in any specified HitZoneGroups.
    - Health - Similar to SCR\_ScenarioFrameworEntitykDamageStateCondition but with propulated selections for character and character hit zone groups.
      * Health Minimum - Minimum health of hit zone or group to satisfy condition. (Inclusive)
      * Health Maximum - Maximum health of hit zone or group to satisfy condition. (Inclusive)
      * Use Default Health - If true, uses the health hitpoint.
      * Hit Zone Names - Is satisfied by health on any specified hit zone.
      * Hit Zones Groups - Is satisfied by health in any specified hit zone groups
    - Tourniquet - Checking the tourniquet status
      * Can Apply Tourniquet Anywhere - If true, is satisfied by tourniquet on any HitZone.
      * Hit Zones Groups - Is satisfied by tourniquet in any specified HitZoneGroups.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Task Status Condition
  + Getter - Layer task(s) to check for condition. If multiple layer tasks are specified, they must all have an acceptable state.
  + Acceptable Task States - If the layer task is any of these states, the condition will be statisfied
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Variable Value Condition
  + Variable Name - Name of the variable
  + Variable Value To Check - Check if the variable has set this value or not
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Stance Condition
  + Getter - Entity to check
  + Requiered Stances - Entity(s) must have at least one of the required stances. For entity(s) that are not characters, the condition will return true
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Damage Context Condition Collider
  + Collider IDs - Accepted collider IDs
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Damage Context Condition Damage Type
  + Accepted Damage Types - Accepted damage types
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Damage Context Condition Hit Zone
  + Hit Zone Group Selector - Hit Zone Group
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Condition Projectile Prefab
  + Projectile Prefab Name - Projectile Prefab Name
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Damage Context Condition Value
  + Activation Value - Activation Value
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Resource Condition
  + Getter - Entity to check
  + Required Resource Conditions - Required resource related conditions
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Resource Condition Value
  + Target Value - Activation amount
  + Comparison Operator - Operator
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Item In Storage Condition
  + Getter - Entity with storage to check
  + Prefab Resources - Prefabs to search
  + Negation - Negation of condition
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Time Specific Condition
  + Hours - Minimal day time hour
  + Minutes - Maximal day time hour
  + Seconds - Maximal day time hour
  + Comparison Operator
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)") Weapon Ammo Condition
  + Getter - Entity to check.
  + Required Percentage - Activation Percentage

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

##### Activation Condition Logic

Which Boolean Logic will be used for Activation Conditions

* Currently supported types: AND, OR, NOT, XOR

Exclude From Dynamic Despawn - It excludes this layer from being despawned by Dynamic Spawn/Despawn system. Layer will get spawned, but it will be skipped and will not get despawned, including all the children layers of this layer

#### OnActivation

This Category handles what happens after this layer is fully spawned and activated

Activation Actions - Executes Scenario Framework actions in listed order (see [Actions](#Actions))

#### Plugins

This Category handles what happens after this layer is fully spawned and activated

Plugins - Attaches Scenario Framework plugins in listed order (see [Plugins](#Plugins_2))

### Area

See [Area](#Area)

This variant can be used to create generic task without any specialised logic and it is expected from the Scenario Creator to control it via other means (Such as Actions).

Same Attributes as [Layer](#Layer)

#### Debug

*Show Debug Shapes In Workbench* - Due the fact that Area can handle both triggers and Dynamic Spawn/Despawn, this sets whether or not debug sphere is visible in workbench or not

#### Activation

Dynamic Despawn - Enables/Disables Dynamic Spawn/Despawn feature for particular area - see [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn)

Dynamic Despawn Range - How close at least one observer camera must be in order to activate spawn/despawn - see [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn)

#### OnActivation

Trigger Actions - Executes Scenario Framework actions in listed order when the Trigger attached to this Area is activated (see [Actions](#Actions))

#### Trigger

This category handles attached trigger spawning. Also check out the [Plugins](#Plugins_2) for the Trigger Plugin to have greater control over the trigger

Trigger Resource - Sets which Trigger prefab will be used

Area Radius - Sets the radius of the Area for this trigger

Once - If trigger is activated (conditions are true), it will be activated just one or every time it is true

### Layer

See [Layer](#Layer)

No additional attributes

### LayerTask

See [LayerTask](#LayerTask)

This variant can be used to create generic task without any specialised logic and it is expected from the Scenario Creator to control it via other means (Such as Actions).

#### Task Enums

This Category handles different enums for Task

Type Of Task - Type of task used for ON\_TASKS\_INIT generation

Task Ownership - Who will be the owner of the task for whom it will be assignable. By default, it will be owned by the given Faction.

Task Visibility - To whom the task will be visible. By default, it will be visible for the given Faction.

Task UI Visibility - Where the task will be visible in UI. By default, it will be visible in the Task List and on the Map.

#### Task

This Category handles setting of task properties

Task Title - Name of the task in the list of tasks

Task Description - Description of the task

Task Prefab - Task prefab

Override Object Display Name - Overrides display name of the spawned object for task purposes

Task Functions On Its Own - Whether the task has functionality on its own or is just a holder for a parent task without actual functionality

Finish Conditions - Conditions that will be checked upon trying to finish a task

Finish Condition Logic - Which Boolean Logic will be used for Finish Conditions.

#### Subtask

This Category handles further setup of the subtask feature

Subtasks Description - Description that is displayed above subtasks in the parent task

Is Subtask - If it is a subtask, it will be grouped under the topmost LayerTask or, if the attribute target parent task is filled, it will group it under that

Is Optional - If this is a subtask, whether or not the subtask is optional

Parent Layer Task - Name of the parent LayerTask if this LayerTask is set to be a Subtask. It will override default behavior, where the parent LayerTask is the topmost LayerTask in hierarchy.

### Task UI

This Category handles UI settings

Task Icon Set - Task icon set

Task Icon Name - Name of the specific icon from the icon set

Progress Bar - Whether or not to show a progress bar

Calculate Progress Bar On Completed Tasks - It will set percentage of progress based on the number of completed tasks that are not optional

Place Marker On Subject Slot - Marker on map is placed directly on the task subject Slot or on layer Slot

Progressed State Change Popup - When this task will get its state changed to PROGRESSED, it will trigger a popup message

#### Task State Changed Actions

Trigger Actions On Finish - What to do once the task is finished

Actions On Created - What to do once the task is created

Actions On Failed - What to do once the task is created

Actions On Cancelled - What to do once the task is cancelled

Actions On Progress - What to do once the task is progressed

Actions On Assigned - What to do once the task is updated

### LayerTaskMove

Layer Task specialised for Task Move. It is expecting to have [SlotMove](#SlotMove) in its spawned hierarchy.

Same Attributes as [LayerTask](#LayerTask)

### LayerTaskDestroy

Layer Task specialised for Task Destroy. It is expecting to have [SlotDestroy](#SlotDestroy) in its spawned hierarchy.

No additional attributes

### LayerTaskKill

Layer Task specialised for Task Kill. It is expecting to have [SlotKill](#SlotKill) in its spawned hierarchy.

No additional attributes

### LayerTaskDefend

Layer Task specialised for Task Defend. It is expecting to have [SlotDefend](#SlotDefend) in its spawned hierarchy.

No additional attributes

#### Task

This Category is further expanded for the Task Defend by additional attributes:

Trigger Name - Task Defend requires a trigger attached to it for it to work in cases where you want to set this task to defend some entity or both entity and area. Input the name of the slot that spawns the trigger

#### Defend Params

This category contains all the Defend parameters that you can set for this type of task

Countdown Title Text - Text that will be displayed above the countdown number

Defend Time - For how long you have to Defend the objective of the task. Value -1 is for indefinitely.

Display Countdown HUD - When enabled, it will display the text and how much time remains for the Task Defend

Countdown HUD - Layout of the Countdown HUD

Faction Settings - Here you set who is considered as a Defender and Attacker for this Task. Each entry then contains these two attributes:

* FactionKey - Faction key that corresponds with the [SCR\_Faction](enfusion://ScriptEditor/scripts/Game/Faction/SCR_Faction.c;5) set in FactionManager
* Count Only Players - When disabled, all units from this faction will be counted with for other Task Defend conditions

Min Defender Percentage Ratio - When compared to the number of attackers, minimum of how much of the characters present in the task area must be from defending side to successfully complete the objective on evaluation

Attacker Layer Names - Layer containing attacker forces. Can be more layers, but these layers must include only AI units/groups and nothing else

Earlier Evaluation - When enabled, it will can finish the task earlier than the countdown when all attackers are eliminated. Can be combined with Delayed Evaluation.

Delayed Evaluation - When enabled, the evaluation will be delayed and defenders will need to eliminate all attackers in order for the task to be successfully completed.
Can be combined with Earlier Evaluation.

Display Delayed Evaluation Text - When enabled, it will display the text to inform players that they have to eliminate all attacker units

Delayed Evaluation Text - Text that will be displayed to inform players that they have to eliminate all attacker units

### LayerTaskClearArea

Layer Task specialised for Task Clear Area. It is expecting to have SlotClearArea in its spawned hierarchy.

No additional attributes

### LayerTaskDeliver

Layer Task specialised for Task Deliver. It is expecting to have SlotPick in its hierarchy spawned and then SlotDelivery spawned somewhere else or to be spawned for full funtionality.

No additional attributes

#### Task

This Category is further expanded for the Task Deliver by additional attributes:

Task Title Updated - Sets new task title when task gets updated

Task Description Updated - Sets new task description when task gets updated

Intel Map Marker Update Delay - If Place Marker On Subject Slot is true, the task marker will be on position of the task item and this attribute can set a delay between the updates for the position

### Slot

See [Slot](#Slot)

No additional attributes

#### Asset

This Category is further expanded by following attributes:

Object To Spawn - Prefab name of the object to be spawned

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Faction Switched Objects - Selects which object to spawn based on the selected Faction Key. Designed to work with Faction Aliases feature.

* Faction Key - Required faction key to spawn this object.
* Object To Spawn - Prefab name of the object to be spawned

ID - Unique name of the entity that can be later used for identification and finding via [Getters](#Getters) in the scenario

Use Existing World Asset - If enabled, slot will not spawn new object, but it will rather use the object already existing in the world in the vicinity of it

Override Object Display Name - Overrides display name of the spawned object for task purposes

#### Randomization

This Category handles the randomisation of asset that is supposed to be spawned

*Randomize Per Faction* - Randomise spawned asset(s) per Faction Key Attribute which needs to be filled as well. Overrides Object To Spawn Attribute.

Entity Catalog Type - Select Entity Catalog type for random spawn

Include Editable Entity Labels - Select Entity Labels which you want to optionally include to random spawn.
If you want to spawn everything, you can leave it out empty and also leave Include Only Selected Labels attribute to false.

Exclude Editable Entity Labels - Select Entity Labels which you want to exclude from random spawn

Include Only Selected Labels - If true, it will spawn only the entities that are from Included Editable Entity Labels and also do not contain Label to be Excluded

#### Composition

This Category handles setting behavior for potential composition to be spawned. It currently has only one attribute:

Ignore Orient Children To Terrain - When disabled orientation to terrain will be skipped for the next composition.
This is used for example when placing sandbags and you want "stack" them at top of each other.

### SlotAI

SlotAI is specialised to handle AI units. It can spawn both singular units or groups. Using the Waypoints category and Misc AIs can be further directed and adjusted.

No additional attributes

#### Waypoints

This Category handles the setting of task properties and can override what is set in the parent TaskLayer of this SlotTask

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") Waypoint Set - Here you can define and set the layer containing waypoints and further adjust how they are going to be used

* Layer Name - Here you put name of the layer that contains SlotWaypoints or directly the name of desired SlotWaypoint. In case you want to have waypoints cycled, just input name of the SlotWaypoint that spawns Cycle Waypoint.

Waypoint Group Names - **obsolete** since [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)"), it will be removed in [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)"). Please relocate it to **Waypoint Set**.

Spawn AI on WP Pos - If true, it will spawn AI on the first WP Slot

WP To Spawn - Default waypoint that will be spawned for the AI if Waypoint Set is not defined or does not contain any waypoints.

#### Balance

This Category handles the setting of task properties and can override what is set in the parent TaskLayer of this SlotTask

Balance On Players Count - Spawns number of AIs in the group based on the player count according to the equation that takes the number of players currently playing and scales it to determine the appropriate number of units to spawn, using a randomly determined starting point in the spawning range. The number of units is linearly mapped so that the more players there are, the closer the spawned units reach the maximum number dictated by the prefab, starting from a base number decided randomly between 1 and 3.

Min Units In Group - Least amount of AIs in the group after balancing occurs. Will not exceed maximum number of units defined in the group prefab.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

#### Common

Contains some common attributes for the AI to be set.

AI Group Formation - AI group formation

AI Skill - AI skill in combat

AI Combat Type - AI combat type

Perception Factor - Sets perception ability. Affects speed at which perception detects targets. Bigger value means proportionally faster detection.

Group Prefab - Each AI needs to have their group and here is the default group prefab defined

### SlotTrigger

Slot specialised to spawn trigger prefabs and then be further adjusted via [Plugins](#Plugins_2)

No additional attributes

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### SlotPlayerTrigger

Slot specialised to spawn trigger prefab already set to detect players and then be further adjusted via [Plugins](#Plugins_2)

No additional attributes

### SlotTask

This slot is is supposed to spawn generic empty task where Scenario Creators are in control of the whole logic via other means (Such as Actions and so on).
Other SlotTask types are inheriting from it and have specialised behaviour already in place.

No additional attributes

#### Task

This Category handles the setting of task properties and can override what is set in the parent TaskLayer of this SlotTask

Task Title - Sets name of the task the player Task List

Task Description - Sets text description of the task in the player Task List

Task Execution Briefing - Here you can put text that can then get written into the Execution part of the Briefing via this action (see [Actions](#Actions))

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)") Task Intro Voiceline - StringID for the Intro Voiceline action to be processed. Processing must be setup after tasks are initialised.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Finish Conditions - Conditions that will be checked upon trying to finish a task

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Finish Condition Logic - Which Boolean Logic will be used for Finish Conditions

### SlotMoveTo

This slot is designed to work with LayerTaskMove and it is supposed to spawn a Trigger prefab with attached Plugin Trigger for the desired functionality of Task Move creation.

No additional attributes

### SlotDestroy

This slot is designed to work with LayerTaskDestroy. It attaches listener to the spawned entity which listens whether or not the entity is destroyed.
If the target entity is a vehicle, it will also gets triggered by flooding the engine with water.

No additional attributes

### SlotKill

This slot is designed to work with LayerTaskKill. It works very similarly as SlotDestroy but this is intended for AI characters.

No additional attributes

### SlotDefend

This slot is designed to work with [LayerTaskDefend](#LayerTaskDefend).
It can either spawn Trigger to work as a Defend Area task, or it can spawn an entity (can be a character or vehicle, anything that has a DamageManager component) and then the task is about to defend provided entity.
Or you can combine it by spawning entity to defend and linking trigger in the LayerTaskDefend from the outside SlotTrigger to expand the possibilities and parameters of this task.

No additional attributes

### SlotClearArea

This slot is designed to work with [LayerTaskClearArea](#LayerTaskClearArea).
It spawns TriggerDominance that is adjusted via Plugin Trigger that needs to be tweaked for each usage and you have to put FactionKey of the faction that will receive the task to Clear Area into the Activated By This Faction Attribute.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)") Ignored Faction Keys - Ignored Faction Keys that won't be used for any calculations for this Slot Clear

No additional attributes

### SlotPick

This slot is designed to work with LayerTaskDeliver. It will spawn the desired item that will be used for this task and it handles updates of the task title + description in the Task category.

No additional attributes

#### Task

Task Title Updated1 - Sets name of the task the player Task List when target Intel is picked up

Task Description Updated1 - Sets text description of the task in the player Task List when target Intel is picked up

Task Title Updated2 - Sets name of the task the player Task List when target Intel is dropped

Task Description Updated2 - Sets text description of the task in the player Task List when target Intel is dropped

### SlotDelivery

This slot is designed to work with LayerTaskDeliver. It spawns TriggerCharacterSlow and Plugin Trigger that is used to detect items spawned from SlotPick.

No additional attributes

#### Task

Associated Task Layers - Here you can define which LayerTaskDeliver is associated with this SlotDelivery in order to deliver Intel there

### SlotExtraction

This slot is designed to work with LayerTaskMove. It is just a slightly adjusted variant of SlotMove but with countdown mechanics set in the Plugin Trigger

No additional attributes

### SlotTriggerClearArea

ⓘ

Placeholder for a planned feature.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### SlotMarker

SlotMarker is specialised to handle map markers. Be aware that we are not using the **Object To Spawn** but specialised attribute in the Map Marker category.

No additional attributes

#### Map Marker

Map Marker Type - Here a map Marker Type is selected. It serves as an API for the MapMarker system and will be using this config ([MapMarkerConfig.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Map/MapMarkerConfig.conf)).  
There are currently two types available:

* Marker Custom - Using the PLACED\_CUSTOM from the config above
  + Map Marker Text - Text which will be displayed for the Map Marker
  + Map Marker Icon - You can choose from the dropdown menu
  + Map Marker Color - You can choose from the dropdown menu
  + Map Marker Rotation - Rotation of the Map Marker (from -180 to 180 degrees)
  + Can Be Removed By Owner - Can this map marker be deleted from map by owner of the map
* Marker Military - Using the PLACED\_MILITARY from the config above
* Map Marker Text - Text which will be displayed for the Map Marker
  + Map Marker Faction Icon - You can choose from the dropdown menu
  + Map Marker Dimension - You can choose from the dropdown menu
  + Map Marker Type1 Modifier - You can choose from the dropdown menu
  + Map Marker Type2 Modifier - You can choose from the dropdown menu

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### SlotWaypoint

SlotWaypoint is made to handle AI Waypoints. Be aware that the attribute in the Waypoint category is used, not the **Object To Spawn** field.

No additional attributes

#### Waypoint

Waypoint - This gives the option to select a waypoint from a huge variety of them.
It provides a simple API for all the waypoint Prefabs located in the Waypoints directory ([Waypoints](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/Waypoints)).

Currently, there are these available waypoints intended to be used:

* Animation
* Artillery Support
* Attack
* Capture Relay
* Cycle
* Defend
* Defend CP (Combat Patrol)
* Defend Hierarchy
* Defend Large
* Defend Large CO (Combat Ops)
* Deploy Smoke
* Follow
* Forced Move
* Get In
* Get In Nearest
* Get Out
* Heal
* Loiter CO (Combat Ops)
* Move
* Observation Point
* Open Gate
* Patrol
* Patrol Hierarchy
* Scout
* Search And Destroy
* Smart Action
* Suppress
* User Action
* Wait

If you want to have Waypoints cycled, just select the Cycle Waypoint. Here you have two options:

* input the Layer name that contains the SlotWaypoints you want to be cycled into the "Layers With Waypoints To Cycle" attribute
* or if this waypoint is inserted into the target layer, you can leave it empty and it will get the parent layer where this Cycle Waypoint is located and it will add all the non-cycle waypoints to be cycled.

## Plugins

Plugins allow you to add more functionalities, usually to slots, that can further help you creating more specific things.
Some of the plugins are already in use, most notably the Plugin Trigger to set attributes to the spawned triggers.
You can inherit from the base class of the plugin to script your own plugins to easily add new features to your scenarios.

### OnDestroyEvent

Plugin intended to be used on Any slot to activate actions upon asset destruction

Actions On Destroy - Upon asset destruction, it will activate set actions (see [Actions](#Actions))

### OnInventoryChange

Plugin intended to be used on Any slot to activate actions upon asset inventory changes in terms of item addition/removal.

Actions On item Added - Once upon an item is added to the inventory of the asset, it will activate set actions (see [Actions](#Actions))

Actions On Item Removed- Once upon an item is removed from the inventory of the asset, it will activate set actions (see [Actions](#Actions))

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### SpawnPoint

Spawn Point plugin is used to manipulate Spawn point properties and provide the possibility to execute actions upon it being used.
These actions will always carry the player entity that just spawned.

Spawn Radius - Find empty position for spawning within given radius. When none is found, entity position will be used.

Faction - Determines which faction can spawn on this spawn point.

Show In Deploy Map Only - If Spawnpoint will be showed just in the Deploy Map

Timed Spawn Point - Use custom timer when deploying on this spawn point. Takes the remaining respawn time from SCR\_TimedSpawnPointComponent

Info - Allows to select which Info to use for filling the name or other properties.

Use Nearby Spawn Positions - Allow usage of Spawn Positions in range

Spawn Positions Usage Range - Spawn position detection radius, in metres

Respawn Time - Additional respawn time (in seconds) when spawning on this spawn point

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)") Pass Spawned Entity - Pass Spawned Entity for Actions On Spawn Point Used. Otherwise it will pass the slot (layer) this plugin is attached to.

Actions On Spawn Point Used - What to do once Spawn Point is used. These actions will always carry the player entity that just spawned.

### Trigger

Trigger Plugin is used to set attributes for the triggers that are being spawned.
It is designed to work with trigger prefabs that inherit from the [SCR\_CharacterTriggerEntity](enfusion://ScriptEditor/scripts/Game/ScenarioFramework/Entities/Triggers/SCR_CharacterTriggerEntity.c;6) (TriggerCharacterSlow and TriggerAnyPlayerSlow) which has variety of use-cases even though its name would suggest it is mainly used for Characters.
With the variety of attributes to set, it allows you to modify the trigger in very interesting way and create a lot of different tasks/scenarios/logics once you understand what each attribute does and what it allows.

One of the most common use-cases for triggers are tasks which are also included in Samples such as Task Move, Task Exfil, Task Clear Area, Deliver Weapons In Vehicle to certain Area or Deliver cars/any other items to certain area.

Area Radius - Radius of the trigger coverage

Activation Presence - This controls the category of presence that can activate this trigger.

* PLAYER - Trigger will require at least 1 player character inside for it to activate
* ANY\_CHARACTER - Trigger will require any character inside for it to activate
* SPECIFIC\_CLASS - This setting is supposed to be used when you are working with Specific Class Names attribute and it will require to have them inside the trigger for it to activate
* SPECIFIC\_PREFAB\_NAME - Works similarly as SPECIFIC\_CLASS but intended mainly for Prefab Filter. However it will use OR with Specific Class Names and other conditions for the trigger to activate

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)") Specific Entity Names - Fill the entity names here for detection. Is combined with other filters using OR.

Specific Class Names - Allows you to set list of classnames that will be detected by the trigger (such as ChimeraCharacter, Vehicle and so on). Is combined with other filters using OR.

Prefab Filter - Allows you to add list of Prefab Filters that each will allow you to specify a prefab name and whether or not it should include child prefabs. Is combined with other filters using OR.

Activated By This Faction - For which faction this trigger is assigned. This also influences the Activation Presence which will additionally check the Faction for all the filtered entities inside.

Custom Trigger Conditions - This allows you to extend the trigger conditions for when the trigger gets activated.
There are already two conditions in-place, but this is created in a way that you can create your own conditions in script and you will see them here.
Conditions are then evaluated in the listed order and if one fails, it will not continue to the other one.

* Specific Class Name Count - Used for counting how many times specific class name is inside the trigger. It works similarly like the Specific Class Names attribute but with addition of a Classname count attribute where you can set the desired number. Additionally, this will add the class name to the trigger filter from this condition so you do not have to add it again in the Specific Class Names attribute.
* Specific Prefab Count - Used for counting how many times specific prefab is inside the trigger. It works similarly like the Prefab Filter attribute but with addition of a Prefab count attribute where you can set the desired number. Additionally, this will add the prefab to the trigger filter from this condition so you do not have to add it again in the Prefab Filter attribute.

Search Vehicle Inventory - Entities (usually items) that are in inventory of vehicles are not detected by triggers in default.
However, this Plugin added the such behavior as it is sometimes needed to detect entities that are situated in inventories of vehicles and this attribute allows just that.

Once - If set to true, the trigger will be activated only once if conditions are true. If set to false, the trigger will get activated every time its conditions are true.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)") Activate On Empty - Activate the trigger once it is empty

Update Rate - How frequently is the trigger updated and performing calculations. Lower numbers will decrease performance.

Minimum Players Needed Percentage - Minimum players needed to activate this trigger when PLAYER Activation presence is selected

Activation Countdown Timer - For how long the trigger conditions must be true in order for the trigger to activate. If conditions become false, timer resets

Notification Enabled - Whether or not the notification is allowed to be displayed

Player Activation Notification Title - Notification title text that will be displayed when the PLAYER Activation presence is selected

Enable Audio - Whether or not the audio sound is played and affected by the trigger

Countdown Audio - Audio sound that will be playing when countdown is active.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Entity Entered Actions - Actions that will be activated when entity that went through the filter entered the trigger and is inside (Be carefull as Framework Triggers activate this periodically if you don't disable the Once attribute)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Entity Left Actions - Actions that will be activated when entity that went through the filter left the trigger

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Finished Actions - Actions that will be activated when all conditions are met and Trigger finishes

## Logic

Logic entities will allow you to create "systems" that can drive scenario logic based on how you build it.

### Shared Attributes

#### Input

Allows you to set Inputs for this counter via the Input Action - do not confuse with Scenario Framework Actions.

##### Input Action

These actions if conditions are true, will increase the counter.

ⓘ

Each Input Action has the Latch attribute which currently does nothing.

###### On Task Event Increase Counter

Optional, used to increase the counter based on the task event update and Task Layer name

Task Layer Name - optional attribute, to either set specific [LayerTask](#LayerTask) or if left empty, it will be processing all the LayerTasks

Event Name - On Which Task Event update it will increase the counter.

###### Check Entities In Trigger

Used to increase the counter based on the selected comparison operator on how many entities are there in the trigger. This is supposed to be used for SlotTrigger.

Getter - To link which Trigger will be compared.

Comparison Operator - Which comparison operator will be used. You can choose from: LESS\_THAN, LESS\_OR\_EQUAL, GREATER\_THEN, GREATER\_OR\_EQUAL or EQUAL

Value - To which value will it be compared using the Comparsion Operator attribute and number of entities from trigger linked from Getter attribute

###### Check Entities In Area Trigger

Same as [Check Entities In Trigger](#Check_Entities_In_Trigger) but it is supposed to be used directly to Area rather than SlotTrigger.

### LogicCounter

Logic Counter allows you to store the count of something and then it can execute actions (see [Actions](#Actions)) either on Activation (a.k.a upon reaching certain count) or on Increase.
It also has Input that can listen to other events but other Scenario Framework components can also have actions that can increase the counter from their side.

Attribute categories are exactly the same as in [Shared Attributes](#Shared_Attributes) with one category as an addition

#### Counter

Specific category for the Counter which is keeping the count

Count To - Up until which number it should count towards for it to trigger actions in OnActivate.

### LogicOR

ⓘ

Placeholder for a planned feature.

Attribute categories are exactly the same as in [Shared Attributes](#Shared_Attributes)

### LogicSwitch

ⓘ

Placeholder for a planned feature.

Attribute categories are exactly the same as in [Shared Attributes](#Shared_Attributes)

## Getters

Getters allow you to retrieve usually entity or a number from something already existing in the Scenario for the purposes of using it usually in some [Actions](#Actions).

### GetArea

Area Name - the name of the wanted Area

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

### GetAreaTrigger

Area Name - the name of the Area from which you want the trigger

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### GetArrayOfPlayers

This Getter will return array of player entities with the possibility to filter them using Faction Keys.

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

* Faction Keys - (Optional) You can filter players by putting the Faction Key of desired faction(s); if not used, all factions will be eligible

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetArrayOfEntities

This Getter will return an array of entities from multiple getters.

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

* Getters - Array of Getters that should return an entity or array of entities

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetArrayOfLayerBases

This Getter will return an array of layer bases.

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

* Layer Base Names - Names of the layer bases to retrieve

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetChildEntityByClassName

This Getter will return a child entity with the specified class name.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Layer Name - Name of the layer containing the child entity
* Class Name - Class Name of the child entity to retrieve

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetChildEntityByPrefabName

This Getter will return a child entity with the specified prefab name.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Layer Name - Name of the layer containing the child entity
* Prefab Name - Prefab Name of the child entity to retrieve

### GetClosestPlayerEntity

This Getter will return the closest Player entity you want to work with using another Getter to get the position of a an object to search from.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Getter - Supposed to be a getter that will return some entity from which position you want to find the closest player
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Faction Keys - (Optional) You can filter players by putting the Faction Key of desired faction(s); if not used, all factions will be eligible

### GetCountEntitiesInTrigger

Trigger Name - Input name of the trigger to get the entity count inside it

Return value: c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12)

### GetEntityByName

This one is one of the most powerful one which will allow you to get any Scenario Framework entity to further work with it.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Entity Name - the name of the entity you want to obtain (can be be any entity)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetEntityFromSlotManager

This Getter will return an entity from a slot manager.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Layer Name - Name of the layer containing the slot manager
* Slot Name - Slot Name from which to retrieve the entity

### GetLastFinishedTaskEntity

This will return the last finished task's entity

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

### GetLayerBase

Layer Base Name - the name of the wanted Layer

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetLayerBaseFromVariable

This Getter will return a layer base from a variable.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Variable Name - Name of the variable containing the layer base

### GetLayerTask

Layer Task Name - the name of the wanted LayerTask

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

### GetListEntitiesInTrigger

Trigger Name - Input name of the trigger to get array of all the entities inside it

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

### GetPlayerEntity

This will return the first player entity it finds. It is supposed to be used mainly for 1-player scenarios.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### GetArrayOfPlayers

This getter will fetch all the players that are currently spawned in-game with the possibility to filter them using Faction Keys.

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Faction Keys - (Optional) You can filter players by putting the Faction Key of desired faction(s). If not used, all factions will be eligible

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### GetRandomLayerBase

This getter will randomly select a layer from provided list

Name Of Layers - From this list, random layer will be selected

Return value: c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154)<[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)>

### GetSpawnedEntity

This is one of the most powerful getter to use as it will allow you to get get access to the entities that Slots are spawning.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Layer Name - Insert name of the Layer you want to get the Spawned Entity from. This name could be a bit misleading, but main usage is on Slot, SlotAI, SlotTrigger and so on.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### GetSpawnedEntityFromVariable

This Getter will return a spawned entity from a variable.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Variable Name - Name of the variable containing the spawned entity

### GetTask

This is one of the most powerful getter to use as it will allow you to get get access to the task that the LayerTask has attached to it.

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Layer Task Name - name of the wanted LayerTask from which to get the attached Task

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### GetTopParentEntity

This getter will return top parent entity from target entity

Getter - Entity to get the top parent from

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### GetVoiceOverActorEntity

This getter will allow to get Actor enum and Actor Entity intended for Voice Over Actions

Return value: c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)

* Actor - Actor enum in VO system
* Actor Entity - Entity playing the voiceover.

## Actions

Actions allow you to perform runtime changes to many aspects of the game, alter different states, adjust entities and so much more.
In many cases, there are things that you would be able to do in-game as a player or as a Game Master very easily.
Currently not everything will be here, but things will be added as time goes on and you can add your own actions as well if you properly inherit from the base class.

All of the actions have one shared attribute that is the Max Number of Activations. In default, it is set to -1, which means infinite number of activations.
You can restrict the number of activations of each action if you happen to be in a situation where certain action could be called several times from different sources but you only want it to be activated once or x amount of times.

Many of the Actions are designed in a way that if they have single and empty Getter, the action will automatically be working with the spawned entity of the Slot to which this action is attached.
This information is specified in each Getter description.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### ActionAI

Allows to perform actions on AIs.

It uses the [Getter](#Getters), but for these actions, the expected target is SlotAI (Using GetLayerBase); or, if this is directly attached on a SlotAI, the Getter can be left empty and will work with the AIs spawned by this slot.

#### AI actions

##### Add Waypoint

Adds waypoint for the targer AI

Getter - Here you have to specify which SlotWaypoint you want to use for the AI (Using GetLayerBase)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Add On Top Of Queue - True checked, this waypoint will be added on a first position. Otherwise it will append it at the end of the waypoint queue.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Clear Previous Waypoints - True checked, previous waypoints should be cleared.

##### On Waypoint Completed

Executes actions if selected waypoint was completed

Getter - Here you have to specify which SlotWaypoint you want to use for the AI (Using GetLayerBase)

Actions On Waypoint Completed - Actions that will get executed upon waypoint completition

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") Remove On Completed - Remove the event on action completion

##### On Threat State Changed

Executes actions based on the Threat state change

AI Threat State - On what Threat State will actions be activated

Actions On Waypoint Completed - Actions that will get executed upon Threat state being changed to desired state

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

#### On Animation Waypoint Started

Executes actions based on the Animation Waypoint Started

* Getter - SlotWaypoint that spawns waypoint
* Actions On Waypoint Completed - Actions that will be executed upon the provided waypoint completion for the provided AI group
* Remove On Completed - Remove the event on action completion
* Animation Index - Index of animation that should trigger the event. Leave -1 and set m\_eEvaluationState to evaluate only animation as a whole.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

#### Set Max Autonomous Distance

Set max autonomous distance of AI group.

Investigation Distance - Distance in metres

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

#### Set Max Autonomous Distance

Set max autonomous distance of AI group.

Investigation Distance - Distance in metres

##### Set Skill

Changes AI Skill in run-time to desired value

AI Skill - AI skill in combat

##### Set Combat Type

Changes Combat Type in run-time to desired value

Combat Type - AI combat type

##### Set Hold Fire

Changes Hold Fire behaviour in run-time

Hold Fire - If AI in the group should hold fire

##### Set Perception Factor

Changes Perception factor in runtime to desired value

Perception Factor - Sets perception ability. Affects speed at which perception detects targets. Bigger value means proportionally faster detection.

##### Set Formation

Changes Formation in run-time

AI Group Formation - AI formation from available formations

##### Set Character Stance

Changes character stance in run-time

AI Character Stance - AI character stance from available formations

##### Set Movement Type

Changes movement type in run-time

AI Movement Type - AI movement type from available formations

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

#### Shoot Flare

Orders given AI unit to shoot flare at given target.

* Getter - Target entity should shoot at.
* Target Offset - Offset from target entity. If target entity doesn't exist, origin is shooter position and target vector is offset from his position - use PointInfo"

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

#### On Agent Count Changed

Actions that will be executed upon the threat state being changed and matching the desired state

* Actions On AI Count Changed - Actions that will be executed upon the threat state being changed and matching the desired state
* Comparison Operator
* Activation Percentage - Activation Percentage

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

#### Move AI Into Vehicle

Teleports AI units into the specified vehicle

* Vehicle Getter - Vehicle Getter
* Allow Partial Move - Allow partial move of AI units if there are not enough compartments for the whole group.
* Allow Driver
* Allow Gunner
* Allow Cargo

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

#### Change Attached AI

* Getter - New target entity, usually selected by name to be hooked into target SlotAI. It is optional; if left empty, this action will just remove all references from target SlotAI.

### ActionMedical

Allows you to change target character medical values or states.

It uses the [Getter](#Getters), but for these actions, expected target is ChimeraCharacter. Usually you will want to use GetSpawnedEntity from some Slot or GetPlayerEntity.

#### Medical Actions

##### Add Particular Bleeding

Adds bleeding to the specific Hit Zone according to its name (see [SCR\_CharacterDamageManagerComponent](enfusion://ScriptEditor/scripts/Game/Components/Damage/SCR_CharacterDamageManagerComponent.c;18))

##### Add Random Bleeding

Adds random bleeding

##### Remove All Bleedings

Removes all bleedings

##### Remove Group Bleeding

Removes bleeding from target hit zone group that you can choose from provided ENUM

##### Set Bleeding Rate

Sets bleeding rate for the target character

##### Set Blood

Sets blood level for the target character

##### Set Permit Unconsciousness

Sets whether or not unconsciousness is enabled for target character

##### Set Regeneration Rate

Sets regeneration rate for the target character

##### Set Resilience

Sets resilience value (used for unconsciousness) for the target character

##### Set Saline Bagged Group

Sets whether target hit zone group is saline bagged or not

##### Set Tourniquetted Group

Sets whether target hit zone group is tourniquetted or not

### Add Item To Inventory

It uses the [Getter](#Getters), but for this action, expected target is some entity that has Inventory component (such as GetSpawnedEntity, GetPlayerEntity or GetEntityByName)

It uses the Prefab Filter (a scripted class) to add prefabs of items and their count into the target inventory

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Add Score To CAH Faction

This action allows you to add score to specified Faction in Capture and Hold scenario

* Faction Key - Target Faction key
* Score To Be Added - The amount of score to be added to the specified faction.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Add Task Progress

This action allows you to add progress percentage to a given task.

* Getter - Which task to work with - Use GetTask
* Percentage - Progress in percent

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") ID - Name of the entity used for identification. If entity with given name exists suffix \_numberOfAttemptToNameEntity is added.

### Append Briefing Entry Text

Appends existing briefing entry texts with the new string on a new line

* Faction Key - Target Faction key
* Custom Entry Name - Name of the entry to be appended
* Target Text - Text that will be appended

### Append Briefing Entry Text Based On Task

Appends existing briefing entry texts with the new string on a new line that is based on Task Execution Briefing (see [SlotTask's Task](#Task_4))

* Faction Key - Target Faction key
* Custom Entry Name - Name of the entry to be appended
* Getter - Here, it is expected to use the GetTask which will contain the Task Execution Briefing (see [SlotTask's Task](#Task_4)) attribute that will get appended

### Change Layer Activation Type

This will change the Layer Activation Type attribute (see [Activation](#Activation)).
It is mainly used for example when you have a layer that is set to be activated via trigger and the trigger activates it and when you have Dynamic Despawn Enabled (see [Dynamic Spawn/Despawn](/wiki?title=Dynamic_Spawn/Despawn&action=edit&redlink=1 "Dynamic Spawn/Despawn (page does not exist)")) and you would like the layer to be respawned with subsequent activations, you will need it to change it to SAME\_AS\_PARENT

* Getter - the name of the wanted Layer
* Activation Type - Sets the Activation Type (see [Activation](#Activation))

### Change Layer Termination Status

This will change the Layer Termination Status, which is an internal value set usually when child layers are terminated as well or if a slot has its asset destroyed.

* Getter - Name of the layer to change the termination status
* Terminated - If layer will be marked as terminated or not

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change Task Icon

This action allows you to change Task Icon in runtime

* Getter - Which task to work with - Use GetTask
* Task Icon Set - Task icon set
* Task Icon Name - Name of the specific icon from the icon set

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change Task Ownership

This action allows you to change Task Ownership in runtime

* Getter - Which task to work with - Use GetTask
* Task Ownership - Who will be the owner of the task for whom it will be assignable. By default, it will be owned by the given Faction.

### Change Task State

This action changes the state of the Task. It also triggers other things that listen to the task changed events.

* Getter - Designed to work with the [GetTask](#GetTask) getter
* Task State - Sets the task's new state once this action is activated

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

### Change Task Title Or Description

This action allows you to change Task Title or Description in runtime

* Getter - Which task to work with
* Task title - New Task title
* Task Description - New task description

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change Task UI Visibility

This action allows you to change Task UI Visibility in runtime

* Getter - Which task to work with - Use GetTask
* Task UI Visibility - Where the task will be visible in UI. By default, it will be visible in the Task List and on the Map.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change Task Visibility

This action allows you to change Task Visibility in runtime

* Getter - Which task to work with - Use GetTask
* Task Visibility - To whom the task will be visible. By default, it will be visible for the given Faction.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change Task Visibility

This action allows you to change Task Visibility in runtime

* Getter - Which task to work with - Use GetTask
* Task Visibility - To whom the task will be visible. By default, it will be visible for the given Faction.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Remove Task Progress

This action allows you to remove Task progress in runtime

* Getter - Which task to work with - Use GetTask
* Percentage - Progress in percent

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Set Task Progress

This action allows you to Set Task progress in runtime

* Getter - Which task to work with - Use GetTask
* Percentage - Progress in percent

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Set Parent Task

This action allows you to Set Parent task to a given task at runtime

* Getter - Task Getter
* Parent Task Layer Name - Parent Task Layer Name
* Is Optional - Whether or not the subtask is optional

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Change Time

This action allows you to change time in-game

* Hours - Time of day (hours)
* Minutes - Time of day (minutes)

### Change Trigger Activation Presence

This will change the trigger activation presence for a given trigger.

* Getter - You can either use the GetEntityByName provided that the entity you are searching for is indeed the Trigger, you can also put there a GetSpawnedEntity, or even the Area as this action will try to retrieve the trigger from numerous types of getters
* Activation Presence

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Change User Action Visibility

This action allows you to change user action visibility during runtime.

* Getter - Target entity that has the UserAction on it (Optional if action is attached on Slot that spawns target entity)
* Action ID - ID corresponding to the action attached on ActionsManagerComponent on target entity
* Visible - If checked, user action will be visible, otherwise it will be hidden.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Change Weather

This action allows you to change weather

* Weather Preset Name - The name of the weather preference as it can be found in weatherStates.conf.
* Random Weather Changes - Weather it can change during gameplay
* Transition Duration - Transition Duration
* Automatic Wind - Automatic Wind behaviour. Untick to further tune it with other attributes.
* Wind Speed - Wind Speed in m/s
* Wind Direction - Wind Direction

### Compare Counter And Execute

This will compare Logic Counter value to the set value and if conditions are true, it can execute other actions

* Comparison Operator (see [comparison](#Check_Entities_In_Trigger))
* Value - Value that is the counter is to be compared to
* Counter Name - Name of the counter entity
* Actions - Actions that will be executed (see [Actions](#Actions))

### Count Inventory Items And Execute Action

This will count items in target entity inventory and optionally execute defined actions.

* Getter - Target entity (Optional if action is attached on Slot that spawns target entity)
* Prefab Filter - Which Prefabs and how many out of each will be added to the inventory of target entity
* Actions To Execute - If conditions from Prefab Filter are true, it will execute these actions (see [Actions](#Actions))

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Damage Wheel

This action allows you to set damage to specific wheels on a vehicle.

* Getter - Target entity to damage (Optional if action is attached on Slot that spawns target entity)
* Slot Names On Slot Manager - Name of Slots that are defined on the SlotManagerComponent on target vehicle
* Health Percentage - Health Percentage to be set for target wheels

### Delete Entity

This will delete provided entity. Be careful as this can be very dangerous action with dire consequences and it cannot be reversed!

* Getter - Any getter that returns an Entity (see [Getters](#Getters))

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Debug Action

This action is specific for Debug Actions for Debug menu

* Debug Action Name - Debug Action Name shown in debug menu
* Debug Actions - List of Actions to perform for debug (see [Actions](#Actions))

### End Mission

Action to end mission with further adjustments

* Override Game Over Type - If this action should override what is already set as a Game Over Type or not. This can be used to end mission via some specific thing and either let it end with the Game Over Type that is already set there by the flow of the mission previously or you can choose to override it and in the Overriden Game Over Type choose a new type.
* Overriden Game Over Type - If Override Game Over Type is set to true, you can choose which type will be used for the end.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Action Based On Conditions

This action allows you execute other actions if certain conditions are fullfilled at the time of Action activation. It performs this check only once, not periodically.

* Activation Conditions - Conditions that will be evaluated in given order
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Activation Condition Logic - Which Boolean Logic will be used for Activation Conditions.
* Actions - Actions to be executed if conditions' evaluation is successful.
* Failed Actions - Actions to be executed if conditions' evaluation is failed.

### Execute Function

This is intended for advanced usages and it is expected some scripting knowledge. It will allow you to call methods on certain objects with parameters.

* Object To Call The Method From - It is basically a getter, but it will limit you with the methods depending on which class you will choose. You can for example trigger a despawn or repeated spawn on Slots if getting them that way and using their methods.
* Method to Call - Name of the method to call
* Parameter...Parameter5 - There are the method parameters that you can pass. It is however very limited as it operates with strings.

### Fail Task If Vehicles In Trigger Destroyed

This can fail task (even a previously finished one) when vehicles in trigger provided by the Getter attribute will get destroyed.

* Getter - Here you are supposed to link the trigger (see [Getters](#Getters))
* Target Layer Task - Name of the Layer Task of which Task will get failed
* Caused By Player - If set to true, it will fail the task only when player caused the destruction

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Feed Param To Task Description

This action allows you to feed Params based on Prefab Filter for the specific Slot Task that will be added to the description (Which needs to have parameters in %1 format)

* Getter - Name of the slot task to influence the description parameter
* Prefab Filter - Which Prefabs and how many of them will be converted to a description string

### Increment Counter

This action increments Counter by 1

* Counter Name - Name of the target Counter

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Decrease Logic Counter

This action decreases Logic Counter by 1

* Counter Name - Name of the target Counter

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### On User Action Event

This action allows you to link it to existing user actions on some other entities and based on the User Action Event, you can execute other actions.

* Getter - Target entity that has the UserAction on it (Optional if action is attached on Slot that spawns target entity)
* Action ID - Corresponding to the action attached on ActionsManagerComponent on target entity
* Getter User - Only listen to changes when UserAction is activated by specific Entity (Optional - leave it empty to trigger by anyone)
* User Action Event - On which user action event this ScenarioFramework action will be triggered
* Actions - Which actions will be executed based on User Action Event settings

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Voice Over Play Line

This action allows you to play single line from the Voice Over Data Config.

* Voice Over Data Config - Config with voice over data for this action.
* Line Name - Name of the line as defined in Voice Over Data Config
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Actor Getter - Entity playing the voiceover. Order must be the same as the order of actor enums in Voice Over Data Config. Reminder: Related .acp needs to be present in the SCR\_CommunicationSoundComponent of target entity.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Player Getter - Player entity or entities that the voice over will be played for. Reminder: Related .acp needs to be present in the SCR\_CommunicationSoundComponent of target entity
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Play Immediately - If Voice Line should play right now or wait for the current voice lines to finish and then play it
* Getter - Entity playing the voiceover. If left empty, default will be player character. Reminder: Related .acp needs to be present in the [SCR\_CommunicationSoundComponent](enfusion://ScriptEditor/scripts/Game/Components/SCR_CommunicationSoundComponent.c;60) of target entity.
* Actions - Actions that will be triggered once Line finishes playing

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Voice Over Play Sequence

This action allows you to play sequence of lines from the Voice Over Data Config

* Voice Over Data Config - Config with voice over data for this action.
* Sequence Name - Name of the sequence as defined in Voice Over Data Config
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Actor Getter - Entity playing the voiceover. Order must be the same as the order of actor enums in Voice Over Data Config. Reminder: Related .acp needs to be present in the SCR\_CommunicationSoundComponent of target entity.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Player Getter - Player entity or entities that the voice over will be played for. Reminder: Related .acp needs to be present in the SCR\_CommunicationSoundComponent of target entity
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Play Immediately - If Voice Line should play right now or wait for the current voice lines to finish and then play it
* Actors - Entities playing the voiceover. If left empty, default will be player character. Reminder: Related .acp needs to be present in the SCR\_CommunicationSoundComponent of target entity.
* Actions - Actions that will be triggered once Sequence finishes playing

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Intro Voiceline Based On Tasks

This action will play voice line based on what tasks were generated and their voice line attribute. Action Process Voiceline Enum And String needs to be called before this action for it to work.

* Sound - Sound to play
* Getter - (Optional) If getter is provided, sound will come from the provided entity

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Change All Task State

This action will change all tasks' task states.

* Layer Tasks To Ignore - Layer Tasks To Ignore
* Ignored Task States - Ignored Task States
* New Task State - New Task State

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Set Grenade Live

This action will set a grenade live.

* Getter - Grenade entity to set live.
* Enable Simulation - If enabled, grenade will be affected by physics

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Spawn Objects From Variable

This action will spawn objects passed by variable

* Name Of Variables To Spawn Objects From On Activation - These variables will be checked for object names once the trigger becomes active.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### On Weapon Ammo Count Changed Action

This action will activate other actions based on ammo count changes

* Getter - Weapon getter.
* Actions - Actions to be executed if conditions' evaluation is successful.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Resource Component Action

This action will activate other subactions working with Resources aka Supplies

* Getter - Target entity for Resource Action
* Resource Component Actions - Resource system actions to be executed

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Resource Component Actions

#### SCR\_ScenarioFrameworkActionOnResourceChanged

* Actions - Actions that will be executed on compartment entered
* Activation Percentage - Activation Percentage
* Comparison Operator - Operator

#### SCR\_ScenarioFrameworkActionOnResourceConsumerChange

* Actions - Actions that will be executed on compartment entered
* Activation Amount - Activation amount
* Comparison Operator - Operator

#### SCR\_ScenarioFrameworkActionSetResourceTypeEnabled

* Getter - entity to manage resource types on.
* Check Children - Check Children
* Enable Resource Type - Enable Resource Type
* Resource Type To Handle - Resource Type To Handle

#### SCR\_ScenarioFrameworkResourceComponentActionTransfer

* Target Getter - Target entity for Resource Action
* Transfer Amount - Transfer Amount
* Allow Partial Transfer - Allow Partial Transfer

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Action Destruction

* Entity Getter - Specific building to be destroyed.
* Radius Destruction - Radius Destruction
* Destruction Radius - Destruction Radius

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Action On Damage

* Getter - Entity to check for damage.
* Damage Context Conditions - Checked Conditions.
* Actions - Actions to be executed if conditions' evaluation is successful.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Action On Damage State Changed

* Getter - Entity to check damage changes on.
* Action - What to do on damage state change

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

### Action Vehicle

* Getter - Target entity for Vehicle Action
* Vehicle Actions - Vehicle actions that will be executed on the target entity

#### SCR\_ScenarioFrameworkVehicleActionDamageHitZoneByName

* Health Percentage - Health Percentage to be set for target hit zone
* Hit Zone Name - HitZone Name

#### SCR\_ScenarioFrameworkVehicleActionDamageHitZonesByGroup

* Health Percentage - Health Percentage to be set for target hitzone

#### SCR\_ScenarioFrameworkVehicleActionResourceUnloadAction

* Resource Type - Resource Type
* Resources To Unload - Resources To Unload

#### SCR\_ScenarioFrameworkVehicleActionSetHandbrake

* Set Handbrake - Set Handbrake

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

### Random Action

This action will select one random subaction and execute it.

* Random Action - Upon activation, one random Action will be selected from these Actions

### Kill Entity

This action allows you to kill any provided entity. Be careful as this is quite dangerous action and it cannot be reverted back.

* Getter - Any getter that returns an Entity (see [Getters](#Getters))
* Randomize Ragdoll - If target entity is Character, it will randomise ragdoll upon death

### Lock or Unlock All Target Vehicles In Trigger

This action will either lock or unlock all target vehicles in trigger provided by the Getter attribute.

* Getter - To get the trigger you want to work with to influence all vehicles inside it (see [Getters](#Getters))
* Lock - If true, it will lock vehicle, if false, it will unlock vehicles

### Lock or Unlock Vehicle

This action will either lock or unlock target vehicle provided by the Getter attribute.

* Getter - To get the entity of a vehicle you want to work with (see [Getters](#Getters))
* Lock - If true, it will lock vehicle, if false, it will unlock the vehicle

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Loop Over Not Randomly Selected Layers

This action allows you to loop over the layers that were not selected by GetRandomLayerBase

* Getter - Use GetRandomLayerBase
* Action - Which actions will be executed for each layer that was not randomly selected

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### On Compartment Entered Or Left

This action allows you to trigger actions if someone gets in or out of the vehicle.
You can further specify to only listen to certain Slots in the vehicle (Driver, passenger on specific seats) and even listen for some specific entity to enter such slots.

* Entered or Left - If true, we execute actions On Compartmented Entered. Otherwise On Compartment Left
* Getter - Target entity (Optional if action is attached on Slot that spawns target entity)
* Occupant Getter - (Optional) If used, it will get executed only when specific entity enters the compartment
* Slot IDs - (Optional) If used, it will get executed only when specific compartment slots are entered (Inspect each Prefab BaseCompartmentManagerComponent to see the slots. From our observation, driver usually has ID number 2)
* Actions - Actions that will be executed on compartment entered

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### On Engine Started Or Stop

This action allows you to trigger actions if target entity engine gets started or stops.

* Started Or Stop - If true, we execute actions On Engine Started. Otherwise On Engine Stop
* Getter - Target entity (Optional if action is attached on Slot that spawns target entity)
* Actions - Actions that will be executed on one of these circumstances

### Play Music

Play define music.

* Music - Name of the music to be played. Music manager needs to be present in the world and music needs to be configured in acp file.

### Play Sound

This action will play sound on player entity position

* Sound - String of with the name of the sound

### Play Sound On Entity

This action will play sound on entity position provided by the Getter Attribute

* Getter - Supposed to be any that will return entity (see [Getters](#Getters))
* Sound - String of with the name of the sound

### Prepare Area From Dynamic Despawn

It adds target Area into the [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn_2) during runtime.

* Getter - Supposed to be a GetArea (see [Getters](#Getters))
* Stay Spawned - If set to false, area will be despawned upon activation of this action
* Dynamic Despawn Range - How close at least player character must be in order to trigger dynamic spawn/despawn

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Prepare Layer From Dynamic Despawn

It adds target Layer into the [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn_2) during runtime.

* Getter - GetLayerBase with Layer Base Name (see [Getters](#Getters))
* Stay Spawned - If set to false, layer will be despawned upon activation of this action
* Dynamic Despawn Range - How close at least player character must be in order to trigger dynamic spawn/despawn

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Process Voiceline Enum And String

This action processes the Voiceline Enum to specific strings that will be then used in Intro Voiceline Based On Tasks action.

* Target Enum - Name of the enum to work with

### Remove Area From Dynamic Despawn

It removes target Area from the [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn_2) during runtime.

* Getter - Supposed to be a GetArea (see [Getters](#Getters))
* Stay Spawned - If set to false, area will be despawned upon activation of this action

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Remove Layer From Dynamic Despawn

It removes target Layer from the [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn_2) during runtime.

* Getter - GetLayerBase with Layer Base Name (see [Getters](#Getters))
* Stay Spawned - If set to false, layer will be despawned upon activation of this action

### Remove Item From Inventory

This action is used to remove items from target inventory. This can be quite dangerous as it will delete it and it cannot be reversed.

* Getter - target entity that has inventory (see [Getters](#Getters))
* Prefab Filter - Here you can set which items and how many out of each will be removed

### Reset Counter

This action will reset the counter back to 0. It is supposed to be activated only from the counter it is to reset.

* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Counter Name - Counter to reset (Optional if this action is attached on Counter)

### Set Briefing Entry Text

Sets Briefing category text with the one you provide

* Faction Key - Target Faction key
* Custom Entry Name - Name of the entry to be changed
* Target Text - Text that you want to use

### Set Briefing Entry Text Based On Generated Tasks

Sets Briefing category text based on generated tasks. This is used to dynamically the texts based on which tasks are actually spawned.

* Faction Key - Target Faction key
* Custom Entry Name - Name of the entry to be changed based on generated tasks
* Target Text - Text that you want to use. Leave empty if you want to utilise the one set in config.

### Set Entity Position

Moves entity to a different position. Similar to [setPosASL](/wiki/setPosASL "setPosASL") from previous titles, teleports the entity.

* Entity Getter - Getter of which Entity we want to work with (see [Getters](#Getters))
* Destination - world coordinates
* Destination Entity Getter - (Optional) You can also use some other entity as a destination coordinates
* Destination Entity Relative Position - (Optional) you can further define offset from the destination entity where to move you entity

### Set Entity Scale

Set a scale of given entity.

* - Entity Getter - Getter of which Entity we want to work with (see [Getters](#Getters))
* - Entity Scale - Intended Scale

### Set Execution Entry Text Based On Generated Tasks

Sets Execution category text based on generated tasks. This is used to dynamically the texts based on which tasks are actually spawned.

* Faction Key - Target Faction key
* Custom Entry Name - Name of the entry to be changed based on generated tasks
* Target Text - Text that you want to use. Leave empty if you want to utilise the one set in config

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Set Faction to CAH Area

This action allows you to set faction to Capture and Hold Area

* Getter - Entity of the CAH Area (Optional if action is attached on Slot that spawns target entity)
* Faction Key - Target Faction key

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

### Set Fuel Consumption

This action allows you to set fuel consumption for target entity.

* Getter - Target entity to manipulate fuel (Optional if action is attached on Slot that spawns target entity)
* Fuel Percentage - Percentage of a fuel to be set

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Set Fuel Percentage

This action allows you to set fuel percentage for target entity.

* Getter - Target entity to manipulate fuel consumption (Optional if action is attached on Slot that spawns target entity)
* Fuel Consumption - Fuel consumption at max power RPM\n[liters/hour]
* Fuel Consumption Idle - Fuel consumption idle\n[liters/hour]

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Set Supply Percentage

This action allows you to set specific percentage of supply on target entity.

* Getter - Target entity to manipulate supply (Optional if action is attached on Slot that spawns target entity)
* Supply Percentage - Percentage of a supply to be set.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Set Signal

This action allows you to set specific value to specific signal on target entity.

* Getter - Entity to set the signal on (Optional if action is attached on Slot that spawns target entity)
* Signal - Signal to set
* Value - Value to set

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Create Variable

This action allows you to create specific variable to specific value.

* Variable Name - Name of the variable
* Variable Value - Value of the variable

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Set Variable

This action allows you to set specific variable to specific value.

* Variable Name - Name of the variable
* Variable Value - Value of the variable

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Get Variable Value

This action gets the Variable Name.

* Variable Name - Name of the variable

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Item Safeguard

This action allows you to attach Item Safeguard logic on any item that you might regard as important and in case of it being in a car that gets destroyed, or in player inventory and similar cases, it this item will be dropped on the ground.
It also prevents Garbage System from deleting this item. It also allows you to execute actions in situations when item is dropped or possessed.

* Getter - Target entity (Optional if action is attached on Slot that spawns target entity)
* Actions On Item Dropped - Actions that will be executed when target item is dropped
* Actions On Item Possessed - Actions that will be executed when target item is possesed by someone/something

### Set Mission End Screen

Sets mission end screen based on the Game Over Type attribute and it can also provide which Subtitle will be used

* Game Over Type - Which Game Over Type will be set upon using this action.
* Subtitle - Which subtitle text will be used

### Set Vehicle Cruise Speed

Set a maximal cruise speed of a given vehicle.

* Getter - Target entity
* Max Cruise Speed - The maximum speed the vehicle can go. (Does not apply to a vehicle driven by players).

### Show Hint

Shows a very simple hint with Title and Text for a certain duration.

* Title - Title of the hint
* Text - Text in the body of the hint
* Timeout - duration in seconds how long will the hint stay before disappearing
* Faction Key - For which Faction these hints will be shown
* Getter - (Optional) Getter to get either a specific player or array of player entities to whom the hint will be shown

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

### Show Layout

Shows a layout to specified players.

* Layout - Resource name of the layout
* Fade In - Fade in time
* Fade Out - Fade out time
* Visibility Time - For how long the layout should stay visible.
* Opacity Value - Target value for opacity.
* ID - Unique layout ID
* Getter - Getter to get either a specific player or array of player entities

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Show Popup Notification

This action allows you to show Popup notification. You can further filter it to specific faction and players.

* Title - Title of the Popup
* Text - Text of the Popup (Keep it short)
* Faction Key - (Optional) You can filter only specific faction
* Getter - Getter to get either a specific player or array of player entities

### Spawn Closest Object From List

This action is used to trigger spawning of certain layers that are set with ON\_TRIGGER\_ACTIVATION and you are supposed to put their names into the attribute List Of Objects.
It will then find the closest one to the entity provided by Getter Attribute.

* Getter - Supposed to be the one that returns any entity you want to look the closest objects from (see [Getters](#Getters))
* List Of Objects - List of the named Scenario Framework components that are supposed to be spawned. In most cases, you just need to put there the top layer which was not activated by other means.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Activation Type - Sets the Activation Type (see [Activation](#Activation))

### Spawn Objects

This action is used to trigger spawning of certain layers that are set with ON\_TRIGGER\_ACTIVATION based on their names listed into the **Name Of Objects To Spawn On Activation** attribute.

* Name Of Objects To Spawn On Activation - List of the named Scenario Framework components that are supposed to be spawned. In most cases, you just need to put there the top layer which was not activated by other means.
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Activation Type - Sets the Activation Type (see [Activation](#Activation))

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Spawn Objects Based On Distance

This action allows you to spawn other objects based on a distance parameters and employ some randomisation

* Getter - Measure distance from what - use getter
* Min Distance - It will select only objects that are at least x amount of meters away
* Max Distance - You can also set max distance to setup the hard limit of the max distance - but be aware that there might be a situation where it would not spawn anything
* List Of Objects - List of objects that are to be compared
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") Activation Type - Sets the Activation Type (see [Activation](#Activation))
* Spawn Objects - Spawn all objects, whether only a random one multiple random ones
* Random Percent - If the RANDOM\_MULTIPLE option is selected, the chance percentage

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Toggle Engine

This action allows you to start/stop engine of target entity.

* Getter - Target entity to turn on/off the engine (Optional if action is attached on Slot that spawns target entity
* Turned On - If true, engine will be turned on. Otherwise it will turn it off

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Toggle Light

This action allows you turn on/off specific lights on target entity (usually a vehicle or a lamp).

* Getter - Target entity to manipulate lights with (Optional if action is attached on Slot that spawns target entity)
* Light Type - Which lights to be toggled
* Turned On - If true, light will be turned on. Otherwise it will turn it off.

### Wait And Execute

This is used to activate other actions with a timed delay.

* Delay In Seconds - How long it will wait before activating set actions.
* Delay In Seconds Max - If this is set to a number larger than Delay In Seconds, it will randomise resulted delay between these two values
* Looped - If true, it will activate actions in looped manner using Delay settings as the frequency. If randomised, it will randomise the time each time it loops.
* Actions - Actions to be activated after the delay has passed.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.5.0 "Category:Arma Reforger/Version 1.5.0") [1.5.0](/wiki?title=Category:Arma_Reforger/Version_1.5.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.5.0 (page does not exist)")

## Nested Subtasks

Layer Tasks are capable of handling singular Tasks but also able to handle nested subtasks. By default, all Layer Task is set to be a parent task, but you can set it to be a subtask as well. This is done by either setting the Parent Task Name in the Layer Task component or putting the Layer Task in hierarchy under the parent Layer Task.

The parent task can handle all subtasks and their states. The subtasks can handle their own states as well. The parent task can also be set to how many tasks are required to be finished, and if the Parent task functions on its own - Has its own functionality so it is a task of its own at the top of its subtasks. The parent task set functionality can be finished before finishing all subtasks, but it will not be finished until all subtasks are finished.

It also has a progress bar feature that is disabled by default. It is calculated based on the number of completed tasks (optional tasks excluded), but this feature can be turned off and the progress bar can be managed by dedicated actions (Add Task Progress, Remove Task Progress or Set Task Progress).

Subtasks can be set to be Optional, meaning they are not necessary to be finished in order to trigger the finish of the parent task. You can also change some task into a Subtask in run-time via Set Parent Task Action.

You can check out the SF-Sample-Subtasks sample to see how easily it can be set up.

## QRF System

**QRF** stands for **Q**uick **R**eaction **F**orce and is a unit that is capable to respond to certain actions as a response in a manner of minutes or hours depending on the size and application of that unit.

### Usage

In Arma Reforger, the QRF system is responsible for reacting to players' actions, such as killing enemy soldiers, by sending a stronger response force each time a certain threshold is met.

### Configuration

As an example of such an application, we can use the Combat Ops scenario on Everon, for which QRF is already configured. The process of adding a new QRF to the area is as follows:

1. Create an [Area](/wiki/Arma_Reforger:Scenario_Framework#Area "Arma Reforger:Scenario Framework") which QRF will be monitoring
2. Go to the [SCR\_ScenarioFrameworkArea](enfusion://ScriptEditor/scripts/Game/ScenarioFramework/Components/SCR_ScenarioFrameworkArea.c;24) component in the Area's "Object Properties" window
3. Find the section called "Activation Actions" where you will have to drag and drop pre made config of QRFDispacher or optionally use the action script itself: [SCR\_ScenarioFrameworkActionQRFDispacher](enfusion://ScriptEditor/scripts/Game/ScenarioFramework/Actions/SCR_ScenarioFrameworkActionQRFDispacher.c;8)
   * (Optional) Configure QRF as wanted by changing the values of all properties, e.g add new or change the cost of existing QRF groups
4. In order to define the QRF units's spawn positions, create a QRF spawn point layer for this area by dragging and dropping the Prefab on the area
5. Copy the name of the QRF spawn point layer and paste it into the "*QRF Layer Name*" field in Area's QRFDispatcher

   ⓘ

   The spawn point layer can be renamed but the name in "*QRF Layer Name*" has to match the actual entity name.
6. Place the QRF spawn point at the wanted location by selecting it in and then moving it to desired location using gizmos

   ⓘ

   It is highly recommended to use function "*Snap to Ground*" in order to ensure that the spawn point is not above or below ground.
7. Configure the spawn point: what can be spawned there, what is the minimum distance in metres from the nearest player

If more spawn points are needed, more QRF spawn point Prefabs can be dragged and dropped on the QRF layer, then configured the same way as explained in steps 6 and 7.

## Dynamic Spawn/Despawn

This system works on [Areas](#Area_2) that are then handling their hierarchy and its purpose is to save performance by having spawned only things that need to be spawned.

There is a continuous check that is being triggered every 4 seconds in default (can be adjusted on GameMode component called [SCR\_GameModeCombatOpsManager](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeCombatOpsManager.c;6)) for all the Areas that have Dynamic Spawn/Despawn enabled.

Furthermore, when Dynamic Spawn is initiated, it is spawning its children with slight delays between them in order to avoid huge stutters when a lot of different things are spawned at once, so upon successful Dynamic Spawn initiation, it can take few seconds for the Area to completely spawn (depending on the complexity of each Area setup).

Each Area has a range that is then compared to the distance from player characters.

If just one player is within a range, Area will spawn all of its content that has Activation Type SAME\_AS\_PARENT set in workbench or this activation was changed in runtime via ScenarioFrameworkAction.

Runtime change is the case for QRF units in CombatOps on Arland Scenario where they are initially not spawned with the Area but via trigger.

Once the trigger spawns these units, it also changes the Activation Type to SAME\_AS\_PARENT so when said Area would despawn and then spawn again, it will spawn these units as well.

On layers that are under the Area, there is an option to "Exclude from Dynamic Despawn" which if set to true will prevent said layer to despawn once it is spawned.

This is used for example for Spawn Points, where we want to spawn them, but then we are not dynamically despawning them using this system.

Furthermore, all of the Slots that are spawning vehicles are set in a way that once someone gets inside of them, it will "Exclude" them from the Dynamic Despawn as well to prevent vehicle deletion upon moving away from Area range.

Slots and AI Slots retain their position where their spawned entity was located before Spawn/Despawn, so for example AI units that moved to different position will get their position saved and once respawned, they will be spawned on their previously saved position.

If AI units are killed, their bodies will remain there for the GarbageCollector to handle and for the vehicles, it will remove their wreck and destroyed wreck will not be spawned again.

All this above can be also setup for Layers themselves that are managed by their parent Area.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

## Faction Aliases

Faction Aliases feature allows you to setup your scenario in a way where you create Aliases for given Faction Keys that can be then easily changed either in the World Editor or even Server Admins via Mission Header.When you are creating scenario, you can designate which Faction will be the default "OPFOR" and "BLUEFOR". For example, the CombatOps on Arland has playable faction US and the adversary force is USSR. You can easily create so called Alias for each of those two with the default value and then, if you would like to change the adversary for to FIA, you can do so with simple change on one place. Then you can use Faction Alias instead of the Faction Key, provided that you are utilizing Faction catalogs or Faction Switched objects to spawn faction-specific objects.

Basically, you setup your mission only once and you can change Factions later on / or when launching the server as you wish, provided that everything is setup correctly.

### Configuration

1. Add [SCR\_FactionAliasComponent](enfusion://ScriptEditor/scripts/Game/GameMode/FactionManager/SCR_FactionAliasComponent.c;11) to your FactionManager in the world
2. Configure the Faction Aliases by reating the Alias and give it a Faction Key
3. Configure Mission Header to mirror the setup of [SCR\_FactionAliasComponent](enfusion://ScriptEditor/scripts/Game/GameMode/FactionManager/SCR_FactionAliasComponent.c;11)
4. Use your Faction Aliases instead of Faction Keys on ScenarioFramework related things
5. Use Randomization together with Faction Catalogs to configure which asset should get spawned or use Faction Switched objects

## Samples

Samples are simple "scenarios" that usually focus on one particular feature of the Scenario Framework or a goal to show how it could be done.

There are many ways how to achieve things and here will be the small selection of endless possibilities that this system can provide.

You can use it to learn from it, copy it and adapt it as you please.

All the samples already have prepared necessary entities for the Framework to work so if you would just copy-pasted it into the empty world, it would not straight up work.

You need to setup your worlds according to the [Scenario Framework Setup Tutorial](/wiki/Arma_Reforger:Scenario_Framework_Setup_Tutorial "Arma Reforger:Scenario Framework Setup Tutorial").

They are located in the /worlds/ScenarioFramework directory where there are root files and Tutorial and then the Samples are in the Samples folder.
Task Move is one of the most simple tasks to have presented here and recommended to start learning the Framework with it.

### TaskMove

*Goal:*

Create a Task Move for the US Faction.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskMove](#LayerTaskMove) and the [SlotMoveTo](#SlotMoveTo) placed in the hierarchy as seen in the image below.

Everything is set basically in default.

*Mechanics:*

Area spawns LayerTaskMove which Spawns SlotMoveTo that spawns Task Move and assigns it to the default US faction and the slot also spawns trigger that is used to detect if some player entered it in order to finish the Task.

Here the radius set to 10 meters and it will require at least 1 player to enter for it to finish this task.

### TaskKill

*Goal:*

Create a Task Kill for the US Faction.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskKill](#LayerTaskKill) and the [SlotKill](#SlotKill) placed in the hierarchy as seen in the image below.

LayerTaskKill has set Task Title and Task Description attributes and SlotKill has Object To Spawn attribute set to spawn Unarmed soldier.

This is done for the sample purposes, having Unarmed soldiers as targets for SlotKill is a war crime.

*Mechanics:*

Area spawns LayerTaskKill which Spawns SlotKill that spawns Task Kill and assigns it to the default US faction and the slot also spawns Unarmed soldier which is the target of the Task Kill.
LayerTaskKill will also process display name of the entity and can display it in provided description, in this case: Unarmed.
In order to finish this task, target entity must be killed.

### TaskExfil

*Goal:*

Create a Task Exfil for the US Faction.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskMove](#LayerTaskMove) and the [SlotExtraction](#SlotExtraction) placed in the hierarchy as seen in the image below.

Everything is set basically in default apart from the SlotExtraction timer set to 20 seconds.

*Mechanics:*

Area spawns LayerTaskMove which Spawns SlotExtraction that spawns Task Exfil (which is basically the Task Move) and assigns it to the default US faction and the slot also spawns trigger that is used to detect if some player entered it and been there for 20 seconds in order to finish the Task.

Here the radius set to 10 meters and it will require at least 1 player to stay inside for 20 seconds to finish this task.

After finishing the task, mission is ended *via* the [End Mission](#End_Mission) action that is attached in OnTaskFinished in LayerTaskMove.

### TaskDestroyRock

*Goal:*

Create a Task Destroy Rock for the US Faction.

This example is here to show how to link entities either already in world or newly spawned before the Slot will be spawned and then being able to work with them.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskDestroy](#LayerTaskDestroy) which has Layer that contains normal Slots that will spawn RPG-7 and SlotDestroy that in this case will not spawn anything but it will search in the vicinity for the prefab that was set in the Object To Spawn attribute thanks to Use Exsiting World Asset attribute being set to true.

Outside of the Scenario Framework, there is a Granite\_Boulder\_01 prefab put into the world and named RockToDestroy.

It was modified in a way that we have added destruction to it via the [SCR\_DestructionMultiPhaseComponent](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionMultiPhaseComponent.c;20) and also the [RplComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/RplComponent.c;17) so it can be properly destroyed.

*Mechanics:*

First the RockToDestroy is placed in the world and then the Area spawns the LayerTaskDestroy which spawns Layer that spawns all the Slots with RPG-7 and SlotDestroy that will search for the Granite\_Boulder\_01 and then it will use it as target entity for the Task.

Task is then finished upon destroying target entity, in this case, the rock.

### TaskDestroy

*Goal:*

Create a Task Destroy for the US Faction - in this case, target will be a vehicle.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskDestroy](#LayerTaskDestroy) which has Layer that contains normal Slots that will spawn RPG-7 and SlotDestroy that spawns UAZ-469 which is linked to the created Task.

*Mechanics:*

Area spawns the LayerTaskDestroy which spawns Layer that spawns all the Slots with RPG-7 and SlotDestroy that spawns UAZ-469.

LayerTaskKill will also process display name of the entity and can display it in provided description, in this case: UAZ-469.

In order to finish this task, you need to destroy the vehicle (you can also drown it in the water).

### TaskDeliverIntel

*Goal:*

Create a Task Deliver Intel for the US Faction.

In this case, we want a specific item to be delivered there it cannot be interchanged with the same prefab.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskDeliverIntel](#LayerTaskDeliverIntel) and the [SlotPick](#SlotPick) together with [SlotDelivery](#SlotDelivery) placed in the hierarchy as seen in the image below.

Everything is set basically in default, but SlotPick has texts filled in the Task category and it is supposed to spawn IntelligenceFolder\_E\_01.et from the Object To Spawn attribute.

SlotDelivery then spawns Trigger and LayerTaskDeliver is linked there in Associated Task Layers attribute.

*Mechanics:*

Area spawns the LayerTaskDeliverIntel which spawns SlotPick that will spawn the intelligence folder for you to pick it up and carry.

In this particular example, SlotDelivery is also spawned from thy LayerTask right from the start and it spawns trigger that is used to detect player character and search for the folder in it.

The spawned asset from SlotPick is linked to this task so truly have to deliver this specific item.

In order to finish the task, deliver the intel to the trigger with the player character.

### TaskDefendTarget

*Goal:*

Create a Task Defend Target for the US Faction.

It can be any entity that can be destroyed, in this case, we chose unarmed US Soldier.

*Setup:*

it is achieved by having an [Area](#Area_2) with [LayerTaskDefend](#LayerTaskDefend) that spawns SlotDefend that will further spawn the Unarmed US Soldier.

*Mechanics:*

Area spawns LayerTaskDefend which spawns SlotDefend which spawns target to defend, in our case Unarmed US Soldier.

There is a timer set for 15 seconds and in order to finish this scenario successfully, you just have to wait whilst Unarmed Soldier still being alive. If he dies, the task will fail.

### TaskDefendArea

*Goal:*

Create a Task Defend Area for the US Faction.

Goal will be to repel enemy forces and have majority at the end of the task countdown.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskDefend](#LayerTaskDefend) that has two layers under it.

Attacker\_USSR which is defined in **Attacker Layer Names** contains three SlotAI that each will spawn Unarmed USSR Soldier on repeated basis.

Each slot has the Repeated Spawn enabled and the layer has it enabled as well and set it to spawn repeatedly 2x each 15 seconds.

Defender\_US layer then contains SlotTrigger that will spawn Trigger, Slot1 which spawns M60 machinegun turret and SlotDefend which will not spawn anything but the trigger is set there instead.

LayerTaskDefend also has Faction Settings set in a way that the attacking faction is considered USSR and it counts all the characters.

Defending faction is set to be US and it will count all the characters as well.

Defend time is set to 60 seconds and Defender percentage ratio is set to 0.51 which means that at the evaluation, there needs to be a majority of defenders in comparison to attackers.

*Mechanics:*

Area spawns LayerTaskDefend which spawns Attacker\_USSR layer that will spawn 3 Unarmed USSR Soldiers initially and 15 seconds later, it will spawn additional wave using the Repeated Spawn feature.

Alongside it, Defender\_US layer will spawn SlotTrigger which will spawn trigger that will be used to detect characters inside the designated area.

Slot1 then spawns M60 machinegun that can be used to quickly eliminate all the attackers.

SlotDefend then spawns trigger.

Area needs to be cleared out of the enemy presence in ratio that the majority of the units inside need to be of US faction.

If it is below this ratio, task will fail.

### TaskDefendAreaAndTarget

*Goal:*

Create a Task Defend Area and Target for the US Faction.

This will be a combination of Defend Area and Defend Target and indeed both conditions will apply at the same time.

*Setup:*

It is achieved by having an [Area](#Area_2) with [LayerTaskDefend](#LayerTaskDefend) that has two layers under it.

Attacker\_USSR which is defined in Attacker Layer Names contains three SlotAI that each will spawn Unarmed USSR Soldier on repeated basis.

Each slot has the Repeated Spawn enabled and the layer has it enabled as well and set it to spawn repeatedly twice every 15 seconds.

Defender\_US layer then contains SlotTrigger that will spawn Trigger, Slot1 which spawns M60 machinegun turret and SlotDefend which spawns Unarmed US Soldier.

LayerTaskDefend also has Faction Settings set in a way that the attacking faction is considered USSR and it counts all the characters.

Defending faction is set to be US and it will count all the characters as well.

Defend time is set to 60 seconds and Defender percentage ratio is set to 0.51 which means that at the evaluation, there needs to be majority of defenders in comparison to attackers.

*Mechanics:*

Area spawns LayerTaskDefend which spawns Attacker\_USSR layer that will spawn 3 Unarmed USSR Soldiers initially and 15 seconds later, it will spawn additional wave using the Repeated Spawn feature.

Alongside it, Defender\_US layer will spawn SlotTrigger which will spawn trigger that will be used to detect characters inside the designated area.

Slot1 then spawns M60 machinegun that can be used to quickly eliminate all the attackers.

SlotDefend then spawns Unarmed US Soldier that is supposed to be defended.

This combines both conditions so the whole time the Unarmed US Soldier needs to be kept alive and at the end of the timer, area needs to be cleared out of the enemy presence in ratio that the majority of the units inside need to be of US faction.

If one of the conditions will fail, it will fail the task.

### TaskClearArea

*Goal:*

Create a Task Clear Area for US Faction.

There is some area that is occupied by enemy forces and you need to clear it out.

*Setup:*

It is achieved by having an [Area](/wiki?title=Area_2&action=edit&redlink=1 "Area 2 (page does not exist)") with [LayerTaskClearArea](#LayerTaskClearArea) that has two layers under it.

First layer holds SlotClearArea which spawns TriggerDominance trigger that is set in a way that it is activated by Player and detects all [SCR\_ChimeraCharacter](enfusion://ScriptEditor/scripts/GameCode/Character/SCR_ChimeraCharacter.c;33).

Second Layer contains 3x SlotAI that will spawn Unarmed USSR soldiers.

*Mechanics:*

Area spawns LayerTaskClearArea which spawns Layer that spawns SlotClearArea which spawns TriggerDominance that is used to periodically check the set area for the enemy presence.

Then Layer\_AI spawns 3 SlotAIs that each spawns 1 Unarmed USSR Soldier. In order to finish this task, you need to clear out all the enemies in the area.

### PatrollingGroup

*Goal:*

Create a AI group that will be patrolling in set patern indefinitely.

*Setup:*

It is achieved by having an [Area](#Area_2) that has Layer AI\_group which has Layer Patrol1 that has SlotAI1 which spawns US Sentry group.
Then there is Layer Patrol\_Waypoints that contains three slots, each spawning Patrol waypoint.
SlotAI1 has modified attribute category Waypoints where in the Waypoint Group Names, there was added a WaypointSet which contains Patrol\_Waypoints and attribute Cycle Waypoints is set to true.
Also Spawn AI On WP Pos is enabled for the SlotAI1.

*Mechanics:*

Area Spawns AI\_group which spawns Patrol1 which Spawns SlotAI1 which spawns US Sentry group that has link to the Patrol\_Waypoints Layer that is spawned right after it which spawns 3 Patrol waypoints.
These waypoints are then assigned to the AI group and are being cycled indefinitely. There is no end state, this group will conduct patrol for eternity.

### FinishTaskToCreateTask

*Goal:*

Create simple sequence of tasks that once one is completed, next one is spawned upon finishing it showing one possible way how to create this workflow.

*Setup:*

It is achieved by having 3 distinct Areas. Each Area has LayerTaskMove with SlotMoveTo in their hierarchy that works exactly the same as [TaskMove](#TaskMove) sample.
Additionally, Area\_A and Area\_B also has LogicCounter.
Each listen to their respective TaskMove set in the Input for the task finish via OnTaskEventIncreaseCounter (see [On Task Event Increase Counter](#On_Task_Event_Increase_Counter)) action and then OnActivate, there is an action Spawn Objects which will spawn next LayerTask.
There are many ways how to create this sequence, Here, we used Counters to showcase them, but it can be achieved without them just by putting the action Spawn Objects to the LayerTask OnTaskFinished action.
LayerTaskMove in Area\_B and Area\_C both have Activation Type attribute set to ON\_TRIGGER\_ACTIVATION which prevents these layers being spawned on scenario start of task generation init.
These Layers will be able to spawn only when called with ON\_TRIGGER\_ACTIVATION action, such as is set in LogicCounter.

*Mechanics:*

All Areas spawn in, but only the Area\_A will spawn the LayerTaskMove with the SlotMoveTo and LogicCounter.
Upon entering trigger that the SlotMoveTo spawns, finishing the objective will increase the counter which will trigger spawning of next LayerTaskMove from Area\_B.
That will do the same as in Area\_A and spawning LayerTaskMove in Area\_C as a final objective.

### EndScreenBasedOnCompletedTasks

*Goal:*

Create a workflow that will alter end screen based on the completed tasks.

*Setup:*

There are 4 areas each containing their respective tasks.
Documents have similar setup to TaskDeliverIntel Sample, but SlotPick 2 upon picking up documents ends the scenario via OnTaskProgress action End mission.
Extraction bears similarities from TaskExfil, but additionaly, it has SlotDelivery\_01 there that is linked for the D\_Documents and Slot\_1 which spawns american flag.
Upon finishing it, it will also end the mission via OnTaskFinished action End Mission.
It has the endScreenCounter as well that is set in a way that upon each task finish, it increases its count up until 4.
OnIncrease actions are then set with Compare Counter And Execute with nested action to [Set Mission End Screen](#Set_Mission_End_Screen).
Task Kill is very similar to TaskKill Sample and those 4 BrickWall\_01\_4m\_1 surround its target.
Move is the same as TaskMove Sample.

*Mechanics:*

All of the Areas spawn their tasks as you may know it from samples mentioned above with minor tweaks to them.
Under the USSR flag, the folder with intel can end the scenario right away and you will get end screen where you did not manage to complete any of the tasks if you do just that.
If you finish at least one, you will get different ending and this goes on and on according to what is set in the endScreenCounter.
In order to finish the scenario completely, you need to finish all the objectives and then end the scenario by exfilling under the US flag an not picking up the intel under the USSR flag.

### DynamicDespawn

*Goal:*

This Sample is here to showcase how to use the Dynamic Spawn/Despawn feature.

*Setup:*

It is achieved by having Area that has three layers in the hierarchy.

Area has Dynamic Despawn attribute enabled and Dynamic Despawn Range set to 30.

Layer "AI" that has several AI slots that each is setup to spawn either group or singular unit.

LayerTaskDestroy "LayerTaskDestroy\_1" which has Spawn Children attribute set to *RANDOM\_ONE* that can spawn either Layer1 or Layer2 which each would spawn SlotDestroy that spawns different vehicle.

Layer "RandomFlag" is set to *RANDOM\_ONE* as well and it will randomly pick one of three Slots that each has different flag set (US, USSR or FIA).

On a GameMode (SF-Tutorial-Empty -> default -> GameModeSF1) component called SCR\_GameModeSFManager, Dynamic Despawn attribute is set to true and Update Rate is lowered to 1 second from 4 seconds for the Sample purposes.

*Mechanics:*

At first, *Area* is Spawned but it does not do anything and does not spawn any children until you move player character inside its Dynamic Despawn Range which is set to 30 meters.

This check is in Sample worlds performed every 1 second, but **it is recommended to have it set around 4 seconds in real scenarios to save up the performance**.

*Area* is also set to visualise this range to make it easier for you to see the range.

Upon entering, Dynamic Spawn will be triggered that will spawn *Layer "AI"* with all the AI slots and their groups.

Then *LayerTaskDestroy "LayerTaskDestroy\_1"* will perform random pick of the child layer and spawn selected task with vehicle to be destroyed.

Lastly, *RandomFlag layer* is spawned where again, one of three slots will be spawned.

All the random selections are saved and layers will remember it.

Upon leaving the range, everything inside will get despawned.

Upon reentry, everything will get spawned in back again, random selection will use the saved layers, positions of vehicles/AIs will be also set to where these entites were before and if some of it got killed/destroyed, it will not spawn (see [Dynamic Spawn/Despawn](#Dynamic_Spawn/Despawn)).

### DetectPrefabInTrigger

*Goal:*

Create a Task to Deliver specific vehicle prefabs in specific counts into the trigger for US Faction.

*Setup:*

It is achieved by having Area that spawns Layer1 which spawns Slots that each will spawn vehicles.
Then LayerTaskMove that spawns SlotMoveTo which has adjusted Plugin Trigger attributes (see image below) to detect specific prefabs in specific numbers.

*Mechanics:*

Area spawns Layer1 which spawns Slots that each spawn vehicles.
Then LayerTaskMove spawns SlotMoveTo that has adjusted Plugin Trigger to be activated by SPECIFIC\_PREFAB\_NAME and using Custom Trigger Conditions, in order to activate this trigger, there needs to be at least two Ural-4320 vehicles or any of their child prefabs aka variants of Ural.
UAZ-469 is requiered to be there in number of three but there needs to be this specific one, not any other variants. Otherwise, the trigger will not get activated and the Task will never finish.
This setup can be used for any prefabs or even classnames to create variety of different tasks and logics.

Attributes of SlotMoveTo1 for Plugin Trigger

### DeliverWeaponsToCrate

*Goal:*

Create a Task Deliver weapons into crate for US Faction.

*Setup:*

It is achieved by having Area that spawns Layer1 with Slot1 that is set to detect a prefab of a crate set in Object To Spawn attribute via Use Existing World Asset attribute.

Then It has Plugin On Inventory Change attached to (see in the image below) it and set Actions On Item Added to Count Inventory Items And Execute Action which then can execute Change Task State and Remove Item From Inventory actions.

LayerTaskMove then spawns SlotMoveTo that spawns trigger which is set to detect a medical crate 10x which will never be in the scenario spawned to create this "dummy" task.

Then outside of the Scenario Framework, there is a WeaponsCrate spawned and adjusted to have inventory. Lastly there are 5x M9 Handgun spawned around the crate.

*Mechanics:*

Area spawns Layer1 which spawns Slot1 that finds the WeaponsCrate in the vicinity and attaches the Plugin On Inventory Change to it that is set to detect 5x M9 Handgun inside it and upon reaching desired count, it will finish the objective and remove those weapons from the crate.

LayerTaskMove spawns SlotMoveTo but that is just used as "dummy" task to have some to work with and set in a way that conditions cannot be fulfilled in this scenario any other way than by that Scenario Framework action.

Attributes of Slot1 for Plugin On Inventory Change

### DeliverWeaponsInVehicle

*Goal:*

Create a Task to Deliver weapons in a vehicle for the US Faction.

*Setup:*

It is achieved by having Area that Spawns Layer1 which spawns Slot1 that will spawn M1025 Humvee that has OnActivation action set with Add Item To Inventory that will add 10x M16A2 into its inventory after spawning.
Then LayerTaskMove has OnTaskFinished action set to Remove Item From Inventory that will target the Slot1 spawned entity and attempt to remove it from there.
Lastly, this Layer will spawn SlotMoveTo that has adjusted Plugin Trigger to search for M16A2 inside of it (see second image below) - it is important to set the Specific Class Names for this use case to the Vehicle so it can search vehicle inventories. In default cases, items in inventory are not searched by the trigger.

*Mechanics:*

Area spawns Layer1 which spawns Slot1 that spawns M1025 Humvee and adds 10x M16A2 to its inventory.
Then gets spawned LayerTaskMove which spawns SlotMove to that spawns trigger which is responsible for searching all for the M16A2 inside it and set in a way that it also searches inventory of vehicles.
This task can be finished either by driving the vehicle straight to the trigger or putting 10x M16A2 by any means.
If they are delivered via provided vehicle, weapons will be removed from said vehicle to showcase this action here.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### GenericTask

*Goal:*

Create a Task linked to user action that will display the hint and finish the task - in this case, target user action will be our infamous Flush the toilet to complete the Scenario.

*Setup:*

It is achieved by having an Area with LayerTask which has SlotTask that has On User Action Event added to the Activation Actions under the OnInit category.
This user action is linked to a namedToilet entity that is placed in the world outside the ScenarioFramework hierarchy.
Action ID is set to 0 as the Flush the toilet action has this ID in this case.
Then there are two additional actions in the Actions list that will be activated upon someone using linked user action.
First will display a hint using Show Hint. Second will change the task state to FINISHED for the LayerTask.
Lastly, the OnTaskFinish has Action Wait and Execute that after 2 seconds will activate End Mission Action.

*Mechanics:*

Area spawns the LayerTask which spawns SlotTask that will attach the User Action Event listener on toilet.
Upon someone activating the user action, it will show a hint, then finish a task and then it will finish the mission.
In order to finish this task, you need to flush the toilet.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)")

### Waypoints

*Goal:*

Create a sequence of different waypoints for the AI group to complete.

*Setup:*

It is achieved by having an Area that has Layer Layer1 which has Slot1 that spawns UH-1H helicopter that names it namedHeli for future reference, then Slot3 spawns UAZ-469 and lastly SlotAI1 which spawns US Fire Team.
Then there is Layer Waypoints that contains various SlotWaypoints.
SlotAI1 has modified attribute category OnInit where in the Activation Actions, there are defined actions to assign waypoints to the AI group and upon completing each waypoint, it adds another in said sequence.
Outside the ScenarioFramework hierarchy, there is BarGate.

*Mechanics:*

Area Spawns Layer1 that spawns UAZ-469, UH-1H and US Fire Team. Then Layer Waypoints spawns waypoints.
These waypoints are then assigned to the AI group one by one and next one is assigned after completing the previous one.
First, Fire Team is ordered to get in UH-1H and then get out. After that, they are ordered to move to the crossroad.
Shortly after that, they are ordered to open a bar gate and then get in UAZ-469 and drive on the crossroad where they get out.
There is no end state, this group will finish waypoints in given order.

### LastStand

*Goal:*

Create a sequence of Task Defend to hold off enemy waves of attack for the US faction.

*Setup:*

It is split into 4 Areas called Stages. Stage\_0 contains LayerTaskMove with SlotMoveTo and Slot2 that spawns an M60 machinegun.
Each of the following Stage is then set to be activated by previous stage finishing task and it contains slightly modified version of DefendArea Sample where most notable differences are in Title, different timer and not requiering to having majority in said area.

*Mechanics:*

Area Stage\_0 spawns LayerTaskMove with SlotMoveTo that spawns Task prompting you to move to said area.
It also Spawns M60 machinegun from Slot2. Upon moving to the designated area and finishing the task, LayerTaskDefend from Stage\_1 will spawn and alongside it the AIs from attached Attacker Layer.
Once the timer runs out, it will spawn contents of Stage\_2 and similarly for the Stage\_3 which then ends the scenario after 4 seconds via the Wait And Execute action to then End Mission.

### TutorialFull

*Goal:*

Create a small show-case mission that can serve as a tutorial for a bit more advanced workflow than it is shown in Samples. It expects knowledge of the basic Scenario Framework samples.

*Setup:*

It is achieved by having several areas that splits the parts of the scenario.
Area\_1\_island spawns LayerTaskDeliver with spawns SlotDelivery that is set to detect Barrel.et and SlotPick which is set to find the nearest barrel and link it as an item to be delivered.
It also spawns two SlotTriggers that upon entry, they will display a hint, play sound and TriggerHintNoFuel spawns TaskMoveToFacility.
Area\_2\_Facility holds previously mentioned task layer that has ON\_TRIGGER\_ACTIVATION.
Once that is activated, it spawns SlotMoveTo which spawns Task Move To and it also spawns SlotTrigger that is set to check for Vehicles in order to display a hint.
Upon finishing the Task Move from the TaskMoveToFacility, it will display a hint to Search for the barrels with oil.
You then need to move to the Area\_3\_Garage near the rock where LayerTaskDestroy spawns SlotDestroy that has in target nearest rock. Upon task Destroy rock being spawned, it will provide a hint.
After the rock being destroyed, Ural-4320 that is spawned there outside of the Framework can be moved to the previously mentioned trigger that is set to check vehicles - the TriggerHintLoadBarrels.
Once there, upon loading barrel into the truck and then moving it to the SlotDelivery\_1 trigger, it will finish this task and display hint set in TaskDeliverFuel.
Then it will spawn contents of the Area\_4\_Landing that will spawn ClearAreaAI which will spawn SlotAI that spawns a group and WPInvasion Slot that spawns attack waypoint.
It also spawns TaskClearArea similar to what you know from the Samples. Upon clearing the Area, the TaskClearArea sets the Mission End Screen and Ends the mission via actions attached in OnTaskFinished.

*Mechanics:*

Area\_1\_island spawns LayerTaskDeliver with Pickup and Delivery points and two SlotTriggers.
Player is spawned directly to the TriggerHintWelcome which issues a hint saying: Your primary task is to activate the radar located on top of the hill. Check your tasks.

After that, you are supposed to go to the hill where the radar is located and here TriggerHintNoFuel says: The radar is not working since the generator lacks the fuel. Find it and transport it to here.

This leads you to the Area\_1\_Facility to finish the LayerTaskMove and then going to the Area\_3\_Garage where the Ural-4320 behind the rock is located.
Upon destroying the rock, you are supposed to move the Ural-4320 back to the facility where oil barrels are located and there you are prompted with another hint telling you to load the barrel.
Upon loading the barel to the truck, move it to the SlotDelivery\_1 and unload it there.
This will spawn enemy AI going to attack your position with TaskClearArea. Upon finishing it, it ends the scenario.

## Compositions

Scenario Framework Compositions are just Scenario Framework components put together for easier and quicker use when creating your scenarios.

In fact, many of these are directly taken from the Scenario Framework Samples so you do not have to reopen those and you can easily put them into your scenarios, perform few adjustments and you are good to go.

* [TaskMove](#TaskMove) sample
* [TaskKill](#TaskKill) sample
* [TaskExfil](#TaskExfil) sample
* [TaskDestroy](#TaskDestroy) sample
* [TaskDeliverVehicles](#DetectPrefabInTrigger) sample (if the prefab is in the trigger, it is delivered)
* [TaskDeliverIntel](#TaskDeliverIntel) sample
* [TaskDefendTarget](#TaskDefendTarget) sample
* [TaskDefendAreaAndTarget](#TaskDefendAreaAndTarget) sample
* [TaskDefendArea](#TaskDefendArea) sample
* [TaskClearArea](#TaskClearArea) sample

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

## 1.1.0 Structural Changes

Update 1.1.0 brought many changes to the Scenario Framework structure from 1.0.0.

⚠

A [Scenario Framework Update Plugin](/wiki/Arma_Reforger:Scenario_Framework_Update_Plugin "Arma Reforger:Scenario Framework Update Plugin") is provided along the 1.1.0 update, updating the Scenario Framework automatically.

The main difference is the switch from normal Slots spawning prefabs of waypoints to the specialised [SlotWaypoint](#SlotWaypoint) that has the necessary API for you to have full control over the waypoint attributes and their behaviour.
This includes a new waypoint management that allows to customise the attributes of waypoints that are about to be spawned, making it easier to influence AI units movement, allowing for more control overall.
Waypoints also have a new visualisation in the World Editor to better see the position of each one.

This aside many new options have had a few changes, the most important one being the switch from **Waypoint Group Names** (deprecated and unused) attribute to **Waypoint Set**.

If Waypoints have to be cycled, just select the Cycle Waypoint. Two methods are available:

* Either input the Layer name that contains the SlotWaypoints to be cycled into the "Layers With Waypoints To Cycle" attribute
* Or, if this waypoint is inserted into the target layer, leave the field empty and the parent layer where this Cycle Waypoint is located will automatically cycle all the (non-cycle) waypoints in it.

⚠

The **Waypoint Group Names** attribute will remain until a further (unspecified for now) version so there can be an update period.  
Be wary that once the attribute is removed, **the plugin will not be able to save old scenarios**.
