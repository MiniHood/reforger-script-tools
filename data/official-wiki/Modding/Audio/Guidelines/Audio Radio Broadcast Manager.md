# [Audio: Radio Broadcast Manager](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Radio_Broadcast_Manager)

The Radio Broadcast system is used to play DJ lines and music from radios diegetically within the gameworld and sync their states across all clients, ensuring that all players hear the same audio.

* The Radio Broadcast Manager itself is implemented from GameCode as a placeable Entity and must be present in the world in order to work.
* The main Prefab of the Radio Broadcast Manager can be found in Prefabs/Systems/Radio: [RadioBroadcastManager.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Systems/Radio/RadioBroadcastManager.et)
* Functionality-wise, the Radio Broadcast Manager creates a fixed playlist and syncs its state across clients whenever they need to access it.

## Properties

| Manager Settings | |
| --- | --- |
| Broadcast Acp | Acp the DJ lines and music are stored in. |
| Regenerate Times | Checking it will perform an update of the saved playback durations for files within the .acp specified in Broadcast Acp |
| Start Music | If checked, the radio playback will start with music. Otherwise it will start with the DJ |
| Music Bank Name | Name of the bank that stores the music within the .acp specified in Broadcast Acp |
| Music Times | Automatically generated and cannot be edited. Stores the playback durations for all music. Can be updated by checking the "Regenerate Times" parameter |
| DJ Bank Name | Name of the bank that stores the DJ lines within the .acp specified in Broadcast Acp |
| DJ Times | Automatically generated and cannot be edited. Stores the playback durations for all DJ lines. Can be updated by checking the "Regenerate Times" parameter |

## Setup

### Radio Broadcast Manager

Reforger uses only one Radio Broadast Manager Prefab, but new ones can be created from scratch or by overriding the present one.

The manager entity setup looks as follows:

1. Add the desired .acp to the "Broadcast Acp" attribute
2. Enter the exact name of the bank that stores the music within the .acp in the "Music Bank Name" attribute
3. Enter the exact name of the bank that stores the DJ lines within the .acp in the "DJ Bank Name" attribute
4. Check the "Regenerate Times" attribute in order to automatically calculate and store the lengths of all files within the Music and DJ banks.

   ⚠

   Whenever files are added or removed to the banks, the times need to be regenerated.
5. The Radio Broadcast Manager needs to be placed into the gameworld in order to work.

### Signals

The Radio Broadcast Manager system will set the following Signals:

| Signal Name | Description |
| --- | --- |
| Offset | Used to offset the currently playing file/bank in order to sync the playback time across all clients. |
| MusicTrackID | Returns the ID (index as an integer) of the currently playing track. |
| DJTrackID | Returns the ID (index as an integer) of the currently playing DJ lines. |
| BroadcastType | Returns 0 or 1 (bool). 0 = Play DJ lines 1 = Play music |

A .sig file with all signals already exists in Sounds/RadioBroadcast: [RadioBroadcast.sig](enfusion://ResourceManager/~ArmaReforger:Sounds/RadioBroadcast/RadioBroadcast.sig)

### ACP Banks

On the most basic level, a RadioBroadcast .acp should contain at least **a bank for music** and **a bank for DJ lines**.
As mentioned above, the name of these two banks must match the names for music and DJ banks specified in the Radio Broadcast Manager. Furthermore:

* The Offset Signal must be connected to the "Offset" input ports of both banks in order to properly sync the playback position
* The MusicTrackID Signal must be connected to the "Index" port of the music bank
* The DJTrackID Signal must be connected to the "Index" port of the DJ bank
* Both banks must have their "Selection" attribute set to "CustomSignalIndex"
* A Selector node steered by the BroadCastType Signal must be used in order to correctly switch between music and DJ lines

### Radio

In order for a radio (or any other entity) to play music and DJ lines to work, it needs:

* An [RplComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/RplComponent.c;17) (replication)
* A [SignalsManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SignalsManagerComponent.c;12) (storing Signals)
* An [ActionsManagerComponent](enfusion://ScriptEditor/scripts/GameCode/UserAction/ActionsManagerComponent.c;5) (creating the UserAction for turning on the radio)
* A [RadioBroadcastComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/RadioBroadcastComponent.c;12) (linking the UserAction to the Radio Broadcast functionality and interfacing between Manager and RadioBroadcastSoundComponent)
* A [RadioBroadcastSoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/RadioBroadcastSoundComponent.c;16)

#### RadioBroadcastComponent

On the RadioBroadcastComponent, the Action created in the ActionsManagerComponent must be selected as a class in "Turn On Action" → "Parent Context List"

#### RadioBroadcastSoundComponent

On the RadioBroadcastSoundComponent, the same .acp file must be defined as in the actual [Radio Broadcast Manager](#Radio_Broadcast_Manager).

## Sound Events

The Sound Events are hardcoded in GameCode and can be found at [Audio: Sound Events - RadioBroadcastSoundComponent](/wiki/Arma_Reforger:Audio:_Sound_Events#RadioBroadcastSoundComponent "Arma Reforger:Audio: Sound Events").
