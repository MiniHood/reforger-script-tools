# [Enfusion Blender Tools](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools)

**Enfusion Blender Tools** (shortened to **EBT**) is a Blender addon allowing for a good workflow between Blender and Workbench, and Enfusion in general. Primarily developed and tested with [4.5 LTS (Long Term Support)](https://www.blender.org/download/lts) version of Blender.

ⓘ

Enfusion Blender Tools **Tutorials** can be found at [Enfusion Blender Tools Tutorials](/wiki/Category:Arma_Reforger/Modding/Official_Tools/Enfusion_Blender_Tools/Tutorials "Category:Arma Reforger/Modding/Official Tools/Enfusion Blender Tools/Tutorials").

## Features

### Import

* [ASC elevation (.asc file)](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_ASC_Elevation "Arma Reforger:Enfusion Blender Tools: Import/Export ASC Elevation") - import an ASC elevation file (Esri grid) as terrain mesh
* [Arma 3 P3D (.p3d file)](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_P3D_Conversion "Arma Reforger:Enfusion Blender Tools: P3D Conversion") - import P3D
* [FBX (.fbx file)](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Materials_Preview "Arma Reforger:Enfusion Blender Tools: Materials Preview") - import FBX with Enfusion Shaders
* Prefab (.et file) - importing models located in prefabs (including prefabs located in hierarchy). **It is only working on prefabs where source FBX are available - that means, you cannot use this function to edit read-only assets like Arma Reforger or downloaded mods.** Function is meant mainly for structures and baking MLODs.

### Export

* [ASC elevation (.asc file)](/wiki/Enfusion_Blender_Tools:_Import/Export_ASC_Elevation "Enfusion Blender Tools: Import/Export ASC Elevation")
* [Enfusion animation (.txa file)](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_Animation "Arma Reforger:Enfusion Blender Tools: Import/Export Animation")
* FBX (.fbx file) - exporting single FBX file with automatic registration of model in Workbench
* [Batch FBX export (.fbx file)](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Batch_FBX_Export "Arma Reforger:Enfusion Blender Tools: Batch FBX Export") - batch exporting of FBX files

### Misc

* [Object Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools "Arma Reforger:Enfusion Blender Tools: Objects Tools") - Various small tools helping with preparation of model for Workbench import
* [Model Quality Assurance](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Model_Quality_Assurance "Arma Reforger:Enfusion Blender Tools: Model Quality Assurance")
* [NLA Strips Baking Tool](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_NLA_Strips_Baking_Tool "Arma Reforger:Enfusion Blender Tools: NLA Strips Baking Tool")
* [Rig Updater](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Rig_Updater "Arma Reforger:Enfusion Blender Tools: Rig Updater")
* [Skeleton Updater](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Skeleton_Updater "Arma Reforger:Enfusion Blender Tools: Skeleton Updater")
* [Portal Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Portal_Tools "Arma Reforger:Enfusion Blender Tools: Portal Tools")

## Installation

[![Location of Enable net API in Workbench Options](/wikidata/images/thumb/2/24/armareforger-enfusionblendertools-net-api.png/600px-armareforger-enfusionblendertools-net-api.png)](/wiki/File:armareforger-enfusionblendertools-net-api.png)

Location of Enable net API option in Workbench Options window

* Download [Blender LTS 4.5](https://www.blender.org/download/lts/4-5/)
* Download **Arma Reforger Tools** from Steam

  ⓘ

  Arma Reforger Tools only appear if Arma Reforger is in your Steam Library.
* Install EBT addon from Arma Reforger Tools\Blender\EBT-ArmaReforger.zip (do not unzip it!)
  + See [How to Install Add-Ons in Blender](https://www.youtube.com/watch?v=LzdoUTvAgXk&t=64s)
* Open **Workbench** and in **Workbench → Options → Workbench** settings turn on **Enable net API (for communication with external applications)** option

[![](/wikidata/images/a/a9/armareforger-enfusionblendertools_options_setup2.png)](/wiki/File:armareforger-enfusionblendertools_options_setup2.png)

Enfusion Blender Tools addon preferences

If you intend to use TXA exporter and want to use Reforger library of [animation export profiles](/wiki/Arma_Reforger:Animation_Export_Profiles "Arma Reforger:Animation Export Profiles"), it is also necessary to set up **Export Profile Folder**

* Unzip Arma Reforger Tools\Blender\EBT-ArmaReforger-Data.zip to any empty folder
* In **Export Profile Folder**, press the '**+**' button and set path to the directory where EnfusionBlenderTools-Data.zip file was extracted (see image)

Similar steps can also be performed to define the custom animation export profiles directory

## Updating

When updating Enfusion Blender Tools, best thing to do is:

* Open Blender
* In Addons section of Blender Preference, remove **Enfusion Tools** manually
* Close Blender
* Open Blender again
* Install the plugin as per installation instructions above

This will ensure that there are no residual bits of addons left in memory which can prevent correct installation of plugin

## Interface

Once addon is properly installed and activated, two new elements - **(1)** & **(2)** should be visible in main interface of Blender.
Depending on used layout, right section of the menu might need to be expanded by clicking on small arrow on the right side of the viewport.

[![armareforger-enfusionblendertools interface.jpg](/wikidata/images/d/d9/armareforger-enfusionblendertools_interface.jpg)](/wiki/File:armareforger-enfusionblendertools_interface.jpg)

### Top Menu

In the top section of the viewport, **Enfusion Tools (1)** tab contains **Import** & **Export** sub menus where it is possible to [Import P3D](#Import_P3D), [ASC file](#Import_ASC) or [FBX models](#Import_FBX) and [Export ASC](#Export_ASC) or [Export TXA](#Export_TXA) animations.

#### Import ASC

Import a .asc terrain file.

#### Import P3D

##### Discard unsupported LODs

LODs like View Cargo/Gunner/Pilot, Roadway, Hitpoints, Paths and similar will be discarded from the import.

##### Layer Presets

Assign detected layers (geometry, physics)

##### Game Materials

###### Rename materials

Rename RVMATs to Enfusion equivalents.

##### Memory Points

##### Convert axis to single point

Convert two points with default rotation to single point with orientation axis.

#### Import FBX

This option activates import of [FBX with Enfusion Shaders](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Materials_Preview "Arma Reforger:Enfusion Blender Tools: Materials Preview").

##### Remove All Objects

Remove any object present in the fbx file.

#### Export ASC

Export the terrain to .asc format.

#### Export TXA

Export the [animation to the TXA format](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_Animation "Arma Reforger:Enfusion Blender Tools: Import/Export Animation").

### Side section

On the right side of the viewport, **Enfusion Tools tab (2)** you have following options:

#### Model Quality Assurance

In this panel **(3)** it is possible to adjust and execute Model Quality Assurance script which checks for common configuration & topology errors in the mesh.

#### Settings

Settings panel **(4)** contains options for Batch FBX export

#### Object Tools

Object Tools **(5)** panel contains controls for [automatic object sorting into collections](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools#Automatic_sorting_of_objects "Arma Reforger:Enfusion Blender Tools: Objects Tools")

#### Material Tools

This section **(6)** contains options [colliders setup](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools#Colliders_.26_Layer_Presets_setup "Arma Reforger:Enfusion Blender Tools: Objects Tools"), light setup and [(re)import of Enfusion Materials into Blender](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Materials_Preview#Materials_Synchronisation "Arma Reforger:Enfusion Blender Tools: Materials Preview")
