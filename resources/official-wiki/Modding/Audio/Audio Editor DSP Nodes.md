# [Audio Editor: DSP Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Audio_Editor:_DSP_Nodes)

**Nodes**

* [Base](#Base)
* [Biquad Filter](#Biquad_Filter)
* [Bitcrusher](#Bitcrusher)
* [Compressor](#Compressor)
* [Distortion](#Distortion)
* [Dynamic Equalizer](#Dynamic_Equalizer)
* [Equalizer4](#Equalizer4)
* [Flanger](#Flanger)
* [QuadDelay](#QuadDelay)
* [LoudnessNormalization](#LoudnessNormalization)
* [MonoToStereo](#MonoToStereo)
* [OnePoleFilter](#OnePoleFilter)
* [PeakLimiter](#PeakLimiter)
* [Phaser](#Phaser)
* [Reverb](#Reverb)
* [Reverb2](#Reverb2)
* [SmallRoomReverb](#SmallRoomReverb)
* [Tremolo](#Tremolo)
* [VariableRolloffLPF](#VariableRolloffLPF)
* [NoiseGate](#NoiseGate)

The following is an overview of all available DSP effect classes that can be used in the Filter node. Assigned DSP effect classes will affect the input ports available to the Filter node.

ⓘ

DSP effect classes can be assigned by clicking on "DSP Object" in a Filter node's Item Details.

## DSP Nodes

### Base

Abstract type that all DSP effects inherit from.

No attributes.

### Biquad Filter

Applies a biquad filter[[1]](#cite_note-1).

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Biquad Type** | Type of filter | * LowPass * HighPass * BandPass * Notch * PeakingEQ * LowShelf * HighShelf * AllPass | Unchecked |
| **Fc** | Cutoff/center frequency of the filter | [20, 16000] | Checked |
| **Q** | Factor that controls the slope of the filter shape | [0.1, 20] | Checked |
| **Gain** | Gain [dB] of the filter Available for "PeakingEQ", "LowShelf", "HighShelf" and "AllPass" **Biquad Types**. | [-18, 18] | Checked |

### Bitcrusher

Introduces distortion by reducing the bit depth of the audio input to 8-bit[[2]](#cite_note-2).

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Bit Mask** | An 8-digit string of '0's or '1's that specifies which of the 8 bits of the bit-crushed signal should be "enabled" (1) or "disabled" (0) | String | Unchecked |

### Compressor

Implements dynamic range compression; Reduces the volume of the input signal if the specified threshold value is exceeded[[3]](#cite_note-3).

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Threshold** | Threshold value of the input level above which the gain begins to be reduced | [-50, 0] dB | Checked |
| **Ratio** | Specifies the value of *n* in the ratio *n*:1. This value controls the amount of gain reduction (e.g. a ratio of 2:1 tells us that if the input signal is 2 dB over the value of **Threshold**, the resulting output level is reduced by 1 dB). | [1, 50] | Checked |
| **Attack** | Specifies the amount of time [ms] that the output level takes to reach the value defined by **Ratio** | [1, 1000] ms | Checked |
| **Release** | Specifies the amount of time [ms] that the compressor takes to "undo" gain reduction in the case that the signal level has decreased | [1, 1000] ms | Checked |
| **Knee Width** | Controls the sharpness of the compressor's "knee" | [0, 20] | Checked |

### Distortion

Introduces distortion using non-linear transfer functions.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Distortion Type** | Defines which model of distortion will be applied | * Reciprocal *Inspired by the function: y = 1 / (abs(x) + 1)* * SoftClipper *Two-stage quadratic clipping function* * HardClipper *Limits all samples above threshold to the threshold value* * BufferNormalize *All samples in the buffer (10ms) are normalized to their peak value. Introduces heavy compression with crackling* * Foldback *Waveform, instead of being clipped off, is reflected back like a mirror so that the top of the waveform is inverted.* | Unchecked |
| **Drive** | In the case of the Reciprocal type, it boosts the input signal. For other types, this value is used with an inverted sign as a threshold. On the output, there is gain compensation applied, so the output loudness is not affected by the Drive value too much | [0, 60] dB | Checked |

### Dynamic Equalizer

Implements dynamic range compression for a specified part of the frequency spectrum.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Biquad Type** | Type of filter, compression will be applied to affected frequency bands | * PeakingEQ *Compression will be applied around an area of which **Fc** represents the center frequency.* * LowShelf *Compression will be applied below **Fc*** * HighShelf *Compression will be applied above **Fc*** | Unchecked |
| **Fc** | Cutoff/center frequency of the filter | [20, 16000] | Checked |
| **Q** | Factor that controls the slope of the filter shape | [0.1, 20] | Checked |
| **Ratio** | Specifies the value of *n* in the ratio *n*:1. This value controls the amount of gain reduction (e.g. a ratio of 2:1 tells us that if the input signal is 2 dB over the value of **Threshold**, the resulting output level is reduced by 1 dB). | [1, 50] | Checked |
| **Threshold** | Threshold value of the input level above which the gain begins to be reduced | [-50, 0] dB | Checked |
| **Attack** | Specifies the amount of time [ms] that the output level takes to reach the value defined by **Ratio** | [1, 1000] ms | Checked |
| **Release** | Specifies the amount of time [ms] that the compressor takes to "undo" gain reduction in the case that the signal level has decreased | [1, 1000] ms | Checked |

### Equalizer4

Implements a 4-band equalizer

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Gain1** | Gain [dB] of the first frequency band | [-18, 18] | Checked |
| **Gain2** | Gain [dB] of the second frequency band | [-18, 18] | Checked |
| **Gain3** | Gain [dB] of the third frequency band | [-18, 18] | Checked |
| **Gain4** | Gain [dB] of the fourth frequency band | [-18, 18] | Checked |
| **Fc1** | Cutoff frequency of the lowshelf filter | [50, 800] | Checked |
| **Fc2** | Center frequency of the first peak filter | [200, 3000] | Checked |
| **Fc3** | Center frequency of the second peak filter | [1000, 8000] | Checked |
| **Fc4** | Cutoff frequency of the highshelf filter | [4000, 16000] | Checked |
| **Qfactor2** | Factor that controls the slope of the first peak filter's shape | [0.1, 20] | Checked |
| **Qfactor3** | Factor that controls the slope of the second peak filter's shape | [0.1, 20] | Checked |

### Flanger

The algorithm mixes two identical signals together, one signal delayed by a small and gradually changing period.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Delay Time** | Delay time of signal mixed to input signal | [0.1, 20] ms | Checked |
| **Feedback** | Feedback loop gain. If -1 or 1, DSP will self oscillate. | [-1, 1] | Checked |
| **Frequency** | Sin shape LFO modulation frequency | [0.1, 10] Hz | Checked |
| **Depth** | LFO depth | [0, 1] | Checked |
| **Damping** | Controls Lowpass filter applied on the feedback loop | [20, 20000] Hz | Checked |
| **Spread** | Defines the LFO offset between the channels | [0, 1] | Checked |

### QuadDelay

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Delay Time 1** | Left front delay line delay time. | [10, 600] ms | Checked |
| **Delay Time 2** | Left right delay line delay time. | [10, 600] ms | Checked |
| **Delay Time 3** | Left back delay line delay time. | [10, 600] ms | Checked |
| **Delay Time 4** | Right back delay line delay time. | [10, 600] ms | Checked |
| **Attenuation Factor** | Adjusts distance attenuation of delayed sound calculated based on delay time. | [0, 1] | Unchecked |
| **Damping Min** | Damping at Delay Time = 10ms. Interpolated between min/max based on delay time. | [0, 1] | Unchecked |
| **Damping Max** | Damping at Delay Time = 600ms. Interpolated between min/max based on delay time. | [0, 1] | Unchecked |
| **Feedback Min** | Feedback at Delay Time = 10ms. Interpolated between min/max based on delay time. | [0, 1] | Unchecked |
| **Feedback Max** | Feedback at Delay Time = 600ms. Interpolated between min/max based on delay time. | [0, 1] | Unchecked |
| **Azimuth** | Sets rotation of delay lines. | [-180, 180] deg | Checked |

### LoudnessNormalization

(Tries to) keep a specified output volume by automatically adjusting the gain of the input signal

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Target Level** | Targetted output RMS level [dBFS] | [-60, 0] dBFS | Checked |
| **Response Time** | Specifies how quickly [s] the normalization reacts to changes in the input signal | [0.1, 10] s | Checked |

### MonoToStereo

Creates a pseudo stereo output from a mono input.

**Works only for channels = 2 setup.**

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Delay Time** | outL = -delayIn outR = delayIn | [0.1, 20] ms | Checked |

### OnePoleFilter

Implements a one pole filter. It has a 6dB/oct slope and is cheaper to compute than a biquad filter.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Filter Type** | Type of Filter | * LowPass * HighPass | Unchecked |
| **Frequency** | Cutoff Frequency of the filter | [40, 15000] | Checked |

### PeakLimiter

Implements a limiter that uses a "Lookahead". Meaning a slight delay (5ms) in audio will be introduced in order to react even to quick peaks.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Threshold** | Threshold value of the input level above which the gain begins to be reduced | [-50, 0] dB | Checked |
| **Attack** | Specifies the amount of time [ms] that the output level takes to reach the value defined by the limiter's ratio | [1, 1000] ms | Checked |
| **Release** | Specifies the amount of time [ms] that the compressor takes to "undo" gain reduction in the case that the signal level has decreased | [1, 1000] ms | Checked |

### Phaser

A cascade of 7 first order all-pass filters modulated by a triangle-shaped LFO

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Frequency** | LFO modulation frequency | [0.1, 10] Hz | Checked |
| **Feedback** | Feedback loop gain. If -1 or 1, DSP will self oscillate. | [-1, 1] | Checked |
| **Offset** | Works only if channelCount > 1 Can be set via a signal, but is evaluated only at the playback start and can not be changed during playback.  Adds start offset to LFO. Offsets are spread evenly between channels. The last channel's LFO is offset by 0.5 \* offset / period. | [0, 1] | Checked without effect (bug) |
| **Spread** | Affects all pass filter frequency settings spread. If 0, all filters are set to the same frequency. | [0, 1] | Checked |

### Reverb

Implements a reverb, designed to model the reverb of a sound emitted into a room as perceived by a listener inside this room

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Room Dimension** | Dimensions of the modeled room | n/a | Unchecked |
| **Source Position** | Position of the sound source within the room | n/a | Unchecked |
| **Mic Position** | Position of the listener within the room | n/a | Unchecked |
| **Absorption** | Absorption factor, emulating absorption of sound from the room's surfaces. | [0, 1] | Unchecked |
| **Damping** | Damping factor of the reverb. Higher value means more damping of higher frequencies | [0, 1] | Unchecked |

### Reverb2

Implements a reverb.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Room Size** | Arbitrary factor for the size of the modeled room | [0, 1] | Unchecked |
| **Damping** | Damping factor of the reverb. Higher value means more damping of higher frequencies | [0, 1] | Unchecked |
| **Width** | Stereo width of the reverb | [0, 1] | Unchecked |

### SmallRoomReverb

Implements a reverb, designed to model reverb within small rooms

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Early Reflections Time** | Arbitrary time factor influencing how long it takes for early reflections to reach the listener | [0, 1] | Checked without effect (bug) |
| **Late Reflections Time** | Arbitrary time factor influencing how long it takes for late reflections to reach the listener | [0, 1] | Checked |
| **Late Reflections Density** | Density factor of late reflections | [0, 1] | Checked |
| **Late Reflections Damping** | Damping factor of the late reflections. A higher value means more damping of higher frequencies | [0, 1] | Checked |
| **Width** | Stereo width of the reverb | [0, 1] | Checked |

### Tremolo

Amplitude modulation using a triangle-shaped LFO.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Frequency** | Frequency of the LFO | [0.1, 100] Hz | Checked |
| **Offset** | Works only if channelCount > 1 Can be set via a signal, but is evaluated only at the playback start and can not be changed during playback.  Adds start offset to LFO. Offsets are spread evenly between channels. The last channel's LFO is offset by 0.5 \* offset / period. | [0, 1] | Checked |

### VariableRolloffLPF

A lowpass filter with variable rolloff

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Fc** | Cutoff frequency of the filter | [100, 400] Hz | Checked |
| **Rolloff** | Filter rolloff | [0, 30] | Checked |
| **Order** | Filter order | [2, 3] | Unchecked |

### NoiseGate

A noise gate is an audio processor that works to eliminate sounds below a given threshold.

| Attribute Name | Description | Value Range | Input Port |
| --- | --- | --- | --- |
| **Threshold** | Defines level at which gate opens. | [-60, 0] dB | Checked |
| **Attack** | Specifies the amount of time [ms] that takes for gate to fully open. | [0, 500] ms | Checked |
| **Release** | Specifies the amount of time [ms] that the gate to fully close. | [0, 500] ms | Checked |

1. [↑](#cite_ref-1 "Jump up") <https://www.earlevel.com/main/2013/10/13/biquad-calculator-v2/>
2. [↑](#cite_ref-2 "Jump up") [Audio bit depth](https://en.wikipedia.org/wiki/Audio_bit_depth)
3. [↑](#cite_ref-3 "Jump up") [Dynamic range compression](https://en.wikipedia.org/wiki/Dynamic_range_compression)
