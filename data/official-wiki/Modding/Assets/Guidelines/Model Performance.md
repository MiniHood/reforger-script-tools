# [Model Performance](https://community.bistudio.com/wiki/Arma_Reforger:Model_Performance)

* ***In general***, try to keep the polycount as low as possible; of course, such decision depends on the asset type and its specific use case.
* ***In general***, the **vertex density** of different types of assets goes in this order of importance (descending, #1 = biggest density):
  1. weapon and gadget
  2. character and gear
  3. vehicle
  4. prop
  5. structure
  6. vegetation

:   ⚠

    This does **not** mean that e.g a vehicle must always have less vertices than a weapon; it means that ***relative to its size***, a vehicle **in general** has less vertices.A knife (weapon) will definitely have less vertices compared to a haul truck (vehicle)...!

* ***In general***, create LODs using Bohemia Interactive's [Enfusion Blender Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools "Arma Reforger:Enfusion Blender Tools"); don't hesitate fixing them too.
  + LOD0, LOD1 and the last LOD are usually created manually, intermediate LODs being creatable by tools.

:   ⚠

    If you import a heavily detailed 3D model as LOD0 without retouching it/creating smaller LODs,the game will keep this model in memory for as long as it will be in game's world.  
    Do not then be surprised by the mod's negative performance impact!

## Instance

Repetitive complex elements must be split to save on memory, allowing then to assemble them in Prefabs.

🏗

This article is a **[work in progress](/wiki/Category:WIP "Category:WIP")**!

## Vanilla Values

As a point of reference, here are vanilla values for multiple common assets; be sure to aim for the general category's ballpark.

ⓘ

These are [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)") (1.6.0.119) values.

| Asset | Category | LOD 0 | | | | | Last LOD | | | | |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Vertices | Faces | Materials | Meshes | Bones | Vertices | Faces | Materials | Meshes | Bones |
| [M16A2\_body.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/M16A2/M16A2_body.xob) | Weapon (Assault Rifle) | 42631 | 49842 | 7 | 7 | 12 | 1142 | 674 | 6 | 6 | 7 |
| [AK74\_body.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/AK74/AK74_body.xob) | Weapon (Assault Rifle) | 34614 | 41452 | 6 | 6 | 9 | 660 | 391 | 6 | 6 | 2 |
| [M9\_body.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Handguns/M9/M9_body.xob) | Weapon (Handgun) | 14891 | 17515 | 2 | 2 | 6 | 213 | 114 | 1 | 1 | 2 |
| [PM\_body.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Handguns/PM/PM_body.xob) | Weapon (Handgun) | 5888 | 7551 | 1 | 1 | 6 | 341 | 309 | 1 | 1 | 2 |
| [Binoculars\_B12.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Items/Equipment/Binoculars/Binoculars_B12/Binoculars_B12.xob) | Gadget | 4719 | 5774 | 1 | 1 | 0 | 56 | 34 | 1 | 1 | 0 |
| [Binoculars\_B8.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Items/Equipment/Binoculars/Binoculars_B8/Binoculars_B8.xob) | Gadget | 6919 | 8681 | 4 | 4 | 0 | 311 | 250 | 1 | 1 | 0 |
| [Binoculars\_M22.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Items/Equipment/Binoculars/Binoculars_M22/Binoculars_M22.xob) | Gadget | 7245 | 9965 | 1 | 1 | 0 | 107 | 76 | 1 | 1 | 0 |
| [Basebody\_Male\_Head\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Basebody/Basebody_Male_Head_01.xob) | Character (Head) | 18932 | 34278 | 7 | 7 | 89 | 654 | 928 | 2 | 2 | 69 |
| [Basebody\_Male\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Basebody/Basebody_Male_01.xob) | Character (Body) | 33181 | 57088 | 4 | 32 | 100 | 1230 | 1296 | 4 | 32 | 29 |
| [Helmet\_M1\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/HeadGear/Helmet_M1/Helmet_M1_01.xob) | Gear (Helmet) | 7876 | 11342 | 1 | 1 | 1 | 156 | 228 | 1 | 1 | 1 |
| [Helmet\_M1\_Cover.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/HeadGear/Helmet_M1/Helmet_M1_Cover.xob) | Gear (Helmet) | 8391 | 12199 | 2 | 2 | 1 | 123 | 183 | 1 | 1 | 1 |
| [Helmet\_SSh68\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/HeadGear/Helmet_SSh68/Helmet_SSh68_01.xob) | Gear (Helmet) | 2976 | 3779 | 1 | 1 | 1 | 51 | 70 | 1 | 1 | 1 |
| [Helmet\_SSh68\_01\_cover.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/HeadGear/Helmet_SSh68/Helmet_SSh68_01_cover.xob) | Gear (Helmet) | 3320 | 3673 | 1 | 1 | 1 | 50 | 48 | 1 | 1 | 1 |
| [Jacket\_BDU\_M81.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Jacket_BDU_M81/Jacket_BDU_M81.xob) | Gear (Top) | 6129 | 7950 | 1 | 1 | 49 | 237 | 208 | 1 | 1 | 45 |
| [Jacket\_KZS.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Jacket_KZS/Jacket_KZS.xob) | Gear (Top) | 6016 | 9968 | 1 | 1 | 35 | 252 | 278 | 1 | 1 | 34 |
| [Jacket\_M70\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Jacket_M70/Jacket_M70_01.xob) | Gear (Top) | 11003 | 17090 | 1 | 1 | 48 | 284 | 205 | 1 | 1 | 10 |
| [Pants\_BDU\_M81.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Pants_BDU_M81/Pants_BDU_M81.xob) | Gear (Legwear) | 4023 | 5554 | 1 | 1 | 20 | 117 | 100 | 1 | 1 | 10 |
| [Pants\_KZS.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Pants_KZS/Pants_KZS.xob) | Gear (Legwear) | 2856 | 5092 | 1 | 1 | 16 | 176 | 220 | 1 | 1 | 20 |
| [Pants\_M70\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Uniforms/Pants_M70/Pants_M70_01.xob) | Gear (Legwear) | 6129 | 7950 | 1 | 1 | 19 | 237 | 208 | 1 | 1 | 10 |
| [CombatBoots\_US\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Footwear/CombatBoots_US_01/CombatBoots_US_01.xob) | Gear (Footwear) | 2176 | 3148 | 1 | 1 | 6 | 133 | 100 | 1 | 1 | 4 |
| [CombatBoots\_Soviet\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Footwear/CombatBoots_Soviet_01/CombatBoots_Soviet_01.xob) | Gear (Footwear) | 2054 | 3140 | 1 | 1 | 9 | 122 | 100 | 1 | 1 | 4 |
| [m151a2\_base.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Wheeled/M151A2/m151a2_base.xob) | Vehicle (Car) | 34709 | 36022 | 12 | 12 | 45 | 131 | 70 | 1 | 1 | 0 |
| [UAZ469\_base.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Wheeled/UAZ469/UAZ469_base.xob) | Vehicle (Car) | 32826 | 34755 | 13 | 13 | 37 | *2658* | *1448* | 4 | 4 | 5 |
| [BTR70\_body.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Wheeled/BTR70/BTR70_body.xob) | Vehicle (Armored) | 148317 | 148127 | 18 | 18 | 48 | 78 | 44 | 1 | 1 | 0 |
| [BRDM2\_base.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Wheeled/BRDM2/BRDM2_base.xob) | Vehicle (Armored) | 186443 | 182515 | 35 | 35 | 85 | 494 | 372 | 1 | 1 | 0 |
| [UH1H\_base.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Helicopters/UH1H/UH1H_base.xob) | Vehicle (Helicopter) | 142885 | 106391 | 19 | 20 | 57 | 183 | 88 | 1 | 1 | 0 |
| [Mi8MT\_base.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Helicopters/Mi8/Mi8MT_base.xob) | Vehicle (Helicopter) | 178222 | 118897 | 23 | 23 | 67 | 118 | 60 | 2 | 2 | 1 |
| [Barracks\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Structures/Military/Houses/Barracks_01/Barracks_01.xob) | Structure | 13455 | 11036 | 4 | 4 | 0 | 194 | 98 | 1 | 1 | 0 |
| [Barracks\_E\_02.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Structures/Military/Houses/Barracks_E_02/Barracks_E_02.xob) | Structure | 95272 | 98776 | 15 | 15 | 0 | 178 | 94 | 1 | 1 | 0 |
| [Barracks\_E\_03.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Structures/Military/Houses/Barracks_E_03/Barracks_E_03.xob) | Structure | 129523 | 119760 | 14 | 14 | 0 | 168 | 92 | 1 | 1 | 0 |
