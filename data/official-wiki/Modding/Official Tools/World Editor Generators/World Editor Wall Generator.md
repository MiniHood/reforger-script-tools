# [World Editor: Wall Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Wall_Generator)

A **Wall Generator** is a generator creating entities on a line defined by a Shape.

Wall generators can be found in the Resource Browser in ArmaReforger > Prefabs > WEGenerators (mostly in Walls and Pavements) and are prefixed with WG\_ for **W**all **G**enerator.

ⓘ

While a wall generator focuses on walls, it can actually generate any kind of entity defined in its properties
(can be used to spawn elements regularly, while [Prefab Generator](/wiki/Arma_Reforger:World_Editor:_Prefab_Generator "Arma Reforger:World Editor: Prefab Generator") may be more adapted for randomness e.g bush lines)

## Requirements

* A Shape (polyline, spline) of at least two points defining the wall's's trajectory (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool"))
* A wall generator entity as a **direct child** to this shape.

## Options

### Middle Object

#### Middle Object

Middle Object Prefab

#### Place Middle At Vertex

Place middle prefab at vertex

#### Place Middle At Vertex Only

Place middle prefab at vertex only

#### Middle Object Pre Padding

Middle object pre-padding, essentially a gap between previous prefab and this

#### Middle Object Offset Right

Middle object offset to the side

#### Middle Object Post Padding

Middle object post-padding, essentially a gap between this asset and the next one

#### Middle Object Offset Up

Middle object offset up/down

### First Object

#### First Object

First Object Prefab

#### First Object Pre Padding

First object pre-padding, essentially a gap between previous vertex first object

#### First Object Post Padding

First object post-padding, essentially a gap between first object and the following one

#### First Object Offset Right

First object offset to the side

#### First Object Offset Up

First object offset up/down

### Last Object

#### Last Object

Last Object Prefab

#### Last Object Pre Padding

Last object pre-padding, essentially a gap between previous vertex first object

#### Last Object Post Padding

Last object post-padding, essentially a gap between first object and the following one

#### Last Object Offset Right

Last object offset to the side

#### Last Object Offset Up

Last object offset up/down

### Global

#### Pre Padding

Global object pre-padding, essentially a gap between previous prefab and this

#### Post Padding

Allow pre-padding on first wall asset in each line segment

#### Overshoot

Allow overshooting the segment line by this amount when placing assets

#### Offset Right

Objects offset to the side

#### Offset Up

Object offset up/down

#### Wall Groups

Contains wall groups which group Wall/Weight/Pre Padding/Post Padding

### Other

#### Pre Pad First

Allow pre-padding on first wall asset in each line segment

#### Exact Placement

Copy the polyline precisely while sacrificing wall assets contact

#### Start From The End

Start the wall from the other end of the polyline

#### Use X As Forward

Rotate object so that its X axis is facing in the direction of the polyline segment

#### Rotate 180

Rotate object 180° around the Yaw axis

#### Use For Very Small Objects

If you want to generate objects smaller than 10 centimetres

#### Debug

Draw developer debug

#### Snap To Terrain

Whether or not walls should be snapped to the terrain
