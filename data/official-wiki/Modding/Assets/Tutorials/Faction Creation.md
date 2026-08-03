# [Faction Creation](https://community.bistudio.com/wiki/Arma_Reforger:Faction_Creation)

## Goals of this tutorial

In this tutorial you will learn about:

* Configuration of characters
* Preparation of units groups
* Creation of new factions
* Integration of new faction into [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master")
* Integration of new faction into [Conflict game mode](/wiki/Arma_Reforger:Conflict "Arma Reforger:Conflict")

ⓘ

This tutorial touches quite a lot of topics and it is recommended to familiarise yourself with [**modded weapon**](/wiki/Arma_Reforger:Weapon_Modding "Arma Reforger:Weapon Modding")  and [**modded car**](/wiki/Arma_Reforger:Car_Modding "Arma Reforger:Car Modding")  tutorials.

📥

Sources files for this tutorial can be found on
[**Arma Reforger Samples Github repository**](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/tree/main/SampleMod_NewFaction)

## Introduction

In this tutorial, process of creating new faction is explained and as an example***, secrets of preparing** Sample Faction - REDFOR* will be unveiled**.**

This page goes through topics like **preparing new characters**, **tweaking their configuration**, **creating new groups**, preparing general **faction configs** & finally, **integration into** both **in-game editor** & **Conflict** game mode.

While the tutorial covers mostly process of creating faction out of already existing assets (or retextures of that content), same principle applies also to new content.

Since process of **retexturing character equipment** is slightly different from other assets like cars, buildings or weapons, this tutorial also contains small explanatory chapter about it.

Sample New Faction contains three new factions & this tutorial describes only steps necessary to prepare **SampleFactionREDFOR**. Steps for **BLUFOR** & **GREENFOR** factions are basically the same as for **REDFOR** faction.

### File structure

New faction requires quite elaborate structure (*based on [Arma\_Reforger:Directory\_Structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure") page*) and below there is tear down of it:

[![armareforger-new-faction-file-structure.png](/wikidata/images/thumb/4/4b/armareforger-new-faction-file-structure.png/800px-armareforger-new-faction-file-structure.png)](/wiki/File:armareforger-new-faction-file-structure.png)

## Character configuration

### Retexturing existing equipment

One of the first steps of creating a retextured piece of equipment is actually creating a new prefab. In this tutorial, a new **red** variant of [CombatBoots\_Soviet\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Footwear/CombatBoots_Soviet_01.et) prefab will be created. This can be done either through:

* **[Inherit](/wiki/Arma_Reforger:Prefabs_Basics#Using_Inherit_Prefab_option "Arma Reforger:Prefabs Basics")** option in **Resource Browser**
* [By dropping instance of prefab to **World Editor** & then making a new prefab by grabbing that instance back selected folder in **Resource Browser**](/wiki/Arma_Reforger:Prefabs_Basics#Drag_and_drop_method "Arma Reforger:Prefabs Basics")

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

In both cases, goal is to create **CombatBoots\_Soviet\_01\_red.et** prefab inside Sample New Faction addon structure.

In comparison to other assets, character equipment is bit more complicated to retexture. **MeshObject** component of wearable equipment is actually not being used for showing the object and instead, visuals are controlled through **Worn Model** & **Item Model** properties. One of them define model which is visible when character is **wearing given object** *(**Worn Model** property)* and the other one is used when item is lying on a ground as an prop which can be picked up by other characters *(**Item Model** property).*

Those two parameters accept both **XOB** models and **prefabs** as well. In both cases, **new materials** can be created by for instance **[duplicating existing materials](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics")**. In this case, [CombatBoots\_Soviet\_01.emat](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Footwear/CombatBoots_Soviet_01/Data/CombatBoots_Soviet_01.emat) was duplicated to SampleMod\_NewFaction addon as *CombatBoots\_Soviet\_01\_red.emat* and then color was changed by simply editing **Color** property in that new material.

📖

**Recommended read:** More details about creating new material creation & some practical example can be found in [**Weapon Modding**](/wiki/Arma_Reforger:Weapon_Modding#New_Materials "Arma Reforger:Weapon Modding") tutorial. Keep in mind that in a lot of cases it might be necessary to use MatPBRMulti shadder, so usage of MatPBRCamo might not be an option.

Before proceeding further, prepare new materials

### Creating retextures using prefabs

Creating **prefabs** for retextures is quite easy since process process is exactly same as for regular items.

[![armareforger-new-faction-equipment-prefabs-1.png](/wikidata/images/thumb/6/6c/armareforger-new-faction-equipment-prefabs-1.png/600px-armareforger-new-faction-equipment-prefabs-1.png)](/wiki/File:armareforger-new-faction-equipment-prefabs-1.png)[![armareforger-new-faction-equipment-prefabs-2.png](/wikidata/images/thumb/2/2d/armareforger-new-faction-equipment-prefabs-2.png/600px-armareforger-new-faction-equipment-prefabs-2.png)](/wiki/File:armareforger-new-faction-equipment-prefabs-2.png)

Those new prefabs should be as simple as possible and they can be created by simply dragging XOB model to **World Editor** view port. As a result of that action, a new **Generic Entity** instance with **MeshObject** component is created and there, assigned materials can be changed as on any other entity. Once assigned **materials** are adjusted, this entity can be transformed to prefab by grabbing it from **Hierarchy** list to desired folder in **Resource Browser** window.

[File:armareforger-new-faction-equipment-new-variant.mp4](/wiki/File:armareforger-new-faction-equipment-new-variant.mp4 "File:armareforger-new-faction-equipment-new-variant.mp4")

### Creating retextures using overrides

[![armareforger-new-faction-overrides.png](/wikidata/images/a/a8/armareforger-new-faction-overrides.png)](/wiki/File:armareforger-new-faction-overrides.png)

Alternatively, as mentioned before, it is also possible to use **Materials Override** parameters which are located under **Worn & Item Model parameters.**

In this case, it is necessary to add new element in **Materials Override** list and after that **assign desired material** to that property. If everything is done correctly, Workbench view port should show changed materials.

Under the hood, engine is remaping all materials assigned to this asset in order specified by **Materials Override** list, therefore caution is recommended since it might happen that order of materials changes after some data tweak. In this case responsibility falls into user hands and **order of the materials has to be adjusted manually by the author of mod**!

Those steps need to be performed for **both Worn & Item Models**. After that is completed, those new prefabs can be assigned to those two respective fields in **BaseLoadoutClothComponent**.

Additionally, it might be also worth to adjust **Preview Model** in **SCR\_UniveralInventoryStorageComponent** > Custom Attributes > PreviewRenderAttributes, which controls how equipment is visible in inventory menu. Please note that right now this entry doesn't support material overrides and only prefabs with custom materials can be used.

In this tutorial, following prefabs (*with respective item & worn variants*) were created for **REDFOR** faction:

* **CombatBoots\_Soviet\_01\_red**.et + (*CombatBoots\_Soviet\_01\_item\_red.et & CombatBoots\_Soviet\_01\_wear\_red.et*) - retexture of [CombatBoots\_Soviet\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Footwear/CombatBoots_Soviet_01.et)
* **Helmet\_SSh68\_01\_red.**et **+** (*Helmet\_SSh68\_01\_item\_red.et & Helmet\_SSh68\_01\_wear\_red.et* **)** - retexture of [Helmet\_SSh68\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/HeadGear/Helmet_SSh68_01/Helmet_SSh68_01.et)
* **Jacket\_M88\_red.**et **+** *(Jacket\_M88\_item\_red.et & Jacket\_M88\_wear\_red.et)* - retexture of [Jacket\_M88.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Uniforms/Jacket_M88.et)
* **Pants\_M88\_red.**et + (*Pants\_M88\_item\_red.et & Pants\_M88\_wear\_red.et*) - retexture of [Pants\_M88.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Uniforms/Pants_M88.et)
* **Vest\_Lifchik\_red.**et **+** *(Vest\_Lifchik\_item\_red.et & Vest\_Lifchik\_wear\_red.et)* - retexture of [Vest\_Lifchik.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_Lifchik/Vest_Lifchik.et)
* **Vest\_Lifchik\_GL\_red**.et + (*Vest\_Lifchik\_GrenadeBelt\_item\_red.et & Vest\_Lifchik\_GrenadeBelt\_wear\_red.et* ) - retexture of [Vest\_Lifchik\_GL.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_Lifchik/Vest_Lifchik_GL.et)

This set should be enough to have truly red faction.

⚠

**Materials Override** tend to be less reliable than prefab method so its recommend to use **Prefabs** for retextures.

## Creating new characters prefabs

### Preparing base character

Before proceeding with creating specific characters types like rifleman, medic or driver, it is generally a good practice to establish a base prefab, which will be used through the faction. Such prefab can contain common configuration like:

* Faction
* Radio protocol
* Identity
* Common parts of equipment (jacket, pants, boots and accessories)

In this example, [Character\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/Character_Base.et) prefab is used to create **Character\_SampleFactionREDFOR\_Base**.et - a base for all REDFOR faction characters. Inheriting from [Character\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/Character_Base.et) prefab should provide a solid base with all necessary components in a state, where only minor adjustments are necessary.

### Configuring new characters

Below there is a table with most essential components which will be adjusted in further steps

|  |  |
| --- | --- |
| **SCR\_CommunicationSoundComponent** | Defines **radio protocol** which will be used by this character |
| **SCR\_CharacterInventoryStorageComponent** | Main component which controls character inventory |
| **CharacterIdentityComponent** | This component holds information about identity of character like **head**, **base body** & **voice type** |
| **BaseWeaponManagerComponent** | This component controls which weapon is equipped by default |
| **BaseLoadoutManagerComponent** | Defines character **loadout** like **uniform, vest or harness, headgear, etc** |
| **FactionAffiliationComponent** | Defines to which **faction** this unit belongs to. |
| **SCR\_EditableCharacterComponent** | This component contains in-game related configuration like **preview pictures, attributes, display name or budget** |
| **SCR\_InventoryStorageManagerComponent** | This components controls what items are preassigned to character & in which part of equipment they should be stored in |
| **CharacterWeaponSlotComponent** | Those components are used to control how many weapons (& what types of weapons) can have, |
| **CharacterGrenadeSlotComponent** | This slot controls hand grenade logic |

#### Changing used radio protocol

[![armareforger-new-faction-character-protocol.png](/wikidata/images/4/43/armareforger-new-faction-character-protocol.png)](/wiki/File:armareforger-new-faction-character-protocol.png)

Starting with **SCR\_CommunicationSoundComponent'*, a default radio protocol will be changed by adjusting assigned*** *audio configuration project files **in** Filenames **property'***. In this tutorial, default, English radio protocol will be replaced with Russian variant, which uses **\_RU suffix.**

#### Changing character identity

[![armareforger-new-faction-character-identity.png](/wikidata/images/4/4c/armareforger-new-faction-character-identity.png)](/wiki/File:armareforger-new-faction-character-identity.png)

Identity of character defines their **voice variant** *(Voice parameter)*, visual **appearance** (*Head & Body parameters*) & **name -** all that configuration is stored in **CharacterIdentityComponent**. Unless working with custom model, there is no need to touch **Body Meshes Config**, since default values provide optimal configuration for wounded & healthy character materials.

By default, **character identity will be randomly generated using parameters stored in faction config** (this described later on that page) and values used in this component will be used only if **Override** property is checked.

#### Changing character faction

[![armareforger-new-faction-character-faction.png](/wikidata/images/6/62/armareforger-new-faction-character-faction.png)](/wiki/File:armareforger-new-faction-character-faction.png)

Character faction affiliation determines faction to which unit belongs to. Unlike in previous Arma titles made on Real Virtuality engine, characters in Reforger are no longer divided into few hardcoded sides but instead, faction system is used. In this setup, it is possible to setup alliances between factions in more flexible way depending on the mission creator needs.

Faction affiliation property is stored in **FactionAffiliationComponent**. This is fairly simple component which accepts name of faction provided as string in **Faction affiliation** parameter. Even without faction config ready (which creation process will be described in later part of this document), it is possible to write name of desired faction - in this case it is **SampleFactionREDFOR -** and create rest of configuration later.

#### Changing character outfit

Main character equipment configuration is stored in **BaseLoadoutManagerComponent**. In this component, it is possible to add **slots**, which determine what kind of prefabs are attached to the character, their type (*used by inventory system)* and what part of base body should be hidden when selected piece of equipment is worn - both to save performance and avoid mesh clipping.

**Character\_Base** prefab contains six major slots which are currently handled by inventory system and in most cases there is no need to add more of them. Below is list of all currently supported slots:

* **Helmet**
* **Jacket**
* **Pants**
* **Boots**
* **Vest**
* **Backpack**

Slots inherited from **Character\_Base** contain properly defined **Name, Area & Meshes To Hide** properties so in this case, only thing to do is changing **prefab** (*through **Prefab** property* **)** which is used on selected slots.

In this tutorial all **REDFOR** faction characters are using same **Helmet**, **Jacket**, **Pants** & **Boots**, so it's safe to proceed further and assign previously created **Helmet\_SSh68\_01\_red.et**, **Jacket\_M88\_red.et**, **Pants\_M88\_red.et**, **CombatBoots\_Soviet\_01\_red.et** to their respective slots in **SampleFactionREDFOR\_Base** prefab. **Vest & backpack** will be assigned in later part of this tutorial to appropriate character classes.

[![armareforger-new-faction-character-overview.png](/wikidata/images/8/8b/armareforger-new-faction-character-overview.png)](/wiki/File:armareforger-new-faction-character-overview.png)[![armareforger-new-faction-character-overview-components.png](/wikidata/images/thumb/2/23/armareforger-new-faction-character-overview-components.png/500px-armareforger-new-faction-character-overview-components.png)](/wiki/File:armareforger-new-faction-character-overview-components.png)

#### Adding items to character storage

[![armareforger-new-faction-character-storage-items.png](/wikidata/images/2/24/armareforger-new-faction-character-storage-items.png)](/wiki/File:armareforger-new-faction-character-storage-items.png)

Once character have basic clothing assigned, it is possible to start linking some key items to REDFOR faction base prefab. Items are linked via **SCR\_InventoryStorageManagerComponent** and with this component it is possible to precisely select where given items are stored.

Goal here would be equipping **Character\_SampleFactionREDFOR\_Base**  with single **first aid kit to jacket** & few other elementary items like **compass, map & radio** to character **pants**.

By default, **Initial Inventory Items** array is empty and new elements have to be added via **plus button** . Each element in the array represents a single container which is linked through **Target Storage** property to one of the prefabs attached to character. Jacket & pants are two separate containers so two new **ItemsInitConfigurationItem** elements have to be created in **Initial Inventory Items** array.

Once that is done, it is possible to change **Target Storage** of those newly created **Initial Inventory Items** elements. Starting with first element, **Prefabs/Characters/Uniforms/Jacket\_M88\_red.et** was selected from **Target Storage** drop-down menu of first entry of **Initial Inventory Items** array. This mean that **Jacket\_M88\_red** that all prefabs listed in **Prefabs to Spawn** array will be created in this container. New elements in **Prefabs To Spawn** array can be added via **plus button** and in this case only single new entry has to be added. After that is complete, **first aid kit** ([FieldDressing\_USSR\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Medicine/FieldDressing_USSR_01.et)) prefab can be assigned to that newly created slot.

Exactly same procedure can be used to add map ([PaperMap\_01\_folded.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Equipment/Maps/PaperMap_01_folded.et)), compass ([Compass\_Adrianov.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Equipment/Compass/Compass_Adrianov.et)), flash light ([Flashlight\_Soviet\_01.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Equipment/Flashlights/Flashlight_Soviet_01.et)) & radio ([Radio\_R148.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Equipment/Radios/Radio_R148.et)) to pants. In this case **Prefabs/Characters/Uniforms/Jacket\_M88\_red.et** should be used as **Target Storage** and **Prefabs To Spawn** array should consist from 3 elements.

### Creating character variants

#### Creating inherited variants

[![armareforger-new-faction-character-equipment-in.png](/wikidata/images/thumb/d/d9/armareforger-new-faction-character-equipment-in.png/200px-armareforger-new-faction-character-equipment-in.png)](/wiki/File:armareforger-new-faction-character-equipment-in.png)

With base character ready, it is time to create actual units which can be used in various game modes or in-game editor. Using **Inherit** option in **Resource Browser**, following new prefabs were prepared:

* **Unarmed** - *Character\_SampleFactionREDFOR\_Unarmed.et*
* **Grenadier** - *Character\_SampleFactionREDFOR\_GL.et*
* **Rifleman** - *Character\_SampleFactionREDFOR\_Rifleman.et*
* **Medic** - *Character\_SampleFactionREDFOR\_Medic.et*
* **Machine Gunner** - *Character\_SampleFactionREDFOR\_MG.et*
* **Squad Leader** - *Character\_SampleFactionREDFOR\_SL.et*

At this point, all those prefabs have identical equipment and their individual equipment can be tweaked. In case of **Unarmed** character, it can be left as it is, since equipment from base class should fit that role. Rest of the classes require some additional work and in this tutorial whole process of adding weapon & additional equipment to character will be explained on **Machine Gunner** character.

First thing in that process is adding additional equipment - in this case it is harness with pouch for machine gun magazine & backpack. Machine gunner harness ([Vest\_SovietHarness\_MG.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_SovietHarness/Variants/Vest_SovietHarness_MG.et)) has to be assigned to **Vest** slot & backpack for extra magazines ([Backpack\_Veshmeshok.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Items/Equipment/Backpacks/Backpack_Veshmeshok.et)) has to be assigned to **Backpack** slot.

#### Changing weapons

[![armareforger-new-faction-character-weapons.png](/wikidata/images/3/3d/armareforger-new-faction-character-weapons.png)](/wiki/File:armareforger-new-faction-character-weapons.png)

Characters inheriting from [Character\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/Character_Base.et) are using **4 weapon slots**.
Three of them are using **CharacterWeaponSlotComponent**, where two of those are used **for primary weapons** like rifles or launchers, one is used for **secondary armament** like handguns and single **CharacterGrenadeSlotComponent** is used, as name would suggest, for throw able items like **hand grenades**.

**Machine Gunner** class prepared in this tutorial is supposed to have following loadout:

* PKM machine gun on first primary slot (**MG\_PKM.et**)
* Makarov handgun on secondary slot (**Handgun\_Makarov.et**)
* RGD-5 grenade on grenade slot (**Grenade\_RGD5.et)**

Starting with **PKM machine gun**, first thing to do is locating correct **CharacterWeaponSlotComponent**. Since goal is to have that PKM machine gun in first primary slot, values of **Weapon Slot Type & Weapon Slot Index** has to be verified. In this case, component with **Weapon Slot Type** should have value " ***primary**"* and **Weapon Slot Index** should be set to **0.**

Once that component is located, the only thing left to do is changing assigned prefab in **Weapon Template** by either clicking on **select button** or by dragging & dropping [MG\_PKM.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/MachineGuns/PKM/MG_PKM.et) prefab on **Weapon Template** property.

Next on the list is **PM handgun**, which should be assigned to **CharacterWeaponSlotComponent** with **Weapon Slot Type** set to " ***secondary**"* and **Weapon Slot Index** set to **2**.
Same as with PKM machine gun, **Weapon Template** property has to be modified so [Handgun\_PM.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/Handguns/PM/Handgun_PM.et) is assigned to that field.

Grenades follow exactly same logic and in this case its even bit easier, since there is only one **CharacterGrenadeSlotComponent** . After locating that component, Grenade\_RGD5.et prefab can be assigned to **Weapon Template** property.

#### Adding magazines to loadout

[![armareforger-new-faction-character-loadout.png](/wikidata/images/3/3a/armareforger-new-faction-character-loadout.png)](/wiki/File:armareforger-new-faction-character-loadout.png)

In principle, procedure of adding magazines to loadout is same as for items. In case of **machine gunner**, additional complication is fact that there are more prefabs to choose from due to additional loadout elements like harness and backpack. Equipment prefabs can consist of multiple child prefabs and in case of [Vest\_SovietHarness\_MG.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_SovietHarness/Variants/Vest_SovietHarness_MG.et), magazine pouch is a separate item attached to the harness itself.

In this example, one magazine is added to **Prefabs/Items/Equipment/Accessories/Pouch\_Soviet\_100rnd\_PKM/Pouch\_Soviet\_100rnd\_PKM.et -** which is part of [Vest\_SovietHarness\_MG.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_SovietHarness/Variants/Vest_SovietHarness_MG.et) prefab - and two magazines goes to backpack **Prefabs/Items/Equipment/Backpacks/Backpack\_Veshmeshok.et**. Additionally, one smoke grenade was added to backpack

## Vehicle configuration

### Creating vehicle variants

Creating vehicle variants is fairly simple as it can be done in two steps:

1. Creating inherited variant (either through **[Inherit in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")** action in **Resource Browser** in **World Editor** view port itself) of selected vehicle prefab in target addon
2. Changing **Faction affiliation** parameter in **FactionAffiliationComponent**

[![armareforger-new-faction-vehicle-faction.png](/wikidata/images/2/2b/armareforger-new-faction-vehicle-faction.png)](/wiki/File:armareforger-new-faction-vehicle-faction.png)

Even if materials of vehicles are not changed, changing faction affiliation is quite important for AI systems and some of the game modes, so those extra faction specific prefabs are sort of mandatory if its planned to equip new faction with some vehicles.

ⓘ

Please note that this page is not describing how to create vehicle retextures itself. For more information go to [Vehicle Modding](/wiki/Arma_Reforger:Vehicle_Modding#Changing_Multi_Material_base_color "Arma Reforger:Vehicle Modding") page.

## Groups

### Creating groups

It is recommended to create new groups by inheriting from [Group\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/AI/Groups/Group_Base.et) prefab which has all necessary components already in place. Procedure for it is same as for any other prefabs and it is possible to either use **[Inherit in Addon](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")** option in **Resource Browser** or go through [**World Editor** view port flow](/wiki/Arma_Reforger:Prefabs_Basics#Drag_and_drop_method "Arma Reforger:Prefabs Basics").

Similar as with characters, when preparing set of new groups for a faction, it is also good habit to create base prefab for all groups so the first step here will be creating **Group\_SampleFactionREDFOR\_Base**.et which is inheriting from **Group\_Base**.et . After that, rest of the groups can be created using previously prepared **Group\_SampleFactionREDFOR\_Base**.et prefab as base. In this tutorial, following small unit rooster is prepared:

* **Group\_SampleFactionREDFOR\_RifleSquad**.et
* **Group\_SampleFactionREDFOR\_FireGroup**.et
* **Group\_SampleFactionREDFOR\_LightFireTeam**.et

**Basic group setup**

Starting with adjusting **Group\_SampleFactionREDFOR\_Base.et**, adjustment has to be performed in properties located directly in entity configuration.

First step towards having a working group prefab will be modifying **Faction** property located in **Groups** tab & filling into that property name of target faction - **SampleFactionREDFOR** in this case.

[![armareforger-new-faction-group-composition.png](/wikidata/images/e/ee/armareforger-new-faction-group-composition.png)](/wiki/File:armareforger-new-faction-group-composition.png)

Next on the list is adjusting of **SCR\_GroupIdentityComponent** and setting there **identity** used by all **SampleFactionREDFOR** unit. This component is purely used for UI representation of group in game and it affects what kind of icon is used in both 3D interface and on 2D map. Changing **Identity** property to **OPFOR** will be result in hostile forces markers according to NATO APP-6 specification. Rest of those parameters like **Icons** or **Amplifiers** can be changed per specific group and is described in detail in later part of this tutorial.

**Changing group composition**

After basics are done, it is time to move forward and start editing actual composition of groups. Composition of groups is set directly in entity properties and is controlled through **Unit Prefab Slot** parameter in **Group Members** tab. Over there it is possible to simple add new groups slots via **plus button** and then assign character prefabs to them.

[![armareforger-new-faction-group-overview.png](/wikidata/images/4/46/armareforger-new-faction-group-overview.png)](/wiki/File:armareforger-new-faction-group-overview.png)

## Factions

### Creating faction config

#### Basic configuration

New faction configs can be only created either by **duplicating** (via ***Dup[licate in "addon name"](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics")*** *option*) one of the existing configs one or by **inheriting from existing faction** through **[Inherit in "*addon name"*](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")** option.

[![armareforger-new-faction-inherit-config.png](/wikidata/images/thumb/d/df/armareforger-new-faction-inherit-config.png/600px-armareforger-new-faction-inherit-config.png)](/wiki/File:armareforger-new-faction-inherit-config.png)

First of all, it is necessary to get configs which we want to duplicate or inherit from in the addon itself. To do so, first we navigate to Configs→ Factions folder and there, it is possible to use **[Override in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics")** [**addon**](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics") functionality located in context menu of **Resource Browser**. **OPFOR.conf** (both of them are located in Configs/Faction folder) can be used for inheritance and **USSR.conf** for duplication.

Retail version of game has packed data and its content locked - that's why such workaround has to applied. In all other cases this step can be skipped

Next step is creating either inherited config (in case **OPFOR.conf** was copied) through **Inherit** option or creating duplicated file via **Duplicate** option if **USSR.conf** was transferred to addon. In both cases, a new config will be ready to be filled with actual data relevant to new faction.

|  |  |  |
| --- | --- | --- |
| **Faction Key** | Unique identifier of the faction that was filled in i.e. **FactionAffiliationComponent** | *SampleFactionREDFOR* |
| **Faction Color** | This option determines which color will be used to represent this faction in various UIs like in-game editor or game modes | *244,5,6,255* |
| **Name** | Display name of faction | *Sample Faction - REDFOR* |
| **Description** | Description to be displayed in players UI |  |
| **Icon** | Icon representing this faction in players UI. Recommended size: 128x128px | *flag\_red\_ca.edds* |
| **Name Upper** | Display name of faction in upper case | *SAMPLE NEW FACTION - REDFOR* |
| **Identity of soldiers** |  | [FactionIdentity\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Identities/FactionIdentity_USSR.conf) |
| **Is Playable** | Controls if the faction will appear in players respawn menu | True |
| **Faction Flag** | Flag representing this faction in players UI. Recommended size: 800x400px. Same texture as for **Icon** can be used | *flag\_red\_ca.edds* |
| **Faction Flag Material** | Faction flag material - used on flag poles in f.e. Conflict |  |
| **Faction Label** | Default faction loadout - this option is currently used (& defined) in Conflict only |  |
| **Friendly To Self** | Are units in this faction friendly to itself? Used in Death Match scenarios |  |
| **Friendly Factions Ids** | List of friendly Faction Keys |  |
| **Callsign Info** | This class controls group callsigns available to this faction. Used in Conflict & in-game editor | [Callsigns\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Callsigns/Callsigns_USSR.conf) |
| **Ranks** | Military ranks available to given faction. Can be used to give custom, localised titles per faction |  |
| **Entity Catalogs** | List of [**Entity Catalogs**](/wiki/Arma_Reforger:Entity_Catalog "Arma Reforger:Entity Catalog") used by this faction. Determines what is i.e. available in Arsenal crates of selected faction |  |
| **Group Flags** | Flag icon for group |  |
| **Group Flags Image Set** | Flag imageset for groups of this faction |  |
| **Flag Names** | Markers names available in Group Flags Image Set |  |
| **Base Callsigns** | Call signs of bases. Used i.e. in Conflict for randomised bases names. |  |

[![armareforger-new-faction-config-faction.png](/wikidata/images/b/bc/armareforger-new-faction-config-faction.png)](/wiki/File:armareforger-new-faction-config-faction.png)

⚠

Respawn system requires [localised string](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation")  in **Name** field - without that you will be greeted with message stating that are no spawn points available.

#### Faction identity

In case of **Identity of soldiers** and **Callsign Info**, it is possible to use already existing configs, which can serve as a templates. Here, **SampleFactionREDFOR** is going to use [FactionIdentity\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Identities/FactionIdentity_USSR.conf) and [Callsigns\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Callsigns/Callsigns_USSR.conf). This can be achieved by **drag & dropping those config files** on respective fields:

* [FactionIdentity\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Identities/FactionIdentity_USSR.conf) (*Configs/Identities*) is assigned to **Identity of soldiers** parameter
* [Callsigns\_USSR.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Callsigns/Callsigns_USSR.conf) (*Configs/Callsigns*) is assigned to **Callsign Info** property

[![armareforger-new-faction-assign-identity.gif](/wikidata/images/a/a0/armareforger-new-faction-assign-identity.gif)](/wiki/File:armareforger-new-faction-assign-identity.gif)

## Integration with in-game editor

### Adding new faction to in-game editor

#### Adding new faction label

All labels usable by in-game editor are defined in **EEditableEntityLabel** enumerator. New entries can be added to it by using **[modded](/wiki/Arma_Reforger:Scripting:_Keywords#modded "Arma Reforger:Scripting: Keywords")** keyword. To do so, a **new script** **file** **with** **unique name** (you can use [mod tag](/wiki/Scripting_Tags "Scripting Tags") as prefix) have to be created and in this tutorial **EEditableEntityLabel.c** file was created in *Scripts\Game\Editor\Enums\Modded* folder. Afterwards, adding new labels is as simple as adding new **all caps entry** & assigning to it a **number.**

```enforce
modded enum EEditableEntityLabel
{
	FACTION_REDFOR = 1653989487,
	FACTION_BLUFOR = 1653989182,
	FACTION_GREENFOR = 1691580533
}
```

Once that is done, scripts can be recompiled and new label should be available in drop down menu of various in-game editor properties.

⚠

To avoid duplicated enum values, it is necessary to use some **large random number**. It is also possible to use current [Unix time](https://en.wikipedia.org/wiki/Unix_time) to reduce likeness of enum values clash.

📖

**Recommended read**: You can learn more about script modding on [**Script Modding**](/wiki/Arma_Reforger:Scripting_Modding "Arma Reforger:Scripting Modding") page

#### Adding new faction to Editable Entity Core

Once there is a label available, it is possible to

1. Use **Override in addon** function on [EditableEntityCore.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Core/EditableEntityCore.conf) located in *Configs/Core* folder
2. Add new entry to **Entity Labels** array. Info part is using typical **SCR\_UIInfo** layout

|  |  |  |
| --- | --- | --- |
| Name | Display name of faction | *REDFOR* |
| Description | Description to be displayed in players UI |  |
| Icon | Icon representing this faction in players UI. Recommended size: 64x64px | *flag\_red.\_ca.edds* |
| Icon Set Name | - |  |
| Order | Integer used to manually modify order of label. Labels with higher numbers are listed first |  |
| Label Type | Label from EEditableEntityLabel enum which should be evaluated | *FACTION\_REDFOR* |
| Label Group Type | Type of label - is it faction, trait, vehicle? | *FACTION* |
| Filter Enabled |  | *True* |

### Registering characters, groups & vehicles

Detailed instructions on how to add assets to in-game editor can be found on [**Asset Browser Mod Integration**](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration "Arma Reforger:Asset Browser Mod Integration") page and it is highly recommended to make yourself familiar with before proceeding anywhere further. Below are some general notes on how the process was performed for **SampleFactionREDFOR** related assets.

#### Adjusting generators configs

Before registering new assets, it is necessary to make some adjustments in config, so  **Register Placeable Entities** plugin can work as intended. This plugin is using **EditablePrefabsConfig.conf** to run some rules which populate register of prefabs. This config links to few more sub configs and in case of **Characters** & **Vehicles**, **EditablePrefabsComponent\_EditableEntity.conf** is used to determine faction label.

[![armareforger-new-faction-generator-config.png](/wikidata/images/9/95/armareforger-new-faction-generator-config.png)](/wiki/File:armareforger-new-faction-generator-config.png)

New rule can be added to **EditablePrefabsComponent\_EditableEntity.conf** in few steps described below:

1. Use **[Override in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics")** [**addon**](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics") function on **EditablePrefabsComponent\_EditableEntity.conf**  located in *Configs/Workbench/EditablePrefabs* folder
2. Add **EditablePrefabsLabel\_Faction** entry to **Entity Label Rules** array
3. Select **FACTION\_REDFOR** in **Label** drop down menu
4. Adjust **Faction To Check** parameter by typing name of faction: **SampleFactionREDFOR** (it should be same string as in **Faction affiliation** parameter in **FactionAffiliationComponent** of characters or units)

Once those steps are complete, it is important to save all changes through Ctrl+S shortcut. In some rare cases changes applied to that config might need full restart of the Workbench.

#### Adding assets to register

After **EditablePrefabsComponent\_EditableEntity.conf**  is adjusted, it is time to move forward and start t

1. Create new config files using **SCR\_PlaceableEntitiesRegistry** class for:
   1. **Characters (** *Characters\_SampleFactionREDFOR.conf* **)**
   2. **Groups (** *Groups\_SampleFaction.conf* **)**
   3. **Vehicles** (*Vehicles\_SampleFaction.conf*)
2. Adjust **Addon** parameter in all those configs so it matches target addon (*SampleMod\_NewFaction)*
3. Adjust **Source Directory** to match location of the prefabs

[![armareforger-new-faction-register-assets.png](/wikidata/images/a/a0/armareforger-new-faction-register-assets.png)](/wiki/File:armareforger-new-faction-register-assets.png)

Once those steps are completed, it is now possible to use  **Register Placeable Entities** plugin on all assets which should be available in in-game editor. To do so, select in **Resource Browser** prefabs which you want to register and then navigate to **Plugins -> In-game editor** section in top bar and click on **Register Placeable Entities**. In new pop up window select config for assets which you want to register (i.e. if you have characters selected, select *Characters\_SampleFactionREDFOR.conf*) and click on **Run** button.

This action should both generate **placeholder images** for assets previews & **adjust** **Auto** **Labels** assigned in one of the respective components like **SCR\_EditableCharacterComponent, SCR\_EditableVehicleComponent**  or **SCR\_EditableGroupComponent.**

[![armareforger-new-faction-editable-components.png](/wikidata/images/3/37/armareforger-new-faction-editable-components.png)](/wiki/File:armareforger-new-faction-editable-components.png)

##### Adjusting editable prefabs configuration

After processing prefabs with  **Register Placeable Entities**  plugin, assets will still need some manual adjustment to i.e. **Authored Labels** or **Military Symbol** *(in case of groups)* configuration. Also, in most cases it will be necessary to add proper display name to assets. In case of characters, some existing localised strings can be used

Table with localised strings

###### Characters

In case of characters, beside changing **Name** of assets in in-game content browser, few additions are necessary to **Authored Labels** array in **SCR\_EditableCharacterComponent**. Plugin responsible for automatic assignment of labels is not being able to distinguish character types automatically in current state so character role types has to be adjusted by hand. For example, Squad Leader (*Character\_SampleFactionREDFOR\_SL)* can receive **ROLE\_LEADER**, Medic can get **ROLE\_MEDIC**, Machine gunner - **ROLE\_MACHINEGUNNER** and so on.

Additionally, characters can also receive additional traits like, **TRAIT\_ARMED**, **TRAIT\_UNARMED** or **TRAIT\_MEDICAL**, which improves filtering in in-game content browser.

###### Vehicles

When it comes to vehicles, it usually boils down to adding special vehicle traits like **TRAIT\_ARMED**, **TRAIT\_UNARMED, TRAIT\_REFUELING**, etc. Some custom labels can be also established based on the needs but by no means its not necessary.

###### Groups

Similar to characters, also groups can use one of the already existing strings for **Name** property.

Table with localised strings
Show text

|  |  |
| --- | --- |
| **String** | **Display Name** |
| ``` AR-Group_AntiTankTeam ``` | Anti-Tank Team |
| ``` AR-Group_Default ``` | Infantry Group |
| ``` AR-Group_FireTeam ``` | Fire Team |
| ``` AR-Group_FireTeamLight ``` | Light Fire Team |
| ``` AR-Group_GrenadierTeam ``` | Grenadier Team |
| ``` AR-Group_Guerrilla_FireTeam ``` | Guerrilla Fire Team |
| ``` AR-Group_Guerrilla_HQ ``` | Guerrilla HQ |
| ``` AR-Group_Guerrilla_MGTeam ``` | Guerrilla Machine Gun Team |
| ``` AR-Group_Guerrilla_ReconTeam ``` | Guerrilla Recon Team |
| ``` AR-Group_Guerrilla_SharpshooterTeam ``` | Guerrilla Sharpshooter Team |
| ``` AR-Group_Guerrilla_Squad ``` | Guerrilla Squad |
| ``` AR-Group_LightAntiTankTeam ``` | Light Anti-Tank Team |
| ``` AR-Group_ManeuverGroup ``` | Maneuver Team |
| ``` AR-Group_MedicalSection ``` | Medical Support Team |
| ``` AR-Group_MGTeam ``` | Machine Gun Team |
| ``` AR-Group_MortarSquad ``` | Mortar Team |
| ``` AR-Group_PlatoonHQ ``` | Platoon HQ |
| ``` AR-Group_SniperTeam ``` | Sniper Team |
| ``` AR-Group_Squad ``` | Rifle Squad |
| ``` AR-Group_SuppressiveFireTeam ``` | Base of Fire Team |

[↑ Back to spoiler's top](#bikisp6a63b9ba6aa40)

Compared to vehicles & characters though, there are no group related labels which have to be filled manually.
However, groups also have special **Military Symbol** class, which holds information about icon which is visible in in-game editor above groups.
Using checkboxes & drop menus available there, it is possible to assign one of [NATO APP-6](https://en.wikipedia.org/wiki/NATO_Joint_Military_Symbology) icon to selected group.

### Adding new faction to Faction Manager

[![](/wikidata/images/thumb/4/4c/armareforger-new-faction-is-playable-change.jpg/300px-armareforger-new-faction-is-playable-change.jpg)](/wiki/File:armareforger-new-faction-is-playable-change.jpg)

Changing **Is Playable** parameter

One of the remaining steps towards integrating new faction into in-game editor is adding it to list of available factions. This can be done in two easy steps:

1. Use **[Override in **addon**](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics") function on [FactionManager\_Editor.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_Editor.et) located in *Prefabs/MP/Managers/Factions* folder**
2. Add new element to **Factions** array by drag and dropping faction config on that property
3. Uncheck **Is Playable** parameter inside of **SCR\_Faction** entry inside **Factions** array - by default, when Game Master session starts, there should be no factions available and it should be up to Game Master to decide which are

After that, faction should be selectable in respawn menu during game master sessions & assets should be using proper colorization in content browser.

[![armareforger-new-faction-adding-faction-to-manager.gif](/wikidata/images/5/59/armareforger-new-faction-adding-faction-to-manager.gif)](/wiki/File:armareforger-new-faction-adding-faction-to-manager.gif)

### Creating new spawn points

Spawn points are a special type of prefabs which are used to determine where player of given faction can spawn.

#### Preparing new spawn point prefab

First step will be creating new spawn point prefab - it is quite easy and involves following steps:

* Using **Resource Browser**, find [SpawnPoint\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_Base.et) prefab
* Click on it with **Right Mouse Button** and select option **[Inherit in Addon](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")**
* Type name of new prefab (i.e. **SpawnPoint\_REDFOR**)
* Open prefab created in previous step (*SpawnPoint\_REDFOR in this case)* either by by using **[prefab edit mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** or placing this prefab in World Editor
* In that prefab type name of your faction (*SampleModFactionREDFOR*) in **Faction** parameter
* Save your changes (using e.g `Ctrl` + `S`)

[![Image armareforger-new-faction-spawnpoint-creation.gif](/wikidata/images/1/13/Image_armareforger-new-faction-spawnpoint-creation.gif)](/wiki/File:Image_armareforger-new-faction-spawnpoint-creation.gif)

#### Creating editable variant of spawn point

After creating base variants, it will be still necessary to make editable variant, which can be used directly in Game Master. Unlike before, editable variant of spawn point have to be created manually and below you can find instructions how to do it:

* In **Resource Browser**, find [E\_SpawnPoint.et](enfusion://ResourceManager/~ArmaReforger:PrefabsEditable/SpawnPoints/E_SpawnPoint.et)
* Click on it with **Right Mouse Button** and select option **[Inherit in Addon](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")**
* Type name of new prefab (i.e. **E\_SpawnPoint\_REDFOR**)
* Open prefab created in previous step (*E\_SpawnPoint\_REDFOR in this case)* either by by using **[prefab edit mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** or placing this prefab in World Editor

* [![SCR_SpawnPoint](/wikidata/images/thumb/b/b9/armareforger-new-faction-setting-editable-spawnpoint.png/500px-armareforger-new-faction-setting-editable-spawnpoint.png)](/wiki/File:armareforger-new-faction-setting-editable-spawnpoint.png "SCR_SpawnPoint")

  **SCR\_SpawnPoint**
* [![SCR_EditableSpawnPointComponent](/wikidata/images/thumb/0/03/armareforger-new-faction-spawnpoint-editable-component.png/472px-armareforger-new-faction-spawnpoint-editable-component.png)](/wiki/File:armareforger-new-faction-spawnpoint-editable-component.png "SCR_EditableSpawnPointComponent")

  **SCR\_EditableSpawnPointComponent**
* [![SCR_FactionAffiliationComponent](/wikidata/images/thumb/5/57/armareforger-new-faction-spawnpoint-faction-affilation.png/500px-armareforger-new-faction-spawnpoint-faction-affilation.png)](/wiki/File:armareforger-new-faction-spawnpoint-faction-affilation.png "SCR_FactionAffiliationComponent")

  **SCR\_FactionAffiliationComponent**

* In properties of **SCR\_SpawnPoint** entity locate **Faction** parameter in unsorted category and write there name of the faction (*SampleFactionREDFOR*)
* In **SCR\_EditableSpawnPointComponent** component do following changes:
  + Write name of your faction in **Faction** parameter
  + Assign **texture** to **Image** property. Currently all factions are using same texture, even though there are separate file names for them, so it's possible to use i.e. [E\_SpawnPoint\_USSR.edds](enfusion://ResourceManager/~ArmaReforger:UI/Textures/EditorPreviews/Auto/Systems/SpawnPoints/E_SpawnPoint_USSR.edds)
  + In Authored Labels array add two new elements: **TRAIT\_SPAWNPOINT** (*it is missing in base class)* and **label of your faction** (in this case it is **FACTION\_REDFOR**)
* In **SCR\_FactionAffiliationComponent** adjusted **Faction** property and write there name of your faction (*SampleFactionREDFOR)*

If everything went fine, spawn point should be ready to be plugged in into Game Master

#### Registering spawn point

Once editable spawn point prefab is ready, it is still necessary to register it in Game Master. This process is well described on [**Asset Browser Mod Integration**](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration "Arma Reforger:Asset Browser Mod Integration") and below is shortened version of it:

* [Duplicate to](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics") addon [Systems.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/PlaceableEntities/Systems/Systems.conf) and call it i.e. *Systems\_SampleFaction.conf*
* Open new config in **Resource Manager**
* Clear **Prefabs** array by clicking on little **arrow (1)** on the right side
* Add editable Spawn Point (*E\_SpawnPoint\_REDFOR.conf*) to **Prefabs** array
* *Optionally:* Modify Addon field so it matches addon name - in this case it would be **SampleMod\_NewFaction**
* Save the config

[![](/wikidata/images/5/52/armareforger-new-faction-registering-system.png)](/wiki/File:armareforger-new-faction-registering-system.png)

Registering spawn point in Systems.conf

When config is ready, it is possible to register it in **EditorModeEdit.et prefab** as described on [Asset Browser Mod Integration](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration#Adding_new_register_file_to_Editor "Arma Reforger:Asset Browser Mod Integration") page.

### Generating editor previews

Units, groups & vehicle previews used by in-game editor can be easily generated by using steps described on [Game Master: Image Generation Tutorial](/wiki/Arma_Reforger:Game_Master:_Image_Generation_Tutorial "Arma Reforger:Game Master: Image Generation Tutorial") page & by loading [Eden\_AssetImages.ent](enfusion://WorldEditor/~worlds/Editor/Slots/AssetImages/Eden_AssetImages.ent) world to generate those pictures. If previous steps were done correctly & assets were properly registered, then new images should be immediately available after generation process is completed.

⚠

Making screenshots disables debug console. It is necessary to restart whole Workbench to restore that functionality!

## Integration with Conflict game mode

### Adding Conflict specific prefabs & configs

#### Creating Conflict specific configs

##### Faction config

Conflict is using extended version of regular faction config, which allows to configure **ranks**, available **structures**, default **AI groups**, **vehicles** or **radio equipment**. Since this specialised version of config is extension of default **SCR\_Faction** class, it is recommended to inherit from previously created **SCR\_Faction** config - in this case it is **SampleFactionREDFOR.conf.**

⚠

Currently configs have to be adjusted in text editor of your choice since Workbench doesn't offer ability to change config class!

Easiest way to create such config is using **Inherit** option on **SampleFactionREDFOR.conf**. Once new config is created - named i.e. **SampleFactionREDFOR\_Campaign.conf**. After this is complete, this file can be edited in text editor of your choice and **SCR\_Faction** on first line of file

```enforce
SCR_Faction : "{C93D9F19E9F3CD97}Configs/Factions/SampleFactionREDFOR.conf" {
	[...]
```

can be replaced with **SCR\_CampaignFaction** like on example below.

```enforce
SCR_CampaignFaction : "{C93D9F19E9F3CD97}Configs/Factions/SampleFactionREDFOR.conf" {
	[...]
```

Once this adjustment is saved, it might be necessary to restart whole **Workbench** in order to have that change correctly loaded.

ⓘ

**Campaign** is one of the code names of **Conflict** game mode and it is still referenced this way in few places.

Afterward, class indication **SampleFactionREDFOR\_Campaign.conf** should correctly show **SCR\_CampaignFaction** class (see green area on picture below).

[![armareforger-new-faction-conflict-config.gif](/wikidata/images/9/9a/armareforger-new-faction-conflict-config.gif)](/wiki/File:armareforger-new-faction-conflict-config.gif)

By default, resulting config will be quite empty and most of the **Conflict** specific parameters have to be filled by hand. Using table below, **SampleFactionREDFOR\_Campaign.conf** was populated with all necessary data.

| Group | Parameters | Description |
| --- | --- | --- |
| Ranks |  | Ranks that players can gain through the game |
| Radio | Faction Radio Encryption Key Faction Radio Frequency | Controls default frequency and |
| AI | AI Group Prefab AI Vehicle Prefab  Defenders Group Prefab  Default Transport Prefab  Spawn AIs | Configuration of AI groups like base defenders or attacking squads |
| AI (Independent) | Remnant Groups | AI groups spawned by **INDFOR** side on territory they are controlling. |
| Structures | Radio Prefab Base Building XXX | Buildings & equipment used by factions |
| Compositions | Campaign Slot Compositions | Compositions which can be build by this faction |

ⓘ

Since setting all those parameters might be time consuming, it is also possible to **copy paste in text editor** some of the values from other **existing Campaign configs**.

**Example code below can be used for inspiration**

Show text

```
SCR_CampaignFaction : "{C93D9F19E9F3CD97}Configs/Factions/SampleFactionREDFOR.conf" {
 m_aRanks {
 SCR_CharacterRank "{528E9D9FAAB617C6}" {
 m_sRankName "#AR-Rank_WestPrivate"
 m_sRankNameUpper "#AR-Rank_WestPrivate-UC"
 m_sRankNameShort "#AR-Rank_WestPrivateShort"
 m_sInsignia "{2AA43D26EB40FF32}UI/Textures/Nametags/nametagPointerRankPrivate_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FAB192253}" {
 m_iRank CORPORAL
 m_sRankName "#AR-Rank_WestCorporal"
 m_sRankNameUpper "#AR-Rank_WestCorporal-UC"
 m_sRankNameShort "#AR-Rank_WestCorporalShort"
 m_sInsignia "{9EB262BC94EA46F3}UI/Textures/Nametags/nametagPointerRankCorporal_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FAB87F519}" {
 m_iRank SERGEANT
 m_sRankName "#AR-Rank_WestSergeant"
 m_sRankNameUpper "#AR-Rank_WestSergeant-UC"
 m_sRankNameShort "#AR-Rank_WestSergeantShort"
 m_sInsignia "{F879E808E02C9442}UI/Textures/Nametags/nametagPointerRankSergeant_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FA4EA5C39}" {
 m_iRank LIEUTENANT
 m_sRankName "#AR-Rank_WestLieutenant"
 m_sRankNameUpper "#AR-Rank_WestLieutenant-UC"
 m_sRankNameShort "#AR-Rank_WestLieutenantShort"
 m_sInsignia "{F879E808E02C9442}UI/Textures/Nametags/nametagPointerRankSergeant_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FA563C005}" {
 m_iRank CAPTAIN
 m_sRankName "#AR-Rank_WestCaptain"
 m_sRankNameUpper "#AR-Rank_WestCaptain-UC"
 m_sRankNameShort "#AR-Rank_WestCaptainShort"
 m_sInsignia "{F879E808E02C9442}UI/Textures/Nametags/nametagPointerRankSergeant_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FA64A043C}" {
 m_iRank MAJOR
 m_sRankName "#AR-Rank_WestMajor"
 m_sRankNameUpper "#AR-Rank_WestMajor-UC"
 m_sRankNameShort "#AR-Rank_WestMajorShort"
 m_sInsignia "{F879E808E02C9442}UI/Textures/Nametags/nametagPointerRankSergeant_ca.edds"
 }
 SCR_CharacterRank "{528E9D9FA77F696E}" {
 m_iRank RENEGADE
 m_sRankName "#AR-Rank_Renegade"
 m_sRankNameUpper "#AR-Rank_Renegade-UC"
 m_sInsignia "{F879E808E02C9442}UI/Textures/Nametags/nametagPointerRankSergeant_ca.edds"
 }
 }
 m_sFactionRadioEncryptionKey "coldBorscht"
 m_iFactionRadioFrequency 42000
 m_AIGroupPrefab "{912FC8862E1726D3}Prefabs/Groups/Campaign/Group_SampleFactionREDFOR_Campaign.et"
 m_AIVehiclePrefab "{16C1F16C9B053801}Prefabs/Vehicles/Wheeled/Ural4320/Ural4320_transport.et"
 m_DefendersGroupPrefab "{75D9E5BA9FB3B532}Prefabs/Groups/Campaign/Group_SampleFactionREDFOR_Campaign_Defenders.et"
 m_DefaultTransportPrefab "{AF946468919822C9}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_SampleFactionREDFOR.et"
 m_RadioPrefab "{54C68E438DD34265}Prefabs/Items/Equipment/Radios/Radio_R107M.et"
 m_BaseBuildingHQ "{85A010184F03C74F}Prefabs/Compositions/Slotted/SlotFlatSmall/Headquarters_S_USSR_01_Campaign.et"
 m_BaseBuildingHQNoSlot "{D9237E85B53087C3}Prefabs/MP/Campaign/Bases/Campaign_HQ_USSR.et"
 m_BaseBuildingStash0 "{F141E5C48AD5EC60}Prefabs/Compositions/Slotted/SlotFlatSmall/SupplyStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingStash0NoSlot "{541E61B144E350AD}Prefabs/MP/Campaign/Bases/Campaign_SupplyDepot_US_Empty.et"
 m_BaseBuildingStash1 "{F141E5C48AD5EC60}Prefabs/Compositions/Slotted/SlotFlatSmall/SupplyStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingStash1NoSlot "{F9EFB460031EA15E}Prefabs/MP/Campaign/Bases/Campaign_SupplyDepot_US_01.et"
 m_BaseBuildingStash2 "{F141E5C48AD5EC60}Prefabs/Compositions/Slotted/SlotFlatSmall/SupplyStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingStash2NoSlot "{95BB8080BFDA2E70}Prefabs/MP/Campaign/Bases/Campaign_SupplyDepot_US_02.et"
 m_BaseBuildingStash3 "{F141E5C48AD5EC60}Prefabs/Compositions/Slotted/SlotFlatSmall/SupplyStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingStash3NoSlot "{20F10D990F83A691}Prefabs/MP/Campaign/Bases/Campaign_SupplyDepot_US_Full.et"
 m_BaseBuildingFuel0 "{4D1E4357BC178D19}Prefabs/Compositions/Slotted/SlotFlatSmall/FuelStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingFuel0NoSlot "{5DC8135B3259C4CA}Prefabs/MP/Campaign/Bases/Campaign_FuelDepot_US.et"
 m_BaseBuildingFuel1 "{BFE69F51F235930E}Prefabs/Compositions/Slotted/SlotFlatSmall/FuelStorage_S_US_01_Campaign.et"
 m_BaseBuildingFuel2 "{BFE69F51F235930E}Prefabs/Compositions/Slotted/SlotFlatSmall/FuelStorage_S_US_01_Campaign.et"
 m_BaseBuildingFuel3 "{BFE69F51F235930E}Prefabs/Compositions/Slotted/SlotFlatSmall/FuelStorage_S_US_01_Campaign.et"
 m_BaseBuildingArmory "{A9A977F2EE4056E1}Prefabs/Compositions/Slotted/SlotFlatSmall/AmmoStorage_S_USSR_01_Campaign.et"
 m_BaseBuildingArmoryNoSlot "{6C05433C85E0FD0C}Prefabs/MP/Campaign/Bases/Campaign_ArmoryDepot_US.et"
 m_BaseBuildingRepair "{FE5831BC3D5E33CD}Prefabs/Compositions/Slotted/SlotFlatSmall/VehicleMaintenance_S_USSR_01_Campaign.et"
 m_BaseBuildingRepairNoSlot "{3D4F99916E1B0AD7}Prefabs/MP/Campaign/Bases/Campaign_VehicleMaintenanceGarage.et"
 m_BaseBuildingHospital "{70BEEEC472F2CD7D}Prefabs/Compositions/Slotted/SlotFlatMedium/FieldHospital_M_USSR_01.et"
 m_BaseBuildingBarracks "{F68F3B7247F6E80F}Prefabs/Compositions/Slotted/SlotFlatLarge/LivingArea_L_USSR_01.et"
 m_BaseBuildingBarracksNoSlot "{1540BCCF536C2895}Prefabs/MP/Campaign/Bases/Campaign_LivingArea_US.et"
 m_BaseBuildingRadioAntenna "{B453B160E6D37FD4}Prefabs/Compositions/Slotted/SlotFlatSmall/Antenna_S_USSR_01_Campaign.et"
 m_BaseBuildingRadioAntennaNoSlot "{1AF9E0A02075CBB0}Prefabs/MP/Campaign/Bases/Campaign_Antenna_US.et"
 m_aCampaignSlotCompositions {
 SCR_CampaignSlotComposition "{5496D16E528E4C29}" {
 m_Resource "{4885DEA01D687DB2}PrefabsEditable/Auto/Compositions/Slotted/SlotFlatSmall/E_Bunker_S_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_SandbagBunker-UC"
 m_sMarkerImage "Building_Fortification"
 m_sIconImage "Slot_Fortification"
 m_iPrice 25
 m_SlotType FlatSmall
 m_bIsServiceComposition 0
 }
 SCR_CampaignSlotComposition "{5496D16E532917AC}" {
 m_Resource "{1200F6499BA275FA}PrefabsEditable/Auto/Compositions/Slotted/SlotFlatSmall/E_SandbagPosition_S_USSR_03.et"
 m_sDisplayName "#AR-Campaign_Building_SandbagPosition-UC"
 m_sMarkerImage "Building_Fortification"
 m_sIconImage "Slot_Fortification"
 m_iPrice 25
 m_SlotType FlatSmall
 m_bIsServiceComposition 0
 }
 SCR_CampaignSlotComposition "{5496D16E6079CB08}" {
 m_Resource "{047B9C8AAB50CE0E}PrefabsEditable/Auto/Compositions/Slotted/SlotFlatSmall/E_MachineGunNest_S_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_MachineGunNest-UC"
 m_sMarkerImage "Building_Fortification"
 m_sIconImage "Slot_Fortification"
 m_iPrice 25
 m_SlotType FlatSmall
 m_bIsServiceComposition 0
 }
 SCR_CampaignSlotComposition "{5496FB6B9D439DCD}" {
 m_Resource "{5AF4E84CA11058F3}PrefabsEditable/Auto/Compositions/Slotted/SlotRoadSmall/E_Checkpoint_S_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_SmallRoadCheckpoint-UC"
 m_sMarkerImage "Building_RoadBlock"
 m_sIconImage "Slot_Roadblock"
 m_iPrice 75
 m_SlotType CheckpointSmall
 m_bCanBeRotated 0
 m_bIsServiceComposition 0
 }
 SCR_CampaignSlotComposition "{5496FB6B958EA37C}" {
 m_Resource "{1DF9835399B603EA}PrefabsEditable/Auto/Compositions/Slotted/SlotRoadMedium/E_Checkpoint_M_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_MediumRoadCheckpoint-UC"
 m_sMarkerImage "Building_RoadBlock"
 m_sIconImage "Slot_Roadblock"
 m_iPrice 125
 m_SlotType CheckpointMedium
 m_bCanBeRotated 0
 m_bIsServiceComposition 0
 }
 SCR_CampaignSlotComposition "{5496FB6B95ED9102}" {
 m_Resource "{612AB9E420809988}PrefabsEditable/Auto/Compositions/Slotted/SlotRoadLarge/E_Checkpoint_L_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_LargeRoadCheckpoint-UC"
 m_sMarkerImage "Building_RoadBlock"
 m_sIconImage "Slot_Roadblock"
 m_iPrice 150
 m_SlotType CheckpointLarge
 m_bCanBeRotated 0
 }
 SCR_CampaignSlotComposition "{54A088BB7FA2DFDA}" {
 m_Resource "{4D1E4357BC178D19}Prefabs/Compositions/Slotted/SlotFlatSmall/FuelStorage_S_USSR_01_Campaign.et"
 m_vBuildingControllerOffset -1.5 0 0
 m_sDisplayName "#AR-Campaign_Building_FuelDepot-UC"
 m_sMarkerImage "Building_FuelDepot"
 m_sIconImage "Slot_Fuel"
 m_iPrice 750
 m_SlotType Services
 m_bCanBeRotated 0
 m_bIsServiceComposition 1
 }
 SCR_CampaignSlotComposition "{54A0A94A1B28CA0A}" {
 m_Resource "{A9A977F2EE4056E1}Prefabs/Compositions/Slotted/SlotFlatSmall/AmmoStorage_S_USSR_01_Campaign.et"
 m_vBuildingControllerOffset -5 0 0
 m_sDisplayName "#AR-Campaign_Building_Armory-UC"
 m_sMarkerImage "Building_Armory"
 m_sIconImage "Slot_Armory"
 m_iPrice 750
 m_SlotType Services
 m_bCanBeRotated 0
 m_bIsServiceComposition 1
 }
 SCR_CampaignSlotComposition "{54A0A94A1AE6C81B}" {
 m_Resource "{FE5831BC3D5E33CD}Prefabs/Compositions/Slotted/SlotFlatSmall/VehicleMaintenance_S_USSR_01_Campaign.et"
 m_vBuildingControllerOffset -4 0 0
 m_sDisplayName "#AR-Campaign_Building_RepairDepot-UC"
 m_sMarkerImage "Building_VehicleWorkshop"
 m_sIconImage "Slot_Repair"
 m_iPrice 750
 m_SlotType Services
 m_bCanBeRotated 0
 m_bIsServiceComposition 1
 }
 SCR_CampaignSlotComposition "{54A0E082FF414F1D}" {
 m_Resource "{70BEEEC472F2CD7D}Prefabs/Compositions/Slotted/SlotFlatMedium/FieldHospital_M_USSR_01.et"
 m_sDisplayName "#AR-Campaign_Building_FieldHospital-UC"
 m_sMarkerImage "Building_Hospital"
 m_sIconImage "Slot_Medical"
 m_iPrice 750
 m_SlotType Services
 m_bCanBeRotated 0
 m_bIsServiceComposition 1
 }
 SCR_CampaignSlotComposition "{54A0E082D40B2C4D}" {
 m_Resource "{F68F3B7247F6E80F}Prefabs/Compositions/Slotted/SlotFlatLarge/LivingArea_L_USSR_01.et"
 m_vBuildingControllerOffset 0 0 0
 m_sDisplayName "#AR-Campaign_Building_Barracks-UC"
 m_iPrice 750
 m_SlotType Services
 m_bCanBeRotated 0
 m_bIsServiceComposition 1
 }
 }
}
```

[↑ Back to spoiler's top](#bikisp6a63b9ba740e0)

#### Vehicle list

Pool of available for request vehicles is defined in config file using **SCR\_CampaignVehicleAssetList** class. By default, Conflict is using [VehicleAssetList.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Campaign/VehicleAssetList.conf) located in *Configs/Campaign* folder and this file is a good candidate for a custom variant.
After clicking on [VehicleAssetList.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Campaign/VehicleAssetList.conf) with Right Mouse Button , **Duplicate** option can be selected from context menu and new config file - like **VehicleAssetList\_SampleNewFaction.conf** - can be prepared.

It is also possible to create new **SCR\_CampaignVehicleAssetList** config from scratch through *Create→ Config File* too.

Having a new config file ready, it is possible to start adjusting thing inside of it.
Every element inside of **Vehicle Asset List** array represents an vehicle which can be requested at **Vehicle Depot** by one of the sides participating in Conflict game mode.
In case when [VehicleAssetList.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Campaign/VehicleAssetList.conf) was duplicated, only things which need to be changed are **Prefab & Prefab Opposite** parameter.

[![armareforger-new-faction-conflict-vehicles.png](/wikidata/images/b/bf/armareforger-new-faction-conflict-vehicles.png)](/wiki/Special:FilePath/armareforger-new-faction-conflict-vehicles.png "Special:FilePath/armareforger-new-faction-conflict-vehicles.png")

**Prefab** parameter controls which asset will be spawned when this vehicle is requested in-game.
In this tutorial **REDFOR** faction vehicles are going to be replaced with variants used by **SampleFactionREDFOR** faction and for instance [UAZ469.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et) prefab is being replaced with **UAZ469\_SampleFactionREDFOR.et.**

Once replacement is completed on **REDFOR** vehicles, it also necessary to go through **BLUFOR** vehicles and update there assets assigned in **Prefab Opposite** parameter with current **SampleFactionREDFOR** counterparts.
This parameter is mainly used when opposing faction captures vehicle depot with some vehicles being still available there.
In this scenario, enemy vehicles will be still available to request but they will also consume total capacity of such depot.
For instance if **BLUFOR** captures depot with 8 UAZs, then in next reinforcement wave only two additional M151A2 cars will be added to the pool.

Conflict game mode supports only three sides in game and those are available in **Faction** drop down menu:

* **BLUFOR** - represent first playable team, which fights for control over the island with **REDFOR** & **INDFOR** forces
* **REDFOR** - represent second playable team, which fights for control over the island with **BLUFOR** & **INDFOR** forces
* **INDFOR** - represent non playable independent forces which are fighting with both **BLUFOR** & **REDFOR** forces. This faction holds at the beginning majority of the bases

Actual factions linked to **BLUFOR**, **REDFOR** & **INDFOR** are defined in **CampaignMPGameMode** prefab which is placed and can be modified in Campaign world itself.
More information about this feature can be found in chapter describing **Conflict** world.

Since **SampleFactionREDFOR** is using color retextures of existing vehicles, it is possible to leave **Display Name & Display Name UC** parameters intact.

In case config is being set of scratch, following table can be used as reference for making list of available vehicles.

| Property | Description |
| --- | --- |
| **Prefab** | Prefab of a vehicle which can be requested |
| **Faction** | One of the |
| **Display Name** | Name which will be shown in UI (i.e. request menu) |
| **Display Name UC** | Same as above but in upper case |
| **Rank ID** | Lowest rank which can request this vehicle |
| **Max** | Max amount of assets of that type that a base can hold |
| **Starting Amount** | Amount of vehicles available at mission start |
| **Reinforcements Amount** | Amount of vehicles which are added in reinforcement wave |
| **Prefab Opposite** | Opposing faction vehicle counterpart. This parameter is used mainly to correctly recognise maximal capacity of a vehicle depot |

### Adding Conflict specific prefabs

Conflict game mode requires few specialised prefabs in order to run properly. For starter, **Conflict** requires unique character prefabs - both for AI & players.
All those **Conflict** specific characters are located in *Prefabs/Characters/Campaign* folder and similar structure is used also for **REDFOR** faction.
Player characters used in Conflict game are typical characters, often derived from existing troops, with addition of few extra components:

* **SCR\_ResourceComponent -** used for handling **Conflict** related **interactions with supplies**
* **SCR\_CampaignBuildingComponent -** used for handling of in-game **construction of structures**
* **MapDescriptorComponent -** used for correctly **showing unit on map**

#### Player Character

Beginning with **Conflict player character**, a new **Campaign\_SampleFactionREDFOR\_Player.et** prefab can be created by inheriting from one of already existing **SampleFactionREDFOR** character - i.e. **Character\_SampleFactionREDFOR\_Rifleman.et** **-** using for example **[Inherit](/wiki/Arma_Reforger:Prefabs_Basics#Using_Inherit_Prefab_option "Arma Reforger:Prefabs Basics")** option.

In case of **SCR\_ResourceComponent**, it is necessary to add new component via **"+ Add Component"** . After that, it will be still necessary to add one element to **Consumers** array of that component.
This can be achieved by drag & dropping [Consumer\_Self.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Resources/Supplies/Consumers/Generic/Consumer_Self.conf) from **Resource Browser** onto **Consumers array in SCR\_ResourceComponent** in **Object Properties** window. After that, a **SCR\_ResourceComponent** component should contain all necessary data for Conflict game mode**.**

[![armareforger-new-faction-resource-component.gif](/wikidata/images/e/e6/armareforger-new-faction-resource-component.gif)](/wiki/File:armareforger-new-faction-resource-component.gif)

Similar to **SCR\_ResourceComponent**, the other two components - **SCR\_CampaignBuildingComponent** & **MapDescriptorComponent** have to be added manually through "**+ Add Component**" button.
Once those components are added, the following tweaks can be applied:

* **SCR\_CampaignBuildingComponent**
  + No changes are necessary
* **MapDescriptorComponent**
  + **Descriptor Visible** should be **unchecked**
  + **Descriptor Type Enum** should be changed to **Unit**
  + **Descriptor Unit Enum** should be changed to **Infantry**

#### AI Character

Next on the list is preparing friendly AI prefab - **Campaign\_SampleFactionREDFOR\_AI.et** - which can be i.e. spotted in friendly bases. Similar to player character, **Character\_SampleFactionREDFOR\_Rifleman.et**  can be used as base for such prefab.

Once new AI character is created, **CampaignInteractions.ct** can be applied to this prefab in same way as for player prefab. This is the only required prefab for AI character and after it is added, **Campaign\_SampleFactionREDFOR\_AI.et** should be ready for action!

### Adding Conflict specific groups

Main playable factions require only few extra groups which can be created based on existing group prefabs.
The main difference for non playable factions (*INDFOR*) is the presence of **Remnants Groups**, which are using **unique group compositions** and their group members are also **not being spawned immediately** when given prefab is created.

ⓘ

**Remnants Groups** are only relevant for given faction if its used as **INDFOR** forces.
If it's not planned to use given faction as independent side then this step can be skipped.

This delayed spawning behavior is achieved by setting **Spawn Immediately** property to unchecked state.

Remnants group can be prepared in few simple steps.
The whole process begins with creating faction specific Remnant group base (***Group\_SampleFactionREDFOR\_Remnants\_Base.et***) prefab using [Group\_Remnants\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Groups/Campaign/Group_Remnants_Base.et) prefab as parent.

In this new base prefab it is necessary to adjust **Faction** parameter and change it to name faction code name **- SampleFactionREDFOR**.

Next, it is possible to use that new prefab to create some extra groups like:

* Patrol Group (**Group\_SampleFactionGREENFOR\_Remnants\_Patrol.et**)
* Fire Team (**Group\_SampleFactionGREENFOR\_Remnants\_FireTeamet**)
* AT Team (**Group\_SampleFactionGREENFOR\_Remnants\_AT.et**)
* MG Squad (**Group\_SampleFactionGREENFOR\_Remnants\_MG.et**)
* Sniper Team (**Group\_SampleFactionGREENFOR\_Remnants\_Sniper.et**)

Once those prefabs are created, only remaining thing to do is adjusting composition of those groups according to their purpose. This can be done by changing **Unit Prefab Slots array**.
It is worth mentioning that **remnants groups can use regular character prefabs**. For example, AT group can consist of **Character\_SampleFactionREDFOR\_AT.et** prefab.

After all groups are created & adjusted, it is possible to assign them to remnants forces rooster. This can be done by adding new elements to **Remnant Groups** array in **SampleFactionREDFOR\_Campaign.conf**.
From this list, game will try to pick random groups depending on the context of area where they are supposed to be spawn (*SCR\_CampaignREmnantsSpawnPoint*).
This is done by setting **Probability & Type in SCR\_CampaignRemnantsGroup** entry in **Remnant Groups** array.

⚠

Currently BLUFOR & OPFOR AI forces are only assigned to base defence and **cannot capture bases on their own**. Configuration of attacking squads & their vehicles is therefor **highly optional**.

Conflict attackers & defenders AI groups consist of previously created Conflict variant of AI characters.
Configuration of those groups is basically the same as regular ones. **Defenders** groups are spawned **in bases controlled by this faction** and they are tasked with defending this territory.
**Attackers** on the other hand should actively try to **capture enemy bases** using vehicle defined in faction config.

[![armareforger-new-faction-conflict-attackers.png](/wikidata/images/6/62/armareforger-new-faction-conflict-attackers.png)](/wiki/File:armareforger-new-faction-conflict-attackers.png)

Starting with creation of base group for **both attackers & defender**, a new **Group\_SampleFactionREDFOR\_Campaign\_Base.et** prefab, inheriting from regular **Group\_SampleFactionREDFOR\_Base.et**  group, was created.

In this base prefab there is only one thing which need to be adjusted**:**

* **Unit Prefab Slots** array should be tweaked so only **3** **Campaign\_SampleFactionREDFOR\_AI.et** prefabs are present.

Next, using **Group\_SampleFactionREDFOR\_Campaign\_Base.et** as paraent, **Group\_SampleFactionREDFOR\_Campaign\_Defenders.et** was prepared. This prefab doesn't need any additional refinements.

Similar to defenders group, attacker group - **Group\_SampleFactionREDFOR\_Campaign.et** - was prepared using **Group\_SampleFactionREDFOR\_Campaign\_Base.et** as parent. This prefab needs two tweaks:

* **Unit Prefab Slots** array should consist from **7** **Campaign\_SampleFactionREDFOR\_AI.et** characters
* **Spawn Immediately** parameter should be **unchecked**. This is done to avoid spike in load when spawning those groups across the island

Finally, when those groups are ready, they can be assigned to appropriate fields in **SampleFactionREDFOR\_Campaign.conf**

* **AI Group Prefab → Attacker group - Group\_SampleFactionREDFOR\_Campaign.et**
* **Defenders Group Prefab → Defenders group - Group\_SampleFactionREDFOR\_Campaign\_Defenders.et**

### Creating new Conflict world

#### Using Conflict template

#### Adjusting scenario

After successfully copying template to the addon, it's time to change playable faction in this scenario.

ⓘ

All changes performed below are only applied to entities existing in that scenario and **Apply To Prefab** button **shouldn't be used.**

[![armareforger-new-faction-conflict-scenario.png](/wikidata/images/6/6a/armareforger-new-faction-conflict-scenario.png)](/wiki/File:armareforger-new-faction-conflict-scenario.png)

In **logic layer**, there are three entities which have to be adjusted:

1. **CampaignFactionManager1 →**  This entity holds information about factions which can be used in this scenario
2. **CampaignMPGameMode1 →**  One of the core **Conflict** entities - here it is possible to adjust all kind of **Conflict gameplay options.**
3. **CampaignLoadoutManager\_1 →**  Defines **characters** (*loadouts*) which players can select in respawn menu

##### Faction Manager

SCR\_CampaignFactionManager
Faction Manager contains list of all factions that participate in Conflict game-mode. Main parameter of that entity is **Factions** array where it is possible to add or replace existing factions with custom ones.

[![armareforger-new-faction-conflict-manager.gif](/wikidata/images/4/49/armareforger-new-faction-conflict-manager.gif)](/wiki/File:armareforger-new-faction-conflict-manager.gif)

In this tutorial, default **USSR** faction is being replaced with **SampleFactionREDFOR**. Such replacement can be achieved by drag & dropping **SampleFactionREDFOR\_Campaing.conf** on **SCR\_CampaignFaction** element representing **USSR** faction inside **Factions** array.

Another notable thing which is set in **CampaignFactionManager1** is **Vehicle Asset List**, which contains link to a config file containing **list of available for request vehicles**. In this tutorial, previously created **VehicleAssetList\_SampleNewFaction.conf** is assigned to this parameter.

##### Game Mode tweaking

SCR\_GameModeCampaignMP

[![armareforger-new-faction-conflict-gamemode.png](/wikidata/images/5/53/armareforger-new-faction-conflict-gamemode.png)](/wiki/File:armareforger-new-faction-conflict-gamemode.png)

**CampaignMPGameMode1** entity contains also of various parameters but since goal here is just to change the playable faction, only **faction key** will be changed. Faction keys are used to connect factions defined in faction manger to one of the three playable sides in **Conflict** game mode.

In this tutorial, **OPFOR Faction Key**, located in **Unsorted** group of parameters, is changed from **USSR** to **SampleFactionREDFOR.**

#### Loadouts

SCR\_LoadoutManger

[![armareforger-new-faction-conflict-loadout.png](/wikidata/images/d/df/armareforger-new-faction-conflict-loadout.png)](/wiki/File:armareforger-new-faction-conflict-loadout.png)

Loadout manager controls which classes player can select in respawn menu. In this case, **USSR loadout** will be replaced with **SampleFactionREDFOR** variant and following parameters will have to be adjusted:

1. **Loadout Name** → Display name of loadout in respawn menu. Here it is replaced with **REDFOR** string
2. **Loadout Resource** → Prefab of spawned character. This parameter can be replaced with previously created Conflict character - **Campaign\_SampleFactionREDFOR\_Player.et** .
3. **Affiliated Faction →** Defines to which factions belongs this loadout. Parameter was replaced with **SampleFactionREDFOR**

#### Testing results in game

Once all those changes are done, it is possible to switch into play mode by either clicking on button with green arrow or by pressing F5 shortcut.

[![armareforger-new-faction-conflict-result.png](/wikidata/images/thumb/6/66/armareforger-new-faction-conflict-result.png/1200px-armareforger-new-faction-conflict-result.png)](/wiki/File:armareforger-new-faction-conflict-result.png)
