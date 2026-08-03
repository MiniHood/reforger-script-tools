# [Animation Editor: Pose 2D Node Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Pose_2D_Node_Tutorial)

A **Pose 2D Node** is primarily used for when you want to map a animation with a pose in each frame to different ranges, such as **Aim Spaces** or **Look At** animations.
This node is a bit more complicated than most others, but once you have created one, it will be easier for you.

ⓘ

The more general details about the node can be found in the [Animation Editor: Nodes](/wiki/Arma_Reforger:Animation_Editor:_Nodes "Arma Reforger:Animation Editor: Nodes") page.

## Tables

Each **Pose 2D Node** has an assorted table for the animation. It looks like this:

Y(-90): X(-90) = 0, X(0) = 1, X(90) = 2

Y(0): X(-90) = 3, X(0) = 4, X(90) = 5

Y(90): X(-90) = 6, X(0) = 7, X(90) = 8

To properly illustrate this, see this table:

|  | | X | | |
| --- | --- | --- | --- | --- |
| -90 | 0 | 90 |
| Y | -90 | 0 | 1 | 2 |
| 0 | 3 | 4 | 5 |
| 90 | 6 | 7 | 8 |

The numbers inside of the circles are the frame numbers in the pose animation, that correspond to a specific set of variable values.

Frame 0 is only played when both X and Y is -90, which is shown in the first line in the table shown before:

**Y(-90): X(-90) = 0**, *X(0) = 1, X(90) = 2*

*Y(0): X(-90) = 3, X(0) = 4, X(90) = 5*

*Y(90): X(-90) = 6, X(0) = 7, X(90) = 8*

Similarly, frame 5 is only played when X is at 90 and Y is at 0, shown here:

*Y(-90): X(-90) = 0, X(0) = 1, X(90) = 2*

**Y(0):**  *X(-90) = 3, X(0) = 4,* **X(90) = 5**

*Y(90): X(-90) = 6, X(0) = 7, X(90) = 8*

If the variables are in any number in between these (eg, Y: -45, X: 45) , the poses will be blended.

## Tips and Tricks

These tables are not limited to **3 by 3 ranges**, but rather, you could have any combination of animations and strange variable values, shown below:

Y(-125): X(-60) = 0, X(0) = 1, X(88) = 2, X(155) = 3

Y(5): X(-150) = 4, X(5) = 5,

Y(29): X(-90) = 6, X(0) = 7, X(90) = 8

The only key rule to all of these is that **you have to go from the smallest number to the highest number in both directions**.The values **in any direction must** **be higher than the previous**, e.g (-60, 0, 88, 155) in X and (-125, 5, 29) in Y.
