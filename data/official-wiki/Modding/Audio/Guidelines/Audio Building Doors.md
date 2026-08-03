# [Audio: Building Doors](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Building_Doors)

Door sounds do **not** use [SoundComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/SoundComponent.c;12) and are played directly via the [AudioSystem](enfusion://ScriptEditor/scripts/Core/generated/Audio/AudioSystem.c;12).

## Prefab Configuration

* The .acp reference is configured on [DoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DoorComponent.c;12)'s **SoundFileName**.
* Sounds positions are configured on [DoorComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DoorComponent.c;12)'s **SoundPoints**.

## Events

* SOUND\_OPEN\_START - triggered only when the door is fully closed
* SOUND\_OPEN\_FINISH
* SOUND\_CLOSE\_START - triggered only when the door is fully open
* SOUND\_CLOSE\_FINISH
* SOUND\_MOVEMENT - triggered when the door starts to move, stopped when the door stops moving
