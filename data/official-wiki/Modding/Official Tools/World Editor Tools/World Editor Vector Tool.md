# [World Editor: Vector Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Vector_Tool)

## Definitions

* **Shape**: a **Polyline** or a **Spline**, defined by points - the shape can be closed or not
* **Polyline**: a group of points linked by straight lines
* **Spline**: a group of points drawing a [Bézier curve](https://en.wikipedia.org/wiki/Bézier_curve)

### Details

**Polylines** and **Splines** share the following properties:

**Coords**: position of the first-placed point

**Angle X**: rotation of the whole shape on the X-axis from the first-placed point

**Angle Y**: rotation of the whole shape on the Y-axis from the first-placed point

**Angle Z**: rotation of the whole shape on the Z-axis from the first-placed point

**Scale**: shape scale, e.g 0.5 being twice as small, 2 being twice as big

**Is Closed**: defines if the shape is closed or left open

**Line Color**: defines the line's colour in World Editor

## Creation

### New Shape

**New Polyline** / **New Spline** is used to create a new shape.

As reminded in the Vector Tool interface,

`Ctrl` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") adds a point **after** the selected one, whereas

`Alt` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") adds a point **before** the selected one.

### Terrain Snapping

#### Snap to terrain

Checkbox: sets if newly-placed points are to snap to the terrain or to world's zero.

#### Snap to anchors

Checkbox: sets if dragged points get secured by neighbour points. **Anchor snap distance** value must be set to a value above zero.

#### Snap to terrain

Snaps selected points to terrain below them.

#### Reset elevation

Resets elevation of selected points to world's zero.

### Point management

#### Split

This "cuts" the shape on the selected point and duplicates it: one for the "previous" shape, one for the "next" shape.

#### Merge

This merges the shapes of selected points that are part of different shapes.

#### Reverse

Reverses all the points of the selected shape.

#### Delete

Deletes selected point(s).

#### Subdivide

Adds an additional point mid-distance between the currently selected point and the next one, or the first one if the last is selected.

#### Select all

Selects all points of the shape.

#### Anchor snap distance

This defines the distance from which a dragged point will snap to a neighbouring one. This can be useful for e.g road connections.

#### Tangent mode

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated.

Defines the tangent's behaviour (for a Spline)
Possible values:

* Mirror
* Same orientation
* Free

#### Gizmo mode

Defines if mouse drag and drop can move on all three axis or only flat 2D (X-Z).

#### Clone point data

Checkbox: when inserting a point with `Ctrl` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") or `Alt` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button"), clones the highlighted point's data into the newly created point.

#### Only active layer

Selects the entities from within the active layer only.

#### Select entities in shapes

Selects all entities within the shape(s) surface.

#### Exclude children

When ticked, **Select entities...** actions will not include children (useful to e.g avoid trees from a Forest Generator)

#### Select entities near shapes

Expands the search surface - **Select near distance** must be defined.

#### Select near distance

Sets the **Select entities near shapes** additional distance, in meters.

#### Occlude shapes by geometry

Hides shapes behind terrain/objects.

#### Show far shapes

Shows distant shapes in the editor, however great their distance is from the camera.

## Generators

A right-click on a Polyline's or Spline's ShapeEntity offers options allowing to create and use [Arma Reforger/Modding/Official Tools/World Editor Generators](/wiki/Category:Arma_Reforger/Modding/Official_Tools/World_Editor_Generators "Category:Arma Reforger/Modding/Official Tools/World Editor Generators").
