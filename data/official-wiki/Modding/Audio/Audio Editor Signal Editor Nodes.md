# [Audio Editor: Signal Editor: Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Audio_Editor:_Signal_Editor:_Nodes)

|  |  |
| --- | --- |
| **Groups**   * [Interfacing Nodes](#Interfacing_Nodes) * [Value-Generating Nodes](#Value-Generating_Nodes) * [Math Operators and Logic Nodes](#Math_Operators_and_Logic_Nodes) * [Exponential Group Nodes](#Exponential_Group_Nodes) * [Conversion Group Nodes](#Conversion_Group_Nodes) * [Saturation Group Nodes](#Saturation_Group_Nodes) * [Trigonometric Group Nodes](#Trigonometric_Group_Nodes) | **Nodes**  * [Input](#Input) * [Output](#Output) * [Value](#Value) * [Variable](#Variable) * [Random](#Random) * [Generator](#Generator_2) * [Time](#Time) * [LFO](#LFO) * [Curve Modulator](#Curve_Modulator) * [Square Modulator](#Square_Modulator) * [Sum](#Sum) * [Sub](#Sub) * [Mul](#Mul) * [Div](#Div) * [Remainder](#Remainder) * [Mod](#Mod) * [Min](#Min) * [Max](#Max) * [Abs](#Abs) * [Pow](#Pow) * [Cond](#Cond) * [Converter](#Converter) * [Interpolate](#Interpolate) * [Env](#Env) * [Smoother](#Smoother) * [Average](#Average) * [Filter](#Filter_2) * [Delta](#Delta) * [Exp](#Exp) * [Ln](#Ln) * [Log2](#Log2) * [Log10](#Log10) * [Db2Gain](#Db2Gain) * [Gain2Db](#Gain2Db) * [St2Gain](#St2Gain) * [Gain2St](#Gain2St) * [Freq2Oc](#Freq2Oc) * [Freq2Mel](#Freq2Mel) * [Mel2Freq](#Mel2Freq) * [Floor](#Floor) * [Ceil](#Ceil) * [Round](#Round) * [Clamp](#Clamp) * [ClampMin](#ClampMin) * [ClampMax](#ClampMax) * [Sin](#Sin) * [Cos](#Cos) * [Tan](#Tan) * [ASin](#ASin) * [ACos](#ACos) * [ATan](#ATan) |

## Interfacing Nodes

### Input

Creating an Input node will automatically create a corresponding input port on the Signal node within the [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor").
It allows the usage of game data within the Signal Editor in two ways:

* Inside the Audio Editor, another node can be connected to the [Signal](#Signal) node's inputs ports. The corresponding Input nodes within the Signal will then use the value passed from the connected node.
* Giving an Input node the same name as a Signal passed via script or present on the connected entity's SignalsManagerComponent will automatically cause the node to use the respective values.  
  e.g naming an Input node "Distance" will result in that it returns the distance from the listener to the entity.

Naming Conventions

Input nodes that are designed to be fed from [constants](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Constants "Arma Reforger:Audio Editor: Nodes") nodes will be NAMED\_IN\_ALLCAPS.
The respective Constant output ports get the exact same name.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Global | If true and the .acp file using the Signal is attached to a SoundComponent, the signal name will be looked up in the list of global game signals instead of the connected entity. | true/false | Unchecked |

### Output

Creating an Output node will automatically create a corresponding output port on the Signal node within the [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor").
It allows the usage of data processed within the Signal Editor to be used within the Audio Editor.

No attributes.

## Value-Generating Nodes

### Value

The Value node will always output a single, specified value.

ⓘ

For readability, it can make sense to change the node's name or comment to the specified value.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Value | The value the node is supposed to output. | [-9223372036854775808, 9223372036854775808] | Unchecked |

### Variable

The Variable node outputs the current Value of an audio variable.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Var Name | Defines which variable from within the Variable Resource the node will output. | N/A | Unchecked |
| Var Resource | Defines which Variable config file will be accessed. | N/A | Unchecked |

### Random

The Random node will output a random number within a specified min/max range.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Min | Minimum value that can be generated. | [-9223372036854775808, 9223372036854775808] | Unchecked |
| Max | Maximum value that can be generated. | [-9223372036854775808, 9223372036854775808] | Unchecked |
| Update Rate | Defines the rate at which the node will generate random values. | * Every Get: generate a random value on each request (Could be multiple times per frame). * Every OnFrame: generate a random value once per frame. * Once: generate a random value once and then continue outputting it on every successive get. | Unchecked |

### Generator

The Generator node generates "impulses" of a specified Value in one frame in specified intervals.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Value To Generate | The value which will be generated. | [-9223372036854775808, 9223372036854775808] | Unchecked |
| Interval | The interval in milliseconds between each value generation. | [-9223372036854775808, 9223372036854775808] | Unchecked |

### Time

The Time node continuously outputs either the time a sound instance has existed or the time the world has been running on the client's machine (in milliseconds).

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Time Type | Defines what time in milliseconds the node outputs. | * Life Time: Outputs the time in milliseconds the sound instance has existed so far. If the node is used by more than one sound that plays from the same .acp at the same time, it will output the Life Time of the first sound that was triggered. * World Time: Outputs the time in milliseconds the world has been running on the local machine so far (**not** how long it has been running on the server). | Unchecked |

### LFO

The L(ow)F(requency)O(scillation) node [oscillates](https://en.wikipedia.org/wiki/Low-frequency_oscillation) between 0 and 1 in a specified shape.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Frequency | Defines the LFO cycle's duration in seconds. | N/A | Unchecked |
| Shape | The shape used for oscillation. | * Sine * Triangle * Square * Saw | Unchecked |

### Curve Modulator

The Curve Modulator node allows it to plot modulation curves by defining the curve's single points.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Points | Allows to add points to the curve. | N/A | Unchecked |
| Points: Time | The time in milliseconds it takes to interpolate from the previous Value to the one of this point. | [1, 1000000] | Unchecked |
| Points: Time Variance | The allowed maximum deviation from the specified point Value. | [0, 500000] | Unchecked |
| Points: Value | The value of the point. | [-9223372036854775808, 9223372036854775808] | Unchecked |
| Points: Value Variance | The allowed maximum deviation from the specified point Value. | [0, 500000] | Unchecked |

### Square Modulator

The Square Modulator node allows it to generate a sequence of pulses with specified peak values,
peak lengths and gaps between peaks.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Points | Allows to add points to the curve. | N/A | Unchecked |
| Points: Value | The value of the peak. | [-9223372036854775808, 9223372036854775808] | Unchecked |
| Points: Value Variance | The allowed maximum deviation from the specified point Value. | [0, 500000] | Unchecked |
| Points: Peak | The time in milliseconds the Value will be held before going back to 0. | [1, 1000000] | Unchecked |
| Points: Peak Variance | The allowed maximum deviation from the specified Peak time. | [0, 500000] | Unchecked |
| Points: Gap | The time in milliseconds the Value will remain at 0 before reaching the next Point. | [1, 1000000] | Unchecked |
| Points: Gap Variance | The allowed maximum deviation from the specified Gap time. | [0, 500000] | Unchecked |

## Math Operators and Logic Nodes

### Sum

The Sum node outputs the sum of all input nodes (simple addition).

No attributes.

### Sub

The Sub(traction) node subtracts the Subtracter value from the Input value and outputs the result.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Subtracter | Defines the value which will get subtracted from the input value. | [-9223372036854775808, 9223372036854775808] | Checked |

### Mul

The Mul(tiplication) node outputs the product of all inputs.

No attributes.

### Div

The Div(ision) node outputs the quotient of the dividend (Input) and the Divisor.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Divisor | Defines the value by which the input value will get divided. | [-9223372036854775808, 9223372036854775808] | Checked |

### Remainder

The Remainder node outputs the remainder of a division. The remainder is the value that is left when the Input (dividend) is not completely divisible by the Divisor.

Formula

```
remainder = dividend - (divisor * roundedQuotient)
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Divisor | Defines the value which will be used as the Divisor. | [-9223372036854775808, 9223372036854775808] | Checked |

### Mod

The Mod(ulo) node outputs the modulus of a division. It is essentially the same as the [Remainder](#Remainder) node, but will floor the Quotient.

Formula

```
modulus = dividend - (divisor * flooredQuotient)
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Divisor | Defines the value which will be used as the Divisor. | [-9223372036854775808, 9223372036854775808] | Checked |

### Min

The Min(imum) node will always output the smallest of the input values fed into it.

No attributes.

### Max

The Max(imum) node will always output the largest of the input values fed into it.

No attributes.

### Abs

The Abs(olute) node will always output the [absolute value](https://en.wikipedia.org/wiki/Absolute_value) of an input value.
Simply put, it makes sure that the output value is always positive.

No attributes.

### Pow

The Pow(er) node outputs a power, in which the Input is set to the power of Exponent.

Formula

```
output = input^exponent
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Exponent | Defines the value of the exponent. | [-9223372036854775808, 9223372036854775808] | Checked |

### Cond

The Cond(ition) node compares the Input value against a Comparator value and returns 1 if the Condition is met, 0 if not.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Condition Type | Defines the Condition the comparison has to meet for the node to return 1. | Available operators   * == (Input and Comparator must be equal) * != (Input and Comparator must be different) * > (Input must be greater than Comparator) * >= (Input must be greater than or equal to Comparator) * < (Input must be less than Comparator) * <= (Input must be less than or equal to Comparator) | Unchecked |
| Comparator | The value the Input value is compared against. |  | Checked |

### Converter

The Converter node allows it to output custom values if the Input falls within specified ranges.
If the input value does not fall within these ranges, a Default value is used.  
Depending on the node's settings, this Default value can either be

* specified in the node's Attributes (or fed via the Default input port) or
* the Input value.

ⓘ

The defined range will be [min, max), meaning the the min value is included,
the max value is not.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Default | Defines the default value which will be returned of the Input value does not fall within any Interval. | [-9223372036854775808, 9223372036854775808] | Checked |
| Default from Input | If true, the node will output the Input value directly if it does not fall within any Interval. | true/false | Unchecked |
| Intervals | Allows to define the node's Intervals/Ranges. | N/A | Unchecked |
| Intervals: Min | Defines the minimum (included) value of the interval. |  | Unchecked |
| Intervals: Max | Defines the maximum (not included) value of the interval. |  | Unchecked |
| Intervals: Out | Defines the output value if the Input value lies within the range. |  | Unchecked |

### Interpolate

The Interpolate node allows it to smoothly interpolate between two Output values (Y) based on an Input value (X).

* If the Input value is <= X min, the Output will be Y min.
* If the Input value is >= X max, the Output will be Y max.
* If the Input value is in between X min and X max, the Output value depends on the interpolation parameters.

Example

```
X min = 10, X max = 20
Y min = 0, Y max = 1
input = 15
input is right in-between 10 and 20
brought to Y range, output is 0.5
```

ⓘ

The min values do not have to be smaller than the max values.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| X min | Defines the the Input value at which the interpolation starts. Output value == Y min. | [-9223372036854775808, 9223372036854775808] | Checked |
| X max | Defines the the Input value at which the interpolation ends. Output value == Y max. | [-9223372036854775808, 9223372036854775808] |
| Y min | Defines the minimum output value. | [-9223372036854775808, 9223372036854775808] |
| Y max | Defines the maximum output value. | [-9223372036854775808, 9223372036854775808] |
| Fade In Type | Defines the curve function of the interpolation. ⚠  Only available if Clamp is activated. | * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Power of... | Unchecked |
| Fade Out Type |
| Clamp | If activated, the interpolation is limited to Y min / Y max. If deactivated, the interpolation can continue in both directions and will keep the correlation between X and Y. | true/false | Unchecked |
| Enable Custom Curve | Allows it to define a custom correlation between Input (X) and Output (Y) in the Curve Editor. Will disable all other attributes. | N/A | Unchecked |

### Env

The Env(elope) node allows it to smoothly interpolate from 0 to 1 and then back from 1 to 0 based on an Input value.

* Between an Input value of A and B, the Output will interpolate from 0 to 1.
* Between B and C, the Output will remain 1.
* Between C and D, the Output will interpolate back to 0.

Example

```
A = 1, B = 4, C = 6, D = 10
Input = 0: result = 0
Input = 1: result = 0
Input = 2: result = 0.333
Input = 3: result = 0.666
Input = 4..6: result = 1
Input = 7: result = 0.75
Input = 8: result = 0.5
Input = 9: result = 0.25
Input = 10: result = 0
Input = 11: result = 0
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| A | Defines the the Input value at which the first interpolation starts. Output value == 0. | [-9223372036854775808, 9223372036854775808] | Checked |
| B | Defines the the Input value at which the first interpolation ends. Output value == 1. | [-9223372036854775808, 9223372036854775808] |
| C | Defines the the Input value at which the second interpolation starts. Output value == 1. | [-9223372036854775808, 9223372036854775808] |
| D | Defines the the Input value at which the second interpolation ends. Output value == 0. | [-9223372036854775808, 9223372036854775808] |
| Fade In Type | Defines the curve function of the interpolation. ⚠  Only available if Clamp is activated. | * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Power of... | Unchecked |
| Fade Out Type |

### Smoother

The Smoother node smoothes out changes in the Input by interpolating from the old to the new value over time.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Fade In Time | Time in milliseconds the interpolation will take if the input value increased. | [999999999] | Unchecked |
| Fade Out Time | Time in milliseconds the interpolation will take if the input value decreased. | [999999999] | Unchecked |
| Fade In Type | Defines the curve function of the interpolation. ⚠  Only available if Clamp is activated. | * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Power of... | Unchecked |
| Fade Out Type |

### Average

The Average node Outputs the average of the Input values within a specified number of frames.

ⓘ

The node is (obviously) framerate-dependent and therefore not reliable when requiring accurate timings; e.g. when storing 30 frames:

* Running the game at (constant) 30 FPS will result in getting the average of the last second.
* Running the game at (constant) 60 FPS will result in getting the average of the last 1/2 second.

Formula

```
average = frameValueSums / framesNumber
// e.g capacity = 3, frame1 = 50, frame2 = 100, frame3 = 120
// average = (50 + 100 + 120) / 3 = 90
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Capacity | Defines how many frames will be stored and evaluated. | [0, 128] | Unchecked |

### Filter

The Filter node is one-pole low-pass filter.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Frequency | Cut-off frequency | N/A | Checked |

### Delta

The Delta node outputs change of input over time.

Formula

```
out = (in - inLast) / timeSlice
```

No attributes.

## Exponential Group Nodes

### Exp

The Exp(onential) node multiplies the Input value with [e (Euler's Number)](https://en.wikipedia.org/wiki/E_(mathematical_constant)).

No attributes.

### Ln

The Ln node outputs the [Natural Logarithm](https://en.wikipedia.org/wiki/Natural_logarithm) of the Input value.

No attributes.

### Log2

The Log2 node outputs the [Binary Logarithm](https://en.wikipedia.org/wiki/Binary_logarithm) of the Input value.

No attributes.

### Log10

The Log10 node outputs the Decadic Logarithm (or [Common Logarithm](https://en.wikipedia.org/wiki/Common_logarithm) of the Input value.

No attributes.

## Conversion Group Nodes

### Db2Gain

The D(eci)b(el)2Gain node converts an Input value (in dB) and outputs it as Gain.

Formula

```
gain = 10^(input / 20)
```

No attributes.

### Gain2Db

The Gain2D(eci)b(el) node converts an Input value (in Gain) and outputs it as dB.

ⓘ

The initial/reference gain value will always be 1.

Formula

```
decibel = 20 * log10(input)
```

No attributes.

### St2Gain

The S(emi)t(ones)2Gain node converts an Input value (in SemiTones) and outputs it as a Gain value.
Alternatively, the Gain value can be interpreted as a relative pitch value.

The Input value hereby represents a change in SemiTones, meaning:

* an Input of 0 outputs 1 (Baseline, no change in pitch),
* an Input of -12 outputs 0.5 (half the baseline pitch/ one octave lover),
* an Input of 12 outputs 2 (double baseline pitch/ one octave higher).

Formula

```
gain = 2^(input / 12)
```

No attributes.

### Gain2St

The Gain2S(emi)t(ones) node converts an Input value (in Gain/relative pitch) and outputs it as SemiTones.

The Input value hereby represents a change in Gain/relative pitch, meaning:

* an Input of 1 outputs 0 (Baseline, no change in SemiTones),
* an Input of 0.5 outputs -12 (12 SemiTones/one octave lower),
* an Input of 2 outputs 12 (12 SemiTones/one octave higher).

Formula

```
semiTones = 12 * log2(input)
```

No attributes.

### Freq2Oc

The Freq(uency)2Oc(tave) node outputs the nth octave of the Input (frequency).

Formula

```
semiTones = input * 2^(Octaves)
```

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Octaves | Defines the maximum output value. | [0, 128] | Unchecked |

### Freq2Mel

The Freq2Mel node converts an Input value in [Hz](https://en.wikipedia.org/wiki/Hertz) and outputs it in [Mel](https://en.wikipedia.org/wiki/Mel_scale).

Formula

```
mel = 1127 * ln (1 + f / 700)
```

### Mel2Freq

The Mel2Freq node converts an Input value in [Mel](https://en.wikipedia.org/wiki/Mel_scale) and outputs it in [Hz](https://en.wikipedia.org/wiki/Hertz).

Formula

```
hertz = 700 * (e^(Mel / 1127) - 1)
```

## Saturation Group Nodes

### Floor

The Floor node rounds an Input value down to its next lowest integer (e.g 1.1 → 1, 1.5 → 1, 1.99 → 1, 2.001 → 2).

No attributes.

### Ceil

The Ceil(ing) node rounds an Input value up to its next highest integer (e.g 1.1 → 2, 1.5 → 2, 1.99 → 2, 2.001 → 3).

No attributes.

### Round

The Round node rounds an Input value up if its first decimal is >= 5 and down if not (e.g 1.4 → 1, 1.49 → 1, 1.5 → 2, 1.51 → 2).

No attributes.

### Clamp

The Clamp node prevents the Output value from becoming larger than Max and smaller than Min.  
If the Input value is smaller than Min, the Output will return Min.  
If the Input value is larger than Max, the Output will return Max.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Min | Defines the maximum output value. | [-9223372036854775808, 9223372036854775808] | Checked |
| Max | Defines the maximum output value. | [-9223372036854775808, 9223372036854775808] | Checked |

### ClampMin

The ClampMin node prevents the Output value from becoming smaller than Min.
If the Input value is smaller than Min, the Output will return Min.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Min | Defines the maximum output value. | [-9223372036854775808, 9223372036854775808] | Checked |

### ClampMax

The ClampMax node prevents the Output value from becoming larger than Max.
If the Input value is larger than Max, the Output will return Max.

Attributes

| Name | Description | Range | Input port |
| --- | --- | --- | --- |
| Max | Defines the maximum output value. | [-9223372036854775808, 9223372036854775808] | Checked |

## Trigonometric Group Nodes

### Sin

The Sin(e) node outputs the sine of the Input value.

No attributes.

### Cos

The Cos(ine) node outputs the cosine of the Input value.

No attributes.

### Tan

The Tan(gent) node outputs the tangent the Input value.

No attributes.

### ASin

The A(rc)Sin node outputs the ArcSin of the Input value.

No attributes.

### ACos

The A(rc)Cos node outputs the ArcCos of the Input value.

No attributes.

### ATan

The A(rc)Tan node outputs the ArcTan of the Input value.

No attributes.

## External References

* [Low-frequency oscillation](https://en.wikipedia.org/wiki/Low-frequency_oscillation)
* [Remainder](https://en.wikipedia.org/wiki/Remainder)
* [Absolute value](https://en.wikipedia.org/wiki/Absolute_value)
* [E (mathematical constant)](https://en.wikipedia.org/wiki/E_(mathematical_constant))
* [Natural logarithm](https://en.wikipedia.org/wiki/Natural_logarithm)
* [Binary logarithm](https://en.wikipedia.org/wiki/Binary_logarithm)
* [Common logarithm](https://en.wikipedia.org/wiki/Common_logarithm)
