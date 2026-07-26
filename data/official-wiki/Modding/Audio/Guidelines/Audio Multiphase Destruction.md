# [Audio: Multiphase Destruction](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Multiphase_Destruction)

The **M**ulti**p**hase **D**estruction (MPD) system handles environmental destruction (e.g., breaking signs, crumbling walls, etc.).
Each MPD entity transitions through "damage phases", altering its model, spawning debris and particle effects, or emitting sounds.

* For efficiency, sounds are managed by the [Audio: SCR SoundManagerModule](/wiki/Arma_Reforger:Audio:_SCR_SoundManagerModule "Arma Reforger:Audio: SCR SoundManagerModule") system, not individual entities. Entities reference enums in [SCR\_DestructionMultiPhaseComponent](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionMultiPhaseComponent.c;20) for sound definitions.
* Sounds are triggered:
  1. When an entity's damage phase changes (using [SCR\_EMaterialSoundTypeBreak](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;335)).
  2. When spawned debris collides with the environment or other entities (using [SCR\_EMaterialSoundTypeDebris](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;309)).
* When triggering a sound:
  1. The enum string (e.g., "METAL\_HEAVY") combines with "SOUND\_MPD\_" to form the sound name (e.g., "SOUND\_MPD\_METAL\_HEAVY").
  2. The system plays the sound from the .acp file linked to the MPDestructionManager.

⚠

In order for Multiphase Destruction to work, a Multiphase Destruction Manager must be present within the world.

## Design

* All sound events are defined in one .acp: [Sounds\Destruction\Multiphase\Destruction\_Multiphase.acp](enfusion://ResourceManager/~ArmaReforger:Sounds/Destruction/Multiphase/Destruction_Multiphase.acp)
* A "manager" needs to be present in the gameworld: [Prefabs\MP\MPDestructionManager.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/MPDestructionManager.et)
* Available sound types are defined via enums ([SCR\_EMaterialSoundTypeDebris](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;309) and [SCR\_EMaterialSoundTypeBreak](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;335))

## MPD Enum Overview

Sounds use enum values. Pick one of the following two to mod:

* [SCR\_EMaterialSoundTypeBreak](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;335) if the sound is supposed to play when the entity changes its damage phase.  
  In this case, the enum must start with "BREAK\_", e.g. "BREAK\_GLASSBOTTLE";
* [SCR\_EMaterialSoundTypeDebris](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;309) if the sound is supposed to play when debris collides with the environment

Add the desired name to the correct enum through modding - make sure to choose a descriptive one. Example:

```enforce
modded enum SCR_EMaterialSoundTypeBreak
{
	BREAK_GLASSBOTTLE,
}
```

| Break Sounds | Debris Sounds |
| --- | --- |
| [SCR\_EMaterialSoundTypeBreak](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;335) | [SCR\_EMaterialSoundTypeDebris](enfusion://ScriptEditor/scripts/Game/Destruction/SCR_DestructionUtility.c;309) |
| Defined via the "Material Sound Type" entry in the "Unsorted" category . These sounds will play once when the entity breaks OR changes its damage phase (PhasesToDestroyed Signal).  Example: A wooden fence breaking and playing a snapping/splintering sound. | Defined in the "Small Debris" classes of the destructible entity. This sound will trigger when the spawned debris collides with something else and a certain impact threshold is exceeded. Example: Broken off wooden fence plank falls over and impacts with the ground. |
| | Enum Value | Description | | --- | --- | | NONE | No sound, default | | BREAK\_GLASS | Small glass objects, e.g. a glass or bottle | | BREAK\_GLASS\_PANE | Larger glass objects, e.g. car or building windows | | BREAK\_GROUNDRADAR | Dedicated sound for breaking ground radars | | BREAK\_MATRESS | Soft thump, used for soft, cloth-like sounds | | BREAK\_METAL | General metal break sound | | BREAK\_METAL\_GENERATOR | When a generator breaks down | | BREAK\_METAL\_NETFENCE | Metal-ish sound with elements of netfence rattling | | BREAK\_METAL\_POLE | Resonant pole break sound | | BREAK\_PIANO | Dedicated sound for breaking pianos | | BREAK\_PLASTIC | Sharp smaller-scale plastic cracking | | BREAK\_ROCK | Break sound for larger rock/stone/asphalt objects, e.g. a massive wall | | BREAK\_SANDBAG | Soft and plastic-y | | BREAK\_TENT | Dedicated sound for tents | | BREAK\_WATERHYDRANT | Dedicated sound for waterhydrants | | BREAK\_WOOD\_SOLID | Universal wooden break sound | | | Enum Name | Description | | --- | --- | | NONE | No sound, default | | BELL\_SMALL | Dedicated sound for small bells | | GLASS | Small glass objects, e.g. bottles | | MATRESS | Dedicated sound for matresses and pillows | | METAL\_HEAVY | Heavy metal impacts | | METAL\_LIGHT | Light, crisp metal impacts | | METAL\_NETFENCE | Metal-y impacts with netfence rattling | | METAL\_POLE | Resonant, medium-sized metal | | PLASTIC\_HOLLOW | E.g. a tube or food container | | PLASTIC\_SOLID | E.g. an old telephone | | ROCK | Concrete chunks | | ROCK\_SMALL | More brick-like, slightly hollow | | SANDBAG | Soft and plastic-y | | WOOD\_PLANK\_SMALL | For small, wooden objects, e.g. fence boards | | WOOD\_PLANK\_LARG | For large(r) wooden objects, such as smaller tree trunks | |

## Signals

Not all signals are available for each type of sound.

| Signal Name | Description | Available for Break Sounds | Available for Impact Sounds |
| --- | --- | --- | --- |
| PhasesToDestroyed | An integer indicating how damaged the entity is.  * 0 = Total destruction * 1 = Broken * > 1 = Anything else | Checked | Unchecked |
| EntitySize | The entity's mass, usually used in order to "scale" sounds. | Checked | Checked |
| CollisionDV | A value reflecting the change in speed of an entity upon contact (how fast was it before, how much was it slowed down?) | Unchecked | Checked |

## Debugging

If any issues arise, the first step should always be to make sure an MPDestructionManager entity exists in the world.
Check if the SCR\_DestructionMultiPhaseComponent is actually enabled (if it is an Entity with integrated MPD functionality, make sure "Enabled" is checked in the "Unsorted" tab).
Entities with integrated MPD functionality will always use the values stored in their Prefabs - meaning a per-instance change will not work. A Prefab change also requires a reload of the current world.

### Diag Menu

If a DestructionMPDestructionManager.et is present in the world, a [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") entry can be found:

```
Diag Menu \ Sounds \ Show MPD Impulse Values
```

Upon colliding, the debris entity will then display the impulse data in the format <impulse value>/<impulse threshold>/<mass>.

### Sound Definition Check

In the main Workbench Window (**not** World Editor), the "Multiphase Destruction Soundless Prefab Search" tool can be accessed via Plugins → Prefabs.
The tool will go through all prefabs and list in the debug console log those which do not have a "Break" or "Impact" sound defined.
