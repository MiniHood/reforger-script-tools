# [Audio Editor: Directivity](https://community.bistudio.com/wiki/Arma_Reforger:Audio_Editor:_Directivity)

Directivity is a property of sound sources whereby the perceived "color" of a sound is dependent on which way the sound source is facing with respect to the listener.
Typically, the more off-axis the listener is positioned with respect to the sound source's direction, the more "dark" the sound is perceived to the listener due to the attenuation of high frequencies.

[![](/wikidata/images/thumb/4/43/armaR-sound_directivity_fig_1.jpg/500px-armaR-sound_directivity_fig_1.jpg)](/wiki/File:armaR-sound_directivity_fig_1.jpg)

Figure 1: Illustration of the directivity of a human voice. The sound is most clear when the listener is positioned on the axis of the "direction vector"

## Implementation

The directivity effect can be modeled using a low-pass filter with a fixed cutoff frequency and variable rolloff, which is modulated by the angle between the listener and the sound source's direction.
Thus, the filter's behaviour can be fully defined with the following parameters:

* **fc**: the filter's cutoff frequency
* **maxRolloff**: the maximum filter's rolloff, in dB/decade, when the listener is positioned directly behind the sound source (180 degrees off axis)

Figure 2 illustrates how the rolloff of the filter transitions from 0 to maxRolloff (in this case 20 dB/dec) based on the listener's angle WRT the on-axis of the sound source.

[![](/wikidata/images/thumb/8/8a/armaR-sound_directivity_fig_2.jpg/500px-armaR-sound_directivity_fig_2.jpg)](/wiki/File:armaR-sound_directivity_fig_2.jpg)

Figure 2: Filter rolloff (using maxRolloff = 20) as a function of the listener's angle WRT the on-axis of the sound source

As no direct implementation of a variable-rolloff lowpass filter exists in the digital domain, it has been decided to approximate the target filter's response with a single high-shelf biquad filter, whose parameters were tuned to fit the target's responses as closely as possible.

Figure 3 illustrates the responses of the modeled filter (colored lines) compared to the corresponding target responses (grey dotted lines) across the full range of possible listener angles (WRT source direction).

[![](/wikidata/images/thumb/3/3c/armaR-sound_directivity_fig_3.jpg/500px-armaR-sound_directivity_fig_3.jpg)](/wiki/File:armaR-sound_directivity_fig_3.jpg)

Figure 3: Modeled vs target filter responses

## Usage

The directivity feature can be accessed in the audio editor via the "Frequency" node. The "Directivity" section of the node provides the user control over the parameters listed above for controlling the filter's behaviour.

## References

1. <https://www.researchgate.net/publication/291154227_SpatOSC_Providing_abstraction_for_the_authoring_of_interactive_spatial_audio_experiences>
