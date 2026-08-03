# [World Editor: Terrain Preparation Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Terrain_Preparation_Tutorial)

## Setup

### World

In World Editor, click on **Create new World** to open the **Create world** interface.

In there, set **world type** to **Base scene** (the new world is to start 100% from scratch, **Sub-scene** would make an "additional layer" on an existing terrain).

### Terrain

In the **Create** tab, search "terrain" to drag and drop a **GenericTerrainEntity** in the world.

**Set its position to [0,0,0]** (terrain collision/contact and Terrain Tools may encounter issues otherwise).

**Save the terrain** (`Ctrl` + `S` or File > Save World).

Right-click on the created GenericTerrainEntity and choose **Create new terrain…** ; this will open the **New Terrain** interface, allowing to set terrain grid resolution values. When done, click **Create**.

*et voilà !* The newly created terrain is now ready to be sculpted.

## Additional Setup

### Camera

In the **Create** tab, search for "camera" to drag and drop a **CameraManager** in the world. This makes the camera attach to the played character as the in-game behaviour, making the preview easier.
Be sure to tick **Play from camera position**
to spawn a unit when pressing **Play**.

### Sun

In the **Create** tab, search for "light" to drag and drop a **GenericWorldLightEntity** in the world.

### Transformation

* **Angle X** - Set light source's pitch angle (e.g -20° to have a low-on-horizon sun)
* **Angle Y** - Set light source's yaw angle: 0 = from the South, 180 = from the North, 90 = from the West, 270 = from the East
* **Angle Z** - Set light source's roll angle (does nothing)

### Light color properties

* **Specular Mul** - Its a mutiplier of the Sun hotspot's specular intensity
* **Sun Angular Size** - the size of the Sun in a hotspot

### Global Light color properties

* **Direct Light LV** - Intensity of the Sun
* **Direct Light Color** - Color of the Sun light
* **Indirect Light LV** - Intensity of "ambient" light
* **Indirect Light Color** - Color of the "ambient" light

### Global Indirect light modificators

* **Probe Reflection EV** - Intensity of reflection at "LOW" roughness materials (chrom, glass etc.)
* **Probe Reflection Color** - Color modificator for Reflection
* **Probe Diffuse EV** - Intensity of diffuse light at "HI" roughness materials (wall, plastic cloth etc.)
* **Probe Diffuse Color** - Color modificator for Diffuse

### Skybox

#### Sky Preset

In world entity properties, set **Sky Preset** to **Atmosphere.emat** using the search bar to find it - this setups a "real world" atmosphere for the world.

ⓘ

The screen will turn white for a moment, this is normal as the HDR takes time to adapt to the new brightness.

⚠

Using a static skybox (e.g **HDRi\_01.emat**) is a quick possibility but does not allow for further configuration such as adding Clouds or Celestial Bodies.

#### Planet Preset

In world entity properties, add **Celestial Bodies** by adding items to the **Planet Preset** list (by clicking "+") and add the following:

* **Sun\_01.emat** - set a material to the skybox's light source
* **Moon\_01.emat**
* **Stars\_01.emat**

#### Clouds

In world entity properties, set **Clouds Renderer** to **SkyVolCloudsRenderer**.

Set **Clouds Preset** to **Clouds\_Volumetric.emat** - this setups the volumetric clouds.

### Ocean

In world entity properties,

Set the **Ocean Material** field to **ocean.emat** - this setups the ocean's surface.

Set the **Ocean Simulation** field to e.g **oceanSimIsland.emat** - this setups the ocean's simulation.

### Post-Process Effects

In the **Prefab Library**, search for "WorldP" to drag and drop [GenericWorldPP\_Default.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/World/DefaultWorld/GenericWorldPP_Default.et) in the world.

**Post-process Entities** are used for many effects:

Typical PP effects:

* SSR - Screen space reflection
* SSDO - Screen space diffuse occlusion
* GodRays - Fog effect (screen space)
* UnderWater - Effect for underwater scenes
* HBAO - Screen space Ambient Occlusion
* Rain Effect - Rain effects in the scene.
* PPAA - Antialiasing postprocess
* HeightmapAO - Voxel ambient occlusion used for trees and rocks
* HDR - Camera setting

Sorting order for basic PP:

* 0 = SSR
* 1 = Godrays
* 2 = UnderWater
* 3..13 = free for game usage
* 14 = SSDO
* 15 = HBAO
* 16 = HDR
* 17 = PPAA

### Weather

In the **Resource Browser**, search "WeatherM" to drag and drop a **TimeAndWeatherManager.et** entity in the world.

Set the weather by opening the **Weather Editor** by its icon in the main toolbar. Load the weather state machine config (**WeatherStates.conf**) and confirm by clicking OK.

⚠

Be sure to properly set geoposition!

|  | Latitude | Longitude |
| --- | --- | --- |
| + | North | East |
| - | South | West |

### Local Environment Probe

In the **Create** tab, search for "env" to drag and drop an **EnvironmentProbeEntity** in the world.

This is used to capture local reflection / diffuse.

### Suggested Default Prefabs

These are suggested default prefabs to use for a simplistic setup of your terrain. Use instead of the base entities above.

Base World (Terrain)

| Prefab | Notes |
| --- | --- |
| PreloadManager.et | Preloads prefabs that will be used often, such as, Character\_US\_AR.et |
| Lighting\_Default.et | Use in place of GenericWorldLightEntity. Contains default settings. |
| SCR\_CameraManager.et | Sets up camera system. Without it, things will break, such as 3rd person sprint camera. |
| MapEntity.et | Sets up map for players to use. Requires topography geometry data and satellite image. |
| GenericWorldPP\_Default.et | Use in place of GenericWorldPPEffect. Contains default settings. |
| FogHaze\_Default.et | Use in place of GenericWorldFogEntity. Contains default settings. |
| TimeAndWeatherManager.et | Modifies weather settings. |
| ProjectileSoundsManager.et | Sets up max audible distances for subsonic/supersonic projectiles. Sets up additional .acp files. |
| AmbientSounds\_Everon.et | Creates a bunch of ambient wildlife sounds. |
| RadioBroadcastManager.et | Manages radio music broadcasts. Contains music start times and DJ times. |
| ForestSyncManager.et | Manages replication for forests when forest destruction is enabled for MP. |
| DestructionManager.et | Manages destruction. |
| MPDestructionManager.et | Manages replication for destruction for MP. |
| EnvProbe\_Default.et | Use in place of EnvironmentProbeEntity. Contains default settings. |
| SoundWorld\_Base.et |  |
| MusicManager\_Base.et | Sets up cycle of background music. |

## Sculpting

Pick the **Terrain Tool** (mountains icon) in the toolbar.

Make sure to select the **Terrain Tool** tab.

### Heightmap Import

In the **Manage** tab, click **Import height map…** to import a png heightmap.

### First Editing

On heightmap's first editing, the **Set normal map options and generate normal map** appears; click on "OK" to apply its default settings, then confirm the following popup.

### Terrain Tool

On the top of the tab are four tabs:

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** Add links

* Manage
* Sculpt
* Paint
* Info & Diags

Use the **Sculpt** tools to extrude, dig, sculpt the ground. See its documentation to know the usage of each section.

Use the **Paint** tools to set surfaces.

ⓘ

To add roads, powerlines, rivers, see:

* [Forest Generator](/wiki/Arma_Reforger:World_Editor:_Forest_Generator "Arma Reforger:World Editor: Forest Generator")
* [Lake Generator](/wiki/Arma_Reforger:World_Editor:_Lake_Generator "Arma Reforger:World Editor: Lake Generator")
* [Powerline Generator](/wiki/Arma_Reforger:World_Editor:_Powerline_Generator "Arma Reforger:World Editor: Powerline Generator")
* [Prefab Generator](/wiki/Arma_Reforger:World_Editor:_Prefab_Generator "Arma Reforger:World Editor: Prefab Generator")
* [Road Generator](/wiki/Arma_Reforger:World_Editor:_Road_Generator "Arma Reforger:World Editor: Road Generator")
* [Wall Generator](/wiki/Arma_Reforger:World_Editor:_Wall_Generator "Arma Reforger:World Editor: Wall Generator")
