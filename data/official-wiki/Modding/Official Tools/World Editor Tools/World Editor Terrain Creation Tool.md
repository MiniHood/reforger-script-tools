# [World Editor: Terrain Creation Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Terrain_Creation_Tool)

⚠

Before anything: **save frequently**. After height map importation, normal maps, terrain texture and layer maps import/generation as they are **not** saved to file after the operation.

ⓘ

For more advices and tips, see [Terrain Tutorial](/wiki/Arma_Reforger:Terrain_Tutorial "Arma Reforger:Terrain Tutorial").

## Manage

### Height Map

The height map is terrain altitude's information. World Editor can work with both .asc and 16-bit .png height map files.
The section offers options to import, export, and rebuild the height map.

#### Import Height Map

This option allows to import a height map from an external source or a World Editor export.

It allows to **invert in X and Z axis**, importing the terrain "from right to left" (a hill being West of the terrain would be East) or "from bottom to top" (South would be North).
Ticking both corresponds to a 180° terrain rotation.

The **resampling heights** option allows to override the terrain's altitude range by linear interpolation. It is required for png import as the png format only holds *relative*

Example

|  |  |  |  |  |  |  |
| --- | --- | --- | --- | --- | --- | --- |
| Original (-10 to +10m range) | -10 | -5 | 0 | 1 | 5 | 10 |
| Imported (0 to +100m range) | 0 | 25 | 50 | 55 | 75 | 100 |

#### Export Height Map

The **Base**/**Modified** option targets the terrain's height map - either the base terrain one, or the one modified by e.g road generators and other entities that edit terrain through script.

#### Rebuild Height Map

This button allows to rebuild the height map manually in the event it is not synchronised properly with the actual data (road, building placements).

#### Bake Selection

This "bakes" the selected entities' terrain modification into the base height map.

For example: if a building is placed on a slope, it flattens the terrain by default; this flattening follows the building and is destroyed with the building.
With this button the building's flattening is stored into the height map and will remain even if that building is moved or deleted.

#### Generate Normal Map

Allows to (re)generate the terrain's normal map. Normal map interpolation settings (minimum and maximum angle for vertex interpolation) are only available when the Interpolation Mode option is set to Auto.

### Satellite Map

#### Import satellite map

This imports a satellite map for the terrain, used for long-distance terrain texture.

### Surface Map

#### Fix block border

This is to be used if hard edges between textures of neighbouring terrain blocks appear.

#### Change surface map size

#### Rebuild terrain materials

**Rebuild terrain materials** must be run if a layer map reimport goes wrong - in the event of wrong or missing clutter or when terrain material tracing does not work properly.

### Options

### Tile Map

The Tile Map is an overhead map of the terrain tiles.

#### Minimap Controls

Hold ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") to drag the map around.

##### Scale To Fit

Fit the minimap to the view widget.

##### Center on Camera

Move the minimap so the camera position is centered.

##### Claim Tiles

Lock tiles so only your account can work on them.

##### Release Tiles

Release tiles so everyone can work on them.

##### Cogwheel

Set Background Image

Set the minimap's texture.

ⓘ

See also [2D Map Creation](/wiki/Arma_Reforger:2D_Map_Creation "Arma Reforger:2D Map Creation").

Clear Background Image

Remove the minimap's texture.

## Sculpt

### Common Values

**Strength**: strength of the effect - range 0..500

**Height** (in **Flatten** mode): wanted terrain's absolute height

**Radius**: radius (in metres) range 0..200

**Falloff**: the "cone/cylinder" slope ratio (see below) - range 0..100

**Angle**: the effect angle - range 0..360

### Common Controls

| Controls | Effect |
| --- | --- |
| `⇧ Shift` + `Mouse up`  `⇧ Shift` + `Mouse down` | Set strength/height |
| `Ctrl` + `Mouse up`  `Ctrl` + `Mouse down` | Set radius |
| `Ctrl` + `⇧ Shift` + `Mouse up`  `Ctrl` + `⇧ Shift` + `Mouse down` | Set falloff |
| `Ctrl` + `⇧ Shift` + `Mousewheel` | Set angle |

* [![Linear](/wikidata/images/thumb/5/54/armareforger-world_editor_terrain_tool_brush_linear.png/375px-armareforger-world_editor_terrain_tool_brush_linear.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_linear.png "Linear")

  Linear
* [![Smooth](/wikidata/images/thumb/c/c1/armareforger-world_editor_terrain_tool_brush_smooth.png/380px-armareforger-world_editor_terrain_tool_brush_smooth.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_smooth.png "Smooth")

  Smooth
* [![Sphere](/wikidata/images/thumb/f/f2/armareforger-world_editor_terrain_tool_brush_sphere.png/398px-armareforger-world_editor_terrain_tool_brush_sphere.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_sphere.png "Sphere")

  Sphere
* [![Inv-Sphere](/wikidata/images/thumb/c/cc/armareforger-world_editor_terrain_tool_brush_inverted_sphere.png/395px-armareforger-world_editor_terrain_tool_brush_inverted_sphere.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_inverted_sphere.png "Inv-Sphere")

  Inv-Sphere

* [![Round](/wikidata/images/thumb/6/68/armareforger-world_editor_terrain_tool_brush_round.png/375px-armareforger-world_editor_terrain_tool_brush_round.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_round.png "Round")

  Round
* [![Rect](/wikidata/images/thumb/6/61/armareforger-world_editor_terrain_tool_brush_rect.png/484px-armareforger-world_editor_terrain_tool_brush_rect.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_rect.png "Rect")

  Rect
* [![User shown shape is identical to Rect Falloff does not impact the result Looks into the Data\Brushes directory](/wikidata/images/thumb/6/61/armareforger-world_editor_terrain_tool_brush_rect.png/484px-armareforger-world_editor_terrain_tool_brush_rect.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_rect.png "User shown shape is identical to Rect Falloff does not impact the result Looks into the Data\Brushes directory")

  User
  + shown shape is identical to **Rect**
  + Falloff does not impact the result
  + Looks into the Data\Brushes directory

* [![Falloff (small)](/wikidata/images/thumb/d/dd/armareforger-world_editor_terrain_tool_brush_falloff_small.png/420px-armareforger-world_editor_terrain_tool_brush_falloff_small.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_falloff_small.png "Falloff (small)")

  Falloff (small)
* [![Falloff (medium)](/wikidata/images/thumb/a/a4/armareforger-world_editor_terrain_tool_brush_falloff_medium.png/373px-armareforger-world_editor_terrain_tool_brush_falloff_medium.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_falloff_medium.png "Falloff (medium)")

  Falloff (medium)
* [![Falloff (large)](/wikidata/images/thumb/c/cf/armareforger-world_editor_terrain_tool_brush_falloff_large.png/346px-armareforger-world_editor_terrain_tool_brush_falloff_large.png)](/wiki/File:armareforger-world_editor_terrain_tool_brush_falloff_large.png "Falloff (large)")

  Falloff (large)

### Sculpt

![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") raises terrain according to Strength.

`Alt` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") lowers the terrain.

### Flatten

Sets the terrain to a defined Height. **Space** picks the height of the current terrain position under the cursor.

### Smooth

Softens edges.

### Noise

Generates Perlin-based terrain noise.

## Paint

A specific terrain tool used for painting surface materials on terrain ground.

* Paint tool is used for painting surface materials on a terrain ground with an adjustable brush. All paint brush properties can be set in the same tab.
* Information where each surface material is painted is stored as a mask inside game files upon saving a project. However, it is possible to both import and export each surface's mask in a .png file.
* As of Arma Reforger v0.9.5 there is a limit of 5 surfaces per block for a terrain.

| Feature | Screenshot | Description |
| --- | --- | --- |
| Basic Brush Settings | [armareforger-world editor terrain tool paint basic brush settings.png](/wiki/File:armareforger-world_editor_terrain_tool_paint_basic_brush_settings.png) | **Show diagnostics for block under cursor** – this option enables a simple surface layer list per block tab in World Editor viewport when using Paint tool and hovering over terrain. Information about block under cursor will be shown. Useful as an overview to manage surfaces in multiple blocks efficiently while painting.  ⓘ  See the [Sculpt](#Sculpt) section above for more information. Strength range: 0..40  Strength of the brush. `⇧ Shift` + `MouseUp`/`⇧ Shift` + `MouseDown` to adjust. Radius range: 0..200  Radius of the brush. `Ctrl` + `MouseUp`/`Ctrl` + `MouseDown` to adjust. Falloff range: 0..100  Fadeout of the brush. `Ctrl` + `⇧ Shift` + `MouseUp`/`Ctrl` + `⇧ Shift` + `MouseDown` to adjust. Angle range: 0..360  Rotation of the brush – useful for rectangular or user brushes. |
| Brush Shape and Type | [armareforger-world editor terrain tool paint brush shape and type.png](/wiki/File:armareforger-world_editor_terrain_tool_paint_brush_shape_and_type.png) | * **Brush shape** – Linear/Smooth/Sphere/Inverted Sphere – these options define the style of painting, icons next to these tabs work as an indication what each option provides. * **Brush type** – Round/Rectangle/User. User brushes are loaded from root/Brushes (i.e. root/Brushes/blot.png). |
| Surface Layer List | [armareforger-world editor terrain tool paint surface layer list.png](/wiki/File:armareforger-world_editor_terrain_tool_paint_surface_layer_list.png) | * Surface layer list – A simple drag & drop list of surface materials. These can be dropped from either Workbench or World Editor's Resource Browser. * The very first surface layer is always used as default and while it can not be removed or imported, it can be changed to a different one.   In this scenario, Grass\_03 is used as a default surface. When masks are stored in game files, they are all merged together. Any information that is remaining from the resulting merged mask is used as default layer. |
| Basic Painting | 🚧  **[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated. |  |
| Extra Surface Layer Operations | [armareforger-world editor terrain tool paint extra surface layer operations.png](/wiki/File:armareforger-world_editor_terrain_tool_paint_extra_surface_layer_operations.png)  [armareforger-world editor terrain tool paint extra surface layer operations path.png](/wiki/File:armareforger-world_editor_terrain_tool_paint_extra_surface_layer_operations_path.png) | * **Change layer's material...** – Change surface material to a one not included in the list. * **Clear surface layer** – selected surface layer's mask will be cleared with default surface layer. * **Fill surface layer (clear other)** – selected surface layer will fill the entire terrain ground, replacing every other surface layer. * **Remove surface layer** – removes surface layer from the list. * **Priority surface mask import** – will automatically import mask of a selected surface layer. * **Priority surface mask import...** – will import a specific surface layer mask. * **Export surface mask** – will automatically export mask of a selected surface layer. * **Export surface mask...** – will export a specific surface layer mask.  * A simple batch import and export feature for surface masks is also available – these options work with masks from all surfaces at once.   *All surface masks are in a .png format and for the best results a predetermined sizing should be followed – this information can be found in the [Info & Diags](#Info_&_Diags) tab (section below).* |

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.7 "Category:Arma Reforger/Version 1.7") [1.7](/wiki?title=Category:Arma_Reforger/Version_1.7&action=edit&redlink=1 "Category:Arma Reforger/Version 1.7 (page does not exist)")

## Holes

Holes are a way to **hide** the terrain. One can create a hole or disable a block for that.

### Options

* **Show terrain in holes and disabled blocks:** the terrain in holes and disabled blocks will appear
* **Show holes diag:** highlight the holes
* **Show disabled blocks diag:** highlight the disabled blocks

### Add/Remove Hole

Create holes in blocks.

### Enable/Disable Block

Disable blocks.

## Info & Diags

This tab contains useful info about blocks and their surface layers. These diagnostics also allow either manual merging or replacing of surface layers in specific blocks.

| Step | Screenshot | Description |
| --- | --- | --- |
| Block and Pixel Under Cursor Diagnostics | [armareforger-world editor terrain tool info n diags block and pixel under cursor diagnostics.png](/wiki/File:armareforger-world_editor_terrain_tool_info_n_diags_block_and_pixel_under_cursor_diagnostics.png) | * In the lower part of the **Info & Diags** tab, there is an extra information about blocks containing painted surface layers. * While in **Info & Diags** tab, you can press Ctrl+X in World Editor's viewport to visualize surface layers painted on the terrain ground. Diagnostic colors are set in each surface layer.  Block  * **Live (under cursor)** – diagnostics will describe the block actually under cursor in World Editor's viewport. If the tile is clicked then this checkbox becomes unticked. * **Camera auto-align** – when a block is selected, camera will automatically align to it. * **Coordinates** – it is possible to input specific tile coordinates and have camera automatically moved to their location. Below is a list of specific pixel coordinates of height map, satellite texture and surface mask. As each has a different size, the coordinates are different. Listed coordinates are Top-left coordinates/Bottom-right coordinates. * Percentage list determines the mask coverage of a surface layer in a selected block. These percentages work with surface layer mask blending as well, meaning that the total value can be up to 500%.  Pixel Under Cursor  * **Coordinates** – same as above. * The Pixel Surface statistics list shows surface layers per pixel. The range is set from 0 to 255 (black - white) and always add up to **255**. It is possible to determine exactly which pixel contains what surface layers through this data. |
| Basic Surface Diagnostics | [armareforger-world editor terrain tool info n diags basic surface diagnostics.png](/wiki/File:armareforger-world_editor_terrain_tool_info_n_diags_basic_surface_diagnostics.png) | * **Merge** – merges selected surface layer with already existing one. * **Replace** – replaces selected surface layer with a layer not present in selected block.   + A 3x3 grid is an indicator of selected block and its neighbours. The orientation is based on cardinal directions.     - **Green** – there is a free slot for that surface layer.     - **Yellow** – selected block.     - **Red** – the limit of 5 surface layers is reached in that block. |
| Surface Layer Merge or Replace Operations | 🚧  **[TODO](/wiki/Category:To-do "Category:To-do"):** Add graphic element? | 1. Select a block in which you want a surface layer merged or replace. 2. Select specific surface in the percentage list by right-clicking on it. 3. Pick between either *Merge* or *Replace* options. 4. If *Replace* is further expanded, pick a surface layer which you want to replace selected one. 5. If there is a red square in the 3x3 grid, the 5 surface layer limit is reached in that block and it needs to be processed by further merging surface layers there to empty the slot. |
