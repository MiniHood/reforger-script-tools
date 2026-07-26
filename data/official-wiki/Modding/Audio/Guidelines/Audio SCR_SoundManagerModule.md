# [Audio: SCR_SoundManagerModule](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_SCR_SoundManagerModule)

[SCR\_SoundManagerModule](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_SoundManagerModule.c;11) is a module that plays simple one-shot sounds without the need for a [SoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SoundComponent.c;12) to exist on a given entity.

* SCR\_SoundManagerModule is a core entity present in every world.
* If functions in SCR\_SoundManagerModule are enough for a given sound, prioritise it before adding a [SoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SoundComponent.c;12) on the entity.
* If the entity already has [SoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SoundComponent.c;12) because of some other sound, do not use SCR\_SoundManagerModule.
* Signals for a given sound can be set only before the sound playback and cannot be updated during the sound playback.

⚠

* SCR\_SoundManagerModule is **not** present on headless applications (e.g dedicated server).
* Do **not** use SCR\_SoundManagerModule for UI sounds.
* Do **not** use SCR\_SoundManagerModule for managing looped sounds. Looped sounds always need SoundComponent to work properly in multiplayer.

ⓘ

Before [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)"), SCR\_SoundManagerModule was named SCR\_SoundManagerEntity.

## Signals

* By default, the following Variables are used:
  + GCurrVehicleCoverage
  + GIsThirdPersonCam
  + GInterior
* the Distance signal can be enabled using the flags in [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10)
* any additional feature-specific signals can be added using the [SCR\_AudioSource](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSource.c;12) API if needed.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)")

## SCR\_AudioSourceDefinition

* Class used for setting sound parameters
* This class is used in [SCR\_SoundDataComponent](enfusion://ScriptEditor/scripts/Game/Components/SCR_SoundDataComponent.c;8), or can be used in any feature in script if needed.
  + Sound Project (resource) - acp that contains m\_sSoundEventName
  + Sound Event Name (string) - Sound event name
  + Static (flag) - If false, sound will follow the entity's position

## SCR\_SoundDataComponent

[SCR\_SoundDataComponent](enfusion://ScriptEditor/scripts/Game/Components/SCR_SoundDataComponent.c;8) is the best way to configure multiple sounds

It contains an SCR\_AudioSourceDefinition array for sounds configuration and SCR\_SoundManagerModule contains methods that can work with this component directly.

## Usage

### SCR\_SoundManagerModule Scripting

Sound is defined using SCR\_AudioSourceConfiguration. You need it to play the sound. Two methods:

* use [SCR\_SoundDataComponent](enfusion://ScriptEditor/scripts/Game/Components/SCR_SoundDataComponent.c;8), which contains the [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10) array and is meant to be used when multiple scripts need to play sound on a given entity.
* define [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10) directly in a class. Useful when better control over [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10) is needed or when there is only a couple of events to be played on the entity

### SCR\_SoundDataComponent Workflow

Store the string event name in script, then two methods:

1. create and play sound using cCreateAndPlayAudioSource([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) owner, [string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) eventName)
2. alternatively create [SCR\_AudioSource](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSource.c;12) using cCreateAudioSource([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) owner, [string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) eventName) - checks if the sound is in the audible range and play [SCR\_AudioSource](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSource.c;12) using cPlayAudioSource([SCR\_AudioSource](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSource.c;12) audioSource) - sets occlusion signals, distance signal, triggers the sound, and adds SCR\_AudioSource to the pool.

### SCR\_AudioSourceConfiguration Workflow

Validate a SCR\_AudioSourceConfiguration instance, then two methods:

* create and play sound using cCreateAndPlayAudioSource([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) owner, [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10) audioSourceConfiguration)
* alternatively Create a SCR\_AudioSource using cCreateAudioSource([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) owner, [SCR\_AudioSourceConfiguration](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSourceConfiguration.c;10) audioSourceConfiguration) and play SCR\_AudioSource using cPlayAudioSource([SCR\_AudioSource](enfusion://ScriptEditor/scripts/Game/Systems/Sound/SCR_AudioSource.c;12) audioSource)
