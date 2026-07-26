# [Animation Editor: Custom Properties](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Custom_Properties)

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** Add images

Each .anm and .txa file can be edited to add events or custom properties when opened in the [Animation Editor](/wiki/Arma_Reforger:Animation_Editor "Arma Reforger:Animation Editor").

Events are mainly used when **syncing animations** or for evaluating transitions.

## Events

After double clicking on an .anm file in the animation editor, you must press edit in the animation editor preview window in order to add any properties to it.

Doing so, you will then get more options inside of the properties window.

### Events

Events can be added by pressing the **+** button under the **Events** property, and removed by selecting the event and pressing the **-** button.

Events are frame-based and fire off events on the given frame.

### Custom Properties

Custom properties can be added by pressing the **+** button under the **Custom Properties** section, and removed by selecting the property and pressing the **-** button.

Custom properties consists of:

* name of the property (string)
* value of the property (also string although animation system can parse this string to other value types - float, int, vector, 3 angles)

Animation movement override:

There are 2 properties that override animation movement (usually EntityPosition bone in the animation)

If EntityPosition is missing in animation or if you want to override the movement with written values, you can add custom properties named "EntityPos" and "EntityRot" to a animation.

* animation movement - this is the whole animation movement - it's not speed
  + name: "EntityPos"
  + value: "x,y,z"
  + example: "0,0,2.5" - moves entity for 2.5 in Z axis in ONE animation loop
* animation rotation - this is the whole animation rotation
  + name: "EntityRot"
  + value: "x,y,z"       - 3 angles in degrees
  + example: "0,45,0" - rotates entity to right (+ right, -left) around Y axis in ONE animation loop

Animation speed override

* speed up factor
  + name "SpeedUpFactor"
  + value > 0 ... x , 1.0 is default
  + example 1.2 - speeds animation of 20%
