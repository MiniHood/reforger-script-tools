# [Procedural Animation Editor: Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes)

## pap Nodes and Buttons

### Signal

This **button** is a shortcut to create a **Signal** (.siga) resource and node.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the input. It is the same as name of the output in \*.siga file. |  |
| Comment | Custom user comment. |  |
| Path | .siga file path |  |

In order to use an **existing** .siga node file, drag and drop it from the **Resource Browser** into the Main Window.

### Constants

Set **constant** key-value pairs.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Metadata |  |  |
| Constant values |  |  |
| **Node Outputs** | | |
| Set Key(s) | Respective Key Value(s) |  |

### Bone

Add a new **bone** to the project.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Bone name | Name of the bone in the preview model. List is only populated if preview model was previously selected. This option is only used for preview of animation - it doesn't affect animation on asset to which it is attached. |  |
| **Node Output** | | |
| Out | Bone name. |  |

### RotationSet

**Set rotation** of the bone. Only functionality of this node is to execute the animation.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Update collider | Defines if collider should be updated (rotated) when this animation is performed. | By default this option is set to off to save some performance. |
| **Node Inputs** | | |
| In | Bone name. |  |
| Rotation | Rotation value. |  |
| **Node Output** | | |
| Out | Value, how much the bone rotates. |  |

### RotationMake

Set the axis in which the bone gets **rotated**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Update collider | Defines if collider should be updated (rotated) when this animation is performed. | By default this option is set to off to save some performance. |
| X axis | Value, how much the bone rotates around the X axis. |  |
| Y axis | Value, how much the bone rotates around the Y axis. |  |
| Z axis | Value, how much the bone rotates around the Z axis. |  |
| **Node Inputs** | | |
| X axis | Value, how much the bone rotates around the X axis. |  |
| Y axis | Value, how much the bone rotates around the Y axis. |  |
| Z axis | Value, how much the bone rotates around the Z axis. |  |

### RotationBreak

**Break rotation** vector into separate elements (x, y, z)

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Rotation input |  |
| **Node Output** | | |
| X axis | Value, how much the bone rotates around the X axis. |  |
| Y axis | Value, how much the bone rotates around the Y axis. |  |
| Z axis | Value, how much the bone rotates around the Z axis. |  |

### TranslateSet

Bone's **translation**. The only functionality of this node is to execute the animation.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Bone to be translated. |  |
| Translation | Translation vectors. |  |
| **Node Output** | | |
| Out |  |  |

### TranslateMake

Set the axis in which the bone is **translated**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| X axis | Value, how much the bone is translated in the X axis. |  |
| Y axis | Value, how much the bone is translated in the Y axis. |  |
| Z axis | Value, how much the bone is translated in the Z axis. |  |
| **Node Inputs** | | |
| X axis | Value, how much the bone is translated in the X axis. |  |
| Y axis | Value, how much the bone is translated in the Y axis. |  |
| Z axis | Value, how much the bone is translated in the Z axis. |  |
| **Node Output** | | |
| Out |  |  |

### TranslateBreak

**Break translation** vector into separate elements (x, y, z)

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Vector (x, y, z) |  |
| **Node Output** | | |
| X axis | Value, how much the bone is translated in the X axis. |  |
| Y axis | Value, how much the bone is translated in the Y axis. |  |
| Z axis | Value, how much the bone is translated in the Z axis. |  |

### ScaleSet

Set bone's **scale**. Only functionality of this node is to execute the animation.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Update collider | If checked, update the model's collider. |  |
| **Node Inputs** | | |
| In | Bone to be scaled |  |
| Scale | Vectors of scale |  |
| **Node Output** | | |
| Out |  |  |

### ScaleMake

Sets the axis in which the bone is **scaled**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| X axis | Value, how much the bone is scaled in the X axis. |  |
| Y axis | Value, how much the bone is scaled in the Y axis. |  |
| Z axis | Value, how much the bone is scaled in the Z axis. |  |
| **Node Inputs** | | |
| X axis | Value, how much the bone is scaled in the X axis. |  |
| Y axis | Value, how much the bone is scaled in the Y axis. |  |
| Z axis | Value, how much the bone is scaled in the Z axis. |  |
| **Node Output** | | |
| Out |  |  |

### ScaleBreak

**Break scale** vector to separate elements (x, y, z)

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Vector (x, y, z) |  |
| **Node Output** | | |
| X axis | Value, how much the bone is scaled in the X axis. |  |
| Y axis | Value, how much the bone is scaled in the Y axis. |  |
| Z axis | Value, how much the bone is scaled in the Z axis. |  |

## siga Nodes

### Input

**Input** is a value (number) provided by the engine.

| Name | Description | Note |
| --- | --- | --- |
| Name | Must reflect how input is named in the engine. |  |
| Comment | Custom user comment. |  |
| Value | Set signal's value. | This has an effect only in the editor for testing and debugging. |
| Min | Sets signal's minimum value. | This has an effect only in the editor for testing and debugging. |
| Max | Sets signal's maximal value. | This has effect only in editor for testing and debugging. |
| ValRT | Sets signal's value in real time. | If no node is selected, this value is shown. Has an effect only in the editor for testing and debugging. In 0.9.5.x version of the Workbench, if the chain contains a Smoother node, this value has no effect. |

### Output

**Output** is a value (number) sent from \*.siga to \*.pap file when it's processed.

| Name | Description | Note |
| --- | --- | --- |
| Name | Signal's name, used in \*.pap file. |  |
| Comment | Custom user comment. |  |

### Value

**Value**, as its name suggests, represents a number defined by the user.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Value | Any number which this node will represent. |  |

### Random

**Random** generates a value in the defined range.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Min | Defines the beginning of the range in which the random number is generated |  |
| Max | Defines the end of the range in which the random number is generated |  |
| Update rate | Defines how often the value is generated.  * Every Get - every time when the branch with the signal is processed (i.e. when the engine is on/off) * Every Frame - every frame of the game. |  |

### Generator

**Generator** sets a value in the predefined interval.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Value to generate | ``` Value to generate by the generator. ``` |  |
| Interval | Interval in milliseconds in which the number is generated. |  |

### Sum

**Sum** all input values together.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Sub

**Subtract** a value from another.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Subtracter | Value which will be used as subtracter, if it's not set on node input. |  |
| **Node Inputs** | | |
| In | ``` The value from which it will be subtracted. ``` |  |
| Subtracter | (Optional) Value which will be used as subtracter. |  |

### Mul

**Multiply** inputs with each other.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Min

**Minimum** value of all inputs.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Max

**Maximum** value of all inputs.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Exp

Returns the base-**exponential** function x, which is e raised to the power x: ex.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Ln

Natural **logarithm**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Log2

**Logarithm** with base 2.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Log10

**Logarithm** with base 10.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Abs

Return **absolute** value.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Div

**Divide** the dividend with the divisor.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Divisor | Value which will be used as divisor, if it's not set on node input. |  |
| **Node Inputs** | | |
| In | The value which will be divided. |  |
| Divisor | Value used as divisor |  |

### Pow

Return the input raised to the power of **Exponent**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Input value. |  |
| Exponent | Value used as Exponent. |  |

### Remainder

Return the **remainder** of dividing input with divisor.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| **Node Inputs** | | |
| In | Input value. |  |
| Divisor | Value used as the divisor. |  |

### Average

**Average** the input values over time.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Capacity | (rounded to the closest power of two) |  |

### Convertor

**Convert** the input into a value defined by the interval it falls into.

| Name | Description | | Note |
| --- | --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. | |  |
| Comment | Custom user comment. | |  |
| Input as default | If checked, input value is used as a default value. | |  |
| Default | Default value. | |  |
| Ranges | Set the range of value which will be converted to output value. i.e.: If the signal value has a range from 0 to 1 and Min is set to 0.1, Max to 0.2 with Out set to 100, the output value will be set to 100 if the Min / Max condition is met. Multiple of ranges can be set in one Convertor. | |  |
|  | Min | Minimal value. |  |
|  | Max | Maximal value |  |
|  | Out | Output value |  |

### Db2Gain

Convert **decibels to gain**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Gain2Db

Convert **gain to decibels**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### St2Gain

Convert **semitone to gain**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Gain2St

Convert **gain to semitone**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Freq2Oc

Convert **frequency to octave**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Env

**Envelope**.

| Name | Description | | Note |
| --- | --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. | |  |
| Comment | Custom user comment. | |  |
| A | x Min | | ENFORCECODEMARKER   ``` if (A == B && C = D) { 	if (input >= A && input < C) 	output = 1; 	else 	output = 0; } ``` |
| B | x Max | |  |
| C | x Min | |  |
| D | x Max | |  |
| Fade-in | Interpolation curve type. Between A and B | |  |
| Fade-out type | Interpolation curve type. Between C and D | |  |
|  | Linear |  |  |
|  | S-Curve |  |  |
|  | Power of 1.41 | f(x) = x^1.41 |  |
|  | Power of 2 | f(x) = x^2 |  |
|  | Power of 3 | f(x) = x^3 |  |
|  | Power of 1/1.41 | f(x) = x^1/1.41 |  |
|  | Power of 1/2 | f(x) = x^1/2 |  |
|  | Power of 1/3 | f(x) = x^1/3 |  |

### Interpolate

**Interpolate** between two values.

| Name | Description | | Note |
| --- | --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. | |  |
| Comment | Custom user comment. | |  |
| X min | Minimal value of X. | |  |
| X max | Maximal value of X. | |  |
| Y min | Minimal value of Y. | |  |
| Y max | Maximal value of Y. | |  |
| Fade-in / Fade-out type |  | |  |
|  | Linear |  |  |
|  | S-Curve |  |  |
|  | Power of 1.41 | f(x) = x^1.41 |  |
|  | Power of 2 | f(x) = x^2 |  |
|  | Power of 3 | f(x) = x^3 |  |
|  | Power of 1/1.41 | f(x) = x^1/1.41 |  |
|  | Power of 1/2 | f(x) = x^1/2 |  |
|  | Power of 1/3 | f(x) = x^1/3 |  |

### Smoother

**Smooth** the input value over time.

| Name | Description | | Note |
| --- | --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. | |  |
| Comment | Custom user comment. | |  |
| Fade-in time | Time in which value transform from lower value to higher value in milliseconds. i.e.: 0 to 1 | |  |
| Fade-in type | Defines the function used to interpolate the value. | |  |
| Fade-out time | Time in which value transform from higher value to a lower value in milliseconds. i.e.: 1 to 0 | |  |
| Fade-out type | Defines the function used to interpolate the value. | |  |
| Input resets timer | If checked, the new input will reset the time of previous interpolation. | |  |
| Always interpolate | ``` Check to make sure that the interpolation of the current value is completed even if a new value arrives. ``` | |  |

### Floor

**Round** **downward**, returning the largest integral value that is not greater than input.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Ceil

**Round upward**, returning the smallest integral value that is greater than input.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Round

**Round** the value.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Clamp

Make sure that the input will be **inside min-max interval**.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Min | Minimal value. |  |
| Max | Maximal value. |  |
| **Node Inputs** | | |
| In | Input value to clamp. |  |
| Min | Minimal value. |  |
| Max | Maximal value. |  |

### ClampMin

Make sure that the input will **never be smaller** than min.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Min | Minimal value. |  |
| **Node Inputs** | | |
| In | Input value to clamp. |  |
| Min | Minimal value. |  |

### ClampMax

Make sure that the input will **never be greater** than max.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
| Max | Maximal value. |  |
| **Node Inputs** | | |
| In | Input value to clamp. |  |
| Max | Maximal value. |  |

### Sin

**Sine** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Cos

**Cosine** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### Tan

**Tangent** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### ASin

**Arcus sine** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### ACos

**Arcus cosine** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |

### ATan

**Arcus tangent** of an input angle in radians.

| Name | Description | Note |
| --- | --- | --- |
| Name | Name of the node. Used for better orientation in the project. |  |
| Comment | Custom user comment. |  |
