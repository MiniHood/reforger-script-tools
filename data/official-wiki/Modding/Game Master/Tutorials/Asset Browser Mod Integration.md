# [Asset Browser Mod Integration](https://community.bistudio.com/wiki/Arma_Reforger:Asset_Browser_Mod_Integration)

Basic rules for adding assets are explained on following page - [**Editable Entities Configuration**](/wiki/Arma_Reforger:Game_Master:_Editable_Entities_Configuration "Arma Reforger:Game Master: Editable Entities Configuration") which covers most of the basic information. Due to that, this tutorial focuses more on creating new register and adding to list used by in game Editor and it is assumed that you have properly configured prefabs in first place (with editable components present and [preview images](/wiki/Arma_Reforger:Game_Master:_Image_Generation_Tutorial "Arma Reforger:Game Master: Image Generation Tutorial") already generated).

## Preparing structure

At minimum, to add new asset to in game editor you need to create a **new register** & **override [EditorModeEdit.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)** prefab. Folder wise, only structure for new config needs to be prepared since **[Override in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics")** function is taking care of it

[![armareforger-editor-adding-assets-file-structure.png](/wikidata/images/6/67/armareforger-editor-adding-assets-file-structure.png)](/wiki/File:armareforger-editor-adding-assets-file-structure.png)

### Creating new register file

Assuming that structure was already prepared, there are two ways how to create a new register config and both of them are described below

#### Creating new config

[![armareforger-editor-adding-assets-creating-config.png](/wikidata/images/thumb/2/2d/armareforger-editor-adding-assets-creating-config.png/200px-armareforger-editor-adding-assets-creating-config.png)](/wiki/File:armareforger-editor-adding-assets-creating-config.png)

[![armareforger-editor-adding-assets-creating-config-class.png](/wikidata/images/thumb/d/d9/armareforger-editor-adding-assets-creating-config-class.png/200px-armareforger-editor-adding-assets-creating-config-class.png)](/wiki/File:armareforger-editor-adding-assets-creating-config-class.png)

New register can be easily created by new config file. Procedure for it same as for any other configs:

* In Resource Browser menu either click with *Right Mouse Button* on empty **Resource Browser** field **(1a)** or use **Create button (2)** in bottom right section of Resource Browser.
* From the context menu, select **Config file** option
* Type in new config name, i.e. *SampleMod\_NewCar\_Vehicles.conf*
* From **Choose a class** window, select **SCR\_PlaceableEntitiesRegistry** option.

#### Duplicating existing config

Alternative method involves duplicating one of the existing register configs located in *Configs\Editor\PlaceableEntities.* To do so, click on i.e. Vehicle.conf with *Right Mouse Button* and select option **[Duplicate to "*name of addon*"](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics")** from context menu that appeared. After typing in new name and confirming it, new config file will be created in target addon, keeping structure of original file.

### Adding new register file to Editor

Once register file is created, it's time to actually link it with in game editor. To do so, **[EditorModeEdit.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)** prefab has to be modified and **Registries** array in **SCR\_PlacingEditorComponent** needs to be expanded.

#### Using Override in functionality

[![armareforger-editor-adding-assets-override-in.gif](/wikidata/images/e/e9/armareforger-editor-adding-assets-override-in.gif)](/wiki/File:armareforger-editor-adding-assets-override-in.gif)

First step, will be creating modified copy of **[EditorModeEdit.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)** prefab. This can be easily accomplished using **[Override in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics")** functionality available in **Resource Browser** context menu. In principle, this operation is performed in two steps:

* Navigate to *Prefabs/Editor/Modes* folder and locate **[EditorModeEdit.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)** prefab
* Click with *Right Mouse Button* on that prefab and select **Override in "*name of addon*"** option

Workbench will recreate structure of original file with only difference being file system - that means that

* *$ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et*

will be copied to

* *$NameOfAddon:Prefabs/Editor/Modes/EditorModeEdit.et*

This new prefab will have exactly same GUID and will override existing prefab.

ⓘ

[![armareforger-editor adding assets overridden file.png](/wikidata/images/f/f0/armareforger-editor_adding_assets_overridden_file.png)](/wiki/File:armareforger-editor_adding_assets_overridden_file.png)

Override in function creates overridden version of [EditorModeEdit.et EditorModeEdit.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et_EditorModeEdit.et) prefab. This mean that prefab in target addon contains only changes - like adding new elements or overriding existing ones - made in addon.

Overridden assets can be easily distinguished by **puzzle icon** visible in **Resource Browser.**

#### Expanding Registries array

With **EditorModeEdit**  prefab being present in addon, it is time to make modification to it. Goal is to add new element to **Registeries** array located in **SCR\_PlacingEditorComponent.**

**EditorModeEdit**  prefab located in the addon can be edited in a same way as any other addon - it is possible to either place it directly in World Editor and then start editing parameters or **Edit prefab** can be used in Resource Manager.

To use **Edit prefab** option, double click with *Left Mouse Button* on **EditorModeEdit.et** file located in addon. This should open t.hat prefab as a new tab in **Resource Manger**. Now only thing left to do is to click on **Edit prefab** button on the right side of the **Resource Manager -** this will open World Editor in special, prefab editing mode which slightly differs from regular worlds created by that editor.

Main difference is fact that when you try to save "world", it doesn't create any new world files - it just saves changes in prefabs.

[![armareforger-editor-adding-assets-edit-prefab.gif](/wikidata/images/c/c1/armareforger-editor-adding-assets-edit-prefab.gif)](/wiki/File:armareforger-editor-adding-assets-edit-prefab.gif)

To add new config to **Registries** array located in **SCR\_PlacingEditorComponent,** simple drag and drop config file from **Resource Browser** on top of **Registries** property. A new element will be added on the bottom of the array with link to config file located in *Configs→ Editor → PlaceableEntities.*

[![armareforger-editor-adding-assets-adding-register.gif](/wikidata/images/c/cd/armareforger-editor-adding-assets-adding-register.gif)](/wiki/File:armareforger-editor-adding-assets-adding-register.gif)

Now only remaining thing to do is saving those changes to the prefab - to do so, either click on **Save Button** or press *Left Ctrl + S* shortcut.

## Registering assets

New register can be populated with new assets in two ways - manually by adding placeable prefabs to Prefabs array or via **Register Placeable Entities** plugin. First method might is easier to use, especially if addon doesn't have that many placeable entities. However, when dealing with larger amounts of prefabs (or larger projects in general) it is recommended to use plugin instead.

### Manual method

Adding placeable entities to register by hand is quite simple.

Open config file containing register (in this case its *SampleMod\_NewCar\_Vehicles.conf*) as a new tab and then navigate in Resource Browser to a location where are placeable prefabs which you want to expose. Then, only thing left to do is simply drag & dropping those prefabs on **Prefabs** array.

[![armareforger-editor-adding-assets-adding-asset-manual.gif](/wikidata/images/d/d4/armareforger-editor-adding-assets-adding-asset-manual.gif)](/wiki/File:armareforger-editor-adding-assets-adding-asset-manual.gif)

Of course, elements in that array can also be manually added or removed via plus & minus signs on the right side of the **Prefabs** array

### Using plugin

[![armareforger-editor-adding-assets-adding-plugin.png](/wikidata/images/8/8a/armareforger-editor-adding-assets-adding-plugin.png)](/wiki/File:armareforger-editor-adding-assets-adding-plugin.png)

Basics of using **Register Placeable Entities** can be found on following page - Editor: Editable Entities: Configuration - RegisterPlaceablePrefabs.

When dealing with addons, it is mandatory to change **Addon** parameter so it matches **addon ID** you are working on **-** in this example it is **SampleMod\_NewCar.** Below you can take a look at fully filled register config.

### Adding modded label

[![armareforger-editor-adding-assets-adding-label.gif](/wikidata/images/6/6f/armareforger-editor-adding-assets-adding-label.gif)](/wiki/File:armareforger-editor-adding-assets-adding-label.gif)

To be able filter modded content from vanilla, it might be necessary to add **CONTENT\_MODDED** label to **Authored Labels** array in **Editable Component** of asset (*depending on asset type, it can be SCR\_EditableVehicleComponent, SCR\_EditableEntityComponent, SCR\_EditableCharacterComponent or SCR\_EditableGroupComponent*).

To do so, navigate to that component and click on plus sign to the right of **Authored** **Labels** array. Next step will be changing of type of newly added label - to do so, click on that new element and select from the drop down menu **CONTENT\_MODDED** parameter.

## Testing result in-game

If everything went good, then all assets added to new register file should be visible in game

⚠

Keep in mind that only **localised** strings are **searchable by name in Asset Browser**. Information about string localization can be found on [**Mod Localisation**](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation") page

[![armareforger-editor-adding-assets-result.png](/wikidata/images/7/7e/armareforger-editor-adding-assets-result.png)](/wiki/File:armareforger-editor-adding-assets-result.png)
