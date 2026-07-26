# [Terrain: Terrain Entity](https://community.bistudio.com/wiki/Arma_Reforger:Terrain:_Terrain_Entity)

TerrainEntity, class GenericTerrainEntity, is responsible for the entire terrain system; rendering terrain mesh and its textures, collision, etc.

The terrain is split into blocks and tiles; TerrainEntity supports non-square shapes of terrain as well; it is possible to set different grid size for X and Z axes.

A world can have 0 to 1 terrain - multiple terrains (e.g in order to avoid terrain size limitation) is not supported.

ⓘ

Visualization of Blocks & Tiles can be turned on from [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") (holding `⊞ Win` + `Alt`), Render → Terrain menu → Block's/Tile's bounding boxes.

### Vertex

(Plural: **vertices**)

ⓘ

See [Vertex (geometry)](https://en.wikipedia.org/wiki/Vertex_(geometry)).

### Block

A **block** is the smallest part of the TerrainEntity which gets LODed, as opposed to the [Real Virtuality](/wiki/Real_Virtuality "Real Virtuality") engine where terrain LODing was done on tiles level.

1 block = 1 draw call

Even though surface mask (for rendering detail surfaces) is split and mapped per tile, a limit of maximum 5 detail surfaces is being applied to blocks.

Enfusion engine currently supports 5 LODs per block.

Each block consists of 32x32 faces (33x33 vertices) down to 2x2 faces (3x3 vertices)

### Tile

A **tile** may consist of 1 up to 32 blocks.

When the camera is close to a tile typically (if provided) super texture ("satellite texture"), normal map and surface mask (detail surfaces) are being drawn. These textures are mapped per tile.

### Terrain Grid Size

Count of terrain faces in rows/columns, remember that 512 faces is 513 vertices, this necessary to keep in mind when importing heightmaps.

### Grid Cell Size

Distance between two neighboring terrain vertices. The lower the value, the higher the detail of terrain.

Increasing/decreasing this value affects the final size of terrain.

### Terrain Size

The terrain's total size is **Terrain Grid Size** multiplied by **Grid Cell Size**.

### Heightmap

The terrain's heightmap is internally in 16 bits integer format and the height scale value is the difference between minimum delta, so e.g. difference between 100 and 101 in internal representation is 0.0315 as default.
This value shows the maximum possible height of terrain, 0.03125 × 65535 (the 16 bit value) = ~2048 meters range.

Changing this value affects elevation range (the vertical range of the terrain).

The lower the value, the higher the precision, but smaller vertical range (and vice-versa).
