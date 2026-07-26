# [World Editor: Forest Generator](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Forest_Generator)

A **Forest Generator** is a generator creating entities in an area defined by a Shape.

Forest generators can be found in the Resource Browser in ArmaReforger > Prefabs > WEGenerators > Forest and are prefixed with FG\_ for **F**orest **G**enerator.

ⓘ

* While a forest generator focuses on forests, it can actually generate any kind of entity defined in its [Forest](#Forest) setting (can e.g be used for rocks on seabed, etc)
* There can be multiple Forest Generators per defined area.

## Requirements

* A Shape (polyline, spline) defining an area (see [Vector Tool](/wiki/Arma_Reforger:World_Editor:_Vector_Tool "Arma Reforger:World Editor: Vector Tool"))
  + Said area can be defined by either a polyline or a spline, but the spline will be considered a polyline (and not consider its curves, only its points)
  + All shapes are considered "closed"
  + At least three points are required
* A forest generator entity as a **direct child** to this shape.

## Options

### Obstacles

#### Avoid Objects

Avoid objects - the trace check is a 10cm cylinder (for trees mostly)

ⓘ

This setting is performance hungry for it does a trace for every tree to be created. For the best performances, uncheck it whenever possible.

#### Avoid Roads

Avoid roads, respecting their clearance setting

#### Avoid Rivers

Avoid rivers, respecting their clearance setting

#### Avoid PowerLines

Avoid power lines, respecting their clearance setting

#### Avoid Tracks

Avoid tracks, respecting their clearance setting

#### Avoid Lakes

Avoid lakes

#### Regenerate By Obstacle Changes

Entirely regenerates if a shape obstacle (listed and checked above) is added/(re)moved in the area

### Generation

#### Seed

Seed used by the random generator of this forest generator

#### Allow Partial Regeneration

Allow partial forest regeneration, regenerates the whole forest otherwise

#### Regenerate Entire Forest

Click to regenerate the entire forest

ⓘ

This is a "button-like" checkbox, used to fix minor inconveniences due to partial regeneration.

### Debug

#### Print Area

Print the area of the forest generator polygon

#### Print Entities Count

Print the count of entities spawned by this forest generator

#### Print Performance Details

Print advanced performance measurements

#### Draw Debug Shapes

Draw general debug shapes

#### Draw Debug Shapes Obstacles

Draw obstacles debug shapes

#### Draw Debug Shapes Rectangulation

Draw rectangulation debug shapes

#### Draw Debug Shapes Regeneration

Draw partial regeneration debug shapes

#### Entities Follow Terrain On Shape Move

Make entities follow terrain level on shape move (keeping their relative Y)

### Forest

Forest's content is defined here - which entity types to spawn, what density, etc.

#### Levels

Forest generator levels to spawn in this forest generator polygon

#### Clusters

Forest generator clusters to spawn in this forest generator polygon

#### Global Outline Scale Curve

Curve defining general outline scaling; from inside (left, forest core) to outside (right, forest outline)

#### Global Outline Scale Curve Distance

Distance from shape over which the scaling occurs

## Levels

Levels define three tree levels:

* Top level - tall trees
* Bottom level - trees growing below top level
* Outline level - forest's border trees

### Top Level

### Bottom Level

### Outline Level

An outline can be one of two types:

* Small
* Medium

## Clusters

Clusters are groups of entities to be created within the forest's surface, but outside of the outline area.
They allow to define various generators with the following properties:

* type(s) of generated objects
  + min and max scale
  + model
  + number of models count
  + min and max models placement radius
* their density (in cluster / hectare or CDENSHA)
* enabled or not

A cluster can be of two types:

* Circle
* Strip

### Options

#### Uncategorised

##### Objects

Define which tree groups should spawn in this cluster

##### Min CDENSHA

The minimum density of the filling in number of clusters for hectare

##### Max CDENSHA

The maximum density of the filling in number of clusters for hectare

##### Generate

Generate this cluster or not

#### Circle

#### Strip

##### Max X Offset

Offset that is applied to generated objects to avoid seeing the exact shape of the curve

##### Max Y Offset

Offset that is applied to generated objects to avoid seeing the exact shape of the curve

##### Frequency

Defines how many times should the curve repeats for this strip

##### Amplitude

Defines the maximum extent of the curve
