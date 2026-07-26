# [World Editor: Parallel Shape Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Parallel_Shape_Tool)

The **Parallel Shape Tool** is a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") tool that allows automatic creation of parallel shapes (polylines or splines) from a selected shape.

## Main Interface

[![](/wikidata/images/thumb/6/64/Parallel_Shape_Tool_UI.png/400px-Parallel_Shape_Tool_UI.png)](/wiki/File:Parallel_Shape_Tool_UI.png)

Parallel Shape Tool Interface

The tool consists of three main sections: [Margins](#Margins), [Debug](#Debug) and [Shape Creation](#Shape_Creation).

### Margins

#### Offset Safety Margin

[m] Offset safety for neater inner curves (Spline only)

* **Default value:** 1.00 m
* **Range:** 0 to 10 m
* **Usage:** Prevents overlapping in tight curves

#### Min Distance Between Points

Minimum distance between points, as to not saturate inner curves (Spline only)

* **Default value:** 1.00 m
* **Range:** 0 to 20 m
* **Usage:** Reduces point density in complex areas

### Debug

#### Show Debug Log

Show more logs in the console for debugging purposes

* **Default:** Disabled

### Shape Creation

#### Snap To Ground

Snap the newly created shapes' points to the ground

* **Default:** Enabled
* **Usage:** Ensures shapes follow terrain relief

#### Use Spline If Possible

Use spline if shape is a spline, may help in sharp turns; generally not needed, polyline should be enough

* **Default:** Disabled
* **Usage:** May help with tight curves

#### Create Shape As Child

Create shapes as the selected shape(s)' children; otherwise, create them on the same level

* **Default:** Enabled
* **Advantage:** Clear hierarchical organization

#### Symmetrical Shapes

Create shapes on the provided offsets and their negative, bordering the selected shape(s)

* **Default:** Enabled
* **Example:** A 5m offset will create shapes at +5m AND -5m

#### Remove Duplicates

Cleans up the duplicated offsets to only keep unique ones - also applies to Symmetrical Shapes above

* **Default:** Enabled

#### Offsets

New shapes' offset(s) - negative means "to the left" of the shape, positive means "to the right" (from its first to its last point)

* **Negative value:** "Left" of the shape (first to last point)
* **Positive value:** "Right" of the shape (first to last point)
* **Range:** -50 to +50 m
* **Default values:** -10.0, 10.0

## Usage

### Create Parallel Shapes

1. Select one or more shapes (polyline or spline) in the World Editor
2. Configure parameters according to your needs:
   * Set offsets in **Offsets** (e.g., 3, 6 to create two shapes at 3m and 6m)
   * Enable **Symmetrical Shapes** to also create shapes on the other side
   * Adjust **Offset Safety Margin** if needed for curves
3. Click the **Create** button
4. New shapes are created accordingly.

### Offset Direction

The left/right orientation is determined by the **shape's direction**:

* From the **first point** to the **last point** of the shape
* **Negative** (-) = left side
* **Positive** (+) = right side

ⓘ

To reverse the direction, reverse the order of the original shape's points.

## Practical Use Cases

### Road with Shoulders

Configuration:

* Offsets: 3, 6
* Symmetrical Shapes: Enabled
* Snap To Ground: Enabled

Result: 4 shapes created

* -6m (left ditch)
* -3m (left edge)
* +3m (right edge)
* +6m (right ditch)

### River with Banks

Configuration:

* Offsets: 2, 5
* Symmetrical Shapes: Enabled
* Offset Safety Margin: 0.5
* Min Distance Between Points: 0.5

Result: Banks that follow the river's meanders

### Simple Parallel Wall

Configuration:

* Offsets: 0.5
* Symmetrical Shapes: Disabled
* Create Shape As Child: Enabled

Result: Single wall 0.5m to the right of the original shape

## Console Messages

The tool displays messages in the Workbench console:

| Message | Meaning |
| --- | --- |
| No entities selected | No entity selected - select a shape |
| The selected entity is not a shape | The selected entity is not a polyline/spline |
| The selected shape requires at least two points | The shape must have at least 2 points |
| The number of created points (...) is not enough | Too many points removed - adjust margins |
| Cannot create shape | Creation error - check logs |

## Limitations

* Requires at least **2 points** per source shape
* Closed shapes are supported but may require adjustments
* Curves with tight turns may require **Offset Safety Margin** adjustment
* Performance may decrease with a large number of points

## Tips

* For roads, use **Offsets: 3** with **Symmetrical Shapes** to instantly create both edges
* Increase **Min Distance Between Points** if created shapes have too many points in curves
* Disable **Snap To Ground** to create aerial structures (bridges, aqueducts)
* Use **Remove Duplicates** to automatically clean up your offset lists
* For complex shapes with tight curves, increase **Offset Safety Margin** to 1.5-2.0m
