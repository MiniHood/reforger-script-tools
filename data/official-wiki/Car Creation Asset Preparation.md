# [Car Creation/Asset Preparation](https://community.bistudio.com/wiki/Arma_Reforger:Car_Creation/Asset_Preparation)

### Preparing mesh

#### Model orientation

One of the most important thing to begin with is making sure that your **model is properly orientated**. Similar to **weapons**, there is one important rule to keep in mind

ⓘ

Everything must be oriented as pointing along/towards the Y+ axis in Blender and 3dsMax and along Z+ axis in Maya

That means, when you are **importing A3 weapon you need to rotate it by 90 degrees to the left!**

[![](/wikidata/images/thumb/8/8a/armareforger-new-car-orientation.png/1200px-armareforger-new-car-orientation.png)](/wiki/File:armareforger-new-car-orientation.png)

*Properly orientated model example*

#### Splitting mesh into parts

In principle, following elements of the vehicle should be split to separate models

* **Vehicle accessories** (i.e. vehicle roof) used for creation of variants of the same vehicle like i.e. cargo or fuel variant of the same truck.
* **Destructible elements** (windows, breakable elements of the vehicle, etc)
* **Emissive lights**

ⓘ

In contrast to weapons, **most of the vehicle parts** are **not using snap points** and instead, they have same origin as parent object.

In this example, windows **(1)** & emissive lights **(2)** were separated to their own FBX files.

[![](/wikidata/images/4/4a/armareforger-new-car-parts.png)](/wiki/File:armareforger-new-car-parts.png)

*Vehicle parts*

If those elements are already separated to unique objects, then it's possible to select them **one by one** and save them as FBX file with export limited to **currently selected object.**

[![armareforger-new-car-parts-export.png](/wikidata/images/d/d9/armareforger-new-car-parts-export.png)](/wiki/File:armareforger-new-car-parts-export.png)

Generally, such simple objects doesn't require skeleton but in case of more complicated elements (like gun mount), it's possible to add them to mesh.

#### Naming of objects

When it comes to naming, there are few important rules to keep in mind.

1. **\_LODx** suffix is used to indicate **Level of Details**. You can have **up to 8 LODs.**
2. **UBX\_, UCX\_, USP\_, UCS\_, UCL\_, UTM\_** prefixes are used to mark **Colliders** (Box, Cylinder, Sphere, Capsule, Cylinder, and Trimesh respectively)
3. **OCC\_** prefix is used for **Geometry Occluders**
4. **COM\_** prefix is used to determine geometry **center of mass**

There are also some general guidelines regarding naming of the vehicle parts on **Vehicle Pipeline - Parts' *page'***. It's strongly recommended to follow those rules since most of the **base prefabs are using those names by default** to reduce duplication & make easier data management. On that page, there are also general guidelines regarding naming objects which are not explicitly mentioned there.

Vehicle can also have various small sockets for destruction, crew positions management and contextual actions. Some of the rules regarding slots naming can be found in **slots section of Vehicle Pipeline** page.

#### Creating colliders

Colliders are special type of objects which are used to calculate various kinds of collisions - be it physic simulation or tracing of bullet penetration. There are few rules regarding those colliders and most of them listed here - [FBX Import - Collider usage](/wiki/Arma_Reforger:FBX_Import#Collider_usage "Arma Reforger:FBX Import"). In case of models imported from Arma 3, there are usually some convex components already present which could be used a base for collision geometry. It's recommended though to make at least fire geometries from scratch.

In case imported convex components are useless (too complex collision geometry or not detailed enough fire geometry) and completely new geometries have to be created, below there are some tips & rules.

##### Collision geometry

This type of geometry should be kept as simple as possible and amount of colliders should be also kept low. Simple type of colliders like convex, cylinders or cubes should be used. Large single shapes should be used to avoid issues with collision imprecision.

[![armareforger-new-car-collison-geo.gif](/wikidata/images/5/5e/armareforger-new-car-collison-geo.gif)](/wiki/File:armareforger-new-car-collison-geo.gif)

##### Fire geometry

Penetration geometries can be bit more detailed and, in contrast to Real Virtuality engine, they don't have be convex. Usage of trimesh geometry can greatly speed up creation of detailed fire geometry.

[![armareforger-new-car-collison-firegeo.gif](/wikidata/images/d/d7/armareforger-new-car-collison-firegeo.gif)](/wiki/File:armareforger-new-car-collison-firegeo.gif)
Some additional information can be also found on Vehicles - Colliders and Fire Geometry.

##### Layer presets

[Layer presets](/wiki/Arma_Reforger:Collision_Layer "Arma Reforger:Collision Layer") are used by engine to determine how given collider should behave when it collides with other object. Layer Presets can be either directly defined in modelling software by adding custom property to object or can be changed in Workbench after successful import of the mesh. If you are using Blender, there is small official tool - [**Enfusion Blender Tools**](/wiki/Arma_Reforger:Enfusion_Blender_Tools "Arma Reforger:Enfusion Blender Tools") - to assist you with assigning correct game materials & layer presets on colliders.

[![](/wikidata/images/a/ad/armareforger-fbx-layers-blender-1.png)](/wiki/File:armareforger-fbx-layers-blender-1.png)

Blender - manual input

**For creating custom attributes in 3dsMax, use only the Parameter editor in Animation menu**, otherwise the properties in exported FBX can't be edited in Maya. So doing it with Object Properties/User defined is **FORBIDDEN**

More details about **Collision Layers, Layer Presets and Interaction Matrix** can be found on [linked page](/wiki/Arma_Reforger:Collision_Layer "Arma Reforger:Collision Layer").

#### Setting skeleton & rigging mesh

When creating skinned objects, Enfusion Workbench expects that the whole object is skinned to some bone. Otherwise importer will try to "fix" it by skinning remaining faces to some root bone. In such case, you might see following error in the console:

`RESOURCES (W): Missing some mesh skinning weights (Object_LOD0). Weighting them to root`

In Blender realms, this means that any object with Armature modifier, must be fully skinned to some existing bone through vertex groups.

If you have some animated collider please keep in mind that **only trimesh colliders can be skinned**. In all other cases you have to use **100% weight**.

In Blender it gets even bit more tricky and **Object Relations** (in ***Object tab***) have to be used.

[![armareforger-new-weapon-relations.png](/wikidata/images/5/52/armareforger-new-weapon-relations.png)](/wiki/File:armareforger-new-weapon-relations.png)

### FBX export settings

Most of the general rules can be found on **[FBX Import page](/wiki/Arma_Reforger:FBX_Import#FBX_Export "Arma Reforger:FBX Import")** . In principle, when exporting from i.e. 3DS Max, you have to make sure that you are exporting in **binary format in version 2014/2015**. Furthermore, **Triangulation** & **Preserve Edge Orientation should be turned off.**

**Blender** wise, there are 3 most important things to keep in mind when exporting FBX

|  |  |
| --- | --- |
| **1. Object Types**: For animated object like car you need to have checked on at least:  **Empty -** which handles all snap points  **Armature** **-** exports skeleton of your weapon  **Mesh -** self explanatory | [armareforger-new-weapon-export-blender2.png](/wiki/File:armareforger-new-weapon-export-blender2.png) |
| **2. Custom Properties**: Without this option all custom properties like **LayerPresets** would be lost! |
| **3. Leaf bones**: Leaf bones are completely unnecessary in Enfusion and it's better to have that option **turned off** |

## Importing Mesh

**Overview**

In this chapter following topics will be explained

* Importing new model
* Importing textures

### Importing & registering new model

Once mesh was successfully prepared and all selections, sockets & snap points are in place, it's time to try our asset in game. Majority of the process is already pretty well described on FBX import - Import process in the Workbench page.

In principle, all you have to do is click with right mouse button on your FBX files and select from context menu **Register and Import** option and then click **as Model** to import selected mesh as a game model.

ⓘ

If you are importing models, it's worth to check "**The same for all FBX files**" option

[![armareforger-new-car-importing.gif](/wikidata/images/thumb/9/93/armareforger-new-car-importing.gif/956px-armareforger-new-car-importing.gif)](/wiki/File:armareforger-new-car-importing.gif)

#### Checking colliders & materials

After initial import was done it's time to make sure that materials & colliders are using proper materials & colliders. By default, Enfusion will try to assign material based on the name of the assigned texture in mesh. If it fails to find such texture, new dummy material (see area marked in orange on screen below) will be created in **data** folder next to the FBX model.

There are 2 typical errors when it comes to collider configuration:

[![](/wikidata/images/thumb/b/b8/armareforger-new-weapon-collider-errors.png/800px-armareforger-new-weapon-collider-errors.png)](/wiki/File:armareforger-new-weapon-collider-errors.png)

Colliders errors example

⚠

Make sure that you have "usage" property defined in collider object properties.

[![armareforger-fbx-layers-blender-1.png](/wikidata/images/thumb/a/ad/armareforger-fbx-layers-blender-1.png/200px-armareforger-fbx-layers-blender-1.png)](/wiki/File:armareforger-fbx-layers-blender-1.png)

More info about it can be found on [Arma\_Reforger:FBX\_Import](/wiki/Arma_Reforger:FBX_Import "Arma Reforger:FBX Import")

⚠

Make sure that correct material is assigned to all colliders

[![armareforger-new-weapon-colliders-material.png](/wikidata/images/thumb/3/3c/armareforger-new-weapon-colliders-material.png/200px-armareforger-new-weapon-colliders-material.png)](/wiki/File:armareforger-new-weapon-colliders-material.png)

More info about it can be found on [Arma\_Reforger:FBX\_Import](/wiki/Arma_Reforger:FBX_Import "Arma Reforger:FBX Import")

#### Skeleton & hierarchy

Since this model is animated, it's very important to check **Export Skinning** checkbox in Import Settings of model. Without that parameter selected, vehicle will be imported without skinning or any sockets!

[![armareforger-new-car-skeleton-import.png](/wikidata/images/b/b7/armareforger-new-car-skeleton-import.png)](/wiki/File:armareforger-new-car-skeleton-import.png)

### Importing textures

In principle, you can use same procedures as the ones described in [|Weapon Modding - New Materials](/wiki/Arma_Reforger:Weapon_Modding#New_Materials "Arma Reforger:Weapon Modding") creation page. By default, Workbench will new materials based on the materials name from the FBX file.
