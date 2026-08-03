# [Damage Effects](https://community.bistudio.com/wiki/Arma_Reforger:Damage_Effects)

**Damage Effects** are used in order to have a data-oriented damage system and to allow for more complex behaviours in damage by defining the interaction between different Damage Effects.

## DamageEffects

DamageEffects are either sources of damage and they generally handle their consequences. They are separated in three types, described below.

|  |  |
| --- | --- |
| InstantDamageEffects These effects are added and instantly get applied. The most common example would be a bullet.  Notice how **InstantDamageEffects** are never removed from the **ExtendedDamageManager**. | [armar-damageeffects instantdamageeffects flowchart.png](/wiki/File:armar-damageeffects_instantdamageeffects_flowchart.png) |
| PersistentDamageEffects Persistent effects are the ones that get added to a DamageManager and stay there until terminated. Their termination is usually handled by the effect itself or because a different DamageEffect got applied.  Persistent effects have EOnFrame support.  An example of a persistent effect would be something like a broken leg or a tourniquet. They stay affecting the limb until the effect is removed.  Note that PersistentDamageEffects don't apply their effect automatically (OnEffectApplied). OnEffectApplied is replicated and should be saved for cases where it is needed. | [armar-damageeffects persistentdamageeffects flowchart.png](/wiki/File:armar-damageeffects_persistentdamageeffects_flowchart.png) |
| DotDamageEffects These effects are a PersistentDamageEffect specialised for applying Damage Over Time (DOT).  Duration-based DOTs are fully supported, and an effect is terminated as soon as its duration ends. | [armar-damageeffects dotdamageeffects flowchart.png](/wiki/File:armar-damageeffects_dotdamageeffects_flowchart.png) |

## DamageManager

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated.

## DamageEffectEvaluator

Every ExtendedDamageManager has its own instance of the DamageEffectEvaluator. Whenever cApplyEffect() gets called on a DamageEffect, cHandleEffectConsequences() gets called.
By writing in this class the consequences of effects getting applied, DamageEffects become more flexible.

### Example

Imagine that the consequence of a bullet effect is to add a bleeding effect. Vehicles should not be getting bleeding effects.

* If the bleeding logic is on the DamageEffect, the DamageManager type needs to be checked before deciding how to handle it
* If the bleeding logic is on the DamageManager, various DamageEffect types must be checked
* If the bleeding should only apply to some characters, the DamageManager logic needs to be overcomplicated - sign of a wrong design.

The solution is to apply the consequences through these evaluators. The evaluator will handle the logic of DamageEffects being applied to the DamageManager.
It is also possible to change what evaluator is used during runtime with ExtendedDamageManager.SetEvaluator().

## ApplyEffect

InstantDamageEffects automatically call cApplyEffect() when they get added to the manager, however this is not the case for Persistent and Dot DamageEffects.
This is because clients do not update their DamageEffects, so any logic written inside of EOnFrame will only happen on the server.

DamageEffectEvaluator.HandleConsequences() gets called when DamageEffects get applied, and since effect application gets automatically replicated to clients, it is possible to use the evaluator (or inside cOnEffectApplied()) to run some logic on clients.

### Example

Let's have a WoundDamageEffect (a persistent effect) that deals damage to a player every 50 meters they run. When it reaches 50 meters, we would call cApplyEffect().
In the evaluator's HandleEffectConsequences, we can play a sound and deal some damage.

Persistent and Dot effects can call cApplyEffect() as many times as needed, however keep in mind that the whole effect gets replicated every time the effect gets applied, so try to keep them to a minimum.

## DamageHistory

The damage history contains all the DamageEffects that got applied to a damage manager since the last time it was cleared.
The damage history is only stored on the server, and it can be used to properly determine who should be given kill credits or assists for helping taking down someone.

## DamageEffects Hijack

HijackDamageEffect has similarities with HijackDamageHandling; it gets called right before a DamageEffect gets added.
It is possible to prevent an effect from being added, as well as change its parameters before any callbacks happen. Sometimes effects are unique and only one of them should be present at a time.

### Example

Let's create a BurningDamageEffect that lasts 4 seconds. If the entity is already burning when the effect gets applied, we want the duration to be refreshed.
If the entity is burning for one second already, the burning effect is updated (reset to four) so the total burning duration will be five seconds.

The proper way to do it is via HijackDamageEffect:

```enforce
event override bool HijackDamageEffect(SCR_ExtendedDamageManagerComponent dmgManager)
{
	// if there are no burning effects present, we add one.
	if (!dmgManager.IsDamageEffectPresent(BurningDamageEffect))
	return false;
	array<ref PersistentDamageEffect> burningEffectArray = dmgManager.GetAllPersistentEffectsOfType(BurningDamageEffect);
	// we know the cast won't fail and that at least there will be 1 element
	BurningDamageEffect burningEffect = BurningDamageEffect.Cast(burningEffectArray[0]);
	// change duration of the effect on the manager
	burningEffect.SetMaxDuration(burningEffect.GetCurrentDuration() + 4);
	// return true so this effect does not get added
	return true;
}
```

## Damage Over Time

While ExtendedDamageManager inherit from DamageManager, their DOT logic is completely different.
ExtendedDamageManager can only take damage over time through DamageEffects, and therefore the following methods are no longer supported on ExtendedDamageManagers:

```enforce
OnDamageOverTimeAdded(EDamageType dType, float dps, HitZone hz)
OnDamageOverTimeRemoved(EDamageType dType, HitZone hz)
IsDamagedOverTime(EDamageType dType)
GetDamageOverTime(EDamageType dType)
RemoveDamageOverTime(EDamageType dType)
```

Any calls to this methods from an ExtendedDamageManager will get ignored by the system.

ⓘ

See [Damage System](/wiki/Arma_Reforger:Damage_System "Arma Reforger:Damage System") for more information.
