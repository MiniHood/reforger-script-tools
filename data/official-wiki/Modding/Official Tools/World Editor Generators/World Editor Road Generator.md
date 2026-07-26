# [World Editor: Road Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Road_Generator)

A **Road Generator** is a generator creating a road along a spline. It can also shape the terrain (extrude or dig) around it to make it follow the spline.

## Requirements

* A spline defining the road's path - open or closed (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool")).

  ⚠

  Polylines are **not** supported.
* A road generator entity as a **direct child** to this shape.
* A road entity as a direct child to this generator.

The road generator entity defines the shape's behaviour (terrain sculpting, road's width etc).  
The road entity defines the road surface (material, etc).

## Prefabs

Road Generator Prefabs can be found in Data\Prefabs\WEGenerators\Roads.

## Road Generator Options

### Terrain

#### Adjust Height Map

If enabled, adjust terrain height map to road

#### Adjust Height Map Priority

Priority of terrain heightmap adjust

#### Falloff Start Width

Distance between road edge and start of fall-off

#### Road Falloff Width

Width of the road fall-off

### Road

#### GenerateRoad

If enabled, regenerate road on shape modification

#### Road Clearance

Width of the clearance by the road

#### Road Width

Width of the road

## Road Options

### Unsorted

#### Spline Points

Spline points used for road generation, synchronised by the Road Generator

#### Is Closed Spline

Is the road a loop, **not** synchronised by the Road Generator

#### Type

Used by map export

#### Width

Road width

#### V Scale

Texture v-coordinate scaling (0.5 = texture stretched to 200% its normal length, 2.0 = 50%, etc)

#### Beginning Piece

Beginning variant

#### Ending Piece

Ending variant

#### Random Seed

Used to randomise road's aspect

#### Force Full Length

Ensures that the road mesh is generated along the whole spline. It may be achieved by stretching the v-coordinate (if both end pieces are set), or "cutting" it at any value (otherwise)

#### Mid Piece Sequence

For materials with multiple middle piece versions: specifies how middle piece sequence should be generated

#### Road Sort

Road's render order within the same material: does not override material's render order!

#### Mat Sort Bias

Bias to be added to the material's sort bias - can be used to override that order

⚠

Avoid using this as much as possible.
