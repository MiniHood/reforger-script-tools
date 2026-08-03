# [Character Gear Creation/Vest/Asset Preparation](https://community.bistudio.com/wiki/Arma_Reforger:Character_Gear_Creation/Vest/Asset_Preparation)

ⓘ

**Previous part** - [Vest](/wiki/Arma_Reforger:Character_Gear_Creation/Vest "Arma Reforger:Character Gear Creation/Vest")

ⓘ

**Next part** - [Prefab Configuration](/wiki/Arma_Reforger:Character_Gear_Creation/Vest/Prefab_Configuration "Arma Reforger:Character Gear Creation/Vest/Prefab Configuration")

💬

**Overview**

This chapter covers following topics:

* Adjusting model to reference character
* Creating skeleton & applying skinning to vest
* Creating animated colliders
* Creating item variants models

## Mesh Preparation

## General

Vest creation is not so much different from headgear, so there is quite a lot of overlap. There are of course some differences nevertheless it is recommended to read [Headgear tutorial](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear "Arma Reforger:Character Gear Creation/Headgear") before proceeding any further.

Similar to headgear, you can use one of the existing models to position your character.

There is one important thing to keep in mind - vests are able to influence model of the jacket and switch it to deflated version of the mesh. Right now, those deflated variants of the jackets are created by hand and they are made in a way, which simulates a jacket when vest is worn on top of it.

In **Jacket\_Deflated\_Template.fbx,** there is voxelised model of M88 jacket which you can use to adjust your mesh, so it fits nicely the character. **Armored Vests** in Reforger are by default without any pouches and it is possible to wear on top of it any available in game harness. If you want to keep your vest compatible with other harnesses available in game, it might be necessary to adjust how **close is vest to the body.** In Blender, you can for instance use **[Sculpt Mode](https://docs.blender.org/manual/en/latest/sculpt_paint/sculpting/introduction/general.html).**

Pay attention especially to rear parts of the body and ensure that jacket is not clipping through the vest. After few iterations, you should have model ready for skinning like on pic below.

✩

**Tip**: You can revisit how the model fits the character after getting this model in-game first

[![armareforger-new-vest-adjusted-model.png](/wikidata/images/thumb/2/24/armareforger-new-vest-adjusted-model.png/1200px-armareforger-new-vest-adjusted-model.png)](/wiki/File:armareforger-new-vest-adjusted-model.png)

## Rigging

Once vest is properly oriented and placed, it is possible to move towards the next step, which is rigging of the model. Unlike Sample Helmet, skinning of Arma 3 models will be way different around chest area so **utilizing weights from A3 is not an option**. Fortunately, weight transfer gives usually a good results and then only manual tweaks are necessary.

### Importing skeleton

[![](/wikidata/images/3/3c/armareforger-new-headgear-skeleton.png)](/wiki/File:armareforger-new-headgear-skeleton.png)

**Armature** and **Empty Objects** necessary for equipment to be animated in game

Similar to Sample Helmet, first step will be copying skeleton from **[Character\_Weights\_Template.blend](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Weights_Template.blend)** to Blender instance containing vest. Exact steps are same on [**Sample Helmet tutorial**](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Importing_skeleton "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation") and if everything went fine, you should end up with something like this.

[![armareforger-new-vest-skeleton-full.png](/wikidata/images/thumb/b/ba/armareforger-new-vest-skeleton-full.png/867px-armareforger-new-vest-skeleton-full.png)](/wiki/File:armareforger-new-vest-skeleton-full.png)

Again, it is important that model contains only **Armature** and **[empty objects](https://docs.blender.org/manual/en/latest/modeling/empties.html) from the template**!

* In total, **163 bones/empty objects** (*156 bones and 6 dummy objects*) should be present when model is imported into **Workbench** - see note in **Importing model** section

### Skinning asset

After skeleton was successfully added to the scene, next step will be skinning of the asset. Main differences compared to skinning of Sample Helmet are:

* Vests are influenced by **dozens of bones** so **initial transfer weight is a must** unless you want to spend significant amount of time making your own skinning from scratch
  + After transferring weights, some manual adjustments are usually required
* Weights for the vest can be transferred from either **[Character\_Weights\_Template.blend](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Weights_Template.blend)** (*for non blender users, there is [Character\_Template.fbx](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Template.fbx), which still needs to be rotated*) or from other vests, like... [Vest\_SampleVest\_01](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/Vests/Vest_Sample/Vest_SampleVest_01.fbx)
  + Using Sample Vest as base should yield better results in most cases - especially areas around armpits should look better

Of course, when Sample Vest was prepared, weights were transferred from another vest - namely 6B2 - therefore don't be surprised to see pictures of it in this tutorial.

#### Transfer weights

Once reference mesh is in, you can follow instruction for [transferring weights](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Transfer_weights "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation") which are mentioned in **Headgear** tutorial.

As an example, 6B2 was used as base for **Transfer Weights** functions. Since 6B2 vests doesn't have neither **arm** or **groin protection**, those parts were moved to separate object called **Addon\_LOD0** in the sample blend file.

Following instructions listed on [Sample Headgear](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Transfer_weights "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation") page, you should get quite good skinning on **central part of the vest**.

When it comes to mesh located in **Addon\_LOD0**, you can use **[Character\_Weights\_Template.blend](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Weights_Template.blend)** to rest of the weights

* [![Transferring weights to Object_LOD0 using 6B2 as source](/wikidata/images/thumb/e/ee/armareforger-new-vest-transfer-weights-from-vest.png/222px-armareforger-new-vest-transfer-weights-from-vest.png)](/wiki/File:armareforger-new-vest-transfer-weights-from-vest.png "Transferring weights to Object_LOD0 using 6B2 as source")

  Transferring weights to **Object\_LOD0** using 6B2 as source
* [![Transferring weights to Addon_LOD0 parts using Character_Weights_Template.blend file](/wikidata/images/thumb/e/e8/armareforger-new-vest-addon-skinning.png/297px-armareforger-new-vest-addon-skinning.png)](/wiki/File:armareforger-new-vest-addon-skinning.png "Transferring weights to Addon_LOD0 parts using Character_Weights_Template.blend file")

  Transferring weights to **Addon\_LOD0** parts using Character\_Weights\_Template.blend file

#### Tweaking skinning

[![](/wikidata/images/thumb/3/3b/armareforger-new-vest-vertex-groups.png/601px-armareforger-new-vest-vertex-groups.png)](/wiki/File:armareforger-new-vest-vertex-groups.png)

**Vertex Groups** present in **Object\_LOD0**

In both cases, it will be still required to tweak skinning in some areas and in **[Weight Paint](https://docs.blender.org/manual/en/latest/sculpt_paint/weight_paint/index.html) mode** but since applying animation to vest in Blender is rather problematic thing (especially after rotation), it might be worth to go back to this stage once model is imported in game.

⚠

Watch out - model of vest is not updated correctly after reimport until you either **launch World Editor in Play Mode** or use **Reload Game Scripts** (Ctrl+R) function.

#### Armature modifier

After doing initial skinning there is one more thing to do in order to link vertex groups with skeleton and thus achieve animated asset:

In **Modifiers** tab, add new [**Armature** modifier](https://docs.blender.org/manual/en/latest/modeling/modifiers/deform/armature.html) via **Add Modifier (1)** button

* In **Object** property, select **Armature** as object to deform with

After that, you could quickly verify if asset is skinned by selecting **Armature** and switching to **Pose Mode.** In this mode you can try to move bones and see if mesh is following them.[![armareforger-new-headgear-aramture-modifier.png](/wikidata/images/8/8f/armareforger-new-headgear-aramture-modifier.png)](/wiki/File:armareforger-new-headgear-aramture-modifier.png)

## Colliders

Vests in **Arma Reforger,** similar to helmets, are using actual colliders for protection of the character so its quite important to set it correctly. Armored vests usually consist of some **protective fabric** like kevlar and some armored plates. In case of 6B2, it has combination of titanium plates and soft, anti shrapnel protection made out of aramid. **In case of sample vest**, it will be required to model both **soft** (*kevlar*) and **hard armor** (*armor plate*).

### Creating colliders

First step towards creating proper collider will be making of a new object, which should receive one of the [colliders prefixes](https://community.bistudio.com/wiki/Arma_Reforger:FBX_Import#Collider_shape) (in this case **UTM\_** prefix was used). In case of vests, in most cases it will be better to use:

* **Simple single sided faces**
* **Game Material** with **predefined thickness**

While in theory it would be possible to create realistic plates with thickness and shape of real life objects, end result would probably clip through character and hit detection could also not work correctly if those colliders would be too close to character body.

📖

**Recommended read:** In section below it assumed that you know how to use either [**Make Parent**](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Setting_relations:_Using_Make_Parentr "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation") function or [**Relations tab**](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Setting_relations:_Using_Relations_tab "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation").

#### Plates setup

[![](/wikidata/images/thumb/9/91/armareforger-new-vest-6b2-plate-setup-real.png/438px-armareforger-new-vest-6b2-plate-setup-real.png)](/wiki/File:armareforger-new-vest-6b2-plate-setup-real.png)

Placement of plates on real life 6B2 vest

Depending on what kind of vests are you doing, it is possible to do some experimentation about setup of colliders although it is worth to keep in mind limitation of collider animation - **every collider object can be only attached to single bone**.

In **case of 6B2**, each row of plates is attached to different **Spine** bone. Such solution has its pros & cons:

* ❌ Requires multiple colliders
* ✅ Follows character more closely which somewhat simulate multiple tiny plate setup of 6B2
* ❌ There are **gaps or overlaps** in fire geometry in **some character poses**

* [![armareforger-new-vest-6b2-plate-setup-game.png](/wikidata/images/thumb/5/57/armareforger-new-vest-6b2-plate-setup-game.png/337px-armareforger-new-vest-6b2-plate-setup-game.png)](/wiki/File:armareforger-new-vest-6b2-plate-setup-game.png)
* [![armareforger-new-vest-6b2-plate-setup-debug.png](/wikidata/images/thumb/d/d2/armareforger-new-vest-6b2-plate-setup-debug.png/380px-armareforger-new-vest-6b2-plate-setup-debug.png)](/wiki/File:armareforger-new-vest-6b2-plate-setup-debug.png)

[![](/wikidata/images/6/69/armareforger-new-vest-colliders.gif)](/wiki/File:armareforger-new-vest-colliders.gif)

Other option for colliders setup might be using **single** bone (*& object*) for plates located on the chest. Such setup is somehow more **realistic for single rigid armored plates** which can be placed in modern plate carriers.

**Sample Vest** is an example of a vest which use single bone - **Spine5 -** for all chest mounted plates. Groin protection plate is connected to **Spine1.**

On picture below, you can observe that there are no visible **gaps** when character is looking upwards and that colliders are also **not clipping with the character**.

* [![armareforger-new-vest-colliders-setup.png](/wikidata/images/thumb/a/ad/armareforger-new-vest-colliders-setup.png/490px-armareforger-new-vest-colliders-setup.png)](/wiki/File:armareforger-new-vest-colliders-setup.png)
* [![armareforger-new-vest-collider-debug.png](/wikidata/images/thumb/d/d7/armareforger-new-vest-collider-debug.png/317px-armareforger-new-vest-collider-debug.png)](/wiki/File:armareforger-new-vest-collider-debug.png)

On soft areas, **[soft\_aramid\_6.5mm.gamemat](enfusion://ResourceManager/~ArmaReforger:Common/Materials/Game/PersonalProtection/soft_aramid_6.5mm.gamemat)** game material can be assigned, which is kind of equivalent of [Kevlar 29](https://technologystudent.com/joints/kevlar2.html) - it might be considered bit outdated material for such vest but for initial setup, it should be fine. Creation of new game material is described in next paragraph.

ⓘ

If trimesh colliders are linked to same bone and option **Merge trimeshes** is checked (by default it is **on**) then all those colliders will be merged to a single object in Workbench. You can observe that by checking number of colliders in **Resource Manager**

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

### Creating material

##### Material Research

Next step in mesh preparation will be doing some research about what material are used on vest that you are making.

📖

**Recommended read:** [**Material creation**](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Creating_material "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation") section of Headgear tutorial.

In this case, **Sample Vest** is supposed to have protection level equal to [**ESAPI rev. G**](https://en.wikipedia.org/wiki/List_of_body_armor_performance_standards#US_military_armor_standards) plate.

###### Material type

Once we have established what we are looking for, it is time to commence search for **type of material** used in such plate. In case of Sample Vest, finding general information about **ESAPI plate** (we are not aiming here for any specific model) was quite easy, since most of **[SAPI ballistic plates](https://en.wikipedia.org/wiki/Small_Arms_Protective_Insert#Materials_and_capabilities)** are made out of [boron carbide](https://en.wikipedia.org/wiki/Boron_carbide).

###### Material density

After finding type of the material, it is time to obtain **density** data. From [boron carbide](https://en.wikipedia.org/wiki/Boron_carbide) Wiki page, you can obtain information about **density** of this material which is equal to **2.50 g/cm3**.

###### Material thickness

Next on the list will be finding of **thickness of the plate**. If its not possible to find a solid numbers on such material, try to use values from similar types of plates. In case of **Sample Vest**, thickness listed on <https://pgd-bodyarmor.com/shop/ballistic-plate-pgd-esapi-iv-sa/> page - **24.9mm -** was used.

###### Material kinetic protection

With all that data, it is is possible to create material with protection similar to desired level. In this case, **Sample Vest** is aiming at NIJ Level IV protection, which is kind of equivalent of **16mm** of [RHA armor](https://en.wikipedia.org/wiki/Rolled_homogeneous_armour).

Using [following spreadsheet](https://github.com/BohemiaInteractive/Arma-Reforger-Misc/blob/main/Configuration/Weapons/Weapons_Penetration_List.xlsx) it is possible to estimate KE coef for such game material.

[![](/wikidata/images/7/73/armareforger-new-headgear-penetration-table.png)](/wiki/File:armareforger-new-headgear-penetration-table.png)

**Material Data** section of **Weapon Penetration List** spreadsheet

##### Game Material creation

Once you have data, you can commence with creation of the material by performing following steps:

* Create new inherited material - usage of one of the existing materials is recommended or duplicate some existing material which inherits from one of the core materials. In case of **Sample Vest** [hard\_aramid\_7.3mm.gamemat](enfusion://ResourceManager/~ArmaReforger:Common/Materials/Game/PersonalProtection/hard_aramid_7.3mm.gamemat) was duplicated
  + Name that new material in a way, which represents its thickness (*i.e. Plate\_SampleVest\_01.gamemat )*
  + This way, you will inherit for instance correct particle effects, sounds and decals
  + In vanilla game those visual & sound effects are still being worked on (0.9.9)
* Change **Density, Thickness & Kinetic Resistance**

### Materials & Layer Preset

Next step will be assigning of material and layer preset to the collider itself. You can do it via [Enfusion Blender Tools Object Tools](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools) or [manually](https://community.bistudio.com/wiki/Arma_Reforger:FBX_Import) (in case you are using different 3D software). Once you know how to change material and layer preset, do following things:

* Set collider **Layer Preset** to **FireGeo**
* Assign **game material** - either one of the existing vanilla ones or the one you have created in previous step

## Splitting model

In case of Sample Vest, model separated in **two parts** - one containing **main vest** and the other one contains **additional arm and groin protection**. This way, you can have **two variants of the vest** quite easily.

To keep this thing sort of manageable, it is possible to use in Blender [**collection linking**](https://docs.blender.org/manual/en/latest/scene_layout/collections/collections.html) & **[Batch FBX Export](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Batch_FBX_Export "Arma Reforger:Enfusion Blender Tools: Batch FBX Export")** function which is part of **Enfusion Blender Tools.** Collection linking allows you to have same object in multiple collections and that means you don't have to create duplicates when making some tweaks to i.e. some common parts which exist in both two models you want to export.

### Setting collection

[![](/wikidata/images/8/8b/armareforger-new-vest-link-to-collection.png)](/wiki/File:armareforger-new-vest-link-to-collection.png)

**Link to Collection** option in **Menu Search**

Let's begin with creating new collection and then **linking objects** to them. To do so, perform following actions:

* In **[Outliner](https://docs.blender.org/manual/en/latest/editors/outliner/introduction.html),** create new **Collection** called for example **Export** - this collection is purely for organizational purposes
  + Create a new sub collection for variants of the vests you want to export. **🟥 Name of this** **collection determines name of the exported FBX! 🟥**
    - In case of **Sample Vest**, two new collection were created - **Vest\_SampleVest\_01** & **Vest\_SampleVest\_01\_addon**
* Select **Armature** & all empty objects in **Memory Points** collection in **Outliner**
* Either:
  + Open [Menu Search](https://docs.blender.org/manual/en/latest/interface/controls/templates/operator_search.html#menu-search) ( `↹ Tab` using [Industry Compatible Keymap](https://docs.blender.org/manual/en/latest/interface/keymap/industry_compatible.html#general) ) and search for **Link to Collection**
    - If you want to **link multiple objects** (*like multiple memory points + armature)* then select **object.link\_to\_collection ➡Link to Collection**. *(note it is also possible to link this way single objects too!)*
    - It is possible to also use **object.collection\_link ➡ Link to Collection** but this one will only let you link single object
  + Use **Link to Collection shortcut** ( `Ctrl + Shift + M` or `Ctrl + Shift + G` using [Industry Compatible Keymap](https://docs.blender.org/manual/en/latest/interface/keymap/industry_compatible.html#general))
* In a pop-up window, select the collection to which you want to link the object
  + In this case it will be **Export➡Vest\_SampleVest\_01**
* Repeat above steps and link armature and memory points to **Export➡Vest\_SampleVest\_01\_addon** collection

Same principle applies to LODs & Colliders, once you are through this procedure for rest of the objects, you should end up with something like this:

[![armareforger-new-vest-collection-linking.png](/wikidata/images/c/c3/armareforger-new-vest-collection-linking.png)](/wiki/File:armareforger-new-vest-collection-linking.png)

You can see that objects that have set **relations**, such as colliders or memory points, are linked under the parent object (in this case, the **Armature**). Objects that are not linked to the collection, like **UTM\_Armor\_Addon** in **Vest\_SampleVest\_01** collection, will be shown **grayed out** and **without controls for visibility**

### Batch FBX Export

Once you have set all collections, you can select folder where to export FBX files in **Settings tab** of **Enfusion Tools**. After that, it is possible to select collections which you want to export (*in this case Vest\_SampleVest\_01 & Vest\_SampleVest\_01\_addon)*, click on it with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button")and then select **Batch Export FBX** option from the context menu.

[![armareforger-new-vest-batch-fbx-export.gif](/wikidata/images/9/96/armareforger-new-vest-batch-fbx-export.gif)](/wiki/File:armareforger-new-vest-batch-fbx-export.gif)

After that you should spot new FBX models in location that you have set.

## Creating Item variant

One of the final steps in equipment preparation is creating a special **item variant** of the model. Similar to the [**Headgear tutorial**](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation#Creating_Item_variant "Arma Reforger:Character Gear Creation/Headgear/Asset Preparation"), you can create such item variant by modifying regular mesh and moving it around. Depending how the mesh was prepared (Marvelous Designer) it might be worth investigating if such folded variant can be done using simulation. In case of this vest, model was prepared by flattening mesh in one axis a little bit and if you don't pursue perfection, then such setup should be sufficient for most of the use cases.

[![armareforger-new-vest-item-variant.png](/wikidata/images/e/ef/armareforger-new-vest-item-variant.png)](/wiki/File:armareforger-new-vest-item-variant.png)

## Importing model

Last step in asset preparation is importing of model. Detailed procedure for importing mesh into Workbench can be found on [FBX Import](https://community.bistudio.com/wiki/Arma_Reforger:FBX_Import#Import_process_in_the_Workbench) page and - if you want some practical example - also in [Weapon Creation tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Model_Import_.26_Registration).

In any case, here is quick instruction how to do it - select both of the FBX files in Workbench and then click with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") on one of them. From the context menu, select **Register & Import** option and then select **as Model** option when asked how do you want to process given file.

* Make sure that **Export skinning** option is checked and reimport the model via **Reimport resource** button
* Make sure that you have 🟥 **230 bones in total 🟥** and there is at least one skinned bone - *i.e. 1 + 229*
  + In **Details** tab, you can find current amount of bones and dummies in **Bones** section. First number represents amount of bones and second, number of dummies. By default, bones which are not affecting any mesh (there is no Vertex Group representing it)

[![armareforger-new-vest-import-workbench.png](/wikidata/images/thumb/8/8b/armareforger-new-vest-import-workbench.png/1358px-armareforger-new-vest-import-workbench.png)](/wiki/File:armareforger-new-vest-import-workbench.png)

ⓘ

**Next part** - [Prefab Configuration](/wiki/Arma_Reforger:Character_Gear_Creation/Vest/Prefab_Configuration "Arma Reforger:Character Gear Creation/Vest/Prefab Configuration")
