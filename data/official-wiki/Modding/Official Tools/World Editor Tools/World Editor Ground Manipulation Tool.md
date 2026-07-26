# [World Editor: Ground Manipulation Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Ground_Manipulation_Tool)

## Usage

While clicking on the point gizmo:

* holding `Ctrl` while dragging a selection up/down will change its **elevation** (altitude)
* holding `⇧ Shift` while dragging a selection up/down will change its 2D **rotation** (direction).

While clicking outside the gizmo:

* holding `Ctrl` or `⇧ Shift` while dragging a selection will change its **3D** rotation.

## Settings

### Snap horizontally

Defines the snap steps for horizontal movement on X and Z axes, in meters.

### Snap vertically

Defines the snap steps for vertical movement on Y axis, in meters.

ⓘ

Also applies to ground-level following selection placement.

### Snap rotation

Defines the steps for rotation on Y axis.

### Transform specs

#### Snap separately

If checked, snaps horizontally/vertically for each selected objects.

If unchecked, acts like one big object.

#### Rotate separately

If checked, selected objects will rotate each on their own axis; if not, they will all rotate around the same axis (as one big object).

#### Keep elevation of all

If checked, sets elevation like one big object for the selection.

#### Negative elevation

If checked, allows for negative elevation value ("object in the ground").

#### Transform children

### Snap to ground

Sets elevation to 0 from the ground.

### Place using physics

ⓘ

This button is inert as of Arma Reforger 1.0.0.

This button places entities with rigid body to the ground using physical simulation.
