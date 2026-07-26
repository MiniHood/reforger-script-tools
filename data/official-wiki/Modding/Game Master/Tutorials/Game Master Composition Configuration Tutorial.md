# [Game Master: Composition Configuration Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master:_Composition_Configuration_Tutorial)

Game Master compositions are prefabs that have to be created, configured and maintained in a specific way in order for them to work correctly in [Arma Reforger:Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master").

## Creating Composition Prefab

### Flexible

Follow these steps if the composition is meant for flexible slots (i.e., the composition can be placed anywhere in the world or in one of compatible slots).

1. Select **Prefabs/Compositions/Slotted/CompositionBase.et** and open it by *Plugins > Edit Selected Prefab(s)* or by pressing **Ctrl+Shift+E**.
2. Create new composition prefab by dragging **CompositionBase** entity from Hierarchy list to desired folder in Resource Browser. Choose the folder based on intended slot type (i.e. Flat slot, Road slot, and their respective sizes).
3. Name the prefab according to this key: `<NAME>_<SIZE ABBREVIATION>_<FACTION>_<ID>`
   * **NAME** - composition name eg: RoadBlock
     + Note that names have no spaces and are written with camel case.
   * **SIZE ABBREVIATION** - L, M or S
     + Large, Medium or Small
   * **FACTION** - either **US**, **USSR**, or **FIA**
   * **ID** - two-digit iterator starting from 01

ⓘ

Naming Examples:

* Flat Composition:
  + *PlayerHub\_L\_US\_01*
* Road Composition:
  + *Checkpoint\_S\_USSR\_01*

### Static

Follow these steps if the composition is meant for static slots (i.e., the composition can be placed only on one specific position in the world).

1. Open the world on which the composition is going to be placed
2. Place **Prefabs/Compositions/Slotted/CompositionBase.et** entity on desired location
3. In entity's **SCR\_SlotCompositionComponent**, set the location prefab the composition belongs to
4. Create new composition prefab by dragging **CompositionBase** entity from Hierarchy list to desired folder in Resource Browser. Choose the folder based on world and location names (e.g., *Eden/Le\_Moule*)
5. Name the prefab according to this key: `<NAME>_<FACTION>_<ID>`
   * **NAME** - custom name describing the composition
   * **FACTION** - either **US**, **USSR**, or **FIA**
   * **ID** - two-digit iterator starting from 01

ⓘ

Naming Example:

* *Camping\_US\_01*

## Configuring Composition

The composition itself has several attributes that influence its behavior.

[![armareforger-gm-compositions-attributes2.jpg](/wikidata/images/0/01/armareforger-gm-compositions-attributes2.jpg)](/wiki/File:armareforger-gm-compositions-attributes2.jpg)

* **Slot Prefab**
  + Prefab this composition can be snapped to in in-game editor. Can be either a flexible slot, or static location in the world.
* **Orient Children To Terrain**
  + When enabled, children will be snapped and oriented to terrain when the composition is transformed.
  + This influences also position of non-editable compositions created in a flexible slot or positioned using dedicated function in the composition component.
  + Use this when child objects are lying on the ground, e.g., tables and chairs under a camo net, but not tables and chairs inside a house.
* **Non Editable Children**
  + When enabled, children will not be turned into editable entities.
  + The composition will appear as a single entity in in-game editor.
  + Use this for smaller compositions which have a lot of smaller objects, e.g., pallets with boxes.

## Editing Composition

Select the composition prefab and edit it by *Plugins > Edit Selected Prefab(s)* or by pressing **Ctrl+Shift+E**.

### Prefab Editing

Keep in mind that [prefab editing](/wiki/Arma_Reforger:Prefabs_Basics "Arma Reforger:Prefabs Basics") has a few caveats:

* After [**adding a new entity**](/wiki/Arma_Reforger:Prefabs_Basics#Adding_entities_to_prefab "Arma Reforger:Prefabs Basics"), move it inside the composition by **Alt+LMB drag**, not just by LMB drag. How to tell them apart...

|  |
| --- |
| Expand**How to tell them apart...** |
| * Entity added to composition instance (**LMB drag**) will have its own prefab mentioned in Hierarchy's *prefab* column. * Entity added to composition prefab (**Alt+LMB drag**) will have the composition prefab in the *prefab* column.   [armareforger-gm-compositions-prefabs-column.png](/wiki/File:armareforger-gm-compositions-prefabs-column.png) |

* When **editing entities**, only their instances are modified. Follow these steps to apply changes to prefab:
  1. Select modified entities
  2. Click on **Apply to prefab** at the bottom of Object properties panel.
  3. In the dialog window, leave the composition prefab selected, but tick **Apply to transformation** option
  4. Confirm
* **Save all changes** to the prefab by saving the world (**Ctrl + S**)
  + Until the world is saved, no changes (not even the ones manually applied using Apply to prefab button) are saved into prefab file.
* To [**delete an entity**](/wiki/Arma_Reforger:Prefabs_Basics#Deleting_entities_from_prefab "Arma Reforger:Prefabs Basics"), use *RMB > Delete from prefab*. Pressing 'Delete' key would simply try to remove an instance of the entity.

### Technical Rules

In later stages, following rules will be automatically checked by Workbench plugin.

* **Use only entities from *Prefabs* folder**, not from *PrefabsEditable* folder! Conversion to editable variants is performed automatically in the next step.
* Entity **bounding boxes should not extend past the borders** of the slot! You can see the bounding boxes by hovering over entities with the mouse, and entities that do cross this boundary while part of the composition prefab will show a warning. This is done to ensure compositions align well with slots in the world.

### Tips

* To toggle surface clutter, use *[Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") > Render > Terrain grass > Render grass*. Use this sparingly, as creating the composition without grass can provide inaccurate representation.
* If your composition contains some editable prefabs, you can convert them back to their sources using *Plugins > In-game Editor > Convert to/from Editable Entities...*, and the choosing **To Non-editable**.

⚠

Steps below are applicable if you're making a composition that should be editable in in-game editor.

If that is not the case, please click RMB on **SCR\_SlotCompositionComponent** and choose **Configure composition** before saving and committing.

This will make sure the composition is set-up correctly, e.g., all child entities are in hierarchy. Conversion to editable variant performs this step automatically.

## Creating Editable Variant

When you're done editing the composition, auto-generate its editable variant. To do so, follow the configuration guide:

### Selecting compositions

In Resource Browser, **select prefab files** (\*.et) of entities you want to process.

Check editable entity types to see which prefabs should be marked as editable.

*Tip: Both Resource Manager and World Editor browsers are supported, but not Resource Manager*

### Creating editable prefab

With files selected, activate ***Plugins > In-game Editor > Create/Update Selected Editable Prefabs***

*Tip: You can also press **Ctrl+Shift+U** to perform this action.*

After the operation is done, please check the log, it will list all processed prefabs. They can end up with one of the following states:

* **Created / Updated** - successful processed
* **Failed** - unable to generate the prefab, most commonly due to presence of child entity with RplComponent
* **Non-editable** - some child entities which don't have editable prefab variant were detected. Please revise them and consider if some of them could be made editable as well.

### Updating editable prefab

Editable prefabs needs to be regenerated every time the source prefab changes. This can be achieved by several ways:

* Activate ***Plugins > In-game Editor > Update All Editable Prefabs***
  + This will update all auto-generated editable prefabs.
  + It will also handle renamed / moved / deleted source prefabs (enable **Only File Changes** attribute to perform this operation only).
* Select an editable or a source prefab and activate ***Plugins > In-game Editor > Create/Update Selected Editable Prefabs***

Changes will be made in **PrefabsEditable/Auto** as well as **UI/Textures/EditorPreviews/Auto** folders

ⓘ

Repeat those steps every time you perform changes in existing composition.

## Registering Composition

1. In Resource Manager, activate ***Plugins > In-game Editor > Register Placeable Entities...***
2. Set config to [Configs/Editor/PlaceableEntities/Compositions/Compositions.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/PlaceableEntities/Compositions/Compositions.conf)
3. Run the process. It will register all prefabs in *Prefabs/Compositions* folder, including the newly added one.

⚠

For newly added compositions through mods, this config should be extended. The new compositions should then be added to that config file.
In that case the config should be set to the modded config file.

## Testing in In-game Editor

1. Load the world [*worlds\GameMaster\GM\_Eden.ent*](enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent%7C)
2. Press play and open the **Entity Browser** by pressing the button in the bottom right or by pressing TAB.
3. Find your composition in the list and click on it to start placing. Compatible slots should light-up.
4. Hover over compatible slot to snap the composition to it.

ⓘ

If no compatible slots are available and snapping doesn't work, it's possible something is wrong with composition configuration.

* [![armareforger-gm-compositions-asset-browser.jpg](/wikidata/images/thumb/8/87/armareforger-gm-compositions-asset-browser.jpg/534px-armareforger-gm-compositions-asset-browser.jpg)](/wiki/File:armareforger-gm-compositions-asset-browser.jpg)
* [![armareforger-gm-compositions-slots.jpg](/wikidata/images/thumb/3/3c/armareforger-gm-compositions-slots.jpg/533px-armareforger-gm-compositions-slots.jpg)](/wiki/File:armareforger-gm-compositions-slots.jpg)
