# [World Editor: Powerline Generator Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Powerline_Generator_Tutorial)

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** Add more cases.

## Junction Creation

A main polyline will become joined by a secondary one; here is how to proceed.

### Main Polyline

The main polyline must have at least 3 points (start, end, and middle one(s)) so a middle point can be used as an anchor for the secondary polyline.

* Define a **Junction Pole** in the main polyline (see above) Click on the main polyline's point to become anchor
* At the bottom of the Vector Tool tab, click "set class" on Junction Data (do not set any additional information in there)

### Secondary Polyline

* Draw the other polyline that will be connected to the main one
* Define the power line poles (especially the **end pole** definition is needed to connect to the main polyline)
* Tick "snap to anchors" in the Vector Tool tab and set the "Anchor snap distance" to at least 1
* Drag and snap the extremity of the secondary polyline to the main polyline's defined anchor

*Et voilà!* The secondary polyline is connected to the main one, using the main one's Junction Pole definition.
