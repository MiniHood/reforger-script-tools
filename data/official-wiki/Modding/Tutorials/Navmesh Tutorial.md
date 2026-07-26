# [Navmesh Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Navmesh_Tutorial)

The goal of this document is to help understanding the navmesh system and modding flow.

## Definitions

* **Soldiers** navmesh is is adapted to human navigation.
* **BTRLike** navmesh is adapted to vehicle navigation. Its generation is disabled as of now (0.9.8) as AIs cannot drive yet.
* **LowRes** navmesh is a low resolution navmesh used for navmesh streaming.

### Navmesh Streaming

On large terrains, Using navmesh streaming and configuring a [LowRes Navmesh](#LowRes_Navmesh) is recommended for RAM and performance reasons.
When needed (on larg terrains) and configured properly (see [Usage](#Usage) below), this navmesh will be used to draw a rough path for the other, more precise navmesh to only use its tiles that are close to the LowRes' found path.
This option saves a lot of RAM and calculations and should be used whenever possible, with the exception of small terrains.

## Creation

This section is about creating a navmesh for an editable terrain (modded terrain or sub-scene of an existing terrain).

1. In the default layer of the terrain, add [SCR\_AIWorld.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/SCR_AIWorld.et) prefab **from Resource Browser** which contains the [NavmeshWorldComponent](enfusion://ScriptEditor/scripts/Game/generated/AI/NavmeshWorldComponent.c;16) component

   ⓘ

   the [SCR\_AIWorld.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/SCR_AIWorld.et) prefab holds **three** [NavmeshWorldComponent.c](enfusion://ScriptEditor/scripts/GameLib/generated/AI/NavmeshWorldComponent.c), the second one (BTRLike) being disabled for now (0.9.8).
2. Let's now create a new navmesh to be set in the NavmeshFilesConfig entry:
   * Open Navmesh Tool in the toolbar [![armareforger worldeditor-navmeshtool-icon.png](/wikidata/images/thumb/2/2f/armareforger_worldeditor-navmeshtool-icon.png/32px-armareforger_worldeditor-navmeshtool-icon.png)](/wiki/File:armareforger_worldeditor-navmeshtool-icon.png) and select its Navmesh Tool tab
   * Set Navmesh to Soldiers the same as in the NavmeshWorldComponent's Navmesh Project property
   * Click Connect to open the Connect window and select Soldiers again - click OK to connect to the local navmesh server
   * Click Generate to open the Recast params options window
     + Keep everything to default values and press OK to display then the Generator area options window
     + Keep everything to default again (to cover the whole area) and press OK
   * The navmesh generation process has been started. This operation may take some time, the progress being displayed at the bottom of the Navmesh Tool tab
3. Click the Save button to save the resulting navmesh to storage (in a .nmn file).

The navmesh file can now be added to the Navmesh Settings/Navmesh File property in SCR\_AIWorld's NavmeshWorldComponent

Select Soldiers in Navmesh Project combobox and in Navmesh Tool/Navmesh to display the created navmesh.

## Modification

This section is about editing an existing navmesh for an editable terrain (modded terrain or sub-scene of an existing terrain).

Place e.g [Office\_E\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Structures/Industrial/Houses/Office_E_01/Office_E_01.et) - note how the navmesh is not impacted and does not adapt around/with the building.

Multiple solutions are available, described below.

### Full Regeneration

A full navmesh regeneration has multiple problems:

* the generation time can be quite big
* the file size can grow quickly as well.

For instance, the [CTI\_Campaign\_Eden navmesh](enfusion://ResourceManager/~ArmaReforger:worlds/MP/Navmeshes/CTI_Campaign_Eden.nmn) that is used on Everon for AI soldiers is over 1GB of size at the time of writing.
As such, generating a whole navmesh for a minor modification the mod that changes one navmesh tile of bigger maps is quite long (depending on the system the operation can take over an hour) and expensive in storage.

There is a better option for that.

### Partial Regeneration

This option allows to only generate tiles that changed and "patch" the full navmesh.

#### Current Tile

The Rebuild Tile button regenerates the tile over which the camera is; to use it, move the camera anywhere over the wanted tile, then press the button to regenerate that specific tile.

This is useful when only a small area has been modified (e.g a couple of buildings or an outpost placed).

⚠

Be wary that a wide composition (e.g outpost) can cover multiple tiles, therefore be sure to check the resulting output in order to avoid partially generated navmeshes.

#### Tile Range

The Tile Range regeneration option allows to regenerate a wanted rectangle of tiles, defined by its South-West/"bottom-left" and North-East/"top-right" points.

In order to do so, X and Y tiles indices can be used but these are not easily available - hopefully, their position value can also be used.
For that, it is important to keep track of min and max (South-West/"bottom-left" / North-East/"top-right") positions of the area to be regenerated;
these values can be obtained using the World Editor camera's position found at the bottom-left of World Editor.

* Open the Navmesh Tool
* Select Soldiers navmesh
* Connect to the local server
* Click Generate
* Click OK in the Recast params window
* Reaching the Generator area options window, click on the Position tab to input the area of generation - enter the min and max values from earlier in the Positions tab (From = min, To = max), then click OK.

The Tool then only generates the defined area and updates the current ***memory*** navmesh, and the map widget updates accordingly.

In order to save the updated navmesh, click Save Generated and save where needed - overwriting the old navmesh being an option.

If the generated area generated more tiles than we need, we can mark only tiles that are actually needed to be saved by marking them on the map by leftclick drag,
right clicking and clicking mark tiles then we could use SaveMarked and save only those that we marked.

## Modding

Everything up to this point expected the map to be freely changeable. But what if we want to modify Everon's navmesh without copying whole world?

### Config Override

This is where overriding of the original config comes in handy. As you might have noticed in the Navmesh Files Config there is a array of Navmesh File Overrides. if we look at the GM\_Eden terrain, we can see that it uses [Navmesh\_GM\_Eden\_Soldier.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Navmesh/Navmesh_GM_Eden_Soldier.conf).

This config file points to a main navmesh (Navmesh File) as well as an array of smaller navmeshes (Navmesh File Overide[sic](/wiki/Template:sic "Template:sic")), array that lists smaller navmeshes that will "patch" the main navmesh file, overriding said areas in the order they are provided.

As the Navmesh\_GM\_Eden\_Soldier.conf file is read-only, we can override it in our mod by right-clicking and choosing "override".
This creates an override file in the mod project, file that can now target GM\_Eden.nmn and the future partial navmesh files we are about to generate.

### Partial Generation

Not unlike [Partial Regeneration](#Partial_Regeneration), connect to the Navmesh server and generate only the area around which the modded terrain has been edited (a new house lot, another castle on the next hill?); then, save it (Save Generated/Save Marked).

This file should now be listed under Navmesh File Overide[sic](/wiki/Template:sic "Template:sic") in that earlier modded .conf file - click on + and point to the partial navmesh file, then save the config file.

⚠

In order to see the changes, the terrain must be reloaded.

## Usage

Navmeshes can then be used in [SCR\_AIWorld](enfusion://ScriptEditor/scripts/Game/AI/SCR_AIWorld.c;23)'s [NavmeshWorldComponent](enfusion://ScriptEditor/scripts/Game/generated/AI/NavmeshWorldComponent.c;16) components:

* Drag and drop a [SCR\_AIWorld.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/SCR_AIWorld.et) Prefab **if** there are none in the world - only one per world can exist!
  + The first NavmeshWorldComponent is for **Soldiers** navmesh - required for on-foot AIs to navigate through the land
  + The second NavmeshWorldComponent is for **BTRLike** navmesh - disabled as of now (v0.9.8) as AIs do not drive yet
  + The third NavmeshWorldComponent is for **LowRes** navmesh - it is **required** and must be linked in the Soldiers navmesh as follows: in Soldier's [NavmeshWorldComponent](enfusion://ScriptEditor/scripts/Game/generated/AI/NavmeshWorldComponent.c;16),
    - check the **Use Navmesh Streaming** checkbox
    - set the **Low Resolution Navmesh** that appeared to **LowRes**

      ⚠

      Do **not** check the **Use Navmesh Streaming** checkbox in the **LowRes** navmesh properties.
* To setup a navmesh in a NavmeshWorldComponent, navigate to its **Navmesh Settings > Navmesh Files Config**. Here, two choices are offered:
  + click **set class** and fill the **Navmesh File** field with a navmesh file
  + drag and drop an existing BaseNavmeshFilesConfig, e.g [Navmesh\_GM\_Arland\_Soldier.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Navmesh/Navmesh_GM_Arland_Soldier.conf).
