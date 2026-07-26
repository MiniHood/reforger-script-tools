# [Audio Editor: Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Audio_Editor:_Nodes)

**Nodes** are the fundamental building blocks that are used within the Audio Editor to create audio signal chains.
There are a variety of different nodes, each with a specific purpose that modifies the sound or introduces some logic at its respective position in the chain.

Connecting a series of nodes together allows to create chains that produce sounds with varying degrees of complexity. This page serves as a reference for the usage of each type of node.

Note that there are certain fundamental nodes that must be present in a chain in order for a sound to be playable. These are namely:

* a node that outputs an audio waveform: either a Bank or a Generator
* a Sound node

Nodes can be grouped into two main categories: [Resource Nodes](#Resource_Nodes) and [Local Nodes](#Local_Nodes).
A node of the Resource type is saved as a separate resource file and can be shared between multiple .acp files, while a node of the local type is stored in the .acp file itself and cannot be shared with other .acp files.

Nodes of the Resource type will appear on the design canvas with round corners, while nodes of the Local type will appear with sharp corners.

|  |  |
| --- | --- |
| **Groups**   * [Resource Nodes](#Resource_Nodes) * [Local Nodes](#Local_Nodes) * [ACP Nodes](#ACP_Nodes) | **Nodes**  * [OutputState](#OutputState) * [FinalMix](#FinalMix) * [Mixer](#Mixer) * [Signal](#Signal) * [BankLocal](#BankLocal) * [Bus](#Bus) * [Sound](#Sound) * [Selector](#Selector) * [Generator](#Generator) * [Constants](#Constants) * [Shader](#Shader) * [Amplitude](#Amplitude) * [Frequency](#Frequency) * [Spatiality](#Spatiality) * [FilterChain](#FilterChain) * [Filter](#Filter) * [Playlist](#Playlist) * [Replacer](#Replacer) * [Stream](#Stream) * [AuxOut](#AuxOut) * [Variable](#Variable) * [Switch](#Switch) |

## Resource Nodes

#### OutputState

#### FinalMix

### Mixer

*aka **OutputState** aka **FinalMix***

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Inputs | A listed representation of the groups defined within the mixer file. | N/A |

The mixer's input ports are variable and depend on its internal logic.

No output ports.

### Signal

Takes one or more inputs and transforms them into one or more outputs that can be used to feed the inputs of other nodes.
The internal logic that defines the input/output behavior can be edited inside the [Signal Editor](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Signal_Editor_Nodes "Arma Reforger:Audio Editor: Nodes") by double-clicking on the node.

No dedicated attributes.

Input ports depend on the internal signal setup.

Output ports depend on the internal signal setup.

## Local Nodes

### BankLocal

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Flags | Disable Streaming | If enabled, streaming is suppressed and the entire sample is loaded into memory as a single block of data. ⓘ  The flag is ignored when the size of the file is > 16 MB. | true/false |
| Flags | Preload | If enabled, all samples in the bank are preloaded into the cache on the first playback. | true/false |
| General | Volume | Volume in decibels (dB), applied to all bank samples. | [-60, +60] |
| General | Volume variance | Volume randomization factor; maximum deviation from the value of Volume, in dB. | [0, +30] |
| General | Pitch | Pitch shift in semitones applied to all bank samples. | [-24, +24] |
| General | Pitch variance | Pitch randomization factor; maximum deviation in semitones from the value of Pitch. | [0, 12] |
| General | Loop count | Number of times that the played sample is repeated. | [0, 255] |
| General | Loop begin | The first sample of the region to be looped.  * Loop count above defines the number of repetition * When Offset below is used, then the value must be greater than Offset | N/A |
| General | Loop length | Length of the loop region, in samples. ⚠  The value of Loop begin + Loop length must be greater than Offset. | N/A |
| General | Infinite Loop | If enabled, the played sample is repeated infinitely until the sound is stopped. | true/false |
| General | Terminate Loop | If enabled, the loop will stop immediately upon sound stop. If false, the loop will finish the current cycle first. | true/false |
| General | Termination Fade Out | Fade out in milliseconds applied when the source is terminated due to limits during playback.  Use Release Shape curve. | [0, 10000] |
| General | Selection | Method for selecting a sample.  * RandomNoRepeat: Samples selected at random, with the exception that the same sample is not repeated twice in a row * Random: Samples selected at random * SequenceFromFirst: Samples played in sequence, starting with the first sample * FirstRandomAndSequence: Samples played in sequence, starting with a randomly-selected sample * CustomSignalIndex: When this option is selected, a new input port is added to the bank that allows a sample to be selected based on the signal value | N/A |
| General | Resampler | Resampling method for converting between different sample rates.  * Point: No interpolation (fastest, least accurate) * Linear: Interpolation (slower, but more accurate than point) * FIR (4/8): FIR filter that implements resampling (slowest but most accurate) | N/A |
| Envelope | Silence | Length of silence in milliseconds added in front of the sample  * Time is not counted into Envelope, Envelope-Attack time starts after Silence | [0, 10000] |
| Envelope | Offset | Length of playback offset of the sample, in milliseconds. For non-looped playback in case when offset is bigger then sample length a playback is skipped. | [0, 10000] |
| Envelope | Random Start Offset | If enabled, a random start offset is applied | true/false |
| Envelope | Attack Shape | Function for the shape of the sound fade in. Defines the "shape" of the volume change during fade-in. <https://www.desmos.com/calculator/s7z4sji5b5>   * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Cos: Curve is "S-shaped", similar to SCurve * EPower (Equal Power) : If fade in and fade out happen at the same time, the equal power curve will ensure there's no "hole" in the overall volue (fade in \* fade in + fade out \* fade out = 1) * Exp: Exponential curve (gets steeper over time) * Quadratic: *TBA* | N/A |
| Envelope | Attack | Time in milliseconds it takes before the sample reaches full volume. |  |
| Envelope | Sustain | Time of Sustain in milliseconds. If the value is > 0, the total playback length will become Fade In + Sustain + Fade Out. | [0, 10000] |
| Envelope | Release Shape | Function for the shape of the sound fade out. This will also apply for Termination fade out. Defines the "shape" of the volume change during fade-out. Shapes are the same as attack shapes, but inverted.   * Linear: * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Cos: Curve is "S-shaped", similar to SCurve * EPower (Equal Power) : If fade in and fade out happen at the same time, the equal power curve will ensure there's no "hole" in the overall volue (fade in \* fade in + fade out \* fade out = 1) * Exp: Exponential curve (gets steeper over time) * Quadratic: TBA | N/A |
| Envelope | Release | Time in milliseconds it takes for the sound to fully fade out. | [0, 10000] |
| Samples | Samples | List of audio samples. There is a limit of max 1024 samples. | N/A |
| Samples | Samples/Filename | Link to the sample (.wav format) | N/A |
| Samples | Samples/Probability | In case of a randomised selection method, this parameter defines the probability that the sample will be selected. Probability is a [weighting factor](https://www.statisticshowto.com/weighting-factor/), not a percentage. | [0, 100] |
| Samples | Samples/Index | Custom signal index | [0, inf+] |
| Samples | Samples/Sample Metadata | Name of a string holding localization data which can be processed by other parts of the game. | N/A |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Volume | Signal (toggleable) | Scaling factor applied to the gain of all samples in the bank |
| Volume variance | Signal | Maximum allowed deviation of specified volume in decibels (dB) |
| Pitch | Signal (toggleable) | Scaling factor applied to the pitch of all samples in the bank |
| Pitch variance | Signal | Maximum allowed deviation of specified pitch in semitones |
| Replacer | Settings node | Allows bank to be controlled by a Replacer |
| Offset | Signal | Length of playback offset of the sample in ms. Override offset defined in "bank" properties. |
| Silence | Signal | Length of silence added in front of the sample in ms |
| Fade In | Signal | Time in ms it takes before the sample reaches full volume |
| Sustain | Signal | If the value is >0, the total playback length will become Fade In + Sustain + Fade Out |
| Fade Out | Signal | Time in ms it takes before the sample fully fades out |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Fade Out | Signal | Time in milliseconds it takes before the sample fully fades out. |

### Bus

Mixes down all audio streams fed to the "In" port to a single output.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Channels | Number of output channels | [1, 8] |
| General | Sample Rate | Sample rate in Hertz | Can be one of: * 8000 * 11025 * 16000 * 32000 * 44100 * 48000 * 64000 * 96000 * 192000 |
| General | Volume D B | Volume modifier in decibels (dB) | [-60, 12] |
| General | Resampler | Resampling method for converting between different sample rates  * Point: No interpolation (fastest, least accurate) * Linear: Interpolation (slower, but more accurate than point) * FIR (4/8): FIR filter that implements resampling (slowest but most accurate) | N/A |
| General | Channel Volume | If enabled, input ports controlling the volumes of each output channel become available. ⓘ  In order to work, a dedicated output channel amount must be selected in the "Channel" attribute. "Default" will not work. | true/false |
| Limitation | Enable Limitation | Enables Limitation and displays Limitation variables. | true/false |
| Limitation | Source Limit | Max number of inputs allowed to play at the same time. Quietest ones are muted if the limit is exceeded. | [1, 255] |
| Limitation | Source Limit Type | Specifies how the Source Limit is evaluated  * Static: Limit is evaluated once at the beginning of playback * Dynamic: Limit is evaluated continuously during playback | N/A |
| Limitation | Volume Compensation | If enabled and a sound is discarded because of the Source Limit, the remaining sounds will receive a volume boost equal to the volume of the discarded sounds in order to preserve the input volume. Example: Sound A is 1dB, B is 2dB, Source Limit is 1 (only one sound is allowed to play). Only B will play because it's louder, but it will play with a volume of 3dB. The 1dB volume of discarded A was added to the 2dB original volume of B. | true/false |
| Variable | Var Name | Defines which variable within the variables config file the bus feeds into. Only available when a file is defined in Var Resource. | N/A |
| Variable | Var Resource | Filepath to Config file containing definitions of Variables | N/A |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| In | Audio In | Audio Input |
| Volume | Signal (toggleable) | Scaling factor applied to the gain. Hidden if per-channel volumes are activated. |
| DSP | Settings node | Accepts a Filter or FilterChain |
| Volume0..7 | Signal (toggleable) | Scaling factor applied to the gain of the respective channels. Only visible if per-channel volumes are activated and an output channel amount is specified. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | N/A |

### Sound

The root node of a signal chain. Must be present in every chain in order for sound to be playable in-game, as this node serves as the "identifier" for the engine.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Volume D B | Volume modifier, in decibles (dB) | [-60, 60] |
| General | Priority | Factor that is taken into account when determining which sources to keep or destroy in case the limit of playing sources is exceeded. Sounds with lower priority are more likely to be destroyed than sounds with higher priority. | [0, 100] |
| General | Delay Time | Delay time in milliseconds before sound is played | [0, 10000] |
| General | Even No Repeat Time | Time in milliseconds below which sound events with the same name will be ignored | [0, 10000] |
| Flags | No In Audible | If enabled, sounds below the threshold of audibility are not played | true/false |
| Flags | Bypass Volume Test | If enabled, volume is not considered when determining which sources to destroy in case the .acp limit of playing sources is exceeded | true/false |
| Flags | Speed Of Sound Simulation | Simulates delayed playback due to the speed of sound | true/false |
| Transformation | Transformation | Specifies the position from which this sound is played | * Entity: Sound is played from the position of the entity which the .acp file is attached to * Bone: Sound is played from the bone specified in Bone Name on the entity which the .acp file is attached to * Offset: Sound is played from the position of the entity with the specified offset applied. A Sound Point can be specified in Offset Name   ⚠  "Bone" and "Offset" types need a SoundComponent on the entity in order to work. |
| Transformation | Static | If enabled, the sound does not move with the entity but remains static at the initial playback position. | true/false |
| Trigger | Type | Specifies how the sound is triggered | * Continuous: Sound is triggered and continues to repeat if the value of the signal connected to the Trigger port is different from 0. * One shot: Sound is triggered once based on the values of Threshold and Reset. * Timer: Periodically triggers the sound event, the period is defined by the Time attribute or the signal connected to the Trigger port. If the Time is 0, the sound is not triggered. |
| Trigger | Threshold | Value that the signal connected to the Trigger port needs to exceed before sound is triggered | Threshold >= Reset |
| Trigger | Reset | Value below which the signal needs to be before the sound can be triggered again | Reset <= Threshold |
| Slave | Event Type | If the sound is a slave (i.e. if the Master port is connected to another sound), this parameter defines its start/stop behavior in response to the master. | * Start-Stop: Slave starts/stops when Master starts/stops * OnStart-Start: Slave starts when Master starts * OnStart-Stop: Slave stops when Master starts |
| Bounding Volume | Type | When different from None, sound position is moved to the closest point within the volume towards the camera. Amplitude and Spatiality is calculated towards this closest point. | * Sphere * Box * Cylinder |
| Variable | Var Name | Defines which variable within the variables config file the sound node feeds into. Only available when a file is defined in Var Resource | n/a |
| Variable | Var Resource | Filepath to Config file containing definitions of Variables | N/A |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| In | Audio In | Audio Input |
| Volume | Signal (toggleable) | Multiplier for the value defined in the Item Details |
| Trigger | Signal | Accepts a signal whose value can be used to trigger playback of the sound |
| DSP | Settings node | Accepts a Filter or FilterChain |
| Master | Master/Slave system | Accepts the Slave output of a Sound node |
| Priority | Signal | Input signal value is added to the value defined in the node. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |
| Slave | Master/Slave system | Connects to the "Master" node of another sound node |
| Aux Out | Aux Out | Allows the audio signal to be routed into an Aux Out node |

### Selector

This node gives the user the ability to split the signal chain into multiple branches.
On playback, the Selector will select one branch to connect to the chain, and leave the others disconnected based on the value of the input signal.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Ports | Min | Lower bound of this port's range | N/A |
| Ports | Max | Upper bound of this port's range | N/A |
| Ports | Volume | Volume modifier in decibels applied to the branch connected to this port | [-60, 12] |
| Ports | Default | If enabled, this port will be selected if the input signal does not fall within the range of any of the ports | true/false |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Signal | Signal | Control signal whose value decides which of the following ports will be selected |
| *User-defined ports* | Audio | The branch connected to this port becomes the output if the control signal falls within the Min/Max range defined for this port |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Generator

Outputs a variety of different waveforms as audio. Used primarily for testing/prototyping purposes.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Function | Specifies the function/shape of the generated waveform | Can be one of: * Sine * Square * Sawtooth * Triangle * White noise * Pink noise * Brown noise * Silence * Pulse * One |
| General | Channels | Number of output channels | [1, 8] |
| General | Sample Rate | Sample rate in Hertz | Can be one of: * 8000 * 11025 * 16000 * 32000 * 44100 * 48000 * 64000 * 96000 * 192000 |
| General | Time | Length of time in milliseconds that the sound lasts | [0, 10000] |
| General | Volume D B | Volume modifier in decibels (dB) | [-60, 60] dB |
| General | Loop Count | Number of times that the generated sound is repeated | [0, 255] |
| General | Frequency | Frequency of the tone. ⚠  Hidden if not applicable to the Type. | [1, 20000] |
| General | Termination fade out | Fade out [ms] applied when source is terminated during playback | [0, 10000] |
| Envelope | Attack Shape | Defines the curvature / shape of the attack (fade in) | Defines the "shape" of the volume change during fade-in <https://www.desmos.com/calculator/s7z4sji5b5>   * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Cos: Curve is "S-shaped", similar to SCurve * EPower (Equal Power) : If fade in and fade out happen at the same time, the equal power curve will ensure there's no "hole" in the overall volue (fade in \* fade in + fade out \* fade out = 1) * Exp: Exponential curve (gets steeper over time) * Quadratic: |
| Envelope | Attack | Length of the attack [ms] to reach the desired volume ⚠  If any of A/S/R is different from 0 then envelope is applied. | [0, 10000] |
| Envelope | Sustain | Time [ms] of Sustain ⚠  If any of A/S/R is different from 0 then envelope is applied. | [0, 10000] |
| Envelope | Release Shape | Defines the curvature / shape of the release (fade out) | Defines the "shape" of the volume change during fade-out. Shapes are the same as attack shapes, but inverted.  * Linear: Curve is linear * SCurve: Curve is "S-shaped". with a steep middle-part (volume = -6db when time = 50%) and gentle beginning and ending * Cos: Curve is "S-shaped", similar to SCurve * EPower (Equal Power) : If fade in and fade out happen at the same time, the equal power curve will ensure there's no "hole" in the overall volue (fade in \* fade in + fade out \* fade out = 1) * Exp: Exponential curve (gets steeper over time) * Quadratic: |
| Envelope | Release | Length of the release [ms] to "fade out" after the generator time ⚠  If any of A/S/R is different from 0 then envelope is applied. | [0, 10000] |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Volume | Signal (toggleable) | Scaling factor applied to the gain |
| Time | Signal | Length of time in milliseconds that the sound lasts |
| Frequency | Signal (toggleable) | Frequency of the tone |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Constants

Allows to create static values that can be connected to input ports that take signal inputs.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Ports | Value | The value of each respective port | N/A |

No input ports.

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Variable amount of output ports | Signal | Outputs value specified in the item attributes |

### Shader

Applies panning and attenuation based on the spatial relation between the in-game listener and sound emitter.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Reverse Panning | If enabled, the panning applied by the Shader is reversed (i.e. sounds coming from the left are panned to the right and vice-versa) | true/false |
| General | ITD | If enabled, a short delay is applied to one channel based on the spatial relation between the listener and emitter. This simulates the interaural time difference (ITD) that humans rely on for localising sounds.  ⓘ  ITD is only applied if headphones mode is enabled in the game's settings, as this happens naturally when using speakers. | true/false |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| In | Audio In | Audio Input |
| Spatiality | Settings node | Accepts a [Spatiality](#Spatiality) node that defines the spatial factor (i.e. the blend between the unprocessed and panned versions of the sound) based on the distance / spatial relation between the listener and sound emitter |
| Amplitude | Settings node | Accepts an [Amplitude](#Amplitude) node that defines how amplitude attenuation is applied based on the distance between the listener and sound emitter |
| Frequency | Settings node | Accepts a [Frequency](#Frequency) node that defines how frequency attenuation is applied based on the distance between the listener and sound emitter |
| DSP | Settings node | Accepts a [Filter](#Filter) or [FilterChain](#FilterChain) node |
| Incident angle (azimuth) | Signal (toggleable) | This port can be used to define the direction that the sound is perceived to be coming from. Using this feature overrides the default panning calculated from the positions of the listener and emitter. The value of the input is assumed to be in radians with a counter-clockwise rotation convention. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Amplitude

Settings node designed for use with [Shader](#Shader).
Contains parameters that control the degree of amplitude attenuation that is applied by the Shader.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Amplitude | Curve | Defines how the attenuation changes with distance - that is, the function that the attenuation vs. distance curve adheres to. | * Linear * SCurve * Logarithm * 1/r * Custom |
| Amplitude | Inner Range | Distance in metres defining a radius around the emitter in which no volume attenuation occurs | [0, 100] |
| Amplitude | Outer Range | Distance in metres at which the sound becomes fully muted. Fade-in starts at the end of the Inner Range | [0, 10000] |
| Amplitude | Slope Factor | Factor at which the curve slopes. 100 makes the curve linear. Only available for the "1/r" Curve type | [0, 100] |
| Amplitude | Attenuation Curve | Allows to plot a custom attenuation curve within the Curve Editor. Only available for the "Custom" Curve type | N/A |
| Directivity | Enable Directivity | If enabled, a Directivity Curve can be set. | true/false |
| Directivity | Directivity Curve | Sets the directivity curve which allows to attenuate sound based on angle. Directivity factor is:   * 1 if listener is directly in front of the emitter, * 0 if directly behind the emitter. * 0.5 if 90° from it (left or right)   ⚠  Only visible if Enable Directivity is enabled. | N/A |
| Variable | Var Name | Defines which variable within the variables config file the amplitude node feeds into. ⚠  Only available when a file is defined in Var Resource. | n/a |
| Variable | Var Resource | Filepath to Config file containing definitions of Variables | N/A |

No input ports.

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Frequency

Settings node designed for use with [Shader](#Shader).
Contains parameters that control the degree of frequency attenuation that is applied by the Shader.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Dynamic Update | If enabled, the filter settings are updated during playback | true/false |
| Distance Attenuation | Enable Distance Att | If enabled, filtering due to distance attenuation is applied | true/false |
| Distance Attenuation | Type | Type of filter applied | * AirAbsorption: Applies a custom filter that models the physical effect of distance on the frequency content of sound propagating through air. Deatiled information here. * AirAbsorbtionLite: Combination of biquad low pass and variable roll of low pass filter that models the physical effect of distance on the frequency content of sound propagating through air. * Lowpass: Applies a generic lowpass filter with tweakable parameters. Greater flexbility in Sound Design, but for Air Absorption less physically accurate than the dedicated filter models. |
| Distance Attenuation | Filter Strength (AirAbsorption models only) | Scaling factor for AirAbsorption filter. ⚠  Only available if the "AirAbsorption" or "AirAbsortionLite" Type is selected. | [0, 4] |
| Distance Attenuation | Inner Range | Distance in metres up tp which the filter's cutoff frequency remains at the maximum value specified by Cutoff Max ⚠  Only available if the "LowPass" Type is selected. | [0, 100] |
| Distance Attenuation | Outer Range | Distance in metres at which the filter's cutoff frequency is at the minimum value specified by Cutoff Min ⚠  Only available if the "LowPass" Type is selected. | [0, 10000] |
| Distance Attenuation | Slope Factor | Controls the slope of the filter; analogous to the "Q factor" of the filter ⚠  Only available if the "LowPass" Type is selected. | [0.01, 100] |
| Distance Attenuation | Cutoff Max | Maximum cutoff frequency of the filter when distance <= Inner Range ⚠  Only available if the "LowPass" Type is selected. | [100, 16000] |
| Distance Attenuation | Cutoff Min | Minimum cutoff frequency of the filter when distance >= Outer Range ⚠  Only available if the "LowPass" Type is selected. | [100, 16000] |
| Directivity | Enable | If enabled, low-pass filtering due to source directivity is applied. Activates the Directivity input port. | true/false |
| Directivity | Directivity Factor | The degree to which sound is affected by the filter (0 = no filtering) | [0, 1] |
| Terrain Effects | Terrain Effects | Enables occlusion caused by the terrain. ⚠  The terrain needs to have a generated SoundMap for it to work. | true/false |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Directivity Factor | Signal (toggleable) | The degree to which sound is affected by the filter (0 = no filtering, 1 0 full filtering.) ⚠  Only becomes visible when Directivity is enabled in the node's Item Attributes. |
| Rolloff | Signal (toggleable) | Roloff of the filter |
| Cutoff Frequency | Signal (toggleable) | Cutoff Frequency of the filter. ⚠  Settings via input ports will not override the node's settings, but rather stack with them. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Settings node | Frequency output to be used on the Shader's Frequency input. |

### Spatiality

Settings node designed for use with [Shaders](#Shaders).
The purpose of this node is to provide the connected Shader(s) with a Spatial Factor value that controls how spatialised the output from the Shader(s) are.  
This Spatial Factor controls the blending between unprocessed and spatialised (i.e. panned) sound.
A value of 0 informs the connected Shader that no panning should be applied while a value of 1 indicates that the sound should be fully panned.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Spatiality | Spatial Factor Source | Defines where the spatial factor is taken from. See the Spatial Factor Sources table below for more details. | * Value * Curve * Signal * AzimuthCurve |
| Spatiality | Value | When this option is selected, the spatial factor is defined by a constant value in the range [0, 1] ⚠  Only available when the "Value" Spatial Factor Source is selected. | [0, 1] defined as static value |
| Spatiality | Curve | When this option is selected, the Curve Editor becomes available, in which the user can define the relationship between the distance to the sound emitter and the spatial factor. Only available when the "Curve" Spatial Factor Source is selected. | [0, 1] defined in Curve Editor |
| Spatiality | Signal | When this option is selected, the spatial factor is defined by the value of the signal connected to the Spatial Factor input port which becomes available. Only available when the "Signal" Spatial Factor Source is selected. | [0, 1] from Spatial Factor signal input |
| Spatiality | AzimuthCurve (Curve0, Curve90) | Same as "Curve", but using two different curves. Curve0 defines the Spatial Factor for when the sound emitter is directly in front or behind the listener, Curve90 defines the Spatial Factor for when the sound emitter is directly to the left or right of the emitter. The Spatial Factor will interpolate for angles in between. Only available when the "AzimuthCurve" Spatial Factor Source is selected. | [0, 1] defined in Curve Editor |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Spatial Factor | Signal (toggleable) | The degree to which sound is spatialised. ⚠  Only becomes visible when the "Signal" Spatial Factor Source type is selected in the node's Item Attributes. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Settings node | Spatiality output to be used on the Shader's Spatiality input. |

### FilterChain

Enables the grouping of multiple DSP nodes together in series. The individual DSP effects are applied in the order that they are connected to the FilterChain.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Ports | Amount of available input ports | [2, 16] |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| *User-defined ports* | Settings node | Accepts a Filter output |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Settings node | To be used in another node's DSP input. |

### Filter

Settings node that applies a specified DSP effect to the node(s) to which it is connected.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Wet Dry | Blend of the dry (unprocessed) and wet (processed) versions of the input sound | [0, 1] |
| General | Volume | Volume modifier (decibels) | [-60, 12] |
| General | DSP Object | Class of the DSP effect to be used | N/A |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Wet Dry | Signal (toggleable) | Blend of the dry (unprocessed) and wet (processed) versions of the input sound [0, 1] |
| Volume | Signal (toggleable) | Scaling factor applied to the gain |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Playlist

Inputs connected to this node will play in sequence, according to the order in which they are connected to it.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Unsorted | Ports | Amount of available input ports | [1, 16] |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| *User-defined ports* | Audio In | Audio Input |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

### Replacer

Settings node designed to be used with the Replacer input ports of [Banks](#BankLocal). Enables the user to replace a string in the filepaths of all samples in the connected Bank.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Strings | String to replace | The string, searched for in the filepaths of all samples in the connected Bank, that we wish to replace. | N/A |
| Strings | Replacement strings | A list of candidate replacement strings and signal value pairs. A Replacement string from the list replaces String to replace in the filepaths of the connected bank's samples if its corresponding signal value matches that of the Control signal. | 0..64 entries |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Control Signal | Signal | Signal which will be evaluated in Replacement strings. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Settings node | To be used in a Bank's Replacer input port. |

### Stream

Abstract node used for connecting streaming audio (e.g voice chat) to a signal chain created in the Audio Editor.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | ID | ID string corresponding to the target stream ID defined in gamecode. | N/A |

No input ports.

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio out | Audio Output. ⚠  Cannot be connected to Selectors or Playlists due to the nature of the node. |

### AuxOut

Accepts one or more [Sound](#Sound) nodes as input and routes the output of those node(s) to the connected [Mixer](#Mixer) port.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Volume | Volume in decibels (dB), applied to all samples in the bank | [-60, +60] |
| General | Mono Mix | Value between 0 and 1 that controls the mix between the unaltered output and a mono-downmixed version of the output | [0, 1] |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Sound | Sound node | The output from the sound node(s) connected to this port will be mixed down and sent to the output. |
| Volume | Signal (toggleable) | Scaling factor applied to the output gain. |
| Mono Mix | Signal (toggleable) | Value between 0 and 1 that controls the mix between the unaltered output and a mono-downmixed version of the output. |
| DSP | Settings node | Accepts a [Filter](#Filter) or [FilterChain](#FilterChain) node. |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio out | Audio Output to be connected with a Mixer node. |

### Variable

Abstract node that outputs the value of a specified [audio variable](/wiki/Arma_Reforger:Audio_Editor:_Audio_Variables "Arma Reforger:Audio Editor: Audio Variables").

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| General | Curve | Opens the Curve Editor, in which a relation between variable value and output signal can be set. | N/A |
| Variable | Var Name | Defines which variable within the variables config file the node reads from. ⚠  Only available when a file is defined in Var Resource. | N/A |
| Variable | Var Resource | Filepath to Config file containing definitions of variables. | N/A |

No input ports.

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Signal | Signal value output (float) |

### Switch

This node is similar to the [Selector](#Selector) node, but gives the user the ability to switch between ports during sound playback.

Attributes

| Category | Name | Description | Range |
| --- | --- | --- | --- |
| Unsorted | Crossfade Time | Time to crossfade between previous and currently playing port. | [100, 100000] ms |
| Ports | Min | Lower bound of this port's range | [-inf, inf] |
| Ports | Max | Upper bound of this port's range | [-inf, inf] |
| Ports | Volume | Volume modifier [dB] applied to the branch connected to this port | [-60, 12] dB |
| Ports | Default | If enabled, this port will be selected if the input signal does not fall within the range of any of the ports | True/False |

Input Ports

| Name | Type | Description |
| --- | --- | --- |
| Signal | Signal | Control signal whose value decides which of the following ports will be selected |
| (Varying number of user-defined ports) | Audio | The branch connected to this port becomes the output if the control signal falls within the Min/Max range defined for this port |

Output Ports

| Name | Type | Description |
| --- | --- | --- |
| Out | Audio Out | Audio Output |

## ACP Nodes

* ACP node, aka "ACP in ACP" allow for parts of an acp to be re-used within another.
* The main purpose of this feature is it to remove cases in which multiple ACPs are near-identical with only minor differences.
* This is achieved by the ability to "expose" input and output ports within an ACP. These exposed ports can then be accessed within another ACP.
* Similar to Signal node, ACP node references .acp project, where node configuration is stored.

### Port Exposition

Ports can be exposed by opening a node's context menu, selecting a parameter and finally activating "Expose".

The exposed ports can then be seen in the acp's Item Detail panel.

Exposed ports name is created from node name and type of the node.

Exposed ports can be removed by:

* unchecking "Expose" on the node attributes or
* in ACP's Item Detail panel

### Group Usage

Groups allow to assign the input ports of multiple nodes to a single ACP input.

If, for example, 4 banks are supposed to have the same pitch, their pitch inputs can be added to a group. This group will then show up as a single input on the ACP node.

Groups can be created by clicking on the **+** Icon in the Groups section of an ACP's Item Detail panel.

A node's input can be assigned to a group via the context menu. In order to assign an input to a Group, it must be enabled and exposed.

### Playback

Right-clicking an ACP node within another ACP will show the "Playback" context menu entry.
This will list all "playable" nodes which, in turn, can be turned on or off for playback. Playback from multiple nodes is supported.

### Other

* The system supports connection validation
* The system supports Signal Simulation
* ACP node is removed in the data build and instead replaced by node content. All connections go through the same accept/connect functions as in the editor, so an error message can appear and the build can fail.

### Known Issues

* Nested ACP nodes are not supported. (No ACP in ACP in ACP)
* Groups accept any input, but no type-check is supported. A group could take an audio stream but be connected to a signal input. This will fail the build.
* There are problems when the group already exists (and is connected) and in a referenced ACP a new port is exposed into this group.

## External References

* [Sample-rate conversion](https://en.wikipedia.org/wiki/Sample-rate_conversion)
* [Interpolation](https://en.wikipedia.org/wiki/Interpolation)
* <https://ccrma.stanford.edu/~jos/resample/resample.pdf>
