# [Character Gear Creation/Headgear/Asset Preparation](https://community.bistudio.com/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Asset_Preparation)

ⓘ

**Previous part** - [Headgear](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear "Arma Reforger:Character Gear Creation/Headgear")

ⓘ

**Next part** - [Prefab Configuration](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Prefab_Configuration "Arma Reforger:Character Gear Creation/Headgear/Prefab Configuration")

💬

**Overview**

This chapter covers following topics:

* Adjusting model to reference head
* Creating skeleton & applying skinning to helmet
* Creating animated colliders
* Creating item variants models

## Mesh Preparation

## General

When it comes to character gear, you might notice that default rotation of character is different compared to other assets. This is caused by the fact, that in game animations are created using motion capture & Motion Builder software, which expects different orientation that the game.

⚠

It is **no longer necessary to rotate equipment by 180 degrees** - keep in mind though that [Character\_Template.FBX](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Template.fbx) & [Head\_Template.FBX](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Head_Template.fbx) have orientation set in a way, which allows to use this model in creation of animations in Motion Builder or similar software.

Positioning of helmet without any reference points is quite tricky and that's why it might be handy to load some reference model. To do so, perform following steps:

* Import **[HeadSize\_Template.fbx](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Head_Template.fbx)** into scene with your model
  + This special contains all vanilla heads available molded into one entity - with this you can ensure that there will be no clipping with any of the existing models
* Adjust position of the helmet and straps so it fit that universal head

If everything went fine, you should end up with something like this:

[![armareforger-new-headgear-head-template.png](/wikidata/images/thumb/5/5c/armareforger-new-headgear-head-template.png/458px-armareforger-new-headgear-head-template.png)](/wiki/File:armareforger-new-headgear-head-template.png)

Once you verified that helmet model doesn't need more adjustments, delete **HeadSize\_Template** mesh from the scene since it will be no longer necessary.

## Rigging

After properly positioning model, it is time for getting skeleton in and then rigging the asset. Sample Helmet is an asset imported from A3, so it already has some skinning on it, which in case of helmets will be fully usable but if you have some fresh asset, you can consider two options, which are described below.

In any case, first step on to do list will be importing of skeleton and empty bones from template file.

### Importing skeleton

[![](/wikidata/images/3/3c/armareforger-new-headgear-skeleton.png)](/wiki/File:armareforger-new-headgear-skeleton.png)

**Armature** and **Empty Objects** necessary for equipment to be animated in game

Importing skeleton into scene where you have your helmet can be done in few simple steps:

* Keep your **Blender** instance with helmet open
* Download & then open **[Character\_Weights\_Template.blend](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Weights_Template.blend)** file in **2nd instance of Blender**
* Switch to **Object Mode**
* Unhide **Memory Points** collection and select **all empty objects** over there (there should be six of them in total)
  + Tip: You can select multiple empty objects Left Shift modifier
* Copy all those objects with `Ctrl` + `C` combination
* Switch over to **Blender** instance containing your helmet and paste objects into Blender with `Ctrl` + `V` shortcut

After that, verify that in primary instance of the Blender (the one with helmet model) following things are true:

* Make sure that you also imported empty objects (from **Memory Points** collections)
* **Armature object** is called **Armature** - keep it this way! When asset is imported into Workbench, **Armature** name at the top of the skeleton is ignored
* It is important that model contains only **Armature** and **[empty objects](https://docs.blender.org/manual/en/latest/modeling/empties.html) from the template**!
  + In total, **163 bones/empty objects** (*156 bones and 6 dummy objects*) should be present when model is imported into **Workbench** - see note in **Importing model** section

[![armareforger-new-headgear-skeleton-full.png](/wikidata/images/thumb/1/1d/armareforger-new-headgear-skeleton-full.png/954px-armareforger-new-headgear-skeleton-full.png)](/wiki/File:armareforger-new-headgear-skeleton-full.png)

### Skinning asset

Once skeleton is present in the scene, it is possible to start process of skinning mesh to the bones. This can be using one of two methods listed below

#### Transfer weights

Currently, only [Basebody\_Male\_Head\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Basebody/Basebody_Male_Head_01.xob) has its face skinned and there are no facial animations yet it in-game, so it's rather tricky to verify if weights are working correctly. In any case, weights transfer is

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

In any case, [weights transfer](https://docs.blender.org/manual/en/latest/sculpt_paint/weight_paint/editing.html#transfer-weights) can be done in following steps:

* Copy **reference mesh** (f.e. **Character\_Weights\_Template.blend** can be used) to the scene with helmet
* Select **reference mesh**
* **Select Helmet mesh with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")**& **`⇧ Shift`**
  + **You might want to separate chin strap to separate object and keep helmet hard shell assigned to Head vertex group**
* Switch to **[Weight Paint](https://docs.blender.org/manual/en/latest/sculpt_paint/weight_paint/index.html) mode**
* From the menu in top left corner, select **Weights** -> **Transfer Weights** option
  + In bottom left corner of the viewport, there should be now **Transfer Mesh Data** menu which you can expand
  + Change **Source Layers Selection** to **By Name**
* Click with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") somewhere in the viewport to confirm the transfer

✩

**Tip**: You can also use **Sample Helmet** as source object for weight transfer.

#### Manual skinning

[![](/wikidata/images/thumb/e/ed/armareforger-new-headgear-vertex-group.png/361px-armareforger-new-headgear-vertex-group.png)](/wiki/File:armareforger-new-headgear-vertex-group.png)

**Vertex Groups** present in **Object\_LOD0**

If you don't want to use transfer weight tool or you quickly want to test if helmet is working in game, then you can skin the helmet manually. To do so, perform following actions:

* Select helmet model (*in this case Object\_LOD0*) and switch to Object Mode
* Navigate to **[Data](https://docs.blender.org/manual/en/latest/editors/properties_editor.html#object-data)** tab in **Properties** window and locate **[Vertex Group](https://docs.blender.org/manual/en/latest/modeling/meshes/properties/vertex_groups/introduction.html)** section
* Add appropriate **Vertex Groups** for your helmet via **+ button (1)**
  + Once vertex group is added, you can change its name by double clicking on it with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")

Now, depending whether you want to simply assign everything to one bone (for instance when you are doing a hat or you don't want to skin chin strap to chin yet), you can either chose one of the options:

##### Simple skinning

As title suggest, this skinning is very simple - whole mesh will be basically skinned to single bone - **Head. To do**

* Add just one **Head** vertex group via **+ button (1)** located at the right side

* Switch to **Edit Mode** while having Object\_LOD0 selected and select all faces with i.e. `Ctrl` + `A` shortcut
* Click on Head vertex group and then click on **Assign** button

✩

**Tip**: Alternatively, if you are using model from A3, you can also try to renaming **Vertex Groups** so they match **Arma Reforger** bones

##### Weight Painting

Using weight paint allows you to skin influences between maximum 4 bones. In order to achieve that, perform following actions:

* Switch to **[Weight Paint](https://docs.blender.org/manual/en/latest/sculpt_paint/weight_paint/index.html) mode**
* Create vertex groups for things you want to skin - in case of helmet with chin strap, **FacialJaw** & **Head** bone would be sufficient
* Select one of the vertex groups and starts painting weights with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")

* [![armareforger-new-headgear-weight-painting-head.jpg](/wikidata/images/thumb/6/6b/armareforger-new-headgear-weight-painting-head.jpg/479px-armareforger-new-headgear-weight-painting-head.jpg)](/wiki/File:armareforger-new-headgear-weight-painting-head.jpg)
* [![armareforger-new-headgear-weight-painting-facialJaw.jpg](/wikidata/images/thumb/7/7d/armareforger-new-headgear-weight-painting-facialJaw.jpg/479px-armareforger-new-headgear-weight-painting-facialJaw.jpg)](/wiki/File:armareforger-new-headgear-weight-painting-facialJaw.jpg)
* [![armareforger-new-headgear-weight-painting-neck.jpg](/wikidata/images/thumb/d/d6/armareforger-new-headgear-weight-painting-neck.jpg/479px-armareforger-new-headgear-weight-painting-neck.jpg)](/wiki/File:armareforger-new-headgear-weight-painting-neck.jpg)

ⓘ

Vanilla helmets are right now only using one bone - **Head** - for whole model (including chin strap) although that might change in the future.

#### Armature modifier

Once asset is skinned with one of the above methods, there is one more thing to do in order to link vertex groups with skeleton and thus achieve animated asset:

* In **Modifiers** tab, add new [**Armature** modifier](https://docs.blender.org/manual/en/latest/modeling/modifiers/deform/armature.html) via **Add Modifier (1)** button
  + In **Object** property, select **Armature** as object to deform with

[![armareforger-new-headgear-aramture-modifier.png](/wikidata/images/8/8f/armareforger-new-headgear-aramture-modifier.png)](/wiki/File:armareforger-new-headgear-aramture-modifier.png)

After that, you could quickly verify if asset is skinned by selecting **Armature** and switching to **Pose Mode.** In this mode you can try to move bones and see if mesh is following them.

## Colliders

Helmets in **Arma Reforger** are using actual colliders for protection of the character so its quite important to set it correctly.

### Creating colliders

First step towards creating proper collider will be making of a new object, which should receive one of the [colliders prefixes](/wiki/Arma_Reforger:FBX_Import#Collider_shape "Arma Reforger:FBX Import") (in this case **UTM\_** prefix was used). For instance, sometimes it is feasible to create nice collider out of regular mesh by removing all unnecessary elements and then [decimating it](https://docs.blender.org/manual/en/latest/modeling/modifiers/generate/decimate.html) below ~150 faces. Other option is to create mesh from scratch which shouldn't be that big task in case of helmet but it might be bit more time consuming so choice is up to creator.

[![armareforger-new-headgear-helmet-firegeo.gif](/wikidata/images/b/bf/armareforger-new-headgear-helmet-firegeo.gif)](/wiki/File:armareforger-new-headgear-helmet-firegeo.gif)

In both cases, here are few things to consider:

* Shape should **fit the helmet**
* Should be **fairly simple**
  + It is not recommended to use exact thickness of the helmet - instead it is enough to use material with predefined thickness
* [UTM (Trimeshes) colliders](/wiki/Arma_Reforger:FBX_Import#Collider_shape "Arma Reforger:FBX Import") can be used

Once collider is done, it will be necessary to connect it to one of the existing colliders. Due to nature of colliders, they **don't support skinning** and they have to be parented to bone via **[Relations](https://docs.blender.org/manual/en/latest/scene_layout/object/properties/relations.html#relations)**

#### Setting relations: Using Make Parent

Any object can be quite easily parented via **[Make Parent](https://docs.blender.org/manual/en/latest/scene_layout/object/editing/parent.html#bone-parent)** action. To do so, perform following steps:

* Select **collider object**
* Select **Armature** as second object with Left `⇧ Shift` + ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")combo
* Switch to **Pose Mode** with both objects selected
* Select **Head** bone
  + Tip: you can turn on bone names for easier navigation!
* Open [Menu Search](https://docs.blender.org/manual/en/latest/interface/controls/templates/operator_search.html#menu-search) ( `↹ Tab` using [Industry Compatible Keymap](https://docs.blender.org/manual/en/latest/interface/keymap/industry_compatible.html#general) )
  + Search for **Make Parent** &  select it
  + Select **Bone** in **Set Parent To** menu
  + In **Make Parent** menu in bottom left corner select **Keep Transform** option

[![armareforger-new-headgear-relation-action.gif](/wikidata/images/a/a5/armareforger-new-headgear-relation-action.gif)](/wiki/File:armareforger-new-headgear-relation-action.gif)

#### Setting relations: Using Relations tab

* Create a new object and in **Relations** tab set relation to **Head**
  + Reposition collider to correct location
* If collider is suddenly very small, make sure that scale of the armature is correct

[![armareforger-new-headgear-relation-manual.gif](/wikidata/images/5/57/armareforger-new-headgear-relation-manual.gif)](/wiki/File:armareforger-new-headgear-relation-manual.gif)

### Creating material

##### Material Research

Before creation of material, it is good to perform some research about ballistic properties and material used on the item that you are making. Game wise, there are two properties that matter:

* **Density**
* **Thickness**

In case of Sample Helmet, which is ported **A3 Avenger** helmet, **ZSh-1-2M** was used a reference which is known outer shell is made of alloy and inner is made out of [Aramid](https://en.wikipedia.org/wiki/Aramid). Since you can't effectively make game material consisting out of multiple materials, aramid based material - [hard\_aramid\_7.3mm.gamemat](enfusion://ResourceManager/~ArmaReforger:Common/Materials/Game/PersonalProtection/hard_aramid_7.3mm.gamemat) - was used as base.

[hard\_aramid\_7.3mm.gamemat](enfusion://ResourceManager/~ArmaReforger:Common/Materials/Game/PersonalProtection/hard_aramid_7.3mm.gamemat) material is used on [PASGT helmet](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/HeadGear/Helmet_PASGT/Helmet_PASGT_01.xob), which is rated at [Threat Level IIIA in NIJ classifications](https://en.wikipedia.org/wiki/List_of_body_armor_performance_standards#NIJ_armor_standard_(United_States)) . In most case, density of the material can be found by searching for "name\_of\_material density". In case of PASGT, it is known that helmet is using Kevlar 29 aramid material. By looking for this material, you can find i.e. such result - [Kevlar\_Technical\_Guide\_0319.pdf](https://www.dupont.com/content/dam/dupont/amer/us/en/safety/public/documents/en/Kevlar_Technical_Guide_0319.pdf) . Over there, it is mentioned that **density of Kevlar 29 material is 1.44 g/cm3**. In case of game, **[different source was used](https://pdf.sciencedirectassets.com/278653/1-s2.0-S1877705817X00040/1-s2.0-S1877705816343983/main.pdf?X-Amz-Security-Token=IQoJb3JpZ2luX2VjEE8aCXVzLWVhc3QtMSJGMEQCIEK4%2BCKLbo1EDDqEEjWytZlEqfMDUb49ZLQBBknp6esuAiAUsikSnabBvPbMFLSu%2FKYH4kdOfUVpJjzADFO4nKI%2BOiq8BQjo%2F%2F%2F%2F%2F%2F%2F%2F%2F%2F8BEAUaDDA1OTAwMzU0Njg2NSIMQaDz%2FzePWMYW%2FBNnKpAF%2BIEojMbengW8pT0m27oIVnH%2BHYNC9VUcw3adwgMBrDykn2MO8ERQlGICQQwBzQtgMI6Mxorpaz%2FENY2lhJSxADTtB6g0800RIk4pkHDIhdvU93zl0ACcq%2BEuQtHlaIVaGQN%2Fj%2BGCgkzwnSHH3qB3lRbPSF7og2knjm15aiuLkpTzKvbnmDZLodCcj%2FUGn8VOfZXO7GgKllXTgdR80%2FeLrITBpde7BhYaMN8tZrIvc95gH9lOGyDHpFQtwmXopArA8S%2F8L28JPBdL6mMUShHW013m26gPskv3QNabek%2BHQfLw3TdSmd9p%2BCD7SVr2Qsb1GrkRNxRcXAKIgIPEh7ooTWBDED6cKQQ7%2FaajEyjCaUDheby35cKqJSRvD9aARAoi%2BR1AF91%2Fyv%2B59lfOzillgKrBYPa4M7lH4FCPNvWUrMccJ3pzbppt0VY0QV%2FjVCUDXlwbSays1Oa4YezDC90L0hLCpgVMQnBFuiknfvTbQ4XyfvEwsw7IBk4EjPkMieYno1BkadvJBvUXk9dxRrK41nFUKLykmH5fYvLqpv4FFRo35yQEUJd2mMX1ta0qJ9SN5X4ZWC1HJJaxbUG5BBS1Gjz%2FqQQxdP4CBFn3gIKlAibSlysLps76Ddd3gOVvE2EeU2iapLIRo%2FG5%2BHPQpRwDeNojBjBI%2FDaVdquZe0s%2FB1m5uRBiFMnaFYZnNdPi5QjgIkuel%2BtIrr4RLcRMUfR%2FXQTGvsSYQ6Fp5YeVnvUsMWpIzHLdFbPI6yaKtmy3w4a8F31yNvSXU%2FpGdvp8tt8jb6uRjALI2kJh%2F%2F3AunSP6idB%2F1G0tYdWQ8XK5s%2FeR4LGOfe1zMA7qbdwkCn9OHpdurbMTDF%2Bq2u6om%2B%2BhxQPVUgwufSnpgY6sgEe5Xoavw4%2F3GOzFY%2FWVffNd%2FX9SbLEygEjguZlEES0YCI1bn6QHk0LZTYRZ7ePbf8eSQ0PzfO8%2BAp2UWjCgvfyHt6NjApqqMBMfNY5A0IsIRAE0U6mFuq%2FyowIfVJHPOhBeVVWWDGI4glAroYoUFuWl66asSHkV7Hdks3g44y8rH%2BxVruC3cNcxnnqckim8%2F4FLwnLwDi2c5UHLOF%2FkpuokmNAs1B%2B2bGGlA7XNNCBnnKm&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Date=20230802T074756Z&X-Amz-SignedHeaders=host&X-Amz-Expires=300&X-Amz-Credential=ASIAQ3PHCVTYU63EHHHC%2F20230802%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Signature=002c92a222d8018889e0a0f65d2e644a9deb948c4b9fb6817f132b8ad642bdf5&hash=51898f147784aff3e83fffa2232d329caf721a81e2c9c8cb6e26501d8a0c4ed6&host=68042c943591013ac2b2430a89b270f6af2c76d8dfd086a07176afe7c76c2c61&pii=S1877705816343983&tid=spdf-fd1ca5b5-9c6d-4991-b5a4-2520f83bd69b&sid=b55cdac99f2b734ada8b5f64ba3c51cf186dgxrqb&type=client&tsoh=d3d3LnNjaWVuY2VkaXJlY3QuY29t&ua=080f5806535305555551&rr=7f04b73209b10a55&cc=nl), which used 1.65 g/cm3** as value.

⚠

Right now **density doesn't influence** penetration behavior per se. Therefor 32mm of Hard Aramid game material will behave exactly the same as 10mm Armor game material.

Quite often you might also find out that thickness of the helmet is not listed anywhere so you have to go bit creative there.

In this case, it can be done in two way - either by changing **thickness** value or by changing **Kinetic Resistance** of the material.

For sample helmet, thickness was increased and set to **10mm**.

* Fill in those values to that new material and then adjust **Kinetic Resistance** so it matches desired protection level
  + You can use [following spreadsheet](https://github.com/BohemiaInteractive/Arma-Reforger-Misc/blob/main/Configuration/Weapons/Weapons_Penetration_List.xlsx) for those calculations

[![](/wikidata/images/7/73/armareforger-new-headgear-penetration-table.png)](/wiki/File:armareforger-new-headgear-penetration-table.png)

**Material Data** section of **Weapon Penetration List** spreadsheet

##### Game Material creation

Once you have data, you can commence with creation of the material by performing following steps:

* Create new inherited material - usage of one of the existing materials is recommended or duplicate some existing material which inherits from one of the core materials. In case of Sample Helmet [hard\_aramid\_7.3mm.gamemat](enfusion://ResourceManager/~ArmaReforger:Common/Materials/Game/PersonalProtection/hard_aramid_7.3mm.gamemat) was duplicated
  + Name that new material in a way, which represents its thickness (*i.e. hard\_aramid\_10mm.gamemat )*
  + This way, you will inherit for instance correct particle effects, sounds and decals
  + In vanilla game those visual & sound effects are still being worked on (0.9.9)
* Change **Density, Thickness & Kinetic Resistance**

### Materials & Layer Preset

Next step will be assigning of material and layer preset to the collider itself. You can do it via [Enfusion Blender Tools Object Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools "Arma Reforger:Enfusion Blender Tools: Objects Tools") or [manually](/wiki/Arma_Reforger:FBX_Import "Arma Reforger:FBX Import") (in case you are using different 3D software). Once you know how to change material and layer preset, do following things:

* Set collider [Layer Preset](/wiki/Arma_Reforger:Collision_Layer "Arma Reforger:Collision Layer") to **FireGeo**
* Assign game material - either one of the existing vanilla ones or the one you have created in previous step

## Creating Item variant

One of the final steps in equipment preparation is creating a special **item variant** of the model. Such model is visible when i.e. given piece of equipment is placed on the ground. This can be done in few steps listed below:

* Make copy of your helmet (FBX/Blend) and open it
* Remove skeleton and all empty objects
* Modify model of helmet so it appears like lying on flat surface
  + For instance, straps can be deformed so they are resting on surface
* No skeleton is needed
* Collider can be even simpler shape, but it is also possible to reuse previously created one
  + **[Layer Preset](/wiki/Arma_Reforger:Collision_Layer "Arma Reforger:Collision Layer")** is changed from *FireGeo* to **ItemFireView**
  + **UCX** or simple shapes are recommended
    - Mesh -> Convex Hull action can be used to create some convex mesh

* [![Visual mesh - notice how chin strap was placed](/wikidata/images/thumb/8/89/armareforger-new-headgear-item-mesh.png/303px-armareforger-new-headgear-item-mesh.png)](/wiki/File:armareforger-new-headgear-item-mesh.png "Visual mesh - notice how chin strap was placed")

  Visual mesh - notice how chin strap was placed
* [![Firegeometry](/wikidata/images/thumb/b/b1/armareforger-new-headgear-item-firegeo.png/303px-armareforger-new-headgear-item-firegeo.png)](/wiki/File:armareforger-new-headgear-item-firegeo.png "Firegeometry")

  Firegeometry

## Importing model

One of the last steps in asset preparation is importing of model. Detailed procedure for importing mesh into Workbench can be found on [FBX Import](/wiki/Arma_Reforger:FBX_Import#Import_process_in_the_Workbench "Arma Reforger:FBX Import") page and - if you want some practical example - also in [Weapon Creation tutorial](/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Model_Import_.26_Registration "Arma Reforger:Weapon Creation/Asset Preparation").

In any case, here is quick instruction how to do it - select both of the FBX files in Workbench and then click with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") on one of them. From the context menu, select **Register & Import** option and then select **as Model** option when asked how do you want to process given file.

* Make sure that **Export skinning** option is checked and reimport the model via **Reimport resource** button
* Make sure that you have **230 bones in total** and there is at least one skinned bone - *i.e. 1 + 229*
  + In **Details** tab, you can find current amount of bones and dummies in **Bones** section. First number represents amount of bones and second, number of dummies. By default, bones which are not affecting any mesh (there is no Vertex Group representing it)

[![armareforger-new-headgear-imported-mesh.png](/wikidata/images/6/66/armareforger-new-headgear-imported-mesh.png)](/wiki/File:armareforger-new-headgear-imported-mesh.png)

## Material preparation

[![](/wikidata/images/thumb/0/05/armareforger-new-headgear-material-setup.png/395px-armareforger-new-headgear-material-setup.png)](/wiki/File:armareforger-new-headgear-material-setup.png)

Sample Helmet material example

If importing of model went fine, new **Enfusion Materials** (*emat*) should be created in **Data** folder next to the model. Those new materials are called based on **material** name in 3D software and by default they are using **MatPBRMaterial** shader. Assuming you have already created PBR materials using for instance **Substance Painter** and [BCR+NMO preset shared on Github](https://github.com/BohemiaInteractive/Arma-Reforger-Misc/tree/main/Art/Substance%20Export%20Profiles) and then imported them in game, you can assign those [textures](/wiki/Arma_Reforger:Textures "Arma Reforger:Textures") to **BCR & NMO Maps** fields in the material itself.

Once basic textures are assigned and visible in Resource Manager viewport, there is still one more thing to do, which will **hide headgear in first person view** . To do so, perform following steps

* Assign [HeadClipping\_A.edds](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Basebody/Data/HeadClipping_A.edds) texture to **Opacity Map**
* **Enable** both **Allow User Alpha Bias** & **Disable User Apha in Shadow** properties

With those parameters set in material, helmet should now correctly dissolve when you switch from 3rd person to 1st person view.

ⓘ

**Next part** - [Prefab Configuration](/wiki/Arma_Reforger:Character_Gear_Creation/Headgear/Prefab_Configuration "Arma Reforger:Character Gear Creation/Headgear/Prefab Configuration")
