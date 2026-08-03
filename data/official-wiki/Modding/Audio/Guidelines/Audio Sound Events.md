# [Audio: Sound Events](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Sound_Events)

Sound Events are strings serving as identifiers for the sound system.
They are defined by naming a [Sound Node](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Sound "Arma Reforger:Audio Editor: Nodes") and used by calling a Sound Event with the same name, e.g. via code or script.

## Script Events

ⓘ

All script events are stored in [SCR\_SoundEvent](enfusion://ScriptEditor/scripts/Game/Helpers/SCR_SoundEvent.c;1).

## GameCode Events

### BaseMuzzleComponent

| Event Name | Description |
| --- | --- |
| SOUND\_DRY | When firing with no bullets in magazine. |
| SOUND\_DRY\_SAFETYON | When firing with safety on. |
| SOUND\_RELOAD | When reloading ACK. Not used right now. Animations are used instead. |

### CaseEjectingEffectComponent

| Event Name | Description |
| --- | --- |
| SOUND\_BULLET\_CASING | Upon collision with the ground. |

### CharacterSoundComponent

| Event Name | Description |
| --- | --- |
| SOUND\_BODYFALL | Upon character falling (Ragdoll). |
| SOUND\_BODYFALL\_TERMINAL | Upon character falling, resulting in death. |
| SOUND\_CHAR\_RAGDOLL | When impacting in Ragdoll state. |
| SOUND\_SWIM\_START | When character starts swimming. |
| SOUND\_SWIM\_STOP | When character stops swimming. |
| SOUND\_WATER\_ENTER | When entering water surface when EntryLevel over threshold *(0.1f).* |
| SOUND\_WATER\_ENTER\_RAGDOLL | When entering water in ragdoll state. |
| SOUND\_WATER\_EXIT | When exiting water surface. |

### CommunicationSoundComponent

| Event Name | Priority | Description |
| --- | --- | --- |
| SOUND\_HIT | 99 | * Played when character gets damage * Player   + Grunt * AI   + Projectile damage: Would play sound that is connected to SOUND\_REPORTS\_STATUS\_HIT   + Other damage: Grunt   **Signals:**   * IsAI - Player = 0, AI = 1, Set on SignalsManagerComponent * DamageType |
| SOUND\_DEATH | N/A | When character dies |
| SOUND\_BREATH\_IN | -20 | Breathing in of character. |
| SOUND\_BREATH\_OUT | -20 | Breathing out of character. |
| SOUND\_BREATH\_REG\_IN | -20 | Breathing in (regular) of character. |
| SOUND\_BREATH\_REG\_OUT | -20 | Breathing out (regular) of character. |
| SOUND\_BREATH\_END | -20 | Upon stamina restored and breathing stops. |

### DoorComponent / SoundPointEventInfo

| Event Name | Description |
| --- | --- |
| SOUND\_OPEN\_START | When opening the door from fully closed state. |
| SOUND\_OPEN\_FINISH | When door is fully opened from closed state. |
| SOUND\_CLOSE\_START | When closing the door from fully opened state. |
| SOUND\_CLOSE\_FINISH | When door is fully closed from open state. |
| SOUND\_MOVEMENT | When the door is opening/closing. |

### ExplosionEffect

| Event Name | Description |
| --- | --- |
| *Custom Event* | Raised when explosion triggered. |

### GrenadeMoveComponent

| Event Name | Description |
| --- | --- |
| SOUND\_HIT | Upon contact & deflection of grenade. |

### HitSoundEffect

| Event Name | Description |
| --- | --- |
| SOUND\_HIT | Upon bullet hitting surface / player. |

### MusicManagerController

| Event Name | Description |
| --- | --- |
| SOUND\_MILITARYBASE | Theme played at military bases. |
| SOUND\_FOREST | Theme played in forests. |
| SOUND\_VILLAGE | Theme played in villages. |
| SOUND\_COASTLINE | Theme played at coastlines. |

### ProjectileSoundsModule

| Event Name | Description |
| --- | --- |
| SOUND\_SONIC\_CRACK | Upon sonic crack when speed > SpeedOfSound. |
| SOUND\_SONIC\_CRACK\_SECONDARY | Upon projectile transitioning from supersonic to subsonic. |
| SOUND\_SONIC\_CRACK\_IMPACT | Upon projectile impacting before it reaches the listener. |
| SOUND\_FLYBY | Upon sonic crack when speed  <= SpeedOfSound. |

### RadioBroadcastSoundComponent

| Event Name | Description |
| --- | --- |
| SOUND\_RADIO\_TURN\_ON | When turning on the radio. |
| SOUND\_RADIO\_TURN\_OFF | When turning off the radio. |
| SOUND\_RADIO | After turning the radio on. |

### Vehicle

| Event Name | Description |
| --- | --- |
| SOUND\_COLLISION | * In a collision between 2 vehicles, sound is played from vehicle with higher mass. * use eventNoRepeatTime to filter out duplicate collisions. |

### VoNComponent

| Event Name | Description |
| --- | --- |
| VON\_DIRECT | VON for direct speech |
| VON\_RADIO | VON for radio transmissions |
| VON\_RAW |  |

### WeaponSoundComponent

| Event Name | Description |
| --- | --- |
| SOUND\_MELEE\_IMPACT | Upon dealing melee damage. |
| SOUND\_SHOT | Upon firing a weapon. |
| SOUND\_SHOT\_END | After firing stops. |
| SOUND\_THROWN | After a throwable item (e.g. grenades) has been thrown. |
| SOUND\_WPN\_TOSAFETY | When changing the fire mode to safety. |
| SOUND\_WPN\_TOAUTO | When changing the fire mode to auto. |
| SOUND\_WPN\_TOBURST | When changing the fire mode to burst. |
| SOUND\_WPN\_TOSEMIAUTO | When changing the fire mode to semi-auto. |
| SOUND\_DRY\_SAFETYON | When firing with safety on. |
| SOUND\_WPN\_TOUGL | When changing muzzle to UGL. |
| SOUND\_ZEROING\_DOWN | Upon zeroing down. |
| SOUND\_ZEROING\_UP | Upon zeroing up. |
