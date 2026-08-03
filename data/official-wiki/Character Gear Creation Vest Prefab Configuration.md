# [Character Gear Creation/Vest/Prefab Configuration](https://community.bistudio.com/wiki/Arma_Reforger:Character_Gear_Creation/Vest/Prefab_Configuration)

ⓘ

**Previous part** - [Asset Preparation](/wiki/Arma_Reforger:Character_Gear_Creation/Vest/Asset_Preparation "Arma Reforger:Character Gear Creation/Vest/Asset Preparation")

💬

**Overview**

This chapter covers the following topics:

* Vest prefab creation & configuration
* Setting protection setup on items
* Adding vest to arsenal

## Prefab Setup

## Creating Prefab

First step in this process will be [inheriting](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics") **[VestArmored\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/VestArmored_Base.et)** or [duplicate one of existing vests](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics") like **[Vest\_6B2\_base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_6B2/Vest_6B2_base.et)**. When inheriting from **[VestArmored\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/VestArmored_Base.et)**, few more steps will be required but those are listed later. On the other hand, unlike helmets, it is possible to configure **vest in multiple ways**, i.e. by using **ClothNodeStorageComponent** or **InventoryItemComponent.**

Additionally, since vests have extra elements separated (arm and groin protection), it will be also necessary to prepare prefab for that attachable prefab. In this case it is possible to use **[EquipmentPart\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/EquipmentPart_Base.et)** for inheritance or **[Vest\_ALICE\_belt.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Vests/Vest_ALICE/Vest_ALICE_belt.et)** for duplication - in both cases some extra work will be required - and call such prefab i.e. ***Vest\_Sample\_01\_Addon.et***

Also, don't forget to properly organize your prefab - it is quite useful to have **base prefab** with **\_base** suffix, since its clear in such setup which file was used as parent for sub sequential child prefabs, which change slightly materials or mesh.

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

[![](/wikidata/images/thumb/3/3a/armareforger-new-vest-baseLoadoutClothComponent.png/489px-armareforger-new-vest-baseLoadoutClothComponent.png)](/wiki/File:armareforger-new-vest-baseLoadoutClothComponent.png)

**BaseLoadoutClothComponent** content

### Tweaking BaseLoadoutClothComponent

Once you have new prefab for your vest, open it in for example **[Prefab Edit Mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** and start with tweaking things located in **BaseLoadoutClothComponent**

* Enable **PhysicsOnWearEnabled** & **AnimateCollidersOnWear** properties
* Assign item model to **ItemModel** in **BaseLoadoutClothComponent** & to property **Object** in **MeshOjbect** component
* Assign worn model to **WornModel**
* **Optionally**: Change **SoundInt** parameter from 400 (*generic vest sounds*) to something different
  + This parameter is responsible for additional sounds that are being played when certain type of gear is worn. For instance if you wear harness, you can hear some extra metal rattle when moving through the landscape. See [Character SoundInfo Signals Reference](/wiki/Arma_Reforger:Character_SoundInfo_Signals_Reference "Arma Reforger:Character SoundInfo Signals Reference") page for more info

After you are done with the main vest prefab, apply similar tweaks to ***Vest\_Sample\_01\_Addon.et***

✩

**Tip**: You can find details how to create variants of vest with different material on [following page](/wiki/Arma_Reforger:Faction_Creation#Retexturing_existing_equipment "Arma Reforger:Faction Creation").

## Inventory Configuration

Configuration of inventory is quite similar compared to weapon or headgear, therefor it is suggested to take a look at [Weapon Creation page](/wiki/Arma_Reforger:Weapon_Creation/Prefab_Configuration#Inventory_Configuration "Arma Reforger:Weapon Creation/Prefab Configuration") or [Headgear tutorial](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Prefab_Configuration#Inventory_Configuration "Arma Reforger:Character Gear Creation/Headgear/Prefab Configuration") for details regarding how each of those params are working.

Depending whether you are planning to make an vest, which has additional things attached to it like pouches on for instance **Soviet harness or ALICE** gear, then it is recommended to use **ClothNodeStorageComponent.**

[![](/wikidata/images/0/04/armareforger-new-vest-cloth-node-setup.png)](/wiki/File:armareforger-new-vest-cloth-node-setup.png)

**ClothNodeStoreComponent** content

If you already have **InventoryItemComponent** (i.e. because you duplicated one of the vanilla prefabs which had that) then it is still possible to change it. This can be done by clicking on **InventoryItemComponent** with and then selecting from the context menu option **Change class.**

[![armareforger-new-vest-change-component.gif](/wikidata/images/9/9a/armareforger-new-vest-change-component.gif)](/wiki/File:armareforger-new-vest-change-component.gif)

When inheriting from **[VestArmored\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/VestArmored_Base.et),** you will need to add **ClothNodeStorageComponent** via **+ Add Component** button and then fill all the data from scratch.

Below is list of things which were adjusted in **ClothNodeStorageComponent** on **base** **Sample Vest prefab:**

* **Item Display Name**
  + **Name** - controls in game display name
  + **Description** - description of the item visible in in-game inventory menu
* **Item Phys Attributes**
  + **Weight** was changed to **10 [kg]** to be more or less in line with real life vest containing 2 large plates and 2 smaller side plates
  + **Size Setup Strategy** was changed to **Manual**
  + **Item Dimensions** were changed to **X:55, Y:35, Z:5** to more or less correspond to its actual dimensions
  + **Item Volume** was set to **5000**
* **PeviewRenderAttributes** in **Custom Attributes** section:
  + **Camera Orbit Angles, FOV &** **Camera Distance To Item** were adjusted so that whole vest fits the preview image in the inventory
  + **Preview Worn Model** option was **checked**

Vanilla vests are using **2x2 slots** and once those tweaks are completed, it should look like that in game:

[![](/wikidata/images/9/9e/armareforger-new-vest-inventory-menu.png)](/wiki/File:armareforger-new-vest-inventory-menu.png)

**Sample Vest** in inventory menu

## Protection configuration

Since 1.3 update, protective items are solely relaying on colliders for its protection. Since colliders alone are working in binary way (there is penetration and damage is being dealt or there is no damage at all), an additional component is required.

[![](/wikidata/images/thumb/4/43/armareforger-new-vest-damage-manager.png/300px-armareforger-new-vest-damage-manager.png)](/wiki/File:armareforger-new-vest-damage-manager.png)

**SCR\_ArmorDamageManagerComponent** configuration

#### Damage pass-through

Vests and helmets are using **SCR\_ArmorDamageManagerComponent** to simulate blunt force trauma dealt by bullets, which **normally wouldn't do any damage** to character since plates completely stopped the bullets. With that component though, character **resilience** is reduced by each shot at the vest, which might result in character being **unconscious** after multiple shots. If all shots landed on armored plates, there will be no injuries or bleeding and after a while character can resume combat.

Since 1.3, if you are inheriting from for instance **[VestArmored\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/VestArmored_Base.et)**, then this component should be already configured correctly. If you are inheriting from some other prefab which is missing it, then configuration of that component can be done in following steps:

1. Add **SCR\_ArmorDamageManagerComponent** to the vest prefab via **+ Add Component** button
2. Add new **HitZone** element to **Additional hit zones** list - name of that hit zone doesn't matter but you can call it f.e. ***Vest***
3. Enable **HZ Default** property inside that new hitzone
4. Change **Max Health** property to **10000**
5. Fill **Collider Names** list with colliders from the vest
   1. Please note that list box will only show bones from model which is defined in **MeshObject** component. It might be necessary to **manually type in collider** names. As a workaround, you can temporarily change model in **MeshObject** component to worn variant (the one containing colliders), fill the list and switch back model assigned in **MeshObject** to previous one

## Creating variant

In order to have some variety, shoulder & groin pads were separated from the main mesh and were moved to another XOB. Configuring those parts as separate prefabs, which can be attached in Workbench, involves few steps listed below.

### Creating equipment part

[![](/wikidata/images/thumb/9/91/armareforger-new-vest-addon-part.png/300px-armareforger-new-vest-addon-part.png)](/wiki/File:armareforger-new-vest-addon-part.png)

Configuration of equipment part

First, lets start with creating new prefab [inheriting from](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics") **[EquipmentPart\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/EquipmentPart_Base.et)**, which should provide basis for **equipment attachment** - this is not new vest variant yet! In this example such prefab was called **Vest\_Sample\_01\_Addon**.

⚠

Keep in mind that right now game doesn't allow you to attach such equipment parts in runtime. Instead, vest prefab with already attached extra parts will be available to be picked up in for example arsenal crate.

Once prefab is created, open it i.e. in **[Prefab Edit Mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** and start adjusting following properties:

* Assign mesh of the item variant of additional parts (*Vest\_SampleVest\_01\_addon\_item.xob*) in **MeshObject** component
* In **RigidBody** component enable **Static** property
* Tweak **SCR\_SoundDataComponent** so that **SCR\_AudioSourceConfiguration** entries with **Sound Event Name** "*SOUND\_PICK\_UP*" & "*SOUND\_EQUIP*" are pointing to **[Items\_PickUp\_Cloth.acp](enfusion://ResourceManager/~ArmaReforger:Sounds/Items/_SharedData/PickUp/Items_PickUp_Cloth.acp)** - this will ensure that more appropriate sounds are played when this item is picked up or equipped
* [**Change class**](/wiki/Arma_Reforger:Prefabs_Basics#Changing_class "Arma Reforger:Prefabs Basics") of **SCR\_UniversalInventoryStorageComponent** to **SCR\_EquipmentStorageComponent** and adjust inside of it following properties:
  + Adjust **Weight** of the item - this will be correctly summed when this addon is attached to the vest. In this case **Weight** was set to **3**
* Add **[BaseLoadoutCloth\_Base.ct](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/BaseLoadoutCloth_Base.ct)** component template to the prefab by [**drag and dropping** in Object Properties window](/wiki/Arma_Reforger:Prefabs_Basics#Component_prefabs "Arma Reforger:Prefabs Basics"). This will add **BaseLoadoutClothComponent** which needs following tweaks:
  + Change **Worn Model & Item Model** parameters to respective models (*in this case Vest\_SampleVest\_01\_addon.xob & Vest\_SampleVest\_01\_addon\_item.xob*)
  + Enable **Physics On Wear Enabled**
  + Enable **Animate Colliders On Wear**

### Creating vest variant

[![](/wikidata/images/thumb/0/04/armareforger-new-vest-addon-prefab.png/499px-armareforger-new-vest-addon-prefab.png)](/wiki/File:armareforger-new-vest-addon-prefab.png)

Configuration of **heavy variant of vest**

After equipment part is ready, it is possible to move on to creation and configuration of actual vest variant. To do so, perform following steps:

* [**Inherit**](/wiki/Arma_Reforger:Prefabs_Basics#Creating_inherited_prefab "Arma Reforger:Prefabs Basics") from previously created **base prefab** (*in this case it is **Vest\_Sample\_01\_base.et**)* and call it i.e. *Vest\_Sample\_01\_Heavy.*
* In **ClothNodeStorageComponet** adjust following things:
  + Change **Name & Description** in **Item Display Name** so it fits more the variant of the item
  + Tweak **Preview Render Attributes** whole vest is still visible in the UI. In this case, **Camera** **Distance To Item** parameter was increased to **4.9**
  + Add two new **Protected Hit Zones** for **shoulder pads** - **LArm** & **RArm**
    - Due to size and fact, that **groin pad** only protects frontal section of the character, this element is using **actual fire geometry** to protect character from incoming projectiles

Weight of the vest itself doesn't have to be adjusted, since game itself is clever enough to combine weight of all items attached to main vest.

#### Adding slots

To add previously created equipment part to the vest, we need to start with adding **new slot** in **BaseLoadoutClothComponent**. To do so, click on **plus + button** on the right side of **Slots** parameter and new slot called i.e. **Addon**.Once slot is created, expand it and:

* Assign previously created equipment part *(Vest\_Sample\_01\_Addon.et)* to **Prefab** field
* Enable **Inherit Parent Skeleton** property

Et voilà! Don't forget to save all changes to the prefab afterwards and if everything went fine, you should be able to test this new variant in game.

[![armareforger-new-vest-addon-slot-configuration.png](/wikidata/images/7/79/armareforger-new-vest-addon-slot-configuration.png)](/wiki/File:armareforger-new-vest-addon-slot-configuration.png)

## Adding to Arsenal

Process of adding headgear to the crates is very similar to the one described on **[Weapon Creation page](/wiki/Arma_Reforger:Weapon_Creation/Prefab_Configuration#Crate_Filling "Arma Reforger:Weapon Creation/Prefab Configuration")**. There are few differences though - notable different labels are used. So in short, if you intend to add helmet to i.e. existing US arsenal box, then perform following steps:

* [Override in](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics") your addon **[InventoryItems\_EntityCatalog\_US.conf](enfusion://ResourceManager/~ArmaReforger:Configs/EntityCatalog/US/InventoryItems_EntityCatalog_US.conf)**
* Open overridden file and locate **Backpack and Vests** section inside of it
* Add new entry to **Entities** list by clicking on **+ button** on the right side
* Assign your prefab to **Entity Prefab** field
* Add new **SCR\_ArsenalItem** *(Arsenal Data)* entry to **Entity Data List** array by clicking on **+ button** on the right side
* Change **Item Type** in **Arsenal Data** to **VEST\_AND\_WAIST**

If everything went fine, you should end up with something like on the picture below

[![armareforger-new-vest-arsenal-setup.png](/wikidata/images/7/79/armareforger-new-vest-arsenal-setup.png)](/wiki/File:armareforger-new-vest-arsenal-setup.png)

## Testing & Diags

Once you done all above steps, you can proceed with testing vest in play mode which is pretty much same as in [Headgear tutorial](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Prefab_Configuration#Testing_.26_Diags "Arma Reforger:Character Gear Creation/Headgear/Prefab Configuration").

[![armareforger-new-vest-diag-on.png](/wikidata/images/thumb/3/36/armareforger-new-vest-diag-on.png/743px-armareforger-new-vest-diag-on.png)](/wiki/File:armareforger-new-vest-diag-on.png)[![armareforger-new-vest-final-result.png](/wikidata/images/7/74/armareforger-new-vest-final-result.png)](/wiki/File:armareforger-new-vest-final-result.png)
