# [Weapon Slots And Bones](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Slots_And_Bones)

## Splitting Weapon

Weapon should be split into two parts - body and attachments - and all attachments can be connected to body via slots.

### Body

There are few guidelines when it comes to setting up weapon model:

* weapon itself has to be without magazine or any attachments
* the body has a "**slot**" points (dummy objects), into which attachments are being snapped in a prefab. We always name this point "**slot\_description**".

[![](/wikidata/images/1/13/armareforger-weaponslots-body.png)](/wiki/File:armareforger-weaponslots-body.png)

AK74 body with slots and bones visible

ⓘ

You can always check position of the existing weapons by loading them in [Resource Browser](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Bones "Arma Reforger:Resource Manager: Model Editor") model viewer and switching to **Bones** tab.

### Attachments

* everything, which is not core part of a weapon, like optics, magazines, grenade launchers, etc.
* each attachment has a "**snap**" point (dummy object), which is being snapped to a "slot" point on a weapon body in a prefab. We always name this point "**snap\_description**".

* [![AK-74 magazine snap point](/wikidata/images/thumb/3/31/armareforger-weaponslots-magazine-snap.png/329px-armareforger-weaponslots-magazine-snap.png)](/wiki/File:armareforger-weaponslots-magazine-snap.png "AK-74 magazine snap point")

  AK-74 magazine snap point
* [![GP-25 snap point](/wikidata/images/thumb/c/c2/armareforger-weaponslots-snappoint-gp25.png/422px-armareforger-weaponslots-snappoint-gp25.png)](/wiki/File:armareforger-weaponslots-snappoint-gp25.png "GP-25 snap point")

  GP-25 snap point

## Slot/Snap points

When figuring out the position of slot points on weapons and snap points on attachments, following steps will help to avoid complications with possible badly modeled mesh or badly placed slot and snap points:

1. Before a production of new assets starts, create rough placeholders for all attachments and set their snap points following the rules below
2. When modeling the weapon and setting up the slot point, try as early as possible to attach all possible attachments exactly **"snap point on slot point"** (even just placeholders if final mesh is not possible) and see if all fits.
3. If some attach mechanism is the same for more weapons, the slot point must be on the the same spot in relation to the attach mechanism on these weapons (so all relevant attachments always fit in).

### Conventions

#### Slots

| What | Naming | Notes | Example |
| --- | --- | --- | --- |
| Magazine well | slot\_magazine | Used in **MuzzleComponent**  * Should be setup according to the magazine * in a place, where front side of the magazine touches the bottom side of the weapon body | [armareforger-weaponslots-magazinewell.png](/wiki/File:armareforger-weaponslots-magazinewell.png) |
| Optics mount ***(non-standard rail)*** | slot\_optics | Used in **AttachmentSlotComponent**  * node should be placed on contact surfaces * for AK dovetail mount - on center of a rounded niche on outer side of the weapon body attach mechanism | [armareforger-weaponslots-slot-optics.png](/wiki/File:armareforger-weaponslots-slot-optics.png) |
| Muzzle | slot\_barrel\_muzzle | Used in **AttachmentSlotComponent**  * suppressors, compensator, etc.  * node for slot should be positioned on the end of the muzzle thread | [armareforger-weaponslots-slot-barrel-muzzle.png](/wiki/File:armareforger-weaponslots-slot-barrel-muzzle.png) |
| Top rail |  | Used in **AttachmentSlotComponent**  * should be mainly used for Picatinny rail or any other universal mounting solution | Slot placement (from [Sample New Weapon](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewWeapon/Assets/Weapons/Rifles/SampleWeapon_01/SampleWeapon_01.blend)) [armareforger-weaponslots-picatinny-top.png](/wiki/File:armareforger-weaponslots-picatinny-top.png)  [armareforger-weaponslots-picatinny-sample.png](/wiki/File:armareforger-weaponslots-picatinny-sample.png)  Example of mount - Picatinny rail (+dimensions)  [armareforger-weaponslots-picatinny.png](/wiki/File:armareforger-weaponslots-picatinny.png) |
| Bottom rail |  |
| Side rail |  |
| Under-barrel | slot\_underbarrel | Used in **AttachmentSlotComponent**  * to be used with underbarrel devices such as UGL or grip | [armareforger-weaponslots-slot-underbarrel.png](/wiki/File:armareforger-weaponslots-slot-underbarrel.png) |
| Bayonet mount | slot\_bayonet | Used in **AttachmentSlotComponent** |  |
| Flash-light | slot\_flashlight | Used in **AttachmentSlotComponent** |  |

#### Attachments

| What | Naming | Notes | Example |
| --- | --- | --- | --- |
| Magazine | snap\_weapon | * snap is located on front upper edge, in the center of the magazine |  |
| Optics | snap\_weapon |  |  |
| Muzzle device | snap\_weapon | * model is centered on the end of the thread |  |
| Under-barrel | snap\_weapon | * UGLs often use specialised mounts, so it needs to be fitted according to weapon | [armareforger-weaponslots-snappoint-gp25.png](/wiki/File:armareforger-weaponslots-snappoint-gp25.png) |
| Bayonet | snap\_weapon |  |  |
| Bipod | snap\_weapon |  |  |
| Flashlight | snap\_weapon |  |  |
| Laser | snap\_weapon |  |  |
| Barrel | snap\_weapon | machineguns |  |

#### Simulation

Slots used by various weapon related components

| What | Naming | Notes | Example |
| --- | --- | --- | --- |
| Eye (aiming) | eye | Used by **SightsComponent**  * for each muzzle (if needed - e.g. integrated UGL yes, double-barrel shotgun no) | [armareforger-weaponslots-eye-point.png](/wiki/File:armareforger-weaponslots-eye-point.png) |
| Barrel (firing dir) - Position | barrel\_chamber | Used by **MuzzleComponent**  * for each muzzle | [armareforger-weaponslots-barrel-chamber.png](/wiki/File:armareforger-weaponslots-barrel-chamber.png) |
| Barrel (firing dir) - Direction | barrel\_muzzle | Used by **MuzzleComponent**  * for each muzzle | [armareforger-weaponslots-barrel-muzzle.png](/wiki/File:armareforger-weaponslots-barrel-muzzle.png) |

#### Bones

Below names are compatible with existing [animation export profiles](/wiki/Arma_Reforger:Animation_Export_Profiles "Arma Reforger:Animation Export Profiles")

| What | Naming | Notes |
| --- | --- | --- |
| Magazine release | w\_mag\_release |  |
| Bipod legs | w\_bipodleg w\_bipodleg\_left w\_bipodleg\_right |  |
| Trigger | w\_trigger |  |
| Safety | w\_safety | Use only in case weapon has separate safety lever. If not, use firemode selector |
| Firemode selector | w\_fire\_mode |  |
| Buttstock | w\_buttstock |  |
| Charging handle | w\_ch\_handle |  |
| Ejection port | w\_ejection\_port |  |
| Bolt | w\_bolt |  |
| Bolt release | w\_bolt\_release |  |
| Slide | w\_slide | pistols |
| Hammer / Striker | w\_hammer / w\_striker |  |
| Cylinder | w\_cylinder | revolvers |
| Rear sight | w\_rear\_sight |  |
| Front sight | w\_front\_sight |  |
| Barrel | w\_barrel | single - recoil operated guns, multiple - rotary cannons |
