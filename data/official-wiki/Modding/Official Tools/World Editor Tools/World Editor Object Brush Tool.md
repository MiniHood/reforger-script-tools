# [World Editor: Object Brush Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Object_Brush_Tool)

**Object Brush** is a scripted [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") Tool.
It allows for quick terrain "painting" with entities, based on the set parameters.

To access it, click the paintbrush icon in World Editor.

## Usage

* ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") to generate Prefabs in the area
* ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") and drag to generate Prefabs along mouse movement
* `Alt` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") then to generate Prefabs along a straight line; `Esc` to cancel
* Hold `Space` to delete Object Brush-created entities (works with all control combinations above)

## Settings

* 1. [Object Brush](#Object_Brush)
* 2. [Objects Config](#Objects_Config_2)
* 3. [Obstacles](#Obstacles)
* 4. [Obstacles - Area](#Obstacles_-_Area)

### Object Brush

| Name | Description |
| --- | --- |
| Radius | Brush's radius |
| Strength | Brush's strength, value being in entity per hectare (100×100m) |
| Scale Fall Off Curve | Define the scale's Fall Off curve - curve goes from centre to border |
| Density Fall Off Enabled | Enable Density Fall Off settings below Density Fall Off only works well for wid brush single click placements |
| Density Fall Off Curve | Define the density's Fall Off curve - curve goes from centre to border |
| Density Fall Off Subarea Count | Define the amount of sub-areas (internal sub-division for performance) |
| Override Brush | Remove previously Brush-placed entities |
| Objects Config | This is where objects to be created are defined - see the section below. An [SCR\_ObjectBrushArrayConfig](enfusion://ScriptEditor/scripts/WorkbenchGame/WorldEditor/ObjectBrush/SCR_ObjectBrushArrayConfig.c;2) config can be drag & dropped here.  ⓘ  Config examples can be found in [Configs/Workbench/ObjectBrush](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/ObjectBrush). |

### Objects Config

| Name | Description |
| --- | --- |
| Min Scale | Maximum scale the object can have - requires Override Randomization enabled |
| Max Scale | Maximum scale the object can have - requires Override Randomization enabled |
| Prefab | The Prefab to use |
| Bot Distance | *unused* |
| Random Pitch Angle | Random X angle ("looking down/up") |
| Random Roll Angle | Random Z angle ("barrel roll") |
| Vertical Offset | *unused* |
| Random Yaw Angle | Random Y angle ("direction") |
| Prefab Offset Y | Static value that will be added to the Prefab's Y position |
| Min Random Vertical Offset | -value..0 negative range of Y randomness (cumulative with Prefab Offset Y) |
| Max Random Vertical Offset | 0..+value positive range of Y randomness (cumulative with Prefab Offset Y) |
| Override Randomization | If checked, Prefab will use:  * Min Scale / Max Scale * Random Pitch, Random Roll, Random Yaw values   Otherwise, Prefab's Library random values will be used |
| Weight | The selection probability weight; if set to zero, the Prefab is ignored |
| Align To Normal | Align the generated Prefab to terrain's normal (makes it perpendicular to the terrain) |
| Scale Falloff | Whether or not using Scale Fall Off parameters |
| Lowest Scale Falloff Value | Minimum scale value the Scale Fall Off can set |

### Obstacles

| Name | Description |
| --- | --- |
| Avoid Objects | Avoid creating Prefabs where other entities are placed |
| Avoid Objects Detection Radius | Objects detection radius - can be zero |
| Avoid Objects Detection Height | Objects detection's max height (from the ground) |
| Avoid Roads | Avoid roads, using their [RoadGeneratorEntity](enfusion://ScriptEditor/scripts/GameLib/entities/editor/RoadGeneratorEntity.c;8) **Road Clearance** value |
| Avoid Rivers | Avoid rivers, using their [RiverEntity](enfusion://ScriptEditor/scripts/GameLib/generated/Entities/RiverEntity.c;21) **Clearance** value |
| Avoid Power Lines | Avoid powerlines, using their [SCR\_PowerlineGeneratorEntity](enfusion://ScriptEditor/scripts/Game/Generators/SCR_PowerlineGeneratorEntity.c;15) **Clearance** value |
| Avoid Land | Avoid anything above ocean's level |
| Avoid Ocean | Avoid anything below ocean's level |

### Obstacles - Area

| Name | Description |
| --- | --- |
| Avoid Forests | Avoid forests (areas with a [ForestGeneratorEntity](enfusion://ScriptEditor/scripts/Game/Generators/ForestGeneratorEntity.c;12)) |
| Avoid Lakes | Avoid lakes (areas with a [LakeGeneratorEntity](enfusion://ScriptEditor/scripts/GameLib/generated/Entities/LakeGeneratorEntity.c;16) **and** with "Is Lake" checked) |
| Area Detection Radius | Area (forest, lake) detection radius *from the first click* Set to zero to detect all areas of the world, takes more time the first time (and more RAM) |
