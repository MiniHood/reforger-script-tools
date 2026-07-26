# [Capture & Hold Setup](https://community.bistudio.com/wiki/Arma_Reforger:Capture_%26_Hold_Setup)

Setting up a new **Capture & Hold** scenario based is easy, thanks to the Prefab system and already existing **base world** available as part of the Capture & Hold mod.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") **Capture & Hold** is now packed in base game ([worlds/CaptureAndHold/CAH\_BaseWorld.ent](enfusion://ResourceManager/~ArmaReforger:worlds/CaptureAndHold/CAH_BaseWorld.ent)) instead of the official mod.

## Creation Steps

* Open Arma Reforger Tools. If this is not visible in your Steam Library, activate "tools" in the filter.
* Click on "Create New"
* Give a good Project Name like "CAH The Cowshed by xyz" so that people can search for it and also directly understand what this is about.

### Initial Setup

Start by opening the Workbench and launch the World Editor from either the Quick Launch screen or Editors context menu.

* [![1. Navigate to Worlds/CaptureAndHold and open one of the following world files (world icon): the CAH_BaseWorld.ent for Everon or CAH_BaseWorld_Arland.ent for Arland.](/wikidata/images/thumb/2/27/armareforger-cah_01_baseworld.jpg/600px-armareforger-cah_01_baseworld.jpg)](/wiki/File:armareforger-cah_01_baseworld.jpg "1. Navigate to Worlds/CaptureAndHold and open one of the following world files (world icon): the CAH_BaseWorld.ent for Everon or CAH_BaseWorld_Arland.ent for Arland.")

  **1.** Navigate to Worlds/CaptureAndHold and open one of the following world files (world icon): the **CAH\_BaseWorld.ent** for Everon or **CAH\_BaseWorld\_Arland.ent** for Arland.
* [![2. In the File context menu, select the New World option.](/wikidata/images/thumb/5/5b/armareforger-cah_02_newworld.jpg/600px-armareforger-cah_02_newworld.jpg)](/wiki/File:armareforger-cah_02_newworld.jpg "2. In the File context menu, select the New World option.")

  **2.** In the **File** context menu, select the **New World** option.
* [![3. Make sure that the Sub-scene (of current world) option is selected in the dialog.](/wikidata/images/thumb/3/3a/armareforger-cah_03_subscene.jpg/600px-armareforger-cah_03_subscene.jpg)](/wiki/File:armareforger-cah_03_subscene.jpg "3. Make sure that the Sub-scene (of current world) option is selected in the dialog.")

  **3.** Make sure that the **Sub-scene (of current world)** option is selected in the dialog.
* [![4. In the File context menu select the Save World As option and pick the scenario's save destination.](/wikidata/images/thumb/0/01/armareforger-cah_04_saveas.jpg/600px-armareforger-cah_04_saveas.jpg)](/wiki/File:armareforger-cah_04_saveas.jpg "4. In the File context menu select the Save World As option and pick the scenario's save destination.")

  **4.** In the **File** context menu select the **Save World As** option and pick the scenario's save destination.
* [![4bis. In our case, we select the root directory. TODO Add info that need to create folder Worlds and save file .ent in that folder](/wikidata/images/thumb/4/4f/armareforger-cah_05_save_dir.jpg/600px-armareforger-cah_05_save_dir.jpg)](/wiki/File:armareforger-cah_05_save_dir.jpg "4bis. In our case, we select the root directory. TODO Add info that need to create folder Worlds and save file .ent in that folder")

  **4bis.** In our case, we select the root directory. **TODO Add info that need to create folder Worlds and save file .ent in that folder**
* [![5. Find a suitable scenario location. In our example, we chose St Philippe.](/wikidata/images/thumb/8/8a/armareforger-cah_06_location.jpg/600px-armareforger-cah_06_location.jpg)](/wiki/File:armareforger-cah_06_location.jpg "5. Find a suitable scenario location. In our example, we chose St Philippe.")

  **5.** Find a suitable scenario location. In our example, we chose St Philippe.

### Scenario Setup

ⓘ

By creating a sub-world of an existing CAH world you are already adding some elements to the world:

[![Reforger CAH Base World.png](/wikidata/images/thumb/8/86/Reforger_CAH_Base_World.png/468px-Reforger_CAH_Base_World.png)](/wiki/File:Reforger_CAH_Base_World.png)

Continue to place the required elements to your world.

Drag and drop the following elements from the Resource Browser:

* [![1. From Prefabs/MP/Modes/CaptureAndHold drag GameMode_CaptureAndHold.et](/wikidata/images/thumb/e/e7/armareforger-cah_07_gamemode.jpg/600px-armareforger-cah_07_gamemode.jpg)](/wiki/File:armareforger-cah_07_gamemode.jpg "1. From Prefabs/MP/Modes/CaptureAndHold drag GameMode_CaptureAndHold.et")

  **1.** From Prefabs/MP/Modes/CaptureAndHold drag GameMode\_CaptureAndHold.et
* [![2. From Prefabs/MP/Managers/Factions drag the FactionManager_USxUSSR.et and from Prefabs/MP/Managers/Loadouts drag the LoadoutManager_USxUSSR.et](/wikidata/images/thumb/5/5e/armareforger-cah_08_managers.jpg/600px-armareforger-cah_08_managers.jpg)](/wiki/File:armareforger-cah_08_managers.jpg "2. From Prefabs/MP/Managers/Factions drag the FactionManager_USxUSSR.et and from Prefabs/MP/Managers/Loadouts drag the LoadoutManager_USxUSSR.et")

  **2.** From Prefabs/MP/Managers/Factions drag the FactionManager\_USxUSSR.et and from Prefabs/MP/Managers/Loadouts drag the LoadoutManager\_USxUSSR.et
* [![3. From Prefabs/MP/Modes/CaptureAndHold/Areas drag CaptureAndHoldArea_Major.et](/wikidata/images/thumb/7/73/armareforger-cah_09_area.jpg/600px-armareforger-cah_09_area.jpg)](/wiki/File:armareforger-cah_09_area.jpg "3. From Prefabs/MP/Modes/CaptureAndHold/Areas drag CaptureAndHoldArea_Major.et")

  **3.** From Prefabs/MP/Modes/CaptureAndHold/Areas drag CaptureAndHoldArea\_Major.et
* [![4. While the SCR_CaptureAndHoldArea entity is still selected, adjust the shape as desired.](/wikidata/images/thumb/5/59/armareforger-cah_10_area_setup.jpg/600px-armareforger-cah_10_area_setup.jpg)](/wiki/File:armareforger-cah_10_area_setup.jpg "4. While the SCR_CaptureAndHoldArea entity is still selected, adjust the shape as desired.")

  **4.** While the **SCR\_CaptureAndHoldArea** entity is still selected, adjust the shape as desired.
* [![5. From Prefabs/MP/Spawning drag SpawnPoint_US.et and SpawnPoint_USSR.et](/wikidata/images/thumb/c/cf/armareforger-cah_11_spawns.jpg/600px-armareforger-cah_11_spawns.jpg)](/wiki/File:armareforger-cah_11_spawns.jpg "5. From Prefabs/MP/Spawning drag SpawnPoint_US.et and SpawnPoint_USSR.et")

  **5.** From Prefabs/MP/Spawning drag SpawnPoint\_US.et and SpawnPoint\_USSR.et

**Note:** The GarbageManager is not necessary anymore.

Now **save** the scenario and proceed.

### System Test

* [![1. Make sure that the Play from camera position option is disabled and press the Play button.](/wikidata/images/thumb/b/b3/armareforger-cah_12_play.jpg/600px-armareforger-cah_12_play.jpg)](/wiki/File:armareforger-cah_12_play.jpg "1. Make sure that the Play from camera position option is disabled and press the Play button.")

  **1.** Make sure that the **Play from camera position** option is disabled and press the **Play** button.
* [![2. Verify that the scenario is working as intended - capture points, respawn, etc. Drawing of the area trigger can help visualising its coverage.](/wikidata/images/thumb/f/fa/armareforger-cah_13_test.jpg/600px-armareforger-cah_13_test.jpg)](/wiki/File:armareforger-cah_13_test.jpg "2. Verify that the scenario is working as intended - capture points, respawn, etc. Drawing of the area trigger can help visualising its coverage.")

  **2.** Verify that the scenario is working as intended - capture points, respawn, etc. Drawing of the area trigger can help visualising its coverage.
* [![3. Press Escape to return to the Edit mode. Select the SCR_CaptureAndHoldArea entity and uncheck Draw Shape.](/wikidata/images/thumb/d/d3/armareforger-cah_14_disable_drawing.jpg/600px-armareforger-cah_14_disable_drawing.jpg)](/wiki/File:armareforger-cah_14_disable_drawing.jpg "3. Press Escape to return to the Edit mode. Select the SCR_CaptureAndHoldArea entity and uncheck Draw Shape.")

  **3.** Press Escape to return to the Edit mode. Select the **SCR\_CaptureAndHoldArea** entity and uncheck **Draw Shape**.

Once done, save your changes once more and close the World Editor.

ⓘ

It is highly recommended to use Bohemia Interactive-prepared scenarios for inspiration and usage examples of individual features and components available in Arma Reforger.

### Scenario Header

To launch the scenario in-game a Scenario header (named MissionHeader) config must be created.

Exit the World Editor and return to the Resource Browser.

In the Resource Browser find your mod root directory and create a "Missions" directory inside.

* [![1. in your directory and select Create Resource → Config File in the context menu.](/wikidata/images/thumb/e/e3/armareforger-cah_15_new_missionheader.jpg/600px-armareforger-cah_15_new_missionheader.jpg)](/wiki/File:armareforger-cah_15_new_missionheader.jpg "1. in your directory and select Create Resource → Config File in the context menu.")

  **1.** ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") in your directory and select Create Resource → Config File in the context menu.
* [![2. Pick a name for your Mission Header and then select the SCR_MissionHeader class from the list.](/wikidata/images/thumb/c/cd/armareforger-cah_16_missionheader_class.jpg/600px-armareforger-cah_16_missionheader_class.jpg)](/wiki/File:armareforger-cah_16_missionheader_class.jpg "2. Pick a name for your Mission Header and then select the SCR_MissionHeader class from the list.")

  **2.** Pick a name for your Mission Header and then select the SCR\_MissionHeader class from the list.

Open the newly created Mission Header config.

In the **World** field, navigate to and select the world file. In our example it is the MyScenario.ent in the mod's root directory.

* [![1. Fill in remaining fields as desired then save the file.](/wikidata/images/thumb/7/79/armareforger-cah_17_missionheader_edit.jpg/600px-armareforger-cah_17_missionheader_edit.jpg)](/wiki/File:armareforger-cah_17_missionheader_edit.jpg "1. Fill in remaining fields as desired then save the file.")

  **1.** Fill in remaining fields as desired then save the file.

### In-Game Test

Launching Arma Reforger with this newly created mod enabled should allow you to see the and launch the scenario directly:

* [![1. Navigate to Scenarios.](/wikidata/images/thumb/6/6e/armareforger-cah_18_scenarios.jpg/600px-armareforger-cah_18_scenarios.jpg)](/wiki/File:armareforger-cah_18_scenarios.jpg "1. Navigate to Scenarios.")

  **1.** Navigate to **Scenarios**.
* [![2. Verify that your scenario is working as intended.](/wikidata/images/thumb/b/bf/armareforger-cah_19_result.jpg/600px-armareforger-cah_19_result.jpg)](/wiki/File:armareforger-cah_19_result.jpg "2. Verify that your scenario is working as intended.")

  **2.** Verify that your scenario is working as intended.

### Publish it to the Workshop

Now that the scenario can be launched from the game, only a few things remain.
Testing the scenario **in multiplayer** is important, making sure that there are enough spawn points and other gameplay details.

ⓘ

For detailed information look at [Multiplayer Scripting](/wiki/Arma_Reforger:Multiplayer_Scripting "Arma Reforger:Multiplayer Scripting").

Last, but not least is to **publish** the mod in the Bohemia Interactive Workshop for other people to see and play!

Important: Until your masterpiece has been tested, you should use Visibility: "Private" (only for you) or "Test" (Test mod category that can be enabled in the Workbench filter).

ⓘ

See [Mod Publishing Process](/wiki/Arma_Reforger:Mod_Publishing_Process "Arma Reforger:Mod Publishing Process").

### Additional Settings Information

This is added as additional information so that you get up and running with a simple version first and then start worrying about the details.

#### Score Ending

* Hierarchy Window: Select the object "GameMode\_CaptureAndHold"
* Object Properties: Select "SCR\_ScoringSystemComponent"
* In the settings for "Scoring:Actions" change the Score Limit

  [![](/wikidata/images/thumb/1/1b/CAH_Score.png/300px-CAH_Score.png)](/wiki/File:CAH_Score.png)

  Set the score to end the CAH game.

#### Time Ending

* Hierarchy Window: Select the object "GameMode\_CaptureAndHold"
* Object Properties: Select "SCR\_CaptureAndHoldManager"
* In the settings for "End Game Duration" change the value which is given in seconds

[![CAH Duration.png](/wikidata/images/thumb/e/e7/CAH_Duration.png/300px-CAH_Duration.png)](/wiki/File:CAH_Duration.png)

#### Scoring: Multipliers

You can adjust several actions so that they will have a bigger impact on the game / scoring.

[![CAH Score Multipliers.png](/wikidata/images/thumb/d/d9/CAH_Score_Multipliers.png/300px-CAH_Score_Multipliers.png)](/wiki/File:CAH_Score_Multipliers.png)

#### Kill Feed

It can be thrilling to leave the default values for this but for some communities the fun starts when they get more details in the "Kill Feed".

With this setting there will be no secrets about who was eliminated by whom.

[![CAH Kill Feed.png](/wikidata/images/thumb/c/c4/CAH_Kill_Feed.png/300px-CAH_Kill_Feed.png)](/wiki/File:CAH_Kill_Feed.png)

#### Activate Unconsciousness

If you want to use it, you must activate it in the GameMode\_CaptureAndHold. By default it is deactivated:

[![capture-and-hold unconsciousness.png](/wikidata/images/thumb/f/f7/capture-and-hold_unconsciousness.png/300px-capture-and-hold_unconsciousness.png)](/wiki/File:capture-and-hold_unconsciousness.png)

#### Spawn-Areas - No-Go Zone - Death Zone

You don't want the enemy to get too close to your spawn point? Simple: Use the spawn areas.

[![CAH CAH-Zone.png](/wikidata/images/thumb/9/91/CAH_CAH-Zone.png/567px-CAH_CAH-Zone.png)](/wiki/File:CAH_CAH-Zone.png)

#### GarbageManager - Delete bodies from map

2024-03 This got disabled for now and is not available - might come back so this documentation stays.
If you are annoyed of too many bodies in your CAH areas, you need to make these adjustments:

[![ReforgerGarbageConfigCAH.png](/wikidata/images/thumb/c/c1/ReforgerGarbageConfigCAH.png/1016px-ReforgerGarbageConfigCAH.png)](/wiki/File:ReforgerGarbageConfigCAH.png)

1. Select your instance of CAH GameMode
2. Select SCR\_BaseGameMode
3. Scroll down and select "Garbage System Config" and click on the search icon
4. Select "Override" in the right mouse button menu for GarbageSystem.conf

This will create a copy of the locked file into your scenario folder so that you are able to edit it.

The system will automatically open the new folder:

[![reforger-garbage-config-2.png](/wikidata/images/thumb/4/4f/reforger-garbage-config-2.png/503px-reforger-garbage-config-2.png)](/wiki/File:reforger-garbage-config-2.png)

Open the file and edit it.

⚠

Only the classes which are defined will be deleted.

This example shows how dead bodies are deleted after 180 seconds when player are further away than 3 meters.

[![reforger-garbage-config-3.png](/wikidata/images/thumb/a/a2/reforger-garbage-config-3.png/798px-reforger-garbage-config-3.png)](/wiki/File:reforger-garbage-config-3.png)

1. Add a new class filter
2. Select "only destroyed"
3. Define Lifetime and the distance a living player has to have when the class is deleted.

### Common Issues

#### Can't initialize the game (World Editor won't start)

"World Editor and some other game depended features will not be available. But rest of workbench will be functional."

[![world-editor-does-not-start.jpg](/wikidata/images/thumb/e/ed/world-editor-does-not-start.jpg/485px-world-editor-does-not-start.jpg)](/wiki/File:world-editor-does-not-start.jpg)

**Solution:** Delete the official CAH from "... \My Games\ArmaReforger\addons". If the error remains, delete all addons.

#### Editor crashes when using the garbage collector

**Solution:** Do not use it. A developer stated said that it is not needed anymore.

#### Repair your C&H scenario after version 1 of Arma Reforger

Sadly, with version 1 of Arma Reforger all Capture And Hold Scenarios have to be repaired and re-published.

* Open your scenario.
* Accept this message with "Yes".

  [![cah-repair-message2023-12-26 15 07 30-Window.png](/wikidata/images/thumb/6/6c/cah-repair-message2023-12-26_15_07_30-Window.png/300px-cah-repair-message2023-12-26_15_07_30-Window.png)](/wiki/File:cah-repair-message2023-12-26_15_07_30-Window.png)

  Place a new CaptureAndHold area as described above since it was removed while validating. Do not worry about the Garbage Collector: It is gone for good.

#### Repair your C&H scenario after version 1.2 of Arma Reforger

With version 1.2 the official Capture And Hold Scenario was implemented in the game and you have to remove the dependency from your scenario and re-publish it.

* Without opening Reforger Tools:
  + Edit the addon.gproj from the main directory of your scenario
  + Remove the Dependency "591AF5BDA9F7CE8B". A corrected version without any other mods will look like this:

    ```
    Dependencies {   
    "58D0FB3206B6F859" 
    }
    ```

Save and open your scenario:

* Publish
* If there are errors or problems selecting a Category: Don't give up - try again. Restart Workbench.

After you have fixed your scenario, it is important to delete the official Capture&Hold from .%userprofile%\Documents\My Games\ArmaReforger\addons\CaptureAMPHold\_591AF5BDA9F7CE8B Otherwise Reforger will run into errors. CAUTION: Move the official Capture&Hold to a safe location, as you will need to copy it back for adjustments to other maps.
