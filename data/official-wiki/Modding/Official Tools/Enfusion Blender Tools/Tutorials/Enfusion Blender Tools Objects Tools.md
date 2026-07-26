# [Enfusion Blender Tools: Objects Tools](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools)

## Introduction

Objects Tools are collection of various helper scripts to manage Enfusion specific properties. Those tools are part of [Enfusion Blender Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools "Arma Reforger:Enfusion Blender Tools")

## Automatic Sorting of Objects

[![armareforger-blender-object-tools-overview.png](/wikidata/images/2/2d/armareforger-blender-object-tools-overview.png)](/wiki/File:armareforger-blender-object-tools-overview.png)

**Sort Objects** operator is located in side panel of viewport and can be found **Enfusion Tools** tab. This tool creates new collections for all objects in the scene similar to the structure that is used in Workbench. That means that for instance all objects with one of the collider prefixes like **UCL, UTM & similar**, will be assigned to **Colliders** collection. Additionally, this tool will also create **sub collections** for **[layer presets](/wiki/Arma_Reforger:Collision_Layer "Arma Reforger:Collision Layer")** it detected on colliders objects, move all **empty objects** to **Memory Points** collection, and put all **armatures** into **Skeletons** collection. Full list of all recognised suffixes & prefixes is attached below.

Sort Objects have following optional parameters:

* **Fold collections** - All collections will be automatically folded after this action is activated
* **Hide collections** - All collections, except first visual LOD, will be hidden
* **Separate by materials** - Objects in LOD collections will be additionally separated by their material

| Prefix/Suffix | Collection |
| --- | --- |
| *UCL\_* *UCX\_*  *UBX\_*  *UTM\_*  *USP\_*  *UCS\_*  *COM\_*  *LC\_* | Colliders |
| *PRT\_* | Light Portals |
| *BOXVOL\_* | Volumetric Boxes |
| *OCC\_* | Occluders |
| *SOCKET\_* | Memory Points |
| *\_LODxx* | LODx |

## Colliders & Layer Presets Setup

[![armareforger-blender-object-tools-layers.png](/wikidata/images/a/af/armareforger-blender-object-tools-layers.png)](/wiki/File:armareforger-blender-object-tools-layers.png)

Before using **Colliders Setup** tool, make sure you that you are running **Workbench with net API enabled**. **Materials Tools** also depend on correctly sorted objects and at least colliders should be located in separate **Colliders** collection in **Blender.**

[![armareforger-blender-object-tools-layers2.png](/wikidata/images/thumb/7/75/armareforger-blender-object-tools-layers2.png/1400px-armareforger-blender-object-tools-layers2.png)](/wiki/File:armareforger-blender-object-tools-layers2.png)

With **Collider Setup** button it is possible to change assigned **Game Material** or **Layer Preset** for one or multiple selected objects in **Colliders** collection (*including sub collections*).

### Assigning Layer Presets

In **Object Mode** material will be applied to whole object and will remove all other materials from it. It is also possible to change **Layer Presets** in this mode.

[![armareforger-blender-object-tools-colliders.gif](/wikidata/images/d/dc/armareforger-blender-object-tools-colliders.gif)](/wiki/File:armareforger-blender-object-tools-colliders.gif)

### Assigning game material

ⓘ

*Game materials menu omits materials which ends with \_base word in order to avoid cluttering the list too much.*

Assignment of multiple different game materials per object is possible only in **Edit Mode**. Such action can be performed by simply selecting some of the faces and then using **Colliders Setup** button

[![armareforger-blender-object-tools-colliders-edit.gif](/wikidata/images/e/e8/armareforger-blender-object-tools-colliders-edit.gif)](/wiki/File:armareforger-blender-object-tools-colliders-edit.gif)
