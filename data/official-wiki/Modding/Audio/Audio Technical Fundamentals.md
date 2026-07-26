# [Audio: Technical Fundamentals](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Technical_Fundamentals)

This page aims to give a high-level understanding of how the audio pipeline within (and from) Arma Reforger works.

## The Audio Pipeline

[![](/wikidata/images/thumb/1/15/armaR_audio_technical_fundamentals.png/300px-armaR_audio_technical_fundamentals.png)](/wiki/File:armaR_audio_technical_fundamentals.png)

A simple, but factual relationship between the different elements.

Depicted is a high-level overview of the game's audio pipeline.
Below is a short explanation of its inner workings.
In order to understand it better, there are more elaborate explanations for each element further below, with some leading to dedicated pages.

### Elements Rundown

* The GameWorld is filled with [Game Entities](#Game_Entities).
* Entities that have a [SoundComponent](#SoundComponent) are able to play sounds.
* These sounds are defined in [Acp Files](#Acp_Files) *via* [Sound Nodes](#Sound_Nodes).
* [Signals](#Signals) are used to interface between the GameWorld and the Audio. They are stored in and accessible from an Entity's [SignalsManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SignalsManagerComponent.c;12) as well as the [GameSignalsManager](enfusion://ScriptEditor/scripts/Game/generated/GameSignalsManager.c;7).
* In a similar way as Global Signals, [Variables](#Variables) can be used to pass values from the GameWorld to the Audio System and vice versa.
* The **GameCode or Scripts** can instruct an Entity's [SoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SoundComponent.c;12) to play a Sound with a given [Sound Event](#Sound_Events) Name.
  + The SoundComponent will then search through its assigned acp files and look for [Sound Nodes](#Sound_Nodes) whose names match the given Sound Event name. If a matching Sound Node was found, it will be processed based on its audio signal chain, taking into account available [Signals](#Signals).
  + The sound will be sent to the a submix input of the [Mixer (FinalMix)](#Mixer) where the submixes are further processed.
  + Enfusion's **Audio System** will handle the main processing of the audio generation.
  + The generated audio will be forwarded to the system'S audio API, e.g [WASAPI](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi) (a Windows audio API), in order to actually play back the sound on the user's system.

### Game Entities

The Enfusion Engine uses a modular entity system. An Entity in and on itself is just a set of functionalities defined via GameCode or even a completely "empty shell".
It is the modules inside, called "Components", that determine the majority of its characteristics.
Therefore, Entities in our GameWorld can be anything - from concrete, complex objects like player characters, weapons and buildings, to abstract systems like an entity that manages the rules of a gamemode.

In order to play a sound, an Entity needs to have a variation of the [SoundComponent](#SoundComponents).

### SoundComponents

ⓘ

See [Audio: Sound Components](/wiki/Arma_Reforger:Audio:_Sound_Components "Arma Reforger:Audio: Sound Components").

### Acp Project Files

Acp project files

**Acp** (AudioComponent) **files** determine how sounds behave ingame. Using the [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor"), .acp files allow the creation of complex audio processing chains that result in the final sound audible in-game.

Furthermore, .acp files are a mandatory part of the game's audio pipeline, as they contain the Sound names referenced by the [SoundComponent](#SoundComponents) when it is instructed to play a sound.

### Sound Nodes

ⓘ

See [Audio Editor: Nodes - Sound](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Sound "Arma Reforger:Audio Editor: Nodes").

### Sound Events

ⓘ

See [Audio: Sound Events](/wiki/Arma_Reforger:Audio:_Sound_Events "Arma Reforger:Audio: Sound Events").

### Signals

ⓘ

See [Audio: Signals](/wiki/Arma_Reforger:Audio:_Signals "Arma Reforger:Audio: Signals").

### Sound Events

ⓘ

See [Audio: Sound Events](/wiki/Arma_Reforger:Audio:_Sound_Events "Arma Reforger:Audio: Sound Events").

### Variables

ⓘ

See [Audio Editor: Variables](/wiki?title=Arma_Reforger:Audio_Editor:_Variables&action=edit&redlink=1 "Arma Reforger:Audio Editor: Variables (page does not exist)").

### Mixer

ⓘ

See [Audio Editor: Nodes - Mixer](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Mixer "Arma Reforger:Audio Editor: Nodes").

## FAQ

Do Entities need a SignalsManagerComponent to use Signals?
:   Yes and no. Some variations of the [SoundComponent](#SoundComponents) set some [Signals](#Signals) automatically, like the distance of the Entity to the current listener.
:   Using the cAddOrFindSignal() method, a Signal can also be set when playing a sound, but it will not be stored due to the lack of a [SignalsManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SignalsManagerComponent.c;12).

How are Signals set on an Entity's SignalsManagerComponent?
:   Some [Signals](#Signals) are set via GameCode and cannot be altered, such as the distance of the Entity to the current listener.
:   Signals can also be set via scripting by using the cAddOrFindSignal() and cAddOrFindMPSignal() methods.

## See Also

* [Audio Editor Documentation](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor")
* [Audio Script Documentation](/wiki?title=Arma_Reforger:Audio_Script_Documentation&action=edit&redlink=1 "Arma Reforger:Audio Script Documentation (page does not exist)")
* [Audio: SoundComponents](/wiki?title=Arma_Reforger:Audio:_SoundComponents&action=edit&redlink=1 "Arma Reforger:Audio: SoundComponents (page does not exist)")
* [WASAPI (Windows Audio Session API) documentation](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi)
