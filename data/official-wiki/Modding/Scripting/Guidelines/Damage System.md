# [Damage System](https://community.bistudio.com/wiki/Arma_Reforger:Damage_System)

The **Damage System** is the system that handles entity damage.

ⓘ

The Damage System explained in this document targets the [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.0 "Category:Arma Reforger/Version 1.2.0") [1.2.0](/wiki?title=Category:Arma_Reforger/Version_1.2.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.0 (page does not exist)") version.

[![armar-damage system flowchart.png](/wikidata/images/thumb/7/7a/armar-damage_system_flowchart.png/600px-armar-damage_system_flowchart.png)](/wiki/File:armar-damage_system_flowchart.png)

## Logic Flow

The workflows goes as follows (see the attached image):

* damage is received
* the c[DamageManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DamageManagerComponent.c;12) checks that:
  + damage handling is enabled
  + damage handling is **not** hijacked
  + received damage should count as a hit
  + check if the entity is dead
  + check if actual damage shalt be dealt
* it then
  + sends damage to the hit zone which calculates the effective damage
  + replicate the hit over the network for clients to calculate damage locally and display damage effects
  + deals damage
  + sends damage to the hit zone again for it to
    - deal damage
    - trigger cOnDamage()
    - pass damage to the parent hit zones and do the same process for themselves
  + triggers DamageManager's cOnDamage()
  + if the DamageStates did not change, exits
  + if they did, trigger the hit zone's cOnDamageStateChanged() then its own cOnDamageStateChanged()

## Classes

Class hierarchy goes as follows:

```enforce
HitZoneContainerComponent // engine-side - itself inheriting from GameComponent, irrelevant here
DamageManagerComponent // engine-side
SCR_DamageManagerComponent // script-side
ExtendedDamageManagerComponent // engine-side
SCR_ExtendedDamageManagerComponent // script-side - be sure to read its comments and the warning below
SCR_CharacterDamageManagerComponent // script-side
```

## ExtendedDamageManagerComponent Changes

The following API will not be useful for damageManagers inheriting from c[ExtendedDamageManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/ExtendedDamageManagerComponent.c;12):

```enforce
void OnDamageOverTimeAdded(EDamageType dType, float dps, HitZone hz);
void OnDamageOverTimeRemoved(EDamageType dType, HitZone hz);
bool IsDamagedOverTime(EDamageType dType);
float GetDamageOverTime(EDamageType dType);
void RemoveDamageOverTime(EDamageType dType);
```

DamageEffects logic replaces the conventional damageOverTime, and has its own API.

|  |  |
| --- | --- |
| ENFORCECODEMARKER   ``` OnDamageOverTimeAdded() ``` | ENFORCECODEMARKER   ``` OnDamageEffectAdded() ``` |
| ENFORCECODEMARKER   ``` OnDamageOverTimeRemoved() ``` | ENFORCECODEMARKER   ``` OnDamageEffectRemoved() ``` |
| ENFORCECODEMARKER   ``` IsDamagedOverTime() ``` | ENFORCECODEMARKER   ``` SCR_CharacterDamageManagerComponent.IsBleeding() GetPersistentEffects() GetAllPersistentEffectsOnHitZone() GetAllPersistentEffectsOfType() ``` |
| ENFORCECODEMARKER   ``` GetDamageOverTime() ``` | ENFORCECODEMARKER   ``` SCR_RegeneratingHitZone.GetHitZoneDamageOverTime() SCR_CharacterBloodHitZone.GetTotalBleedingAmount() ``` |

## SetHealth()

The c[HitZone](enfusion://ScriptEditor/scripts/Game/generated/HitZone/HitZone.c;12).SetHealth() method "magically" changes the amount of HP a hit zone has.

This means that the flowchart above will not take place since damage is not actually being dealt - this also means there is no instigator. Instead, only cOnHealthSet() and cOnDamageStateChanged() are called.

cSetHealth() should be used very sparingly, for example in cases where one wants to force a character death, even when damage handling is disabled for that manager (remember, this method is not *dealing damage*, the health is magically changing).

An alternative to cSetHealth() is to call HandleDamage and deal the entity's max health as true damage. By doing it this way, the entity is guaranteed to take enough damage to be destroyed, and the callback structure of a more "natural" cause of death above is maintained.

### Bad Usage Example

Keep in mind that doing HandleDamage with true damage max health is not always the best way to destroy an entity. For example, killing a player inside a vehicle.

At first glance, killing the player by dealing damage equal to their max hp sounds like a good idea (and it would work fine on vanilla). However, now think of the (non-existent) Terminator mod.

If the Terminator is inside of a vehicle and the vehicle explodes, it should survive it (because of small damage multipliers, a large health pool, etc.), but since the applied damage is equal to max health, it would die.

This is why the best way to destroy entities is by dealing a "realistic" amount of damage that achieves the wanted goal, but that could technically be survivable by a modded entity.

When writing damage code, try to think of this fictional Terminator mod to ensure as much compatibility as possible.
