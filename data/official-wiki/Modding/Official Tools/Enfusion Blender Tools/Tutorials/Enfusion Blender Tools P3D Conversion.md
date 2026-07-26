# [Enfusion Blender Tools: P3D Conversion](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_P3D_Conversion)

The Enfusion Tools plugin for Blender offers an easy, semi automated way to import & convert to Enfusion standards any model from Real Virtuality 4 engine in unbinarised (MLOD) P3D format. They are distributed with the Arma Reforger Tools package on Steam.

⚠

ODOL P3Ds **cannot** be imported.

### Installation

P3D conversion is part of Enfusion Blender Tools therefor it is necessary to install that addon before proceeding. Installation instructions can be found on [Enfusion Blenders Tool page](/wiki/Arma_Reforger:Enfusion_Blender_Tools#Installation "Arma Reforger:Enfusion Blender Tools").

⚠

Since 0.9.7 version of the addon, **running Workbench with net API enabled** in necessary!

### Related Plugin Settings

[![armareforger-p3d-conversion-settings2.png](/wikidata/images/thumb/4/4c/armareforger-p3d-conversion-settings2.png/500px-armareforger-p3d-conversion-settings2.png)](/wiki/File:armareforger-p3d-conversion-settings2.png)

There is one parameter in this plugin which affect some of the P3D conversion features.

Addon settings can be found in **Edit → ⚙ Preferences... → Addons → 3D View: Enfusion Tools**

### Related parameters

#### Material Conversion Table

Path to material conversion table. By default plugin will use one of conversion table already distributed with the addon

## Interface

#### P3D Import

[![armareforger-p3d-conversion-import.png](/wikidata/images/thumb/b/ba/armareforger-p3d-conversion-import.png/800px-armareforger-p3d-conversion-import.png)](/wiki/File:armareforger-p3d-conversion-import.png)

To begin P3D import, navigate to **Enfusion Tools → Import** tab & then select "**Arma 3 P3D (.p3d)**". This will bring you typical Blender File View window where you can select file to import & specify some parameters.

#### Import Window Interface

This window allows to select the P3D file to be imported & specify import parameters.

* Discard unsupported LODs - see [Obsolete LODs Discarding](#Obsolete_LODs_Discarding) section
* Layer Presets
  + Here you can define which layer presets will be assigned to each imported LOD
* Rename Materials - see [Add Layer Presets & Game Materials section](#Add_Layer_Presets_.26_Game_Materials) section
* Convert axis to single point - see [Convert Memory Points Axis to Single Socket](#Convert_Memory_Points_Axis_to_Single_Socket) section

[![armareforger-p3d-conversion-import2.jpg](/wikidata/images/thumb/9/92/armareforger-p3d-conversion-import2.jpg/800px-armareforger-p3d-conversion-import2.jpg)](/wiki/File:armareforger-p3d-conversion-import2.jpg)

#### Imported Content Overview

Once P3D import is completed, all your objects will be organised into separate collections. Collections are created using following rule set:

* Each resolution LOD will be in its own - **LODx** - collection,
* Geometry related objects (Geometry, Geometry PhysX, View Geometry, etc) are stored in **Colliders** collection
* Memory points are stored in... **Memory points** collection

[![armareforger-p3d-conversion-import-mesh.jpg](/wikidata/images/thumb/d/d5/armareforger-p3d-conversion-import-mesh.jpg/800px-armareforger-p3d-conversion-import-mesh.jpg)](/wiki/File:armareforger-p3d-conversion-import-mesh.jpg)

### Functions

#### P3D Import

Any non binarised P3D model created for Arma 3 or DayZ can be imported into Blender using this plugin

#### Obsolete LODs Discarding

All non compatible LODs are discarded during import. Only the following LODs are imported:

* Resolution LODs
* Geometry
* Geometry PhysX
* View Geometry
* Fire Geometry
* Memory

There is the possibility to import unsupported LODs by unchecking "**Discard unsupported LODs**" option.

#### Objects Renaming

All imported objects are renamed to Enfusion standards so they can be correctly recognised during import to Workbench.

There are 2 main rules:

* All resolution LODs are receiving **\_LODx suffix**. LODs are automatically renumbered from 0 to 15.
* Geometry LODs receive **UTM\_ prefix**.

⚠

The model can only sport up to **16 resolution LODs**. Visual LODs above that number are discarded!

#### Add Layer Presets & Game Materials

Geometries are receiving proper **Layer Presets** and **Game Materials** depending on their type and RVMATs they have assigned.

Rules for material renaming can be customised by creating your own version of **conversion\_gameMaterials.txt** & then selecting that custom file in addon settings

#### Convert Memory Points Axis to Single Socket

Rotation & translation transformations in RV engine are using 2 memory points to determine axis of movement. Enfusion is using rotation of a single point to determine the origin of transformation.

## FBX Batch Conversion

[![armareforger-p3d-conversion-batch-flow.png](/wikidata/images/9/9e/armareforger-p3d-conversion-batch-flow.png)](/wiki/File:armareforger-p3d-conversion-batch-flow.png)

### Preparation

Batch file **convertToFBX.bat** is bundled together with the plugin. You can use this batch file to convert multiple P3Ds at once. This file is located in zip file in the following location: BohemianBlenderTools\p3d\_import\batch

After extracting it to your drive, default parameters will have to be modified:

#### Blender Executable Location

Quite self explanatory - provide the path to the Blender executable

```
set Object=C:\Program Files\Blender Foundation\Blender 2.92\blender.exe
```

#### Plugin Location

This path should lead to P3DtoFBX.py file which should be located in the plugin's installation directory.

```
set Script=%APPDATA%\Roaming\Blender Foundation\Blender\2.92\scripts\addons\BohemianBlenderTools\p3d_import\P3DtoFBX.py
```

Once those paths are adjusted, you can simply drag and drop single or multiple P3D files on **convertToFBX.bat**. New FBX files will be created next to the original P3Ds.
