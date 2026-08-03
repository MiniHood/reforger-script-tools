# [Scenario Framework Setup Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Scenario_Framework_Setup_Tutorial)

This Tutorial will quickly show you how to setup ScenarioFramework for a new world and create simple scenario.

It is also highly recommended to read through the [Scenario Framework](/wiki/Arma_Reforger:Scenario_Framework "Arma Reforger:Scenario Framework") documentation or have it open and ready.

## World Setup

Open [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") and load the world of your choice with **File > Load World** (or `Ctrl` + `O`).

For the purposes of this tutorial, we will use [Arland](enfusion://WorldEditor/worlds/Arland/Arland.ent).

Create a new world using **File > New World** (or `Ctrl` + `N`); in the popup window, select "Sub-scene (of current world)" and click OK.

Once the process is done, save the file using **File > Save World** (or `Ctrl` + `S`), save in the directory of your choice.

Try and use the same directory structure as Arma Reforger has, so e.g MyModDirectory\worlds\WorldName\WorldName.ent.

## Scenario Framework Setup

Once we have that, we need to setup necessary things for the Scenario Framework to work.

Open the Game Mode Setup plugin through **Plugins > Game Mode Setup**.

1. Use the two dots (..) in the Template field to target [ScenarioFramework.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/GameModeSetup/ScenarioFramework.conf). Once done, click Next.
2. The **World Scan** screen appears asking if you want to scan the world for proper game mode setup. As this is a clean copy of a world, this is not required - click Skip.
3. The **World Configuration** screen appears to ask if you want to create required entities - click Create entities this time to generate all the required entities.
4. The **World Configuration Completed** screen appears now, with a potential list of issues to fix; click Next.
5. The **Mission Header** screen asks about creating a mission header in order for the scenario to appear in the main menu's scenario list - click Create header to do so, then Next on its confirmation and Close on the ending screen.

Once you are finished, you can find the created entities in the current layer.

## Scenario Framework Usage

### Spawn Area Creation

Before we start creating all the fancy tasks and scenario workflows, we need to add a spawn for the player.

1. To keep things organised, we create a layer: right-click on the world's subscene (named as the created world, at the bottom of the Hierarchy panel) and we name it "Start".
2. Then we make it the active layer by right-clicking on it and choosing "Set as active" (a double-click on it does the same thing).
3. Then we add a Scenario Framework Area by going to the ArmaReforger/Prefabs/Systems/ScenarioFramework/Components directory and drag and dropping it into the scene ([Area.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Components/Area.et))
4. Then drag a Layer, but this time drop it on the Area in the Hierarchy panel in order to have it as a child entity of the Area
5. Then drag a Slot on the previously created Layer so it looks like this:  
   [![armaR-scenario framework tutorial hierarchy.png](/wikidata/images/f/f2/armaR-scenario_framework_tutorial_hierarchy.png)](/wiki/File:armaR-scenario_framework_tutorial_hierarchy.png)
6. Now, select the Slot ("Slot1") in the Hierarchy tab and find the [SCR\_ScenarioFrameworkSlotBase](enfusion://ScriptEditor/scripts/Game/ScenarioFramework/Components/SCR_ScenarioFrameworkSlotBase.c;6) component to click on it in the **Object Properties** panel.
7. Under the Asset category, we click on two dots (..) for the **Object To Spawn** attribute to assign it the [SpawnPoint\_US.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_US.et) Prefab. That way, we have added spawning Spawnpoint for the US Faction using the Scenario Framework.

Press `Ctrl` + `S` to save the world.

You can already try if it is working for you and if you will get spawned on the position where you put that Slot which holds the Spawnpoint by pressing the World Editor's Play button (or `F5`).

Let's add another Slot under the same Layer to give players the Arsenal in order for them to customise their loadout.

We will do it the same way as with the SpawnPoint but using [ArsenalBox\_US.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Military/Arsenal/ArsenalBoxes/US/ArsenalBox_US.et) (the one in Prefabs/Props/Military/Arsenal/ArsenalBoxes/US/) for the **Object To Spawn** attribute.

In order to give a player some vehicle to move around, we will add a third slot and use the [M1025.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/M998/M1025.et) Prefab (or the [M1025\_MERDC.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/M998/M1025_MERDC.et) for the camo version).

In a similar fashion, you can build up your starting area any way you like.

The end result should look like this:

:   :   [![armaR-scenario framework tutorial hierarchy complete.png](/wikidata/images/c/ca/armaR-scenario_framework_tutorial_hierarchy_complete.png)](/wiki/File:armaR-scenario_framework_tutorial_hierarchy_complete.png)

### Add a Move Task

Now is the time for some Tasks to be created.

For starters, we will create one of the most simple task that is Task Move.

1. Before we start, it is good to organise these things via layers in a similar fashion as the Start layer, so we will create the FirstTask layer and set it as active (see [Spawn Area Creation](#Spawn_Area_Creation) above for a reminder of how to create a layer and set it as current).
2. Then we find a suitable location and we create an Area there by drag and dropping it from the Components folder the same way we did with the spawn area.
3. But then, instead of the Layer, we will find the [LayerTaskMove.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Components/LayerTaskMove.et), drag and drop it in the freshly created Area (Area2 if you did not rename any).
4. Then, we drag and drop a [SlotMoveTo.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Components/SlotMoveTo.et) into this LayerTaskMove.
5. Notice that the slot's sphere is larger. This visualises the trigger radius for the Task Move you just created.
6. All you have to do now is launch the game (Play button or `F5`), open the map and then open the Task List (default `J`) to see the Task.

And when you move your character in the trigger, you will complete the objective.

### Add a Random Destroy Task

Since this was pretty easy, let's expand this scenario a little bit by adding a Task Destroy vehicle once you complete the Task Move.

1. To keep things organised, we create another layer and name it SecondTask, then set it as active.
2. Again, we create an Area, but to make it more interesting, we will randomise the kind of vehicle we will want to destroy.  
   It can be achieved in various ways but the safest approach is to put a [LayerTaskDestroy.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Components/LayerTaskDestroy.et) under Area3 hierarchy
3. After selecting the *SCR\_ScenarioFrameworkLayerTaskDestroy* component, set its **Spawn Children** attribute value to RANDOM\_ONE and change the **Activation Type** to ON\_TRIGGER\_ACTIVATION
4. Now, add two Layers under this hierarchy
5. Under each of these Layers, add a [SlotDestroy.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/ScenarioFramework/Components/SlotDestroy.et) Prefab
6. Now let's define what we want to destroy. In SlotDestroy1 we will put [UAZ469.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et) and in the SlotDestroy2 [Ural4320\_transport\_covered.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/Ural4320/Ural4320_transport_covered.et) as **Object To Spawn**.
7. Then we will head back to the FirstTask layer where we added the Task Move, select the LayerTaskMove1, click on its SCR\_ScenarioFrameworkLayerTask component and in the category **OnTaskFinished**, we will add an Action by clicking on the plus **+** icon
   * Search for "SpawnObjects" to find the SCR\_ScenarioFrameworkActionSpawnObjects action and select it
   * Now we expand the action by clicking on that arrow and click on the plus **+** icon in the **Name Of Objects To Spawn On Activation** attribute
   * And then input the LayerTaskDestroy1 name as the first item in the list
8. Now you can try it in-game. This setup will result in a behavior that will not spawn Task Destroy right away, but only after you complete the Task Move. Since we added the RANDOM\_ONE and two layers under the LayerTaskDestroy1, it will randomly pick one of the two SlotDestroy, helping to make less repetitive scenarios.

### Add a Game Over

1. To conclude this scenario, we can end it by adding an EndMission action in the **OnTaskFinished** on the LayerTaskDestroy1
   * Tick the **Override Game Over Type** attribute checkbox
   * Set Game Over Type to FACTION\_VICTORY\_SCORE which will end the mission upon completing Task Destroy.

That's it!

Now go ahead and try other Layer/Slot types.

If needed, see the [Scenario Framework](/wiki/Arma_Reforger:Scenario_Framework "Arma Reforger:Scenario Framework") documentation for more details!
