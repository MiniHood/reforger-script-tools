# [Enfusion Blender Tools: Import/Export Animation](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_Animation)

Animations for Enfusion are exported into an intermediate human-readable format TXA. This format needs to be further compiled by the Enfusion engine into ANM - a binary format used during the runtime.

## Animation

### Export animations from Blender

TXA Export is available in top section of 3D viewport. To perform export, following things are necessary:

* at least one **Armature** & **Action** present in the current scene
* at least one folder is defined in **Export Profile Folder** with some valid animation export profiles *(see [**Enfusion Blender Tools**](/wiki/Arma_Reforger:Enfusion_Blender_Tools#Installation "Arma Reforger:Enfusion Blender Tools") page for instructions)*
* running **Workbench** with **net API enabled** *(see **[Enfusion Blender Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools#Installation "Arma Reforger:Enfusion Blender Tools")** page for instructions)*

If all above conditions are meet, from top bar select **Enfusion Tools -> Export -> Enfusion Animation (.txa)** to initiate animation export.

[![armareforger-blender-txa-export-toolbar.jpg](/wikidata/images/0/04/armareforger-blender-txa-export-toolbar.jpg)](/wiki/File:armareforger-blender-txa-export-toolbar.jpg)

Once that button is pressed, following window should be visible:

[![armareforger-blender-txa-export-animation3.png](/wikidata/images/thumb/f/f8/armareforger-blender-txa-export-animation3.png/800px-armareforger-blender-txa-export-animation3.png)](/wiki/File:armareforger-blender-txa-export-animation3.png)

#### Top section

[![armareforger-blender-txa-export-animation-top-section.png](/wikidata/images/1/14/armareforger-blender-txa-export-animation-top-section.png)](/wiki/File:armareforger-blender-txa-export-animation-top-section.png)

Top section contains following things:

* **Save location** - folder where the exported actions will be stored. You can either type in the location or use **Change save location** button located on the right side
  + When changing location, it is possible to use **relative path** for saving the TXA - this is useful feature when collaborating with multiple people
* **Folder with export profiles -** folder with animation export profiles which should be currently used. This list populated through **Export Profile Folder** parameter located in **Enfusion Blender Tools settings.**
* **Armature name -** name of the armature which will be used for export. Bone transformations from this armature are processed by the exporter and stored in exported TXA file. This parameter is only visible if you have more than one armature in the scene
* **Secondary armature name** *(optional)* **-** name of the secondary armature which will be used during export . This is parameter is useful when armature which you want to export, is constrained to another armature and doesn't have any motion itself and you want to export multiple actions at once. In case of Rigify rig, in this field should be selected name of the Rigify control rig. This parameter is only visible if you have more than one armature in the scene
* **Hide -** Collapses export profiles in all actions
* **Show** - Expands export profiles in all actions
* **Disable** - Disables export profiles in all **visible** (*expanded)* actions
* **Enable** - Disables export profiles in all **visible** (*expanded)* actions
* **Search** [![armareforger-blender-txa-search-icon.png](/wikidata/images/thumb/d/d6/armareforger-blender-txa-search-icon.png/22px-armareforger-blender-txa-search-icon.png)](/wiki/File:armareforger-blender-txa-search-icon.png) - Filter visible actions. **Confirm filtering by pressing enter.** Only **visible** and **expanded** export profiles will be exported
* **Toggle Compact View** [![armareforger-blender-txa-toggle-compact-view.png](/wikidata/images/3/33/armareforger-blender-txa-toggle-compact-view.png)](/wiki/File:armareforger-blender-txa-toggle-compact-view.png) - Toggles between regular and more compact list of all actions

#### Action list

[![armareforger-blender-txa-export-animation-list.png](/wikidata/images/9/99/armareforger-blender-txa-export-animation-list.png)](/wiki/File:armareforger-blender-txa-export-animation-list.png)

**Action list** contains **actions** (marked in orange) found in the scene. [Actions marked as assets](https://docs.blender.org/manual/en/latest/files/asset_libraries/introduction.html#bpy-ops-asset-mark) are hidden from this list. Each individual action has an **arrow**, which controls the visibility of **Action Export List** - this option is quite useful when dealing with large amount of actions like on example picture above.

Please note that only **expanded & visible** actions are exported

⚠

If you **don't see any action in the action list**, make sure your action is not marked as asset. See above description for more details

#### Action export list

[![armareforger-blender-txa-export-export-list.png](/wikidata/images/1/16/armareforger-blender-txa-export-export-list.png)](/wiki/File:armareforger-blender-txa-export-export-list.png)

Action export list is the hear of TXA exporter. Over there it is possible to **add exports** & define **how those actions will be processed**. New entries in that list can be added by clicking on **+ Add** button located on the right side. Removal of exports can be done in similar way by pressing **X** button on the right side.

* **Enable checkbox -** control if this export is active or not. In inactive state **no txa** will be generated
* **Profile -** [animation export profile](/wiki/Arma_Reforger:Animation_Export_Profiles "Arma Reforger:Animation Export Profiles") which will be used on this export. By default, assuming [Enfusion Blender Tools Data](/wiki/Arma_Reforger:Enfusion_Blender_Tools#Installation "Arma Reforger:Enfusion Blender Tools") was correctly installed, there should be dozens of available animation exports which were used on Reforger assets. Selected profile has to correspond with the skeleton - otherwise no animation will be exported.
* **File** - name of resulting txa file
* **Diff pose** *(optional)* - name of the action which will be used as difference target for additive transformations. If you are using animation profile which doesn't have any additive transformations, you can leave that field empty
* **Diff pose frame** *(optional)* - frame of **diff pose** which will be used as a base for additive animations export
* **Additional armature** *(*optional*)* - Bones from this armature will be considered during the export. It can be used in export of vehicle animation, where you have door & character movement keyframed in single action - this way you don't have to switch between main Armature names

⚠

Selecting proper **export profile** is crucial for correct export of animation! In case of custom rig, it might be necessary to create **custom animation export profiles**

Once all desired animation actions are selected and properly configured for export (meaning they have **correct animation export profile** selected), it is now possible to finally perform export to TXA. To do so, press **OK** button in bottom section of the export window.

### Import animation to Enfusion

By default, **Enfusion Blender Tools** will try to **automatically register & import animation** if save location is located in one of the currently loaded addons.

If you are using **symbolic links**, then it might be necessary to manually register and import TXA animation - there are two ways how to do it:

* Drag and drop the TXA file into the [Resource Browser](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager").
* Click on TXA with Right Mouse Button and select **"Register and Import"** option from the Context Menu.

Once animation is imported, .ANM file will be automatically regenerated every time Workbench detects change in .TXA. If TXA was modified when Workbench was closed, manual reimport might be necessary.

### Troubleshooting

#### Incorrect save location

* Make sure that location exist
* Make sure that correct type of slashes are used when copy pasting path
* Make sure that relative path option is disabled if you are trying to save TXA file on different drive than blend file is located

#### TXA registration failed (Unable to translate to relative path)

Possible causes:

* Save location is outside of loaded projects in Workbench
* Symbolic links are used

If you see this warning, then be aware that automatic registration of txa animation is not going to work and you might need to manually register & import such file in **Resource Manager**.
Currently tool is not able to resolve symbolic links and won't be able to automatically register files in such cases. If such files

#### Unable to create metafile

If you get error like

```
Enfusion Tools - TXA Export WARNING - TXA exported to ... but Workbench couldn't create metafile! Check the export path.
```

It means selected file location is outside of any loaded by Workbench addons. Manual registration and import of file might be necessary afterwards.
