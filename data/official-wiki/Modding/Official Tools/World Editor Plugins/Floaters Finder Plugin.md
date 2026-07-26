# [Floaters Finder Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Floaters_Finder_Plugin)

| Floaters Finder |
| --- |
| [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin |
| `Ctrl` + `Alt` + `↟ PgUp` |
| Detect floating entities that do not belong in the air |
| **File:** [SCR\_FloatersFinderPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/WorldEditor/SCR_FloatersFinderPlugin.c) |

**Floaters Finder** is a plugin that detects ***possibly*** misplaced entities:

## Parameters

### Search

* **Active Layer Only**: only search in the currently active layer
* **Search For Entities Below Terrain**: check for entities entirely below terrain by bounding box vertices' altitude
* **Search For Vegetation**: look for misplaced vegetation (trees and bushes) floating above or below normal level

### Angle

* **Use Prefab Angles**: use the Prefab's random angles as acceptable margin, otherwise use **Max Tree Angle** below
* **Max Tree Angle**: maximum allowed angle within which a tree is not considered "fallen"

### Vertical Offset

* **Check Above Entities Surface**: check entity's altitude from **entities** below it (twice slower)
* **Use Prefab Vertical Offset**: use the Prefab's vertical offset range, otherwise use **Min/Max Vertical Offset** below
* **Min Vertical Offset**: minimum (included) vertical offset
* **Max Vertical Offset**: maximum (included) vertical offset
* **Check Below Water**: check for items being below water level (water = ocean only for now); the entity is selected if under water, no matter the offset

### Performance

* **Camera Search Radius**: search radius around the camera in metres - 0 = all entities
* **Show Search Radius Sphere**: show a sphere indicating the search radius
* **Max Selected Entities**: maximum number of selected entities in World Editor
* **Trace Vegetation Precisely**: trace only checks for the vegetation bounding box's full height, allowing it to be under a bridge but maybe missing a small tree being in a big rock (need Search/**Search For Vegetation**)
* **Trace Origin Distance**: trace origin's distance in metres from above the entity to determine if it is below another entity AND **Trace Vegetation Precisely** above is unchecked

### Output

* **Output Findings To File**: write a file with links to all findings
* **Use Web Prefix**: prefix file links with the https prefix

## See Also

* [Snap And Orient Entities To Terrain Plugin](/wiki/Arma_Reforger:Snap_And_Orient_Entities_To_Terrain_Plugin "Arma Reforger:Snap And Orient Entities To Terrain Plugin")
