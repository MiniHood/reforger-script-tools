# [World Editor](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor)

## Top Bar

The top bar contains World Editor quick access buttons grouped in three sections:

* **Basic Actions**
* **Basic Tools**
* **Scripted Tools**

These sections can be enabled or disabled using the right-click contextual menu on the top bar.

### Create new World

Create an entirely new world or a sub-scene of the currently loaded world. Shortcut: `Ctrl` + `N`

### Load World

Load an existing world. Shortcut: `Ctrl` + `O`

### Save World

Save the currently loaded world. Shortcut: `Ctrl` + `S`

### Go to World File

Display the loaded world into the **Resource Browser**.

### Undo last action

Undo the last applied action to the world. Shortcut: `Ctrl` + `Z`

### Redo last action

Reapply the last undone action. Shortcut: `Ctrl` + `Y`

### Copy selected object

Copy the currently selected object(s). Shortcut: `Ctrl` + `C`

### Paste on same position

Paste the copied object(s) in the same position of the copied object. Shortcut: `Ctrl` + `⇧ Shift` + `V`

### Cut selected object

Cut the currently selected object(s). Shortcut: `Ctrl` + `X`

### Play Game

Play the current world. Default shortcut: `F5`

Multiple options appear by clicking on the side arrow:

* **Play inside viewport** - make the game play within the actual viewport
* **Play in fullscreen** - make the game play in full screen

* **Play from camera position** - place a player-controlled unit right on viewport's camera position. Default unit is controlled by [**Default Player Controller**](/wiki/Arma_Reforger:Resource_Manager:_Options#Chimera_Global_Config "Arma Reforger:Resource Manager: Options") parameter in **[Resource Manager options](/wiki/Arma_Reforger:Resource_Manager:_Options#Chimera_Global_Config "Arma Reforger:Resource Manager: Options")**.

* **Server localhost** - start the game with a background server.

  ⚠

  Connection is possible with direct IP - a real address, **not** a loopback address like e.g 127.0.0.1!
* **Server localhost + PeerTool** - start the game with a background server and local clients. Clients are created accordingly to PeerTools settings and connected to the local server.

ⓘ

For Multiplayer Debugging, see [Script Editor - Debugging](/wiki/Arma_Reforger:Script_Editor#Debugging "Arma Reforger:Script Editor").

### Toggle Gizmo Space

Switch object manipulation tools (e.g. move, rotate) between World reference and Object reference. Shortcut: `X`

### Entity select filter

Set entity cursor selection from every, active or inactive layer(s).

### Select parent entities

Select parent entities when selecting a child entity.

### Ground Manipulation Tool

Move the selected object(s) by dragging it with the mouse pointer to the surface under the cursor, such as **terrain** surface, **object** surface or **water** surface. Shortcut: `Q`

ⓘ

Not to be confused with [Terrain Tool](#Terrain_Tool).

### Move Tool

Move the selected object(s) on the screen's plane: moving the mouse up will move the object higher compared to the current screen, mouse down lower, left and right to the screen's left and right, always keeping the same distance from the screen's plane. Shortcut: `W`

This tool also provides the **move** widget on the selected object(s), represented by colored arrows along X, Y and Z axes (in world or object reference, based on **Gizmo Space**). Click and drag to change the values.

### Rotate Tool

Rotate the selected object(s).

This tool provides the **rotate** widget on the selected object(s), represented by colored angular arcs between the axes X, Y and Z (in world or object reference, based on **Gizmo Space**). Click and drag to change the values. Shortcut: `E`

### Scale Tool

Scale the selected object(s), meaning to shrink or expand the object.

This tool provides the **scale** widget represented by a vertical green bar along the Y axis. Click and drag to change the value. Shortcut: `R`

### Vector Entity Tool

Draw a polyline (multiple straight segments) or a spline (curve from points), used by various generators (forests, walls, lakes etc.). Shortcut: `V`

### Bounding Volume Tool

Edit the entity's Bounding Box property with a visual tool.

### Terrain Tool

See [World Editor: Terrain Creation Tool - Sculpt](/wiki/Arma_Reforger:World_Editor:_Terrain_Creation_Tool#Sculpt "Arma Reforger:World Editor: Terrain Creation Tool"). Shortcut: `Ctrl` + `T`

### Navmesh Tool

Generates navmesh, AI navigation grids.

### Open Weather Editor

Open the Weather Editor panel.

### Measure Tool

Measure length along one or multiple 3D lines (only works on a terrain entity).

### Import Objects

Import objects from a CSV file.

See [Object Import Tool](https://community.bistudio.com/wiki/Arma_Reforger:Object_Import_Tool) dedicated article.

### Autotest Tool

Run test classes through the [Autotest Framework](/wiki/Arma_Reforger:Autotest_Framework "Arma Reforger:Autotest Framework").

### Polyline Area Tool

Edit the selected polyline to make it a rectangle/circle according to provided parameters.

### Windows Prefabs Generator Tool

Generate prefabs for destroyable windows.

### Batch Prefabs operations tool

### Coords Tool

Quickly navigate to a set of coordinates or generate a World Editor link to current position/orientation.

```
enfusion://WorldEditor/worlds/TutorialWorld/TutorialWorld.ent;16.3078,30.6084,14.5955;-10.8,48,0
```

ⓘ

This link is valid if Register "enfusion://" protocol has been done from within [Resource Manager: Options](/wiki/Arma_Reforger:Resource_Manager:_Options "Arma Reforger:Resource Manager: Options").

### Autospawner Tool

Spawn prefabs/XOBs from the Resource Browser's selected directory on a grid into an active layer.

Used for debug.

### Export Map data

Export data of various types from the current world in profile directory (Documents\My Games\ArmaReforger\profile)

### Import Shapefile

Import vector data from SHP file.

### Export Geographic data

Export vector data as Geographical data in profile directory (Documents\My Games\ArmaReforger\profile)

## Hierarchy/Create

### Hierarchy

This sub-panel lists entities present in the world. Entities are grouped into **layers** for clarity.

Red = Entity

Blue = Prefab

### Create

This sub-panel is a quick access to scripted entities that can be created in the world. Such entities can be user-defined by the EntityEditorProps class attribute.

## Viewport

This is the 3D world render that allows to edit terrain or preview a mission setup.

By default, pressing `Esc` leaves the preview; `F10` is a hardcoded equivalent to `Esc` (within World Editor) and can be used as a counterpart.

## Object Properties/Tool Properties

### Object Properties

This sub-panel is divided in sections - appearing sub-panels depend on the Entity's type; only the most common are described here:

##### Entity information

The Entity's root type is listed on the left, and on the right is a field to name the entity to be able to refer to it by script.

**Components** are then listed under the class.

The scope of changes brought in this interface can be set to the entity instance, the entity definition itself, modded entities, or the parent prefab(s).

#### Transformation

A way to change a lot of position and orientation values:

* coordinates
* angles
* scale (in range 0..1000)

#### General

##### Flags

* **Traceable:** the Entity can be detected *via* tracing methods
* **Visible:** the Entity is invisible in-game (still being visible in World Editor)
* **Static:** the Entity is expected to remain in position and will not be moved around
* **Feature:** the Entity is a "feature" of the terrain and will be drawn no matter the distance
* **Proxy:** the Entity is a sub-element of a prefab and should be hidden when the parent prefab is
* **Editor Only**: the Entity is present only in the editor
* **Relative Y**: the Entity's altitude will be relative to the surface below it

#### Script

Provide script "slots" for events and/or other cases

#### Unsorted

The uncategorised available settings.

### Tool Properties

This sub-panel changes depending on the currently selected tool.

## Viewport

### Navigation

With the mouse wheel alone ![Middle Mouse Button](/wikidata/images/thumb/e/e1/mouse-button-middle.png/32px-mouse-button-middle.png "Middle Mouse Button") you can move the camera forward and backward.

With the right mouse button ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") pressed:

* By moving the mouse, you can change the orientation of the camera.
* The mouse wheel ![Middle Mouse Button](/wikidata/images/thumb/e/e1/mouse-button-middle.png/32px-mouse-button-middle.png "Middle Mouse Button") changes the speed at which the camera moves - scroll up to increase the speed, scroll down to decrease it.
* Use the `W` and `S` keys to move the camera forward and backward along the camera orientation at the set speed.
* Use the `A` and `D` keys to move the camera sideways.
* Use the `Q` and `Z` keys to move the camera up and down.

![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") Double-clicking an entity in the Hierarchy subwindow focuses the camera on that entity. Pressing the `F` key focuses on the currently selected entity as well.

## Resource Browser/Prefab Library

Both Resource Browser and Prefab Library are content browsers, and use a color code underlight for entity (.et) types:

* **blue**: regular prefabs
* **purple**: Prefab Library entity
* **orange**: [editable Prefab](/wiki/Arma_Reforger:Game_Master:_Editable_Entities_Configuration "Arma Reforger:Game Master: Editable Entities Configuration") (used by in-game editor)

### Resource Browser

The Resource Browser provides a view of all the data present and available in the game's data as well as the user's profile directory.

It shows the project's data structure (see [Directory Structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure")) and displays every data type (e.g. config, textures but also [prefabs](/wiki/Arma_Reforger:Prefabs_Basics#Prefab "Arma Reforger:Prefabs Basics"), etc.).

### Prefab Library

The Prefab Library offers an interface to browse [Prefabs](/wiki/Arma_Reforger:Prefabs_Basics#Prefab "Arma Reforger:Prefabs Basics") that are available in the game's data.

Unlike the Resource Browser, it does not display content from sub-directories in the Entity Tab. Prefab Library is also not available in Resource Manager, since its closely tied to world creation process.

Prefabs are sorted by categories, represented by directories present in **Data\PrefabLibrary** (see Directory Structure):

* Decals
* Generators
* Infrastructure
* Props
* Rocks
* Structures
* Vegetation
* Walls

#### Directory Tab

This shows the directory structure and allows the wanted directory to be picked.

#### Entity Tab

This lists the entities found in the open directory.

#### Details Tab

This tab is filled only when an entity is highlighted in the Entity Tab.

| Entry | Description |
| --- | --- |
| Place By | Two options:  * origin * boundBox |
| Random Yaw | Determine if a random yaw (Y-axis rotation, direction) should be applied on object's placement. Randomization is applied in a 0..360° range |
| Align To Normal | Set if yes or no the entity should be placed perpendicular (normal) to the surface on which it is |
| Random Scale | Define whether or not the entity should be randomly scaled, between min-max set values |
| Random Vert Offset | Random Vertical Offset - define whether or not the entity should have a random vertical offset, between min-max (possibly negative) set values |
| Random Pitch Angle | Define if and how much of a random pitch ("front/back roll") should be applied to the entity |
| Random Roll Angle | Define if and how much of a random roll ("left/right (barrel) roll") should be applied to the entity |

ⓘ

Prefabs are simple wrapper entities that point to other entities. They can be renamed without any impact but their display name.

Changing a prefab's entity target will replace all the entity using this template by this new entity.

e.g. a Prefab named "small tree" being used in many forests, changing the prefab's entity from TreeA to TreeB will replace **all** the TreeA by TreeB entities.

## Log Console

The Log Console displays error, warning and informative messages. It can be cleaned by pressing the Clear Console button.

It also displays three buttons that can be combined:

* **Show only error messages**: only shows "Error" and "Fatal" logs
* **Show only warning messages**: only shows "Warning" logs
* **Show only info messages**: only shows "Spam", "Verbose", "Debug", "Normal" logs

If no buttons are selected, the Log Console shows all the messages.

The last button, **Filter Options**, allows for advanced filtering as shown on the side.
