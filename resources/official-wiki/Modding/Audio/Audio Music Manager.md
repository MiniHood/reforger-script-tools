# [Audio: Music Manager](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Music_Manager)

The **Music Manager** system, as the name suggests, manages Arma Reforger's music playback.

* The Music Manger itself is implemented from GameCode as a pleacable Entity ([MusicManager](enfusion://ScriptEditor/scripts/Game/generated/Entities/MusicManager.c;16)) and must be present in a world in order to work.
* **Prefabs** of the Music Manager for different scenarios can be found in *Prefabs → Sounds → Music* (e.g [MusicManager\_Campaign.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Sounds/Music/MusicManager_Campaign.et)).
* Within the Music Manager, **Music Classes** can be added. These music classes inherit either from [LocationMusic](enfusion://ScriptEditor/scripts/Game/generated/Music/LocationMusic.c;12) or [ScriptedMusic](enfusion://ScriptEditor/scripts/Game/generated/Music/ScriptedMusic.c;12) and signal the Manager when they want to play music.
* The Music Manager handles what music to play based on a **priority system**.

## Properties

| Fade Out Settings | |
| --- | --- |
| Fade out time interruption | Time (in ms) it takes for the current music to fully fade out when interrupted by an in-game event, e.g by a gunshot or explosion. |
| Fade out time force play | Time (in ms) it takes for the current music to fully fade out when the Music Manager instructs to forcefully play another music. |
| Music Settings | |
| Time between music | Time (in s) of rest between finishing a track and starting the next one. Does not apply when forcefully playing music. |
| Min interruption volume | Minimum volume of an "interruption event" required to stop the currently playing track. |
| Music (Array) | Array of all Music classes. |

## Music Classes

### LocationMusic

| Properties | |
| --- | --- |
| Acp | .acp file to play the sounds/music from. |
| Priority | Priority of the music. |
| Interruptible | Whether or not it can be interrupted by "interrupting" in-game events. |
| Time between repeats | Time (in s) of rest between finishing a track and starting the next one. |
| Category | See [Categories](#Categories) |

The primary use case for the LocationMusic class is it to play music based on the player's location, using the SoundMap:

| SoundMap | Sound Events |
| --- | --- |
| EForestProperty | SOUND\_FOREST |
| ESeaProperty | SOUND\_COASTLINE |
| ECityProperty | SOUND\_VILLAGE |

### ScriptedMusic

| Properties | |
| --- | --- |
| Acp | .acp file to play the sounds/music from. |
| Music Name | Name of the Sound Event |
| Priority | Priority of the music. |
| Interruptible | Whether or not it can be interrupted by "interrupting" in-game events. |
| Time between repeats | Time (in s) of rest between finishing a track and starting the next one. |
| Category | See [Categories](#Categories) |

As the name suggests, [ScriptedMusic](enfusion://ScriptEditor/scripts/Game/generated/Music/ScriptedMusic.c;12) is used in order to play specific tracks / Sound Events under specific conditions.
These conditions are directly scripted in the respective child of ScriptedMusic itself.

Example: [SCR\_RespawnMusic](enfusion://ScriptEditor/scripts/Game/Music/SCR_RespawnMusic.c;1) contains the logic to play the respawn music upon entering the deployment screen.

The most straightforward way of accessing the [MusicManager](enfusion://ScriptEditor/scripts/Game/generated/Entities/MusicManager.c;16) within a ScriptedMusic class looks as follows:

```enforce
override void Init()
{
	ChimeraWorld world = GetGame().GetWorld();
	if (!world)
	return;
	m_MusicManager = world.GetMusicManager();
	if (!m_MusicManager)
	return;
}
```

## Categories

The Music Manager system features (hardcoded) Categories. Categories can be assigned for each Music class in its Category section.
The Category system is used to mute all tracks belonging to a category, using the cMuteCategory() and cServerRequestMuteCategory() methods.

List of available categories:

* Ambient
* Menu
* Misc
* USER1
* USER2
* USER3

## Music Interruption

Music can be Interrupted by in-game events if:

* the event's volume is equal to or exceeds the value defined in **"Min interruption volume"** and
* the music's **"Interruptible" flag** is set to true.

These in-game events are hardcoded in GameCode and currently consist of **weapon shots** and **explosions using ExplosionEffect**.
How long it takes for the music to fully fade out when interrupted is defined in the Music Manager's **"Fade out time interruption"** attribute.

## ACP Setup

Music generally does not require any specific .acp setup, but any sound node must link into the Mixer's "Music" port.
