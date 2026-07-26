# [2D Map Creation](https://community.bistudio.com/wiki/Arma_Reforger:2D_Map_Creation)

In order to display a 2D map when using the map item in-game, a [MapEntity.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/World/Game/MapEntity.et) must be placed.  
It is possible to use custom geometry and/or texture using the following fields:

* Map Geometry Data - this file contains road network, buildings, airfields etc information for them to be displayed on the 2D map
* Satellite Background Image - this file contains the map background texture: forests, shaded relief, etc.

  ⓘ

  This tutorial explains how to generate a **Terrain Rasterization Background Image** to be used as Satellite Background Image, but any other texture can be used (e.g a satellite map).

[![](/wikidata/images/thumb/d/d9/armareforger_2D-map-tutorial-generate.png/300px-armareforger_2D-map-tutorial-generate.png)](/wiki/File:armareforger_2D-map-tutorial-generate.png)

1. Export Map Data Tool button  
 2. Export Map Data Tool tab  
 3. Export button

## Generate Map Geometry Data

The Map Geometry Data field takes a topography data file (.topo extension).

### Generate TOPO

1. Run [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") with the wanted world
2. Locate **Export Map Data** tool in toolbar (marked as 1)
3. Click **Export Map Data** tab (marked as 2) to display the tool's panel if needed
4. Set the **Type** to Geometry2D
5. Set up the settings under the **Geometry 2D** category, such as color and scale
6. Click the **Export** button (marked as 3)

   ⚠

   Be patient, as the Geometry2D export operation **will** take time.

## Generate Terrain Rasterization Background Image

The Satellite Background Image field takes a .edds file.

### Generate TGA

1. Run [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") with the wanted world
2. Locate **Export Map Data** tool in toolbar (1)
3. Click **Export Map Data** tab (2) to display the tool's panel if needed
4. Set the **Type** to Rasterization
5. Set up the settings under the **Rasterization** category, such as color and scale
6. Click the **Export** button (3)

This will have generated the 4096×4096 upside-down *worldname*.tga file - it now needs to be converted to PNG in order to be properly imported in Enfusion to the .edds format.

### Convert to PNG

TGA can be converted with an image editor - here [Paint.NET](https://www.getpaint.net/download.html#download) and [GIMP](https://www.gimp.org/downloads/) will be used.

#### Paint.NET

* Open Paint.NET
* Open the TGA file `Ctrl` + `O`
* Flip it vertically: Image > Flip Vertical
* Use the Save As option `Ctrl` + `⇧ Shift` + `S`
* Save as PNG using the dialog's dropdown

#### GIMP

* Open GIMP
* Open the TGA file `Ctrl` + `O`
* Flip it vertically: Image > Transform > Flip Vertically
* Use the Export As option `Ctrl` + `⇧ Shift` + `E`
* Select PNG file type and click Export

### Import PNG

* Place the PNG file in the Data\UI\Textures\Map\worlds directory
* Navigate to the PNG file in [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager")
* Right-click > Register
* Go to the Import Settings tab, set Color Space to **ToSRGB**
* Click the **Import resource** button at the top of the tab to generate the .edds file.

## Setup

1. Find the world's MapEntity by using the Hierarchy tab's Search field - if it does not exist, create it by drag-and-dropping a [MapEntity.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/World/Game/MapEntity.et) Prefab into the world.
2. Fill **Map Geometry Data** with the .topo file generated previously
3. Fill **Satellite Background Image** with the .edds file generated previously
