# [World Editor: Power Line Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Power_Line_Generator)

A **Power Line Generator** is a generator creating a power line (cables and poles) along a shape.

## Requirements

* A Shape (polyline, spline) defining the power line's path (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool")).
* A power line generator entity as a **direct child** to this shape.

## Prefabs

Road Generator Prefabs can be found in Data\Prefabs\WEGenerators\Powerline.

## Options

### Pole Setup

There are four pole types:

* Start Pole - the pole type that starts it all
* Default Pole - the standard pole that will run the distance between Start Pole and End Pole
* End Pole - the pole that ends the power line
* Junction Pole - the pole that connects to another powerline

#### Distance

Defines the expected distance between poles.

#### Default Pole

Defines the default pole's entity.

#### Start Pole

Defines the start pole's entity.

#### End Pole

Defines the end pole's entity.

#### Default Junction Pole

Defines the junction pole's entity.

#### Rotate 180 Degree Yaw Start Pole

Rotates the start pole 180° to "face" the other poles.

#### Rotate 180 Degree Yaw End Pole

Rotates the end pole 180° to "face" the other poles.

#### Clearance

Defines the minimum distance from the polyline for other objects to be generated ("empty space" around poles line).

### Randomisation

#### Random Pitch Angle

Defines the angle randomization (in 0..180° range) of pitch - "rolling towards the back".

#### Random Pitch On Both Sides

Makes the Random Pitch Angle go both sides (forward and back), effectively doubling the range (-180..+180°)

#### Random Roll Angle

Defines the angle randomization (in 0..180° range) of roll - "rolling towards the right".

#### Random Roll On Both Sides

Makes the Random Roll Angle go both sides (left and right), effectively doubling the range (-180..+180°)

#### Apply Pitch And Roll Default

Applies the Pitch and Roll settings to the default pole.

#### Apply Pitch And Roll Start

Applies the Pitch and Roll settings to the start pole.

#### Apply Pitch And Roll End

Applies the Pitch and Roll settings to the end pole.

### Power Lines

#### Powerline Material

Defines the power line cable material to be used between poles.

### Debug

#### Draw Debug Shapes

Used to display debug shapes on generation.

### Unsorted

#### Color

Defines the line's color.
