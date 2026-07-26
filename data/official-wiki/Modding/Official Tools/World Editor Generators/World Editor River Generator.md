# [World Editor: River Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_River_Generator)

A **River Generator** is a generator drawing a river defined by a Shape.

River generators can be found in the Resource Browser in ArmaReforger > Prefabs > WEGenerators > Water > River and are prefixed with R\_ for **R**iver Generator (the RG\_ prefix being already used by [Road Generator](/wiki/Arma_Reforger:World_Editor:_Road_Generator "Arma Reforger:World Editor: Road Generator")).

## Requirements

* A Shape (polyline, spline) defining the river's path (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool"))
  + Said shape can be either a polyline or a spline, but the polyline cannot be closed
  + At least two points are required
* A river generator entity as a **direct child** to this shape.

## Options

### River

#### Width

River's base width

ⓘ

Note that this width is the *possible* water surface width; the *visible* water surface width will be less due to partial hiding by terrain banks.

#### Spline Offset Up

Vertical offset of the generated mesh from the spline

#### Reverse Flow

Invert texture coordinates to make the river flow the other direction

#### Material

The material used for the river's surface

#### Clearance

River's clearance ("empty space around it" for e.g Forest Generator to not plant trees there)

#### Shore Wetness

Enable shore wetness

### Physics

#### Surface

Water surface game material

#### Physics Layer

Interaction layer of the river water body

#### Geometry As OBB

Generate physics geometry as an OBB (Object Bounding Box, a "brick") instead of exact polyline shape

#### Min Depth

Min depth of river for physics geometry

#### Water Offset

Water surface offset for physics geometry
