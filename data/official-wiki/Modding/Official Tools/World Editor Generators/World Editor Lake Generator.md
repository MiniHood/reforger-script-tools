# [World Editor: Lake Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Lake_Generator)

A **Lake Generator** is a generator drawing a lake defined by a Shape.

Lake generators can be found in the Resource Browser in ArmaReforger > Prefabs > WEGenerators > Water > Lake and are prefixed with LG\_ for **L**ake Generator.

## Requirements

* A Shape (polyline, spline) defining the lake's perimeter (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool"))
  + Said shape can be either a polyline or a spline, but the spline will be considered a polyline (and not consider its curves, only its points)
  + All shapes are considered "closed"
  + At least three points are required
  + It is recommended to not "cross" the shape's line unless having visual glitches (upside-down lake surface) is the wanted effect
* A lake generator entity as a **direct child** to this shape.

## Options

### Unsorted

#### Material Name

Water material

#### Surface Material Name

Water surface material

#### Physics Layer

Interaction layer of this water body

#### Reverse Point Order

Reverse winding order of the generated mesh - used if points are not in clockwise order

#### Flatten By Bottom Plane

Place the lake's surface at the lowest point's altitude; otherwise the highest

#### Geometry As OBB

Generate physics geometry as an OBB (Object Bounding Box, a "brick") instead of exact polyline shape

#### Min Depth

Min depth of lake for physics geometry

#### Water Surface Offset

Water surface offset for physics geometry

#### Shore Wetness

Enable shore wetness

#### Is Lake

Identify the purpose of this water body (e.g a water-filled sink is not a lake)
