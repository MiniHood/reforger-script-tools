# [Weapon Stats-Modifing Attachments](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Stats-Modifing_Attachments)

## Overview

Some attachments, like Bayonets and Silencers, change vital internal stats of a weapon they are attached to. These changes are maintained and controlled by the weapon stats manager. The basic variant is **SCR\_WeaponStatsManagerComponent** and needs to be put on a weapon in order to make it "modifiable".  The class can be overridden/inherited by a mod in order to introduce more moddable weapon attributes.

There are a number of rules that need to be followed to make this work. I'll go over them below. Every attachment is its own entity, and must have an **InventoryItemComponent**. The latter has a list of custom attributes which we use as a basis for modification. If an attachment adds muzzles, like an UGL, it also ***must have*** its own **SCR\_WeaponStatsManagerComponent** or subclass thereof.

## Weapon Stats that can be modified

There are a number of stats on a weapon that can be modified without resorting to modding. We divide these into two groups:

* **Muzzle-Specific**: Attributes that override the specifics of a muzzle ***MUST have*** an attachment slot that is a child of the **MuzzleComponent**, so that the system can easily assign the right muzzle to the attachment. That means that if a weapon has more than one muzzle, or there are extra muzzles on attachments (like a Grenade Launcher), a muzzle specific attachment only modifies the muzzle it is attached to. For example, a silencer makes the rifle more quiet, but not the UGL.
* **Global**: Attributes that override global parameters of the weapon, for example obstruction length.

Any attachment can have any number of global attributes it modifies, but only muzzle attachments can have extra muzzle specific attributes.

The following table lists all stats that can currently be modified, their data type, and whether they are global or muzzle specific:

| Parameter | Scope | Data Type | Purpose |
| --- | --- | --- | --- |
| Muzzle Velocity Coefficient | Muzzle | float | A modifier for the muzzle velocity of the bullet being fired. This is a floating point value that is multiplied with the existing muzzle velocity coefficient as defined by the weapon. A value of 1.0 does not change anything, any value greater than one will make the bullet faster, while a lower value will make it slower. Most suppressors add a bit of muzzle velocity because of the extra barrel length |
| Dispersion Factor | Muzzle | float | A modifier for the muzzle dispersion. Normally, a longer barrel means less dispersion. A value of 1.0 is neutral, while a value between 0 and 1 will make the weapon shoot more precise. This value scales the dispersion diameter specified in the muzzle component, *not* the range. |
| Linear Recoil Factors | Muzzle | vector | A three element vector that is used to scale the linear recoil curve of a muzzle. 1.0 again is a neutral value, while values between 0 and 1 make the weapon have less linear recoil in that direction, and values greater than 1 will make the recoil stronger. The values in the vector are applied to x, y and z direction, respectively |
| Angular Recoil Factors | Muzzle | vector | As above, but with angular instead of linear recoil |
| Turn Factor Recoil Factors | Muzzle | vector | As above but for turn recoil curves. The Z value is stored but since the Z value is usually set to zero in a weapon, it will likely not have any inmpact. |
| Override Shot Sound | Muzzle | boolean | A boolean that specifies that the weapon is silenced. A silenced weapon will trigger SOUND\_SUPPRESSED\_SHOT sound event instead of the normal SOUND\_SHOT. It can also be used to modify AI reaction. |
| Extra Obstruction Length | Global | float | A floating point number (in meters) that defines how much longer the weapon becomes through this attachment. This is used for weapon obstruction, meaning that a mounted suppressor will trigger obstruction earlier than a normal weapon. See also Hierarchies below |
| Muzzle Effect Override | Muzzle | boolean | If set, muzzle effects of the weapon may be disabled and replaced by those on the attachment. Most notably, effects that have the "Override by Attachment" flag set will not play if the attachment sets this flag. Instead, it will use the effects on the attachment. Some effects like case ejection and ejection port smoke normally come without the override flag set, and those will always be played. See also Hierarchies below |
| Is Bayonet | Global | bool | If this flag is set, then the melee attacks with this weapon will use a different animation and sound effects to make it look like the player is stabbing with a bayonet instead of the buttstock hit. |
| Melee Damage | Global | float | This is a floating point multiplier that is applied to the weapon damage when this attachment is fitted to the weapon. Typically this would be used for bayonets, but it isn't restricted to it. |
| Melee Range | Global | float | A factor for the weapon range when using melee attacks. It's a multiplier applied to the existing melee range of the weapon |
| Melee Accuracy | Global | float | A factor for weapon accuracy with melee attacks. The factor is multiplied by the existing melee accuracy of the weapon, meaning that positive values lower than 1.0 make the attack more accurate (the accuracy specifies the radius of a sphere that is used to "track" for hit objects). |

## Hierarchies

Attachments can be mounted on other attachments. Certain rules apply here:

* If an attachment is attached to a muzzle, all subsequent attachments that are attached to this specific attachment inherit the muzzle they are assigned. A usage example is given below.
* Rules apply to the values depending on hierarchy. Most factors are multiplied along the chain of attachments, while booleans usually only require a single attachment in the chain to have this flag set for it to take effect.
* Overridden muzzle effects are subject to the same rules as the original muzzle's effect, meaning that if an attachment further down the chain overrides muzzle effects, then muzzle effects on all attachments before that may be skipped if the have the "override by attachment" flag.
* **Obstruction Length** is a special case - for this, the longest chain of attachments is used, measured in the given obstruction length (not number of attachments)

Let's take as an example a weapon that has its barrels as attachments, You would have a generic 16 inch barrel, and a 14 inch barrel. Both barrels have an attachment point at the tip to mount a flash hider/muzzle brake or suppressor.

* The ammo would be attuned to the 16 inch barrel, making its muzzle velocity factor 1.0.
* A muzzle brake would not change that, so it doesn't override the value
* A suppressor would make the bullet 5 percent faster.

That means the 16 inch barrel has a factor of 1.0 , and the suppressor has 1.05.

The 14 inch barrel is shorter so it's muzzle velocity factor is 0.9.

This gives a total velocity factor of:

* 1.0 with the 16 inch barrel
* 0.9 with the 14 inch barrel.
* 1.05 with a silenced 16 inch barrel
* 0.945 with a silenced 14 inch barrel

Likewise, both barrels would have a muzzle effect attached that has an exaggerated muzzle flash which is set to be overridden by attachment. A typical basket-type muzzle brake would have an effect with the typical "bloom".-effect. Finally a silencer would have its own, subdued effect. The muzzle brake must be removed (at least on the M4/M16) in order to attach a silencer, so any attachment would always override the barrel's effect.

✩

More details about relations between barrel length and velocity can be found on [Weapon Creation](/wiki/Arma_Reforger:Weapon_Creation/Prefab_Configuration#Barrel_Length_vs_Muzzle_Velocity "Arma Reforger:Weapon Creation/Prefab Configuration") page

### Attaching/Detaching Hierarchies

An entity getting attached may, by itself, already have other attachments on it, and it may do so without having its own stats manager. For example, consider a weapon whose barrel is an attachment itself (Attachment resp. attaching doesn't necessarily be a user action, prefabs will also get "attached" to the weapon in terms of code path). That barrel might come with a muzzle brake already attached, and both might modify certain aspects of the weapon, like muzzle velocity, dispersion etc. There will definitely be a difference in muzzle flash between a naked barrel and a barrel with a muzzle brake or flash hider.

When a hierarchy is attached (runtime or during initialization), the attachment is performed from the weapon outwards. This means that the barrel is handled first, followed by the muzzle brake. This is so that the muzzle break "overlays" any changes done through the barrel, since they will be more relevant.

Detaching works the other way around. The attachment furthest down the chain is detached first (in the example, the muzzle brake), then the barrel.

It is of course sill possible to only detach the muzzle brake and replace it with a flash hider or suppressor.

## How to Configure an Attachment

## SCR\_WeaponAttachmentAttributes

As part of its custom attribute list, the typical **InventoryItemComponent** has a **WeaponAttachmentAttributes** entry that, by default, only lists an Attachment Type class that is used to find out if an attachment fits a certain slot. **SCR\_WeaponAttachmentAttributes** is a "smart" subclass of that, but in itself doesn't add functionality. That is done by subclasses. These are the currently defined subclasses:

|  |  |
| --- | --- |
| [SCR\_WeaponAttachmentSuppressorAttributes](enfusion://ScriptEditor/Scripts/Game/Inventory/SCR_WeaponAttachmentSuppressorAttributes.c;1) | This offers the typical values of a weapon suppressor attached to a muzzle like muzzle velocity coefficient, dispersion, muzzle effects, silenced, obstruction length and recoil factors |
| [SCR\_WeaponAttachmentBayonetAttributes](enfusion://ScriptEditor/Scripts/Game/Inventory/SCR_WeaponAttachmentBayonetAttributes.c;1) | This offers the typical values of a bayonet, like weapon melee damage/range/accuracy, is bayonet and obstruction length |

Currently, only one of these can be added at any time (all subsequent ones are ignored). In the future, though, this behaviour might change. This is because the **ItemAttributeCollection** API does not currently support anything but finding a single attribute.

## Under the Hood - How to make your own attachments

## Custom SCR\_WeaponAttachmentAttributes

If an attachment doesn't fit in the above categories, a subclass of **SCR\_WeaponAttachmentAttributes** can be used. Any subclass must implement two scripted functions:

```enforce
class SCR_WeaponAttachmentAttributes : WeaponAttachmentAttributes
{
	/*! Apply Modifiers to weapon on attach

	This is called when an attachment has been attached to a weapon.
	\param statsManager The stats manager that called this function. All modifiers should be set through this
	\param muzzleIndex The index of the muzzle, if applicable. This has no real meaning to this function except
	that it should be passed on to muzzle related functions. Can be -1 (which indicates that this attachment
	was done on a non-muzzle attachment slot) or a non-zero integer. See below
	\param attachedEntity The entity being attached

	The muzzle index can be interpreted only as a far as -1 means that the attachment was not attached to
	a muzzle. Other than that, the index should be passed on to functions without modification.

	*/
	bool ApplyModifiers(BaseWeaponStatsManagerComponent statsManager, int muzzleIndex, IEntity attachedEntity)
	{
		return false;
	}
	/*! Clear the modifiers from a weapon on detach
	This is called before an attachment is removed from a weapon.
	\param statsManager The stats manager that called this function. All modifiers should be set through this
	\param muzzleIndex The index of the muzzle, if applicable. This has no real meaning to this function except
	that it should be passed on to muzzle related functions. Can be -1 (which indicates that this attachment
	was done on a non-muzzle attachment slot) or a non-zero integer.
	\param attachedEntity The entity being detached

	See also ApplyModifiers

	*/
	void ClearModifiers(BaseWeaponStatsManagerComponent statsManager, int muzzleIndex, IEntity attachedEntity)
	{
	}
}
```

As an example, here is the scripting of the **SCR\_WeaponAttachmentSuppresorAttributes**:

Show text

```enforce
class SCR_WeaponAttachmentSuppressorAttributes : SCR_WeaponAttachmentAttributes
{
	[Attribute("1.0", UIWidgets.EditBox, "Muzzle Velocity Coefficient. Values >1 speed up the bullet, values below slow it down")]
	protected float m_fMuzzleSpeedCoefficient;
	[Attribute("1", UIWidgets.CheckBox, "Override sound for shots")]
	protected bool m_bOverrideShot;
	[Attribute("1", UIWidgets.CheckBox, "Override muzzle effects")]
	protected bool m_bOverrideMuzzleEffects;
	[Attribute("1.0", UIWidgets.EditBox, "Muzzle Dispersion Factor. Affects the radius of impact at the given range in the weapon. Smaller than 1.0 makes the weapon more accurate")]
	protected float m_fMuzzleDispersionFactor;
	[Attribute("0.0", UIWidgets.EditBox, "Extra Obstruction Length")]
	protected float m_fExtraObstructionLength;
	[Attribute("1 1 1", "Linear Recoil Factors for X/Y/Z Direction. Smaller values reduce recoil.")]
	protected vector m_vLinearFactors;
	[Attribute("1 1 1", "Angular Recoil Factors for X/Y/Z Direction. Smaller values reduce recoil.")]
	protected vector m_vAngularFactors;
	[Attribute("1 1 1", "Turning Recoil Factors for X/Y/Z Direction. Smaller values reduce recoil.")]
	protected vector m_vTurnFactors;
	override bool ApplyModifiers(BaseWeaponStatsManagerComponent statsManager, int muzzleIndex, IEntity attachedEntity)
	{
		if (muzzleIndex == -1)
		return false; // Should not happen
		if (!statsManager.SetMuzzleVelocityCoefficient(attachedEntity, muzzleIndex, m_fMuzzleSpeedCoefficient))
		return false;
		if (!statsManager.SetMuzzleDispersionFactor(attachedEntity, muzzleIndex, m_fMuzzleDispersionFactor))
		return false;
		if (m_bOverrideShot)
		if (!statsManager.SetShotSoundOverride(attachedEntity, muzzleIndex, true))
		return false;
		if (m_bOverrideMuzzleEffects)
		{
			if (!statsManager.SetMuzzleEffectOverride(attachedEntity, muzzleIndex, true))
			return false;
			array<Managed> comArray = {};
			attachedEntity.FindComponents(MuzzleEffectComponent, comArray);
			foreach (Managed component : comArray)
			{
				MuzzleEffectComponent muzzleEffect = MuzzleEffectComponent.Cast(component);
				if (muzzleEffect)
				statsManager.AddMuzzleEffectOverride(attachedEntity, muzzleIndex, muzzleEffect);
			}
		}
		if (!statsManager.SetExtraObstructionLength(attachedEntity, m_fExtraObstructionLength))
		return false;
		if (!statsManager.SetRecoilLinearFactors(attachedEntity, muzzleIndex, m_vLinearFactors))
		return false;
		if (!statsManager.SetRecoilAngularFactors(attachedEntity, muzzleIndex, m_vAngularFactors))
		return false;
		if (!statsManager.SetRecoilTurnFactors(attachedEntity, muzzleIndex, m_vTurnFactors))
		return false;
		return true;
	}
	override void ClearModifiers(BaseWeaponStatsManagerComponent statsManager, int muzzleIndex, IEntity attachedEntity)
	{
		if (muzzleIndex == -1)
		return;
		statsManager.ClearMuzzleVelocityCoefficient(attachedEntity, muzzleIndex);
		statsManager.ClearMuzzleDispersionFactor(attachedEntity, muzzleIndex);
		if (m_bOverrideShot)
		statsManager.ClearShotSoundOverride(attachedEntity, muzzleIndex);
		if (m_bOverrideMuzzleEffects)
		statsManager.ClearMuzzleEffectOverride(attachedEntity, muzzleIndex);

		statsManager.ClearExtraObstructionLength(attachedEntity);
		statsManager.ClearRecoilLinearFactors(attachedEntity, muzzleIndex);
		statsManager.ClearRecoilAngularFactors(attachedEntity, muzzleIndex);
		statsManager.ClearRecoilTurnFactors(attachedEntity, muzzleIndex);
	}
}
```

[↑ Back to spoiler's top](#bikisp6a635bf198c29)

## SCR\_WeaponStatsManagerComponent

Each entity that has a modifiable attributes must have its own **SCR\_WeaponStatsManagerComponent**. This usually only affects attachments that have their own muzzle, like an under barrel grenade launcher, and only if that muzzle can be modified. If a weapon doesn't have a stats manager, it will function normally but any attachment will not cause any stats modification. Likewise, an attached grenade launcher (for example) would not support changing any attributes if there is no stats manager component on it.

If an attachment has its own stats manager, then muzzle specific attributes are handled by that stats manager, while global attributes are passed along to the parent.

## BaseWeaponStatsManagerComponent API

**BaseWeaponStatsManagerComponent** has a set of at least three functions per modifiable attribute. They usually follow the same scheme:

* **SetXXX** is used to override a weapon stat
* **ClearXXX** is used to clear the override and return to "normal" values for this attachment
* **GetXXX** is used to see if and how the attribute is overridden

### SetXXX

Setters set the override on a specific attachment, and cause a recalculation of the override values.

Setters usually have two or three arguments, depending on whether they set a global or muzzle attribute:

1. The first argument is always the entity that is being attached/detached.
2. The second argument depends on the scope of the changes stat. If it is a muzzle stat, there is a muzzle index passed in as second argument, which specifies which muzzle this attach operation went to. This value is always passed in via the ApplyModifier call, and passing anything else is an error
3. The third (or second, depending on scope) parameter is the value(s) to set.

**SetXXX** returns true on successful completion. It returns false when the value could not be modified, for example, if a muzzle stat is modified and the muzzle index is -1.

### ClearXXX

Clear  calls clear the override settings on a specific attachment and cause a recalculation of the override values.

They usually come with one or two arguments:

1. The first argument is the entity that is being attached/detached
2. If a muzzle value is cleared, the second argument is the index of the muzzle

**ClearXXX** returns true if successful, or false if there was an error, typically if a muzzle index was not appropriate

### GetXXX

While the **SetXXX** and **ClearXXX** calls are usually only used while attaching/detaching, Get calls are used within the components that can be modified. For example, the **WeaponSoundsComponent** will check if the weapon is suppressed using this call type and issue the appropriate sound event.

The usually have only one or two arguments:

1. The first argument is the muzzle index, passed in only for stats of the Muzzle category.
2. The second argument is a reference to a variable where the result is stored.

The return value of the function is true if the stat is overridden, and the variable reference is updated with the value of the override. If the value is not overridden, the function returns false.

**Example of using a Getter**

```enforce
if (m_statsManager) // If we have a stats manager for our component...
{
	float fDamageFactor;
	if (m_statsManager.GetMeleeDamageFactor(fDamageFactor)) // If melee damage was overriden
	return fDamageFactor * m_fDamage; // Apply the damage modifier and return the result
}
return m_fDamage; // return the result if no stats manager
```

## API

The following table lists the appropriate calls for each of the modifiable stats:

|  |  |
| --- | --- |
| Muzzle Velocity Coefficient | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMuzzleVelocityCoefficient([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) muzzleCoeff) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMuzzleVelocityCoefficient([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMuzzleVelocityCoefficient([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) coeff) |
| Dispersion Factor | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMuzzleDispersionFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fFactor) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMuzzleDispersionFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMuzzleDispersionFactor([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fFactor) |
| Linear Recoil Factors | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetRecoilLinearFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vLinearFactors) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearRecoilLinearFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetRecoilLinearFactors([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vLinearFactors) |
| Angular Recoil Factors | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetRecoilAngularFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vAngularFactors) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearRecoilAngularFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetRecoilAngularFactors([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vAngularFactors) |
| Turn Factor Recoil Factors | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetRecoilTurnFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vTurnFactors) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearRecoilTurnFactors([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetRecoilTurnFactors([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [vector](enfusion://ScriptEditor/scripts/Core/generated/Types/vector.c;12) vTurnFactors) |
| Override Shot Sound | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetShotSoundOverride([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bOverride) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearShotSoundOverride([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetShotSound([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bOveride) |
| Extra Obstruction Length | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetExtraObstructionLength([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) extraLength) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearExtraObstructionLength([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetExtraObstructionLength(out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) extraLength) |
| Muzzle Effect Override | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMuzzleEffectOverride([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bOverrde) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMuzzleEffectOverride([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMuzzleEffectOverride([int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) iMuzzleIndex, out [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bOverride) |
| Is Bayonet | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetIsBayonet([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bIsBayonet) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearIsBayonet([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetIsBayonet(out [bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) bIsBayonet) |
| Melee Damage | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMeleeDamageFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fDamageFactor) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMeleeDamageFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMeleeDamageFactor(out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fDamageFactor) |
| Melee Range | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMeleeRangeFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fRangeFactor) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMeleeRangeFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMeleeRangeFactor(out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fRangeFactor) |
| Melee Accuracy | c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) SetMeleeAccuracyFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach, [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fAccuracyFactor) c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) ClearMeleeAccuracyFactor([IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) attach)  c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) GetMeleeAccuracyFactor(out [float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) fAccuracyFactor) |

### Special case: Muzzle Effects

Muzzle Effects are a special case. The attachment must add all effects that it has and wants to play to the stats manager via the call

```enforce
bool AddMuzzleEffectOverride(IEntity attach, int iMuzzleIndex, MuzzleEffectComponent muzzleEffect)
```

See the example for silencers above for an example how to use this. These muzzle effects stay in effect until the appropriate cClearMuzzleEffectOverride call.
