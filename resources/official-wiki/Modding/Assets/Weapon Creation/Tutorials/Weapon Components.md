# [Weapon Components](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Components)

## Slot Classes

## PointInfo

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Pivot ID | *string* |  | Name of Pivot ID. This list is populated by bones and empty object from the mesh. Please note that sometimes that list might be empty, even though mesh has bones. In such situation it is enough to go to another component and then get back to it again. |
| Offset | *vector* | meters | Offset from the origin |
| Angles | *vector* | degrees | Orientation around axes |

## EntitySlotInfo

*Inherits from PointInfo*

*Requires name*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Auto Transform | *bool* |  | If this option is set to true, then attached prefab will move with parent object |
| Child Pivot ID | *string* |  | Offset defined by child node |
| Enabled |  |  |  |
| Prefab | *resource* | .et | Name of the resource which should be attached to parent object |
| Disable Physics Interaction | *bool* |  | Should attached object not collide with any parent |
| Inherit Parent Skeleton | *bool* |  | Should attached object inherit parent skeleton |
| Activate Physics On Detaching | *bool* |  | Should physics be activated when entity is detached from slot? |
| Deactivate Physics On Attaching | *bool* |  | Should physics be deactivated when entity is atached to slot? |

## InventoryStorageSlot

*Inherits from EntitySlotInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Name | string |  | **editor only** human readable name of the slot. Has no effect on the functionality whatsoever |

## EquipmentStorageSlot

*Inherits from InventoryStorageSlot*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Allowed Item Types | *array of strings* |  | Item types that are allowed to be attached to current slot (see ECommonItemType enum for possible values) |
| Affected By Occluders | *array of strings* |  | Loadout ocluder types that disable item in this slot |
| Animated Mesh Reference | *resource* | xob | In case of entity owner have changeable mesh at runtime, setting animated mesh reference allows you to configure alternative state of slot transformation |

## SCR\_EquipmentStorageSlot

*Inherits from EquipmentStorageSlot*

## LoadoutSlotInfo

*Inherits from InventoryStorageSlot*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Area | *integer* |  | Area this item is assigned to. Available options:  * None * HeadCover * FaceCover * Jacket * Vest * Pants * Boots * Cover * Backpack * Attachment1 * Attachment2 * Attachment3 * Attachment4 * Attachment5 * Attachment6 * Attachment7 * Attachment8 |
| Meshes To Hide | *array of strings* |  | Array of meshes which should be hidden when this item is equipped. List of meshes can be obtained by looking at the base body **Meshes** section under LOD settings |

## RegisteringComponentSlotInfo

*Inherits from EntitySlotInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Register Actions | *bool* |  | register actions (defined in **ActionsManagerComponent**) to parent entity |
| Register Damage | *bool* |  | register damage to parent entity |
| Register Controllers | *bool* |  | register controllers to parent entity |
| Register Weapons | *bool* |  | register weapons to parent entity |
| Register Compartments | *bool* |  | register compartments (defined in **SCR\_BaseCompartmentManagerComponent**) to parent entity |
| Register Lights | *bool* |  | register lights (defined in **BaseLightManagerComponent**) to parent entity |

## SoundPointInfo

*Inherits from PointInfo*

## DecalSlotInfo

*Inherits from PointInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Projection Offset | *float* |  | Projection offset from PointInfo target transform |
| Projection Rotation | *vector* |  | Projection rotation around PointInfo target transform |
| Far Plane | *float* |  | Far plane - maximum distance for decal projection box |
| Stretch | *float* |  | Stretch of decal |
| Scale | *float* |  | Scale |
| Lifetime | *float* |  | Decal lifetime, value <= 0 is indeffinite |
| Create On Init | *bool* |  | Create this decal on entity init |
| Debug Draw | *bool* |  | Enable debug |
| Material | *resource* | emat | Decal material |

## EmissiveGlassSlot

*Inherits fromEntitySlotInfo*

## EmissiveLightSurfaceSlot

*Inherits fromEntitySlotInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Light Type |  |  |  |
| Light Side |  |  |  |
| Emissive On Multiplier |  |  |  |
| Emissive Off Multiplier |  |  |  |
| Configurations Override |  |  |  |

## UI Info Classes

## UIInfo

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Name | *string* |  | Name to be displayed in player UI |
| Description | *string* |  | Description text to be displayed in player UI |
| Icon | *resource* |  | Resource for icon to be displayed in player UI |

## GrenadeUIInfo

*Inherits parameters from WeaponUIInfo & UIInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Icon Ammo Type | *resource* |  |  |
| Ammo Type |  |  | Magazine type (AP, Tracer, Ball, ...). Used for Weapon Info UI. |
| Show Ammo Type Text |  |  | Show ammo type extra text next to ammo type icon(s) in Weapon Info UI. |
| Ammo Type Flags |  |  | Set magazine ammo type flags |

## MagazineUIInfo

*Inherits parameters from UIInfo*

|  |  |  |  |
| --- | --- | --- | --- |
| Show Caliber | *bool* |  |  |
| Ammo Caliber | *string* |  | Magazine ammunition caliber |
| Ammo Type | string |  | Magazine type (AP, Tracer, Ball, ...). Used for Weapon Info UI. |
| Show Ammo Type Text | bool |  | Show ammo type extra text next to ammo type icon(s) in Weapon Info UI. |
| Ammo Type Flags |  |  | Set magazine ammo type flags |
| Mag Indicator | class |  |  |

## MuzzleUIInfo

*Inherits parameters from UIInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Show Caliber | *bool* |  | Show caliber indicator in weapon UI. |
| Caliber | *string* |  | Caliber identification, e.g. 7.62×39mm |
| Firemode Icon Imageset | *resource* | imageset | Imageset with most of weapon info textures |
| Firemode Glow Imageset | *resource* |  | Imageset with most of weapon info textures |
| Firemode Single | string |  | Name of the icon representing single fire mode in **Firemode Icon Imageset** |
| Firemode Burst | string |  | Name of the icon representing burst fire mode in **Firemode Icon Imageset** |
| Firemode Auto | string |  | Name of the icon representing full auto fire mode in **Firemode Icon Imageset** |
| Firemode Safty | string |  | Name of the icon representing safe mode in **Firemode Icon Imageset** |
| Mag Indicator |  |  |  |

## SCR\_FactionUIInfo

*Inherits parameters from UIInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Name Upper | *string* |  | Name of the faction in upper case |

## WeaponUIInfo

*Inherits parameters from UIInfo*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Show Weapon Name | *string* |  | Show weapon name indicator in weapon UI |
| Mag Indicator | *string* |  |  |

## General components

## ActionsManagerComponent

This component is responsible for various interactions like picking up the weapon or some special interactions (custom action for i.e. folding iron sights)

## MeshObject

Ideally, weapon should have some kind of mesh

* **Object** - path to weapon model

## RigidBody

Component responsible for physic simulation (including interactions) of the weapon. Requires MeshObject with at least one collider. Weapon should ideally have collision

* **Layer Preset** - should be empty
* **Model Geometry** - should be checked - to use colliders in weapon model

## Hierarchy

Enables entity hierarchy (required by RplComponent)

## SignalsManagerComponent

Can be used for controlling values by animations, sounds, UI, particles and so on.

* Doesn't require any extra configuration for weapon

## RplComponent

Enables proper replication of this entity over network

## Weapon related components

## MuzzleEffectComponent

Component responsible for muzzle flash effects. On weapons, scripted version (SCR\_MuzzleEffectComponent) should be used

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Auto Update | *bool* |  | Should be updated independently of the component? |
| Attach To Parent | *bool* |  | Should be attached to parent entity |
| Emitter Entity Class | *resource (.et)* |  | Entity class to be spawned, it will have particle effect as visual object |
| Particle Effect | *resource (.ptc)* |  | Particle effect |
| Effect Orient Up | *bool* |  | Should effect be oriented Y up (true) or using parent entity normal (false)? |
| Enabled On Dedicated Server | *bool* |  | Should the effect works on a dedicated server? |
| Effect Position | *EntitySlotInfo* |  | Position from which particle effect will originate |
| Reset On Fire | *bool* |  | Resets emitter when muzzle is fired |

## SCR\_MuzzleEffectComponent

Defines muzzle flash particle effects (.ptc) and its effects

Should be child of of **MuzzleComponent**

*Inherits from MuzzleEffectComponent*

### Muzzle Flash

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Col | *vector* | color | Color of muzzle flash effect |
| Radius Of Flash | *float* |  | Radius of muzzle flash effect |
| LV | *float* |  | LV of muzzle flash effect |
| EV Clip | *float* |  | EVClip of muzzle flash effect |
| Shadows | *bool* |  | Should muzzle effect cast shadows |
| Max Time Of Flash | *float* | seconds | Maximumum time of muzzle flash effect |
| Offset | *vector* |  | Offset of light from muzzle flash origin |

## CaseEjectingEffectComponent

Component responsible for simpler (compared to **SCR\_MuzzleEffectComponent**) particle effects when muzzle is being fired. Used for spent case effects and gun barrel smoke.

Should be child of of **MuzzleComponent**

*Inherits from MuzzleEffectComponent*

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Water Surface | *float* |  | Surface value for water |
| Has Sound | *bool* |  | Should the effect trigger sound? |

## SCR\_MeleeWeaponProperties

Defines melee attack properties of the weapon

### Global

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Damage | *float* |  | Size of damage dealt by the weapon |
| Range | *float* | meters | Range of the weapon |
| Execution Time | *float* | seconds | How long it takes to execute the attack |

### Hit detection

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Num Of Measurements | *integer* |  | Number of measurements in frame |
| Collision Probes Pos | *array of vectors* |  | List of coords where the collision probes will be placed (related to model) |
| Num Of Failed Probes Tolerance | *integer* |  | Number of failed probes check that cause the attack will be canceled immediately |

## SCR\_WeaponStatsManagerComponent

The SCR\_WeaponStatsManagerComponent is a base component applied to weapons, allowing them to be modified by attachments like bayonets and silencers, which change both global and muzzle-specific stats. This component can be extended by mods to introduce additional moddable weapon attributes.

More info can be found on [Weapon Stats-Modifing Attachments](/wiki/Arma_Reforger:Weapon_Stats-Modifing_Attachments "Arma Reforger:Weapon Stats-Modifing Attachments") page

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Enabled | *bool* |  | Determines if components is active or not |

## SCR\_WeaponAttachmentsStorageComponent

Defines inventory properties of the weapon like display name, size, preview image and also **weapon IK pose**

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Attributes | *class* | SCR\_ItemAttributeCollection | Class containing various item attributes |
| Wb Placement From Attributes |  |  | When placing item in world editor, respect properties specified in ItemAttributeCollection (listed in Attributes class) |
| Priority | *integer* |  | Storage priority |
| Storage Purpose | *array of bools* |  | Storage usage purpose |
| Use Capacity Coefficient | *bool* |  | Use item volume coefficient for capacity |
| Capacity Coefficient | *flooat* |  | Ammount of item volume to be considered as allowed cumulative capacity. Only visible when Use Capacity Coefficient is set to true |
| Max Cumulative Volume | *float* | cm^3 | Maximum cumulative volume capacity in cm^3. Only visible when Use Capacity Coefficient is set to false |
| Max Item Size | *float* | cm^3 | Maximum allowed item size (item can be rotated) cm^3 Only visible when Use Capacity Coefficient is set to false |

### SCR\_ItemAttributeCollection

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Item Display Name | *class* | MagazineUIInfo | UI related attributes |
| Item Phys Attributes | *class* | ItemPhysicalAttributes | Physical attributes |
| Item Animation Attributes | *class* | ItemAnimationAttributes | Animation attributes |
| Custom Attributes | *array of classes* |  | Item specific attributes can be set here |
| Common Item Type | *enum* |  | Common item type identifier. Available options:  * NONE * BANDAGE * AMMO * MG\_AMMO * FOOD * BINOCULARS * COMPASS * FLASHLIGHT * RADIO * WATCH |
| Size | *enum* |  | Slot size. Available options:  * SLOT\_1x1 * SLOT\_2x1 * SLOT\_2x2 * SLOT\_3x3 |
| Slot Type | *enum* |  | Slot type. Available options:  * SLOT\_ANY * SLOT\_BACKPACK * SLOT\_LBS\_BUTTPACK * SLOT\_VEST * SLOT\_WEAPONS\_STORAGE * SLOT\_GADGETS\_STORAGE * SLOT\_LOADOUT\_STORAGE * SLOT\_LOOT\_STORAGE |
| Draggable | *bool* |  | Sets item movable by drag'n'drop |

## WeaponSoundComponent

Provides (signals) connection to sound engine

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Mute | *bool* |  | Should this component be muted by default? |
| Filenames | *array of resources* |  | List of audio projects which are assigned to this asset |
| On Frame Update | *bool* |  | Enable/disable on-frame updating of the component |
| Script Callbacks | *bool* |  | Callbacks for scripted methods |
| Distance Management | *bool* |  | Updating sounds and ability to play them automatically enabled or disabled based on their distance from camera |
| Include Inactive | *bool* |  | Distance management for inactive component |
| Sound Points | *array of classes* | SountPointInfo | Array of custom of sound position. With this parameter it is possible to specify place from which the sound will be played |

## WeaponAnimationComponent

General weapon animation components (includes weapon related animations for characters)

Should be child of of **WeaponComponent**

*Inherits from BaseItemAnimationComponent*

Exposes following variables to be used in animation

* TriggerPulled (bool) - Indicates if weapon's trigger is pulled
* Firing (bool) - Indicates if weapon is currently firing
* State (int) - Represents the firemode state
* SightElevation (float) - Controls weapon sight elevation/zeroing
* Cocked (bool) - Indicates if weapon's hammer is cocked
* Empty (bool) - Indicates if weapon barrel is empty
* LastBullet (bool) - Indicates if last bullet is in the weapon
* Bipod (bool) - Indicates if bipod is deployed
* BipodPitch (float) - Bipod pitch angle
* BipodYaw (float) - Bipod yaw angle
* BipodRoll (float) - Bipod roll angle
* OpenBoltState (bool) - Indicates bolt position state

Additionally, following commands are exposed

1. [CMD\_Weapon\_Reload](/wiki/Arma_Reforger:Animation_Editor:_Character_Action_Commands "Arma Reforger:Animation Editor: Character Action Commands") - Controls reload animations
2. CMD\_Weapon\_MagRelease - Controls magazine release animations (not used anywhere)
3. CMD\_Weapon\_OpenCover - Controls weapon cover opening animations (was used on MG, currently not functional)
4. CMD\_Weapon\_Unfold - Controls weapon folding/unfolding animations (used on LAW)

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Anim Graph | *resource* |  | Animation graph resource |
| Anim Instance | *resource* |  | Animation graph instance |
| Start Node | *string* |  | Node where to start from |
| Always Active | *bool* |  | Should component be always active? Otherwise will be active only when synced with Character. |
| Anim Injection | *class* | AnimationAttachmentInfo | Character animation injection to be set to character's anim graph attachment node |
| Bind With Injection | *bool* |  | Bind commands only when character injection is setup |
| Auto Command Bind | *bool* |  | Automatically Bind all commands from template |
| Anim Commands To Bind | *array of classes* |  | Only visible when Auto Command Bind is set to false |
| Auto Variables Bind | *bool* |  | Automatically Bind all variables from template |
| Anim Variables To Bind | *array of classes* |  | Variables which should be additionally bind to weapon animation graph. Only visible when Auto Variables Bind is set to false |
| Mesh Visibility Configuration | *class* | MeshesVisibilitySwitchConfig | Animation commands to be synchronised with character graph |
| Rest On Deactivation | *bool* |  | On component deactivation, should the internal animation state be reset and the base pose applied? |
| Simulate On Headless | *bool* |  | Should we simulate current animations on headless builds? |
| Deactivation Delay | *float* |  | Delay before animations go to sleep when inactive state was requested |

### AnimationAttachmentInfo

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Anim Graph | *resource* |  | Animation graph name |
| Anim Instance | *resource* |  | Animation graph instance |
| Start Node | *string* |  | Node where to start from |
| Binding Name | *string* |  | Binding name in root graph |

## SightsComponent

GameComponet responsible for sights mechanics (like weapon ironsights)

Should be child of of **MuzzleComponent**

*Has two special actions in context menu available after clicking on Component name with Right Mouse Button:*

* *Process zeroing data*
* *Toggle sight point diag*

### BaseSights

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Sights Positions | *class* | PointInfo | Position of the aiming down sight camera. Usually "eye" Pivot ID is used |
| Sights Ranges | array | SightRangeInfo | Zeroing Range, Rotation and Translation |
| Sights Ranges Default Index | integer |  |  |
| Sights FOV Info | *class* | SCR\_FixedFOVInfo | Field of View defining info object |
| Sights Point Front | *class* | PointInfo | The sight point further from the character's eye |
| Sights Point Rear | *class* | PointInfo | The sight point closer to the character's eye |
| ADS Time | float | seconds | Time to ADS in seconds |
| ADS Update Owner Activate State | bool |  | Activate/Deactivate owner on ADS |
| Camera Recoil Amount | float |  | Percentage of recoil applied to camera (0-300%) |

### Sound

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Sound Int | *integer* |  | Value of corresponding sound signal when sight is used. |

### SCR\_FixedFOVInfo

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Field Of View | *float* | degrees | Fixed field of view in degrees |

### SightRangeInfo

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Range | *integer* |  | Range information (x = animation value, y = range in meters) |
| Weapon Position | class | PointInfo | ADS camera position adjustments for this Range |

## AttachmentSlotComponent

For attaching various attachment on weapon (scopes, flashlights, etc.).

Should be child component of **WeaponComponent** (*for weapon related accessories like scopes, flashlights, etc*) **or MuzzleComponent** (*for muzzle related attachments like suppressors*)

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Attachment Slot | *class* |  | Defines position of the attachment |
| Attachment Type | *class* |  | Object which specifies compatibility. |

## WeaponComponent

General weapon components.

*Has two special actions in context menu available after clicking on Component name with Right Mouse Button:*

* *Process zeroing data*
* *Toggle sight point diag*

### Weapon Obstruction

This category contains properties that control how the weapon handles obstruction when the character is in close proximity to objects in the environment. These settings affect how the weapon and character respond to obstructions to prevent clipping and enhance authenticity .

| Param | Type | Default value | Unit | Description |
| --- | --- | --- | --- | --- |
| Should Handle Obstruction | *bool* | `true` |  | Determines whether the weapon should handle obstruction when equipped. If set to ***true,*** obstruction handling is enabled for this weapon. |
| Obstruction Test Character Offset | *vector* | `(0.0, 0.0, 0.0)` | m | Specifies an offset from the character's reference point (usually the neck) to the weapon handling point. This offset is used during obstruction tests to accurately represent the weapon's position relative to the character. |
| Obstruction Test Added Length | *float* | `0.0` | m | Defines additional length added to the end of the obstruction test ray (which extends from the character's reference point plus any offsets). This accounts for extra distance beyond the weapon's length that should be considered during obstruction tests. |
| Obstruction Max Torso Z Offset | *float* | `0.15` | m | Specifies the maximum distance along the Z-axis (forward/backward direction) that the character's torso can move back when in significant or full obstruction. During slight obstruction, the torso position is smoothly interpolated up to this maximum value. This prevents the weapon from clipping through objects by adjusting the character's posture. |
| Sign Obstr Weapon Constant Offset | *vector* | `(0.1, -0.25, -0.1)` | m | ***Significant Obstruction Weapon Constant Offset*** Sets a base offset for the weapon's position when in significant obstruction. This constant offset is applied via Inverse Kinematics (IK) to reposition the weapon appropriately when obstructions are detected. |
| **Slight Obstr Weapon Max Z Offset** | *float* | `0.05` | m | ***Slight Obstruction Weapon Max Z Offset*** Specifies the maximum distance along the Z-axis that the weapon can be moved back via IK during maximum slight obstruction. The weapon's position is smoothly interpolated up to this maximum value based on the level of obstruction. |
| Sign Obstr Max Weapon Z Offset | *float* | `0.2` | m | ***Significant Obstruction Max Weapon Z Offset*** Specifies the maximum distance along the Z-axis that the weapon can be moved back via IK during maximum significant obstruction. This provides additional repositioning for more severe obstructions. |
| Can Go to Full Obstruction | *bool* | `true` |  | Determines whether the weapon can enter a full obstruction state. If set to `false`, the weapon will not go into full obstruction regardless of how much it clips into objects, potentially allowing firing but causing visual clipping. |
| Obstruction Test BB Scale | *vector* | `(0.05, 0.05, 0.025)` | % | Determines the scale applied to the weapon's bounding box during obstruction tests. Adjusting this scale changes the sensitivity of obstruction detection by effectively altering the size of the weapon's collision volume used in tests. Only bounding box of model defined in Mesh Object component is used, meaning if you have modular weapon where stock is attachment that is extending the size of the weapon, then you should consider adjusting this bounding box parameter. |

### Camera Impact

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Sights Camera Roll Scaler | *float* |  | Recoil Roll (angular Z rotation) camera scaler, 0 allowed |

### Sound

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Sound Int | *integer* |  | Value of corresponding sound signal when weapon is hold |

### Rest

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Signals Source Access | *class* |  |  |
| Use Aiming Type | *string* |  | Defines which aiming component will be used Available options:   * None * Weapon * MainTurret * TargetingPod |
| Is Throwable | *bool* |  | Is this weapon a throwable item (like grenade)? |
| Weapon Type | *string* |  | Defines how AI will recognize this weapon. Available options:   * None * Rifle * GrenadeLauncher * SniperRifle * RocketLauncher * MachineGun * Handgun * FragGrenade * SmokeGrenade |
| UI Info | *class* | WeaponUIInfo |  |
| Weapon Slot Type | *string* |  | Weapon slot type |
| Weapon Offset | *vector* |  | Linear offset of weapon when held |
| Weapon Offset ADS | *vector* |  | Linear offset of weapon when aiming |
| Can Be On Sling | *bool* |  | Should current weapon be allowed to be placed on sling? |

## MuzzleComponent

Responsible for "firing" part of weapon, has definition of muzzle, fire modes, magazine compatibility (magwells), magazine position, recoil, spread and sway

Should be child component of one of the **WeaponComponent**

### Magazine

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Magazine Well | *class* |  | This parameter defines Magazine Well class of muzzle. All magazines which are also using same **Magazine Well** class (or one parented to it) will be usable in this muzzle |
| Magazine Template | *resource* |  | This parameter defines which magazine prefab is pre-attached to weapon. If you want to have no magazine attached to weapon by default, leave this parameter empty |
| Magazine Position | *class* | InventoryStorageSlot | Defines where magazines are attached. Usually **Pivot ID** "*slot\_magazine*" is used and **Child Pivot ID** "*snap\_weapon*" |

### Weapon moddifiers

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Weapon Aim Modifiers | *array of classes* | *...AimModifier* classes |  |
| Bullet Init Speed Coef | *float* | coefficient | How much this muzzle changes initial speed of bullet. F.e. longer barrels achieve higher velocity. |
| Dispersion Diameter | *float* | meters | Diameter of dispersion cone at Dispersion Range. |
| Dispersion Range | *float* | meters | Distance from target where Dispersion Diameter is being measured |

### Fire Modes

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Fire Modes | *array of classes* | BaseFireMode | Array of weapon fire modes |

#### BaseFireMode

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Max Burst | *float* | rounds | Max amount of bullets that will be fired one after another when trigger is pressed. Behavior is further defined by **Burst Type** parameter |
| Burst Type | *string* |  | Only visible when **Max Burst** is above 0 Available options   * **Uninterruptable -** Burst cannot be interrupted - after pressing the fire button, weapon will fire exactly as many bullets as it is specified by Max Burst parameter (unless magazine will run out of ammo * **Interruptable -** Weapon will file as many rounds as defined in Max Burst as long as trigger is pressed. Weapon will have burst memory * **InterruptableAndReseting -** As above but without burst memory |
| Max Salvo | *float* | rounds | Amount of bullets fired at once after pressing trigger once |
| Rounds Per Minute | *float* | rounds per minute | Cyclic rate of fire of weapon in rounds per minute |
| UI Name | *string* |  | UI Name - unused |
