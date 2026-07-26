# [Audio: Voice over Network](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Voice_over_Network)

The Voice over Network (VoN) system handles the voice communication between players, for both direct speech and transmitting via radios.
In order to either send or receive VoN, an entity is required to have a [VoNComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/VoNComponent.c;19).

On a basic level, the "VoN Pipeline" looks as follows:

* The VoNComponent gets a compressed (OPUS) audio stream as packets from Enfusion's AudioCaptureDevice class.
  + The AudioCaptureDevice also handles Silence Detection and Audio Normalization.
  + The audiostream has a hardcoded ID (This simply being "Stream").
* This data is then sent to the Server. It determines which clients should be able to hear the audiostream.
* The server distributes the audiostream to the determined clients. Their character's VoNComponent decodes it, sets Signals and plays it back, routed through the ACP specified on the **receiver**'s VoNComponent.

## Direct Speech VoN

Prerequisites
:   Sender and receiver both need to have a VoNComponent with a properly set up ACP

Functionality
:   If the sender is talking and the receiver is within the "Speaking Range" defined in the sender's VoNComponent, the receiver will hear the sender via the VoN\_DIRECT Sound Event.

## Radio VoN

Prerequisites
:   * Sender and receiver both need to have a VoNComponent with a properly set up ACP
    * A [RadioManagerEntity](enfusion://ScriptEditor/scripts/Game/generated/Entities/RadioManagerEntity.c;16) must be present within the GameWorld
    * Sender and receiver must each possess a radio item set up with a [BaseRadioComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/BaseRadioComponent.c;16)

Functionality
:   If the sender is transmitting on their radio and the receiver is within radio range, on the same frequency and using the same encryption key, the receiver will hear the sender via the VoN\_RADIO Sound Event. The Signals listed in the Signals section will be set.

## Related Classes

### VoNComponent

| Attribute | Description |
| --- | --- |
| Filename | Reference to the ACP file used for VoN. |

### BaseRadioComponent

Radio properties

| Attribute | Description |
| --- | --- |
| Encryption key | Key (string) that will be checked against for encryption. Radios can only receive from and transmit to radios with the same encryption key. |
| Turned On | Whether or not the radio starts as turned on. |

Unsorted (transceiver)

| Attribute | Description |
| --- | --- |
| Channel Frequency | Default frequency of the transceiver |
| Transmitting Range | Maximum transmitting range (m) of the transceiver |
| Min tunable frequency | Minimum tunable frequency (Hz) of the transceiver |
| Max tunable frequency | Maximum tunable frequency (Hz) of the transceiver |
| Frequency resolution | The "steps" (Hz) in which the transceiver can be cycled through |

### Radio Manager

No configurable attributes.

## Signals and Audio Variables

The following Signals will be directly set on the listener's ACP continuously during stream playback:

| Signal | Description |
| --- | --- |
| TransmissionQuality | Quality of the transmission (Float, 0..1) |
| Distance | Distance to the sender |
| Interior | Whether or not the sender is considered "inside" (0,1) |
| RoomSize | Size of the room the sender is in (m3) |
| UnderPlayerControl | Whether or not the sender is controlled by the local player (0,1) |
| IsInPlayerVehicle | Whether or not the sender is in the same vehicle as the player (0,1) |
| GInterior | Whether or not the current camera position is "inside" (Read from [GameSignalsManager](enfusion://ScriptEditor/scripts/Game/generated/GameSignalsManager.c;7)) |
| GRoomSize | Size (m3) of the room the camera is in (Read from [GameSignalsManager](enfusion://ScriptEditor/scripts/Game/generated/GameSignalsManager.c;7)) |
| GIsThirdPersonCam | Whether or not the player's camera is in third person (0,1) (Read from [GameSignalsManager](enfusion://ScriptEditor/scripts/Game/generated/GameSignalsManager.c;7)) |
| GCurrVehicleCoverage | The current coverage of the vehicle the player is in (0 if not in vehicle) (Read from [GameSignalsManager](enfusion://ScriptEditor/scripts/Game/generated/GameSignalsManager.c;7)) |

## Sound Events

The VoN sound events can be found here: [Audio: Sound Events - VoNComponent](/wiki/Arma_Reforger:Audio:_Sound_Events#VoNComponent "Arma Reforger:Audio: Sound Events").

## ACP Setup

VoN can be processed like any other audio signal within the Audio Editor, with two exceptions:

* The correct (hardcoded) Sound Event names must be used and
* A Stream node with the ID "Stream" must be used as an audio source in the signal chain.

## Pipeline

[![armaR-audio von pipeline.png](/wikidata/images/d/dd/armaR-audio_von_pipeline.png)](/wiki/File:armaR-audio_von_pipeline.png)
