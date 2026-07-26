# [World Editor: Prefab Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Prefab_Generator)

A **Prefab Generator** is a generator creating entities on a line defined by a Shape.

Prefab generators can be found in the Resource Browser in ArmaReforger > Prefabs > WEGenerators (mostly in the Bushline directory), and are prefixed with PG\_ for **P**refab **G**enerator
(thus conflicting with [**P**ower line **G**enerator](/wiki/Arma_Reforger:World_Editor:_Power_Line_Generator "Arma Reforger:World Editor: Power Line Generator")).

ⓘ

To generate entities more regularly, see [Wall Generator](/wiki/Arma_Reforger:World_Editor:_Wall_Generator "Arma Reforger:World Editor: Wall Generator").

## Requirements

* A Shape (polyline, spline) defining the wall's's trajectory (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool"))
  + At least two points are required
* A prefab generator entity as a **direct child** to this shape.

## Options

### Unsorted

#### Prefab Names

Prefab list

#### Weights

Prefab weights - the higher the more chance for the Prefab to spawn

#### Only To Vertices

If checked prefabs are placed only to vertices

#### Distance

Distance between spawned assets along the spline/polyline

#### Align With Shape

If checked prefabs are aligned with the shape

#### Use X As Forward

Only when aligning with shape

#### Flip Forward

Flip forward

#### Offset Right

How much we offset from center to the side

#### Offset Variance

How much offset variability we want in meters when using offset from the center

#### Gap

Gap from the centerline

#### Random Spacing

Random Spacing Offset

#### Offset Up

How much we offset in world up vector

#### Offset Forward

Forward offset

#### Perlin Dens

Prefab spawn density uses Perlin distribution

#### Perlin Threshold

Spawns prefabs if perlin value is above this threshold

#### Perlin Asset Distribution

Prefab spawn type uses Perlin distribution. Prefabs in the prefab array are stacked up from lower to higher indicies on the Y axis

#### Perlin Size

Prefab size uses Perlin

#### Perlin Frequency

Perlin frequency, higher values mean smoother transitions

#### Perlin Seed

Perlin seed

#### Perlin Amplitude

Perlin Amplitude

#### Perlin Offset

Perlin Phase Offset

#### Perlin Throw Away

Disables asset generation below perlin threshold

#### Draw Debug

Draw developer debug
