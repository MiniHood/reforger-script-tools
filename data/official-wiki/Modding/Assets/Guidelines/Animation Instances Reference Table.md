# [Animation Instances Reference Table](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Instances_Reference_Table)

Below tables provide reference of animations and [export profiles](/wiki/Arma_Reforger:Animation_Export_Profiles "Arma Reforger:Animation Export Profiles") which should be used when [exporting TXA animations from Blender](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_Animation#Export_animations_from_Blender "Arma Reforger:Enfusion Blender Tools: Import/Export Animation").

## Rifle

[**Sample New Weapon**](/wiki/Arma_Reforger:Weapon_Creation "Arma Reforger:Weapon Creation") ([download link](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewWeapon/Assets/Weapons/Rifles/SampleWeapon_01/Anims/SampleWeapon_01_Animations.blend)) is used here as a **reference**, so not all animations are present - like *Finger\_trigger\_in* or *Sight*.
In such a case, **animations of AK74** are provided to give some guidance on what should be filled in.

[![](/wikidata/images/thumb/8/83/armareforger-export-profiles-rifle-reference.png/800px-armareforger-export-profiles-rifle-reference.png)](/wiki/File:armareforger-export-profiles-rifle-reference.png)

Sample New Weapon export profiles example

### Weapon Instance

| Action name | Description | Animation take used | Action used | Export profile | Export as |
| --- | --- | --- | --- | --- | --- |
| BoltPose | Poses of the bolt.  * First frame represents closed bolt * Last frame represents open bolt | 4 | w\_sampleweapon\_fire | 724\_Weapon\_Bolt | w\_rfl\_sampleweapon\_01\_bolt\_pose |
| Finger\_trigger\_in |  | 6 |  | 999\_Empty |  |
| Finger\_trigger\_out |  | 6 |  | 999\_Empty |  |
| Fire | Animation of weapon being fired - trigger is pressed and bolt is doing one full cycle | 3 | w\_sampleweapon\_fire | 724\_Weapon\_Bolt\_Trigger | w\_rfl\_sampleweapon\_01\_fire |
| Idle | Empty animation | 1 |  | 999\_Empty |  |
| Idle\_finger | Empty animation | 1 |  | 999\_Empty |  |
| IKOffset | No animation assigned | - | - | - | - |
| Reload\_InsertMag | Animation of inserting magazine into the weapon. 722\_Weapon\_MagRelease profile can be used if there is animated weapon magazine release on the weapon | 72 | rfl\_sampleweapon\_erc\_insert\_mag | 999\_Empty | w\_rfl\_sampleweapon\_01\_mag\_insert |
| Reload\_RemoveMag | Animation of removing magazine from the weapon. 722\_Weapon\_MagRelease profile can be used if there is animated weapon magazine release on the weapon | 59 | rfl\_sampleweapon\_erc\_remove\_mag | 999\_Empty | w\_rfl\_sampleweapon\_01\_mag\_remove |
| ReloadActionBolt | Animation of racking the bolt | 36 | rfl\_sampleweapon\_erc\_bolt\_rack | 999\_Empty | w\_rfl\_sampleweapon\_01\_bolt\_rack |
| Safety | Safety mode animation. First frame safety mode, second single mode and third one is used for full auto | 3 | w\_sampleweapon\_safety | 701\_Rifle\_Idle | w\_rfl\_sampleweapon\_01\_safety |
| Sight | Iron sight adjustment animation. Amount of frames is equal to amount of zeroing modes defined in the weapon | 11 |  | 723\_Weapon\_Sight |  |
| Switch\_Mode\_Off | Empty animation for switching safety switch off. This animation need to have same amount of frames as the one in player instance | 16 |  | 999\_Empty |  |
| Switch\_Mode\_On | Empty animation for switching safety switch on. This animation need to have same amount of frames as the one in player instance | 16 |  | 999\_Empty |  |
| Trigger | Animation of trigger when its pressed | 4 | w\_sampleweapon\_fire | 721\_Weapon\_Trigger | w\_rfl\_sampleweapon\_01\_trigger |

## Player Instance

| Action name | Description | Number of frames | Animation take used | Export profile | Exported as |
| --- | --- | --- | --- | --- | --- |
| BoltPose | - | - | - | - | - |
| Finger\_trigger\_in |  | 6 |  | 502\_IK\_RightHand |  |
| Finger\_trigger\_out |  | 6 |  | 502\_IK\_RightHand |  |
| Fire | Animation of weapon being fired - trigger is pressed and bolt is doing one full cycle | 3 |  | 401\_RIndexABS |  |
| Idle | Empty animation | 1 | using same animation as **Safety** | | |
| Idle\_finger | Empty animation | 1 |  | 502\_IK\_RightHand |  |
| IKOffset | IK offset animation | 1 | p\_sampleweapon\_ik | 101\_FullBodyABS | p\_rfl\_sampleweapon\_01\_offset |
| Reload\_InsertMag | Animation of inserting magazine into the weapon | 72 | rfl\_sampleweapon\_erc\_insert\_mag rfl\_sampleweapon\_pne\_insert\_mag | 252\_UpperbodyADD\_ArmsABS | p\_rfl\_sampleweapon\_01\_erc\_mag\_insert p\_rfl\_sampleweapon\_01\_pne\_mag\_insert |
| Reload\_RemoveMag | Animation of removing magazine from the weapon | 59 | rfl\_sampleweapon\_erc\_remove\_mag rfl\_sampleweapon\_pne\_remove\_mag | 252\_UpperbodyADD\_ArmsABS | p\_rfl\_sampleweapon\_01\_erc\_mag\_remove p\_rfl\_sampleweapon\_01\_pne\_mag\_remove |
| ReloadActionBolt | Animation of racking the bolt | 36 | rfl\_sampleweapon\_erc\_bolt\_rack rfl\_sampleweapon\_pne\_bolt\_rack | 252\_UpperbodyADD\_ArmsABS | p\_rfl\_sampleweapon\_01\_erc\_bolt\_rack p\_rfl\_sampleweapon\_01\_pne\_bolt\_rack |
| Safety | Safety mode animation. First frame safety mode, second single mode and third one is used for full auto | 3 | p\_sampleweapon\_ik | 502\_IK\_RightHand | p\_rfl\_sampleweapon\_01\_safety |
| Sight | Iron sight adjustment animation. Amount of frames is equal to amount of zeroing modes defined in the weapon | 11 | *empty anim, need same length as Sight in weapon* | 999\_Empty | - |
| Switch\_Mode\_Off | Empty animation for switching safety switch off. This animation need to have same amount of frames as the one in player instance | 16 | *it is possible to use same anim as Switch\_Mode\_On* | 262\_ArmsADD\_RHandABS | - |
| Switch\_Mode\_On | Empty animation for switching safety switch on. This animation need to have same amount of frames as the one in player instance | 16 | p\_rfl\_sampleweapon\_01\_modes\_on | 262\_ArmsADD\_RHandABS | p\_rfl\_sampleweapon\_01\_modes\_on |
| Trigger | Animation of finger when trigger is pressed | 4 | using same animation as **Fire** | | |
| Animation IK Pose | Inverted Kinematics pose defined inside **SCR\_WeaponAttachmentsStorageComponent** in Item **Animation Attributes** sections under name **Animation IK Pose** | 1 | p\_sampleweapon\_ik | 501\_IK | p\_rfl\_sampleweapon\_01\_ik |
