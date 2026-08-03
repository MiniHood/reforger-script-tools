# [World Editor: Shape Area Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Shape_Area_Tool)

The **Shape Area Tool** is a World Editor tool for Enfusion that turns polylines and splines into specific geometric shapes (circles, rectangles, stars) or creates these shapes directly in the middle of the screen.

ⓘ

* To make a "diamond" (rhombus), use Circle and set the number of points to 4
* Points are created **clockwise**.

## Main Interface

[![](/wikidata/images/thumb/4/4c/Shape_Area_Tool_UI.png/400px-Shape_Area_Tool_UI.png)](/wiki/File:Shape_Area_Tool_UI.png)

Shape Area Tool Interface

The tool consists of five main sections: [Global](#Global), [Circle](#Circle), [Rectangle](#Rectangle), [Star](#Star) and [Points](#Points).

### Global

#### Shape Type

Shape type to be created

* **Circle:** Can be used to create polygons
* **Rectangle:** Rectangle shape
* **Star:** Star shape
* **Default:** Circle

#### Width

Wanted shape's width in meters

* **Default value:** 48.000 m
* **Range:** 0.001 to infinity

#### Length

Wanted shape's length in meters

* **Default value:** 32.000 m
* **Range:** 0.001 to infinity

### Circle

#### Circle Segments Count

Number of segments (points) in the circle

* **Default value:** 12
* **Minimum:** 3
* **Recommendation:** A minimum of 12 points is recommended for a round circle
* **Special usage**:
  + 3 points = triangle
  + 4 points = diamond/rhombus
  + 6 points = hexagon
  + 8 points = octagon

### Rectangle

#### Rectangle Segments Per Side

Number of segments per side

* **Default value:** 1
* **Minimum:** 1
* **Usage:** Increase to subdivide sides (useful for terrain following)

### Star

#### Star Branches Count

Number of branches in the star

* **Default value:** 5
* **Minimum:** 2

#### Star Inner Radius Ratio

Star inner radius ratio - e.g 0.75 \* width-length radius

* **Default value:** 0.500
* **Range:** 0.01 to 1
* **Usage:** Controls the "depth" of star branches

### Points

#### Snap To Terrain

Snap shape points to terrain

* **Default:** Enabled
* **Usage:** Ensures the shape follows terrain relief

#### Centre On Position

Whether or not points should be around the shape entity's origin or if the points should be in positive position only (shape origin being bottom-left)

* **Default:** Enabled (centered)
* **Usage:** Disable to place origin at bottom-left corner

#### Shape Closing

Define whether to close, open or leave as is the selected shape(s)

* **Leave as is:** Keep current state
* **Open:** Open the shape
* **Close:** Close the shape
* **Default:** Leave as is

## Usage

### Create a New Shape at Screen Center

1. Configure shape parameters:
   * Choose the **Shape Type**
   * Set **Width** and **Length** ("Height")
   * Adjust specific parameters (segments, branches, etc.)
2. Center the camera on the desired position
3. Click on:
   * **Create Polyline** for a polyline
   * **Create Spline** for a spline
4. The shape is created at the center of the screen

### Convert Existing Shapes

1. Select one or more shapes (polylines or splines) in the World Editor
2. Configure conversion parameters
3. Click on **Convert Sel. Shapes**
4. Selected shapes are transformed according to defined parameters

## Practical Use Cases

### Circular Plaza or Roundabout

Configuration:

* Shape Type: Circle
* Width: 50
* Length: 50
* Circle Segments Count: 16
* Snap To Terrain: Enabled
* Centre On Position: Enabled

Action: Create Polyline  
Result: 50m diameter circular plaza perfectly adapted to terrain

### Diamond / Rhombus

Configuration:

* Shape Type: Circle
* Width: 20
* Length: 20
* Circle Segments Count: 4
* Centre On Position: Enabled

Action: Create Polyline  
Result: 20m × 20m diamond shape

### Decorative Star

Configuration:

* Shape Type: Star
* Width: 30
* Length: 30
* Star Branches Count: 8
* Star Inner Radius Ratio: 0.4

Action: Create Spline  
Result: 8-branch star with pronounced branches

### Hexagon

Configuration:

* Shape Type: Circle
* Circle Segments Count: **6**
* Width: 25
* Length: 25

Action: Create Polyline  
Result: Regular 25m hexagon

## Console Messages

The tool displays messages in the Workbench console:

| Message | Meaning |
| --- | --- |
| No shapes were selected, no conversion needed | No shape selected for conversion |
| Cannot modify shape(s), points calculation went wrong | Error in point calculation |
| Cannot snap shape(s) to terrain, entity not found from source | Cannot project to terrain |

## Limitations

* Circle shapes require at least **3 points**
* Star requires at least **2 branches**
* Snap to terrain requires valid terrain under points

## Tips

* To create regular polygons, use Circle and adjust segment count (3 for triangle, 4 for diamond, 5 for pentagon, etc)
* For shapes following rough terrain perfectly, increase **Rectangle Segments Per Side**
* Use **Star Inner Radius Ratio** at 0.3-0.4 for very pronounced stars, 0.7-0.8 for softer stars
* Disable **Centre On Position** if you want to precisely place a corner of the shape
* To convert multiple shapes at once while keeping their positions, select them all and use **Convert Sel. Shapes**
* For organic paths (trails, rivers), create a circle with few points (6-8) for a less regular appearance
