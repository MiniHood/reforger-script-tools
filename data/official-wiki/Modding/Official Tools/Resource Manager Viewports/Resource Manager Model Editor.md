# [Resource Manager: Model Editor](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Model_Editor)

## Horizontal bar

| Tool name | Description |
| --- | --- |
| **Show/hide grid** | Shows or hides 1m grid on XZ plane |
| **Move tool** | Shows move gizmo in the origin of the object |
| **Rotate tool** | Shows rotate gizmo in the origin of the object |
| **To world/object space** | Switches between world and object space when using move and rotate tools |
| **Show/hide all bones** | Shows or hides bones in model (if has any). It is possible then to select bone in viewport by clicking on it and move or rotate it via move and rotate tools |
| **Reset model bones transformations to init state** | Pretty self-explanatory |
| **Show/hide volumetric info** | Shows generated information about volumetric shadows Heightmap shadows |
| **Colliders** | Shows all colliders in object Can show them in two modes:   * Flat-shaded * Transparent |
| **Snap position to ground** | Snap object to the ground by its origin |
| **Reset entity rotation** | Revert any rotation (**not** 3D movement) the object endured. |
| **Change FOV** | Changes field of view of the camera (valid values are 5-120) |
| **Background color** | Shows color picker so you can choose custom background color. Also it is possible to reset it to default color. |
| **Camera auto placement** | 🚧  **[TODO](/wiki/Category:To-do "Category:To-do"):** seems to disable **View > Perspective** and **Focus selected object** |
| **Turn camera to POI on mouse wheel** | When enabled, Scroll wheel will zoom in maximum to the object. When disabled, Scroll wheel will zoom towards the centre of the screen. |
|  |  |
| **EV tools - lock, plus and minus** | Enables user controlling exposure values - locking it or increasing and decreasing EV values. |
|  |  |
| The following tools appear only with .xob files (not e.g .et) | |
| **Pick material** | Enables pick tool which shows wireframe overlay over part of the model you hover over, on LMB click, it selects material in Materials tab. Shortcut: `Ctrl` + Left Mouse Button |
| **Focus selected object** | Focuses on model in viewport, also accessible via F key. |

## Viewport UI

### View

Switch between perspective and orthographic view

* The perspective view, allows to rotate the camera around
* All other views can only pan around:
  + Top
  + Bottom
  + Front
  + Back
  + Right
  + Left

### Shading

Shading options are divided in two sections - **Lighting** and **Channels**.

It enables users to isolate each "pass" in viewport. Only one lighting and channel can be isolated at once.

| Lighting | Channels |
| --- | --- |
| * Lit * Wireframe * Wireframe only * Transmittance * Detail lilghting * Direct diffuse * IBL diffuse * Direct reflection * IBL reflection * Full diffuse * Full reflection * Emissive * Vertex normals only | * None * Albedo * Vertex color * Roughness model * Roughness scene * Metallic * World normals * Mask * Ambient occlusion * Albedo (linear) * Wetness * Porosity * VFX |

### Overlays

#### Wireframe

Display wireframe overlay over the mesh.

#### Vertex normals

Display vertice normals.

#### Vertex highlight

Display vertices in mesh.

### Options

#### Search bar

Search through all property names to find the needed one.

ⓘ

(EV) is sometimes used in the following documentation (in e.g **Environment**, **Lighting** and **Camera Settings**);  
this means Exposure Value - see [Exposure value](https://en.wikipedia.org/wiki/Exposure_value) for more information.

#### Scene preset

##### Preset names

This is a read-only list of available presets.

##### Active preset

Choose the preset to use/edit.

#### Scene settings

##### Grid

Show or hide the grid.

##### Grid Color

Set grid's color.

##### Num. cells

Define how many cells will compose the grid's X and Y axis.

##### Cell scale

Define cells' size. Default value is 1 (meter).

##### Ground

Show or hide the ground.

##### Ground material

Set the material used for the ground.

##### Compass

Show or hide compass (top-right).

##### Axis

Show or hide X/Y/Z axis (bottom-left).

#### Wireframe settings

##### Opacity

Set the wireframe's display opacity.

#### Overlay settings

##### Color

Define the overlay color (when selecting e.g Overlay > Wireframe).

##### Constant screen size

If enabled, zooming in will not change overlay's size.

If disabled, overlay will remain the same size and zooming in/out will show it bigger/smaller.

##### Vertex highlight scale

Scale vertex highlight in range 0..4.

##### Vertex normal scale

Scale vertex normal in range 0..4.

#### Environment settings

##### Top background color

Set environment top light's color.

##### Zenith background color

Set environment zenith (horizon) light's color.

##### Bottom background color

Set environment bottom light's color.

##### Environment map

Set the environment's skybox.

##### Background opacity

Set skybox's opacity over Top/Zenith/Bottom background color.

##### Hide env map

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** Hide skybox?

##### Environment exposure (EV)

Set environment exposure.

##### Angle

Rotate the skybox in range 0..360 (degrees).

#### Lighting settings

##### Light color

Set the world light's color.

##### Light exposure (EV)

Set world light's exposure.

##### Angular size in degrees

Set world light's angular size in degrees, range 0..20°.

##### Rotation

Set world light's angle of incidence in degrees. X range -90..+90°.

Keyboard shortcut: `⇧ Shift` (Y axis) and `Alt` + `⇧ Shift` (X axis).

#### Camera settings

##### Camera exposure (EV)

Set the camera exposure.

##### Lock exposure

Lock the camera exposure to its current value (even if adjusting).

##### FOV (Degrees)

Set the camera's Field of View.

##### Camera position

Set camera's position - unit is in metre. { 0, 0, 0 } is the centre of the area.

##### Camera angles

Set camera's angle - unit is in degrees.

#### Postprocessing

##### User postprocesses

Set post process

By default, ssr.emat and hbao.emat are set.

ⓘ

hdr.emat (among others) can be added to have a render closer to what it is in-game.

#### Preset buttons

##### Save preset

This button will save the currently edited preset. If none is actually selected, it acts as pressing New preset.

##### New preset

Create a new preset to be used in

ⓘ

For now, a created preset **cannot** be renamed nor deleted from the interface.  
**Files** (.pre) are created in the profile directory, in the ResourceViewerPresets sub-directory and can be renamed/deleted there.

### Compass

The "compass" is a 3D representation of the camera's point of view.

Its sections can be clicked to make the camera "face" and centre the chosen section.

## Details Tab

### Materials

Materials (.emat) are surface information that can be applied on models.

ⓘ

See also [Resource Manager: Material Editor](/wiki/Arma_Reforger:Resource_Manager:_Material_Editor "Arma Reforger:Resource Manager: Material Editor").

This tab allows to:

* Browse through model's materials
* Click on them to highlight them with wireframe overlay in viewport
* Click on the icon of material to edit it while observing change directly on model
* Isolate material to see just part of the mesh which uses it

### LODs

A LOD (Level Of Detail) is a version of a 3D model that is more or less detailed in order to save resources according to e.g its distance from the camera's point of view. For more information, see Level of detail Wikipedia page.

This tab allows to:

* Browse through LODs, look at their statistics (number vertices, faces, materials, meshes)
* Force to show only specific LOD if you click on LOD X tab
* Check out specific meshes in each LOD (they will be highlighted in viewport with wireframe overlay) and view UVs for each mesh

#### Force selected LOD to render

Enable the selected LOD to render in the viewport.

#### Unselect LOD

Deselect the currently selected LOD.

By default, LOD 0 will be the one displayed.

### Colliders

A collider is a very simple 3D shape that provides collision information to the physics engine. See also Collider Wiktionary page.

#### Options

##### Hide mesh

Hide the object's mesh when colliders are shown (e.g with "Filter colliders" selected).

##### Show center of mass

##### Show collider outlines

##### Transparent colliders

##### High contrast background

##### Filter colliders

##### Show invalid shapes

#### Layers

#### Center of mass

#### Vertices

#### Faces

#### Colliders

Browse through colliders and list their statistics:

* Type
* Material
* (Relative) Coordinates
* Angles
* Vertices
* Faces
* LayerPreset
* Interaction layers

### Occluders

ⓘ

An occluder is a virtual plane shape meant to hide (occlude) objects behind it to save on processing/drawing cycles. Its geometry is kept simple for the calculations to remain efficient.

This tab allows to:

* Project or show all occluders at once or separately by selecting them in the list
* See their type, number of vertices and coordinates with bounds

#### Project all occluders

#### Show all occluders

### Land Contacts

Land Contacts are points in model (dummies, empties) which are positioned on place where model should snap to the ground.

It is mostly used on fences, walls, etc.

They are marked in viewport as horizontally placed green circles.

### Bones

* Viewing bones, moving bones around (translation, rotation)
* Hierarchy
* Difference between bone and empty
* Link to "general" animations docu, where there should be more info

### Runtime Details

In here, you can see various information about model:

* Current LOD
* Number of Faces
* Number of Vertices
* Number of Landcontacts
* Approximate size (in meter)
* Allocated VB (Video B… uffer?
* Number of materials
* Number of textures
* Total textures size (on the disk)
* Volume info (grid usage)
* Surface, Triangle and Indices information from under the cursor (when on model)

## Import Settings Tab

Each tab represents the import option for the related platform.

### General

#### Present

Whether the resource will be imported and available in data.

#### Source File

Can be used to override source file to any compatible file (model) in folder.

#### Common

Common parameters which can be shared via config file (for the file type).

### Miscellaneous

#### Merge Meshes

Importer will merge visual meshes in each LOD.

#### Export Skinning

Imports skinning (if any is present in source file).

#### Export Scene Hierarchy

Imports scene hierarchy (if any is present in source file).

Usable for e.g. dummies (empties, locators), which are then used as sockets, or just general position info.

#### Modify material names

Switches "\_" to "/" in material name upon import.

#### Legacy Support

Import resource with legacy (enforce) rules.

### Transform

User can scale, offset and rotate model directly on import.

### Visual

#### Remove LODs

Removes LODs, from the lowest.

#### No Optimize

Skips topology optimalization.

#### Material Assigns

Materials can be reassigned to different ones per platform.

### Physics

#### Merge Tri Meshes

Imported will merge all trimeshes to least amount possible.

#### Phys Material Assigns

Obsolete, should not be present.

#### Geometry Params

Layer Preset, Surface Properties, Mass and Margin can be set for each collider.

#### COM Autocenter

Autocenters the center of mass (to the volume center of the object).
