# [BaseDoorComponent](https://community.bistudio.com/wiki/Arma_Reforger:BaseDoorComponent)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)")

Starting with Arma Reforger 1.6.0, a new base class was created for doors named c[BaseDoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/BaseDoorComponent.c;12).
Both c[DoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DoorComponent.c;12) and c[SlidingDoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SlidingDoorComponent.c;12) derive from this class.
This means that centity.FindComponent([DoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DoorComponent.c;12)) does not detect sliding doors anymore.

## Removed Angles from Sliding Doors

Sliding doors now derive from c[BaseDoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/BaseDoorComponent.c;12) and use distance parameters as they should.

Before 1.6.0, sliding doors derived from c[DoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DoorComponent.c;12) and had the "angle" parameters for their opening and closing distances (e.g. "Angle Range" and "Closed Angle").

ⓘ

The angle parameters are kept (hidden) for the sake of backward compatibility;
however data must still be updated using the new distance parameters or log warnings will happen.

## Door Width Axis

The axis plays a very important role in collision checking for both sliding and rotating doors.
For rotating doors, rotation is always around the Y axis, so that should never be the door width axis.
If a door does not stop when it hits a character, it probably means the door width axis is incorrect, or when using the Test Contacts option, the provided collider is incorrect.

Before 1.6.0, the door width axis was not used consistently.

## Smoothing Animations

Doors now support smoothing animations. There are 4 presets, **Linear** (default, old behavior), **EaseIn** (start of the animation is smoothed), **EaseOut** (end of the animation is smoothed), and **EaseInOut** (both ends are smoothed).

ⓘ

The animation is still played over the same time period, so when smoothing curves are used the door will move faster at some points during the animation.

* [![Linear interpolation](https://community.bistudio.com/wikidata/images/a/af/armar-doorcomponent_linear.gif)](/wiki/File:armar-doorcomponent_linear.gif)

  Linear interpolation
* [![Linear interpolation](/wikidata/images/thumb/a/af/armar-doorcomponent_linear.gif/120px-armar-doorcomponent_linear.gif)](/wiki/File:armar-doorcomponent_linear.gif "Linear interpolation")

  Linear interpolation
* [![Ease in interpolation](/wikidata/images/thumb/2/20/armar-doorcomponent_easein.gif/120px-armar-doorcomponent_easein.gif)](/wiki/File:armar-doorcomponent_easein.gif "Ease in interpolation")

  Ease in interpolation
* [![Ease out interpolation](/wikidata/images/thumb/b/b7/armar-doorcomponent_easeout.gif/120px-armar-doorcomponent_easeout.gif)](/wiki/File:armar-doorcomponent_easeout.gif "Ease out interpolation")

  Ease out interpolation
* [![Ease in/out interpolation](/wikidata/images/thumb/c/cd/armar-doorcomponent_easeinout.gif/120px-armar-doorcomponent_easeinout.gif)](/wiki/File:armar-doorcomponent_easeinout.gif "Ease in/out interpolation")

  Ease in/out interpolation

The smoothing curve is a [5th order Smoothstep](https://en.wikipedia.org/wiki/Smoothstep#5th-order_equation) curve.
It is also possible to to provide the smoothing slope at each end of the curve using the **Custom** curve mode.

The first value (X) is the slope of the curve at the start of the animation, and the second value (Y) is the slope of the curve at the end of the animation. Slope must be between 0 and 1.
A slope of 0 means flat (very smooth) and 1 means a 45 degrees slope, so lower values mean smoother animations.
For example, X=0 and Y=1 is similar to an EaseIn curve, X=1 and Y=0 is similar to an EaseOut curve, and X=0 and Y=0 is similar to an EaseInOut curve.
