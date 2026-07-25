# [Prop Creation](https://community.bistudio.com/wiki/Arma_Reforger:Prop_Creation)

## Goals of this tutorial

In this tutorial you will learn about:

* How to prepare prop in FBX format for Enfusion
* How to import FBX into Workbench
* How to configure new prop in Workbench
* How to add procedural animation with sound & custom action to prop prefab

ⓘ

If you **don't have any experience with Workbench** yet, it is recommended to **go through [modded weapon tutorial](/wiki/Arma_Reforger:Weapon_Modding "Arma Reforger:Weapon Modding")** to familiarize with some of the concepts present in the **Workbench**

📥

Sources files for this tutorial can be found on
[**Arma Reforger Samples Github repository**](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/tree/main/SampleMod_NewProp)

## Preparation

💬

**Overview**

Preparation phase consist of things like:

* Preparing basic structure
* Preparing mesh
* Exporting mesh

ⓘ

It is recommend to read before [FBX Import](/wiki/Arma_Reforger:FBX_Import "Arma Reforger:FBX Import") & [Textures](/wiki/Arma_Reforger:Textures "Arma Reforger:Textures") pages before proceeding.

## Preparing structure

While keeping to official structure is not mandatory and there are no engine restrictions asset wise about it, it's recommended to follow guidelines listed here - [Data (file) structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure") - to ensure that all automation plugins are parsing your assets correctly and make it later easy to navigate.

Therefore, your first task will be preparing following file structure

[![armareforger-new-prop-file-structure.png](/wikidata/images/thumb/1/11/armareforger-new-prop-file-structure.png/800px-armareforger-new-prop-file-structure.png)](/wiki/File:armareforger-new-prop-file-structure.png)

### Cutting object to parts

Portable desk source model contained all optional elements, like desk wing and lids, in single model. In Arma 3, subvariants of such assets would be created by **hiding each element via model config animations**. Something like that is **no longer necessary nor recommended** (*hiding of mesh has some performance drawback*) - instead, additional parts of the model can be either added by **SlotManagerComponent** or through **Hierarchy**.

[![armareforger-new-prop-object-builder-parts.gif](/wikidata/images/9/9b/armareforger-new-prop-object-builder-parts.gif)](/wiki/File:armareforger-new-prop-object-builder-parts.gif)

In case of those asset, **one wing** *(PortableDesk\_01\_wing.fbx)* and **one lid** *(PortableDesk\_01\_lid.fbx)* should be moved to separate FBX files and then all of them should be removed from the main mesh (*PortableDesk\_01.fbx)*

[![armareforger-new-prop-split-object.png](/wikidata/images/8/8d/armareforger-new-prop-split-object.png)](/wiki/File:armareforger-new-prop-split-object.png)

### Adding sockets

[![armareforger-new-weapon-empty-create.png](/wikidata/images/2/2f/armareforger-new-weapon-empty-create.png)](/wiki/File:armareforger-new-weapon-empty-create.png)

Sockets are empty objects *(in Blender nomenclature)* which can be used to attach elements (prefabs) or actions to it. In Blender, such sockets can be added via **Add→ Empty→ Plain Axis** option.

It is quite important to properly orientate those empty objects since their position & rotation will be used in various systems, like in **SlotManagerComponent** for attaching various parts of rugged desk (*this is covered in detail in later part of this tutorial*).

At this stage, main, **PortableDesk\_01.fbx**, file should have following sockets

* **socket\_wing\_01** & **socket\_wing\_02** - for both wing models
* **socket\_lid\_01** & **socket\_lid\_02 -** for both lids

[![armareforger-new-prop-sockets-model.png](/wikidata/images/8/85/armareforger-new-prop-sockets-model.png)](/wiki/File:armareforger-new-prop-sockets-model.png)

Then, **PortableDesk\_01\_lid.fbx** & **PortableDesk\_01\_wing.fbx** should have corresponding snappoints - i.e. **snap\_wing** and **snap\_lid**.

Those points would be used later to **connect** (aka snap) **both models together** so their **position and orientation is extremely important**!

[![armareforger-new-prop-snap-points.png](/wikidata/images/a/a8/armareforger-new-prop-snap-points.png)](/wiki/File:armareforger-new-prop-snap-points.png)

### Setting skeleton & rigging mesh

This asset has six drawers which can be animated. In order to make those drawers animated, it is necessary to add proper skeleton to the model and then skin those movable selections to newly created bones.

[![armareforger-new-prop-blender-add-bone.png](/wikidata/images/9/95/armareforger-new-prop-blender-add-bone.png)](/wiki/File:armareforger-new-prop-blender-add-bone.png)

In Blender, new armature can be easily created by adding new **Single Bone** in *Add → Armature* menu when in **Object Mode**. This new bone can be called **root** and it will be serving as main bone of this skeleton.

Similar to sockets, orientation of bones is quite important for animation. Unlike in RV engine, animations are **no longer using two vertices to create axis of transformation** but instead **rotation & position of bone** is used to determine origin of animation.

Once root **bone** was created, it is possible to add bone for each of the drawers like on picture below and then proceed with adding **vertex groups**, **Armature modifier** and general skinning.

✩

**Tip**: If you are not familiar with skinning assets in Blender, it is recommend to read or watch some Youtube tutorials. On [Weapon Creation tutorial](/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Mesh_Skinning "Arma Reforger:Weapon Creation/Asset Preparation") there is also segment explaining quite well how skinning can be done.

[![armareforger-new-prop-blender-full-skeleton.png](/wikidata/images/f/fd/armareforger-new-prop-blender-full-skeleton.png)](/wiki/File:armareforger-new-prop-blender-full-skeleton.png)

### Colliders & material names

[![armareforger-new-prop-blender-scene-colection.png](/wikidata/images/a/a6/armareforger-new-prop-blender-scene-colection.png)](/wiki/File:armareforger-new-prop-blender-scene-colection.png)

Colliders are special type of objects which are used to calculate various kinds of collisions - be it physic simulation or tracing of bullet penetration. There are few rules regarding those colliders and most of them listed on [**collider related segment of FBX Import page**](/wiki/Arma_Reforger:FBX_Import#Collider_usage "Arma Reforger:FBX Import").

When importing asset from previous Arma game - like in this example - you are most likely going to have already convex components ready from **Geometry**, **Fire Geometry**, **View Geometry** or **Geometry Physx** LODs. If you are using [P3D importer](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_P3D_Conversion "Arma Reforger:Enfusion Blender Tools: P3D Conversion") which is part of [Enfusion Blender Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools "Arma Reforger:Enfusion Blender Tools"), then those components will be automatically converted to trimesh colliders ( *UTM\_ prefix*). Since trimesh colliders are not recommended for general collision, it might be wise to verify if object can be converted to trimesh.

If you are using Blender, there is small handy tool to assist you with assigning correct game materials & layer presets on colliders - more details about can be found on [Arma\_Reforger:Enfusion\_Blender\_Tools:\_Objects\_Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools "Arma Reforger:Enfusion Blender Tools: Objects Tools").

In this sample, rugged desk uses simple box mesh, so usage of **UBX\_ prefix is recommended**.

Next thing when it comes to configuration of colliders is **configuration of Layer Preset**. This is done through **usage** custom property (*details about adding that property can be found on following [**FBX Import page**](/wiki/Arma_Reforger:FBX_Import#Setting_Layer_Preset_on_colliders_.28usage_parameter.29 "Arma Reforger:FBX Import")). For simple prop like this one, layer preset should be set to **PropFireView**. If you have multiple colliders, you have to add that property to each of them!*

After layer preset is set, it is time for setting [**Game Material**](/wiki/Arma_Reforger:FBX_Import#Physical_material_.28Game_material.29 "Arma Reforger:FBX Import"). This is done by assigning to collider a material with specific name and again, details of naming are described on FBX Import page. For this example prop, **Plastic\_6mm** game materials is used, since it is assumed that single wall is 3mm thick and due to geometry is a simple box, those walls are counted twice.

Below is screen showing [**Game Materials** & **Layer Presets** selection menu](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools#Colliders_.26_Layer_Presets_setup "Arma Reforger:Enfusion Blender Tools: Objects Tools") from **Enfusion Blender Tools**. Using this simple tool it is possible to easily select desired game material and layer preset, which will be automatically. Please note that before using that feature it is necessary to [sort objects](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools#Automatic_sorting_of_objects "Arma Reforger:Enfusion Blender Tools: Objects Tools") and [setup plugin](/wiki/Arma_Reforger:Enfusion_Blender_Tools#Installation "Arma Reforger:Enfusion Blender Tools") correctly.

[![armareforger-new-prop-blender-game-materials.png](/wikidata/images/a/aa/armareforger-new-prop-blender-game-materials.png)](/wiki/File:armareforger-new-prop-blender-game-materials.png)

## FBX export settings

Most of the general rules can be found on [**FBX Import page**](/wiki/Arma_Reforger:FBX_Import#FBX_Export "Arma Reforger:FBX Import") . In principle, when exporting from i.e. 3DS Max, you have to make sure that you are exporting in **binary format in version 2014/2015**. Furthermore, **Triangulation** & **Preserve Edge Orientation should be turned off.**

**Blender** wise, there are 3 most important things to keep in mind when exporting FBX

|  |  |
| --- | --- |
| **1. Object Types**: For animated object like this prop, you need to have **checked on** at least:  **Empty -** handles all snap points  **Armature** **-** exports skeleton of your weapon  **Mesh -** self explanatory | [armareforger-new-weapon-export-blender2.png](/wiki/File:armareforger-new-weapon-export-blender2.png) |
| **2. Custom Properties**: Without this option all custom properties like **LayerPresets** would be lost! |
| **3. Leaf bones**: Leaf bones are completely unnecessary in Enfusion and it's better to have that option **turned off** |

## Preparing textures

When importing textures of weapon from previous Real Virtuality games like Arma 3 there is no real automated or simple method of conversion spec-gloss textures to PBR (*Physicial Based Rendering*) Metal Rough ones - current industry standard. Therefore in most cases it's much easier to do textures from scratch in i.e. Substance Painter.

There are tons of materials on the internet how to create proper PBR texture and it's highly recommend to search for it via some popular search engines. More information about maps and textures used by Reforger can be found on [Arma\_Reforger:Textures](/wiki/Arma_Reforger:Textures "Arma Reforger:Textures") page

For this asset, assuming that Substance Painter is used, it is recommended to use one of the shared export profiles - Arma Reforger - BCR + NMO, which should create textures compatible with Reforger.

## Importing & registering new model

Importing and registering of new model is quite simple and this involves following steps:

* Locating FBX file that you wish to convert in **Resource Browser**
* Clicking on that FBX with RMB and then selecting "**Register and Import**" option from the context menu
* From pop up menu with title **Register Resource (File(s)**, select  "**as Model**" option

After that, conversion of FBX into XOB will begin and depending on complexity of the model it might take some time. In case of that simple prop.

Once model is imported, a **new XOB** should appear in **Resource Browser -** this file can be now opened in new **Resource Manager** tab for **preview** and **game related parameters tweaking** by double clicking with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") on it.

[![armareforger-new-prop-register-fbx.gif](/wikidata/images/0/05/armareforger-new-prop-register-fbx.gif)](/wiki/File:armareforger-new-prop-register-fbx.gif)

⚠

While Workbench still supports relative paths for materials, this method is discouraged and should not be used.

⚠

Both Layer Preset and Game Material can be changed in Import Settings window, although it is recommended to have that stored in model itself. Otherwise you might loose your data if colliders are changed (by f.e. renaming of objects).

### Import settings

If everything went fine, main **Resource Manager** window should look like that:

[![armareforger-new-prop-imported-model-xob.png](/wikidata/images/f/f3/armareforger-new-prop-imported-model-xob.png)](/wiki/File:armareforger-new-prop-imported-model-xob.png)

On the right side of the viewport, there is a panel where you can select between two tabs:

* [**Details**](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Details_Tab "Arma Reforger:Resource Manager: Model Editor") - shows various statistics (i.e. vertex, face or face count) and information about  materials, colliders, skeleton or LODs present in the imported model. This tab is by **default selected** when opening new model.
* [**Import Settings**](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Import_Settings_Tab "Arma Reforger:Resource Manager: Model Editor") - in this tab it is possible to modify how FBX is imported to the game and it is possible to modify **transformation, materials, physics or other misc parameters** here.

After briefly checking if model has correct number of [LOD (*levels of details*)](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#LODs "Arma Reforger:Resource Manager: Model Editor") and it looks visually fine, it's time to move to **Import Settings** tab, where there are couple important things to set.

### Physics setup

After initial import was done it's time to make sure that materials & colliders are using proper materials & colliders.  If it fails to find such texture, new dummy material (see area marked in orange on screen below) will be created in **data** folder next to the FBX model.

If everything was setup correctly in Blender, imported model should already have one collider imported with correctly assigned **Layer Preset** *(PropFireView)* and **Surface Properties** (*plastic\_3mm.gamemat*). Otherwise, here is the list of some common issues:

#### Geometry Params list is empty

If both **Colliders** array in **Details** and **Geometry Params** array in **Import Settings** are empty, check for following errors:

1. Check if objects which are supposed to be colliders have **correct prefix** - i.e. UBX, UCX, etc. See [FBX Import](/wiki/Arma_Reforger:FBX_Import#Collider_shape "Arma Reforger:FBX Import") page for more information
2. Check **Console Log** if there are no errors regarding convexity or shape of the collider. See [FBX Import](/wiki/Arma_Reforger:FBX_Import#Collider_shape "Arma Reforger:FBX Import") page for more information

#### Wrong Game Material

If impact effect on the asset are using default dirt impact effects or the following error appears in console

```
RESOURCES : GetResourceObject @"{411A617924FC3D61}Assets/Props/Military/Camps/PortableDesk_01/PortableDesk_01.xob"
RESOURCES (E): Wrong&patched resource GUID in @"Assets/Props/Military/Camps/PortableDesk_01/PortableDesk_01.xob" for property UBX_Component01 resource name @"Plastic_6m.gamemat"
RESOURCES : GetResourceObject @"{D4D49F200A5937E1}Plastic_6m.gamemat"
RESOURCES (E): Failed to open
```

then check for following errors:

1. Make sure in 3D software of your choice that material assigned to collider is correctly named, i.e. *Plastic\_6mm\_6A7CBD6BBCFC5148 - see [Game Material section of FBX Import page](/wiki/Arma_Reforger:FBX_Import#Physical_material_.28Game_material.29 "Arma Reforger:FBX Import").*

⚠

Keep in mind that **changes to materials or layer presets in FBX** are only imported if **Geometry Params list is empty**. See **[FBX Import - Game Materials](/wiki/Arma_Reforger:FBX_Import#Physical_material_.28Game_material.29 "Arma Reforger:FBX Import")** page for more information

#### Wrong Layer Preset

If you see warning triangle **Colliders** section of **Details** tab or *All interaction layers used* error, then check for following things:

1. Make sure that collider has correct Layer Preset assigned
2. Make sure that **Custom Properties** option is checked in FBX export settings

[![armareforger-new-weapon-collider-errors.png](/wikidata/images/b/b8/armareforger-new-weapon-collider-errors.png)](/wiki/File:armareforger-new-weapon-collider-errors.png)

#### Changing parameters

If for some reason you don't want to touch FBX and want to change Layer Preset or assigned game materials in Workbench itself, then it is possible to do that by manipulating **Layer Preset & Surface Properties** parameters in **Physics** section of **Import Settings** tab. Changes to those parameters require usage of **Reimport resource** button - otherwise information stored in XOB would not be updated. This method is **not recommended though.**

[![armareforger-new-prop-import-change-params.gif](/wikidata/images/4/4d/armareforger-new-prop-import-change-params.gif)](/wiki/File:armareforger-new-prop-import-change-params.gif)

### Skeleton & Hierarchy

In case of models with skeleton (*Armature)* it is quite important to check **Export** **Skinning** option in **Miscellaneous** section of **Import Settings.**

Since Rugged Desk is supposed to have six animated drawers, it is important to **enable** that option and then **reimport** the model via **Reimport resource (PC)** button.

[![armareforger-new-prop-export-skinning.png](/wikidata/images/e/e1/armareforger-new-prop-export-skinning.png)](/wiki/File:armareforger-new-prop-export-skinning.png)

If skeleton was imported correctly'***,** Details tab'* in right section of the Resource Manger view port should show some **non zero number** in [**Bones section**](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Bones "Arma Reforger:Resource Manager: Model Editor"). In case of **Rugged Desk**, this object should have **6** **skinned bones, 4 sockets** and **one dummy hierarchy element (root), which should be visible as Bones (6 + 5)**. First number in the bracket indicates number of skinned bones and 2nd one indicates of empty bones (*either bones which are not skinned to anything or empty objects*)

[![armareforger-new-prop-skeleton-full-imported.png](/wikidata/images/0/09/armareforger-new-prop-skeleton-full-imported.png)](/wiki/File:armareforger-new-prop-skeleton-full-imported.png)

For Rugged Desk **wing** & **lid** it is possible to just use **Export Scene Hierarchy** - this should allow to use empty objects created before for snapping purposes.

## Importing textures

By default, **Workbench** importer will create **new materials** (emats) using **MatPBRBasic** shader in **Data** folder next to the imported model, based on material names located in FBX. If material suitable for the mesh already exist somewhere in available projects, it is possible to change linked material by changing elements in [**Material Assigns**](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Material_Assigns "Arma Reforger:Resource Manager: Model Editor") located in **Visual** section of [**Import Settings -**](/wiki/Arma_Reforger:Resource_Manager:_Model_Editor#Import_Settings_Tab "Arma Reforger:Resource Manager: Model Editor") this option will be later used for **Portable Desk** lid & wing, which share same materials.

Once those materials are ready, you can use same procedures as the ones described on Sample Modded Weapon - Documentation - Creating New Materials page.

## Adding basic configuration

💬

**Overview**

Following topics will be covered in this paragraph

* Creating prefab
* Basic asset configuration

## Creating prefab

Prefabs for props can be prepared in multiple ways and it also depends on amount of functions that are planned to be added to such asset.

In general, it is recommend to use one of those two prefabs as a base:

* [**Props\_Base.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Props_Base.et%7C) - prefab for simple, **non destructible** assets
* [**Destructible\_Props\_Base.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Destructible_Props_Base.et%7C) - prefab dedicated for assets which **can be destroyed**

⚠

If you don't use one of the mentioned prefabs as a base, then you will have to go through more steps like setting collision. In some cases base prefabs might be missing **RigidBody** component and in such scenario, it will be necessary to add such component and enable **Model Geometry** option inside of it.

Since this sample is planned to have destruction, [**Destructible\_Props\_Base.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Destructible_Props_Base.et%7C) prefab can be used as a base.

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

In general, it is possible to create new inherited prefab in two ways:

* [By dropping parent prefab in World Editor and drag and dropping that entity into Resource Browser field](/wiki/Arma_Reforger:Prefabs_Basics#Drag_and_drop_method "Arma Reforger:Prefabs Basics")
* By using **[Inherit in Addon](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics")** action on prefab selected in **Resource Browser**

No matter what method was picked, it is recommended to create base prefab for props, which might have some variants. In this tutorial **Portable Desk** have 3 texture variants prepared, so such base class will be more than welcome.

In this case, new prefab, which inherits from [**Destructible\_Props\_Base.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Destructible_Props_Base.et%7C) , should be called **PortableDesk\_01\_Base.et**

Final step in initial configuration  will be assigning previously imported mesh. This can be done by simply assigning **Rugged Desk XOB** into **Object** property of **MeshObject** component in **PortableDesk\_01\_Base.et** prefab.

[![armareforger-new-prop-desk-prefab.png](/wikidata/images/f/f9/armareforger-new-prop-desk-prefab.png)](/wiki/File:armareforger-new-prop-desk-prefab.png)

If change was applied to **MeshObject** component of **prefab instance**, don't forget to press **Apply to prefab** button and then save currently loaded world!

**Creating texture variants**

[![armareforger-new-prop-prefab-material-assign.png](/wikidata/images/5/5a/armareforger-new-prop-prefab-material-assign.png)](/wiki/File:armareforger-new-prop-prefab-material-assign.png)

Since this assets is supposed to have three color variants, new prefab variants have to be prepared

1. Create new prefabs, inheriting from **PortableDesk\_01\_Base.et**,  either via **World Editor** viewport **drag and drop** method or via **Inherit Prefab** action available in **Resource Browser** attached to **World Editor**
2. Once new prefabs are created, open them and one by one, change assigned materials in **Materials** array inside **MeshObject** component to appropriate material created before.
3. **Apply changes to the prefabs** if changes were applied to entity instance, otherwise (*if hierarchy tree was used*) this step can be skipped
4. **Permanently save changes** to the prefabs by either **saving current world** or, if in **prefab edit mode**, **save the prefab** directly.

In the end, following files should be present in next to the **PortableDesk\_01\_Base.et**

* **PortableDesk\_01\_Black.et**
* **PortableDesk\_01\_Olive.et**
* **PortableDesk\_01\_Sand.et**

## Creating prefabs for smaller parts

[![armareforger-new-prop-prefab-variants-list.png](/wikidata/images/3/36/armareforger-new-prop-prefab-variants-list.png)](/wiki/File:armareforger-new-prop-prefab-variants-list.png)

Procedure described above should be also applied to **lid** and **wing prefabs** so they have base prefab (inheriting from [**Destructible\_Props\_Base.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Destructible_Props_Base.et%7C) ) and color variants. Those prefabs should be really simple at this stage so **MeshObject** and **RigidBody** components which they inheriting from [**Destructible\_Props\_Base**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Props/Core/Destructible_Props_Base.et%7C) should be enough.

In the end, following set of prefabs should be created

* **PortableDesk\_01\_wing\_base.et**
  + **PortableDesk\_01\_wing\_Black.et**
  + **PortableDesk\_01\_wing\_Olive.et**
  + **PortableDesk\_01\_wing\_Sand.et**
* **PortableDesk\_01\_lid\_base.et**
  + **PortableDesk\_01\_lid\_Black.et**
  + **PortableDesk\_01\_lid\_Olive.et**
  + **PortableDesk\_01\_lid\_Sand.et**

## Creating prop variants

In order to create a variant of Portable Rugged Desk with for instance both wings and lids attached, it is possible to utilize **SlotManagerComponent** to attach additional elements. Before going any further though, it is recommend to start with creating a new base prefab - **PortableDesk\_01\_Base\_Full.et** - which will serve as a base asset  with all additions attached. Since the core of the desk remains the same, this new base prefab should inherit from **PortableDesk\_01\_Base.et**

Once base prefab is there, follow below steps to :

* Add new **SlotManagerComponent** via **[+ Add Component](/wiki/Arma_Reforger:Prefabs_Basics#Adding_new_components_to_entity_instance "Arma Reforger:Prefabs Basics")** button
* Select **SlotManagerComponent** and add new element to **Slots** array by clicking on plus sign on the right side of that property
  + Select **EntitySlotInfo** from the list that appeared
  + Type new name of that slot. For instance slot for wing can be called **Wing\_L**
  + In **Wing\_L** element, locate **Pivot ID** property and select **socket\_wing\_01** from the list
  + In **Child Pivot ID** type name of the snap point which is located in the mesh - in this case it is called **snap\_wing**
  + In **Prefab** property, assign prefab which you want to use - in this case it will be **PortableDesk\_01\_Base\_Wing.et**

This procedure should be repeated for the rest of the slots - **Wing\_R, Lid\_1** & **Lid2**. In case the prefabs from the slots are incorrectly attached, please make sure that orientation of sockets and snap points is correct. Otherwise, the new **PortableDesk\_01\_Base\_Full.et** should look like that:

[![armareforger-new-prop-prefab-slot-manager.png](/wikidata/images/4/4c/armareforger-new-prop-prefab-slot-manager.png)](/wiki/File:armareforger-new-prop-prefab-slot-manager.png)

Beside using **SlotManagerComponent**, it is also possible to utilize hierarchy. This is especially useful when you for instance want to create a mini composition out of prefab, like add a telephone, some notebooks or ashtrays to it. After adding **Hierarchy** component to both parent and children prefab, it is even possible to utilize **Pivot ID** to snap prefabs to desired place. There is no way to define **Child Pivot ID** using that method though and thats why **SlotManagerComponent** was used here.

Principles of adding entity to prefab are described on [Prefabs Basics page](/wiki/Arma_Reforger:Prefabs_Basics#Adding_entities_to_prefab "Arma Reforger:Prefabs Basics")

[![armareforger-new-prop-prefab-add-hierarchy.gif](/wikidata/images/3/34/armareforger-new-prop-prefab-add-hierarchy.gif)](/wiki/File:armareforger-new-prop-prefab-add-hierarchy.gif)

## Adding procedural animation

💬

**Overview**

Following topics will be covered in this paragraph

* Creating basic procedural animation
* Applying animation to the asset

ⓘ

Before proceeding further it is recommend to make yourself familiar with [Procedural Animation Editor documentation](/wiki/Arma_Reforger:Procedural_Animation_Editor "Arma Reforger:Procedural Animation Editor") & [Procedural Animation Editor tutorial](/wiki/Arma_Reforger:Procedural_Animation_Editor_Basics_Tutorial "Arma Reforger:Procedural Animation Editor Basics Tutorial").

## Creating new procedural animation project

[![armareforger-new-prop-procedural-animation-new-project.png](/wikidata/images/7/7b/armareforger-new-prop-procedural-animation-new-project.png)](/wiki/File:armareforger-new-prop-procedural-animation-new-project.png)

[![armareforger-new-prop-procedural-animation-create-project.png](/wikidata/images/3/3b/armareforger-new-prop-procedural-animation-create-project.png)](/wiki/File:armareforger-new-prop-procedural-animation-create-project.png)

First step in creation of new procedural animation project would be launching of **Procedural Animation Editor**. After that, new project can be created by navigating to **File → New → Project..**. ( *alternatively, `Ctrl` + `⇧ Shift` + `N` shortcut can be used*) in order to create new **Procedural Animation Project**. **Procedural Animation Project** is quite similar in purpose to animation definitions in **Real Virtuality** **Model Config** but with **much better signal processing** *(more about it next chapter)*.

Once that step is completed, a new pop up window will appear asking for save location. In **Arma Reforger**, asset specific procedural animations are stored next to the model somewhere in inside **Asset** folder structure. Common animations, like turrets or wheel animations which are used by multiple vehicles, are located in **Anims/Proc** folder.

Here, situation is quite simplified and **Rugged Desk** animations are assumed to be specific to this asset and are placed in **Assets/Props/Military/Camps/PortableDesk\_01/Anims/** folder.

## Creating new procedural animation signal

Once new **Procedural Animation Project** is created, it is possible to populate it with actual data. First step towards this goal would be creating new **signal**. To differentiate it from **Audio Editor** signals, **Procedural Animation Editor signals have .siga** extension.

Signals are quite essential part of procedural animation since they are controlling how signals from scripted or engine events are processed. For those familiar with **Real Virtuality Engine**, **signals** are somewhat similar in their purpose to **source** parameter in **Model Config -** this time though it is possible to do much more complex processing of signals like mixing values, non linear movement and mixing two or more **signals** together.

## Setting  up signal

To create **new signal**, navigate to **File -> New -> Signal...** in top bar. This should open new **Create Signal** dialog. Select location for your new signal and after clicking on **Ok** button, this file should be opened in new tab. After that, get back to the **Procedural Animation Project and** click with **![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")** on [**Signal**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#Signal "Arma Reforger:Procedural Animation Editor: Nodes") button located in the top bar of **Procedural Animation Editor**. Over there, try to locate previously created **signal** *(.sig)* file and confirm the selection by pressing on **Ok** button.

Once that is completed, this signal should be assigned to the currently opened project.

Alternatively, instead of using **Signal** button, it is possible to **drag & drop** signal from **Resource Browser** the main window.

[![armareforger-new-prop-procedural-animation-create-signal.gif](/wikidata/images/d/d1/armareforger-new-prop-procedural-animation-create-signal.gif)](/wiki/File:armareforger-new-prop-procedural-animation-create-signal.gif)

At minimum, **signal** should contain [**Input**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#Input "Arma Reforger:Procedural Animation Editor: Nodes") and [**Output**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#Output "Arma Reforger:Procedural Animation Editor: Nodes") node and those can be added by clicking with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") on **Input** and **Output** buttons respectively in the **top section of main window**. At this point, it is possible to simply connect **Out** connector from **Input** node to **In** connector from **Output** node and such signal would already work. However, beauty of **Procedural Animator Editor** lies in nodes that can be used to process the signal and therefore, [**Smoother**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#Smoother "Arma Reforger:Procedural Animation Editor: Nodes") node will be added in this signal project to, as name suggest, smooth the signal.

Same as with the **Input** and **Output** nodes, **Smoother** can be simply added by clicking on **Smoother** button in top section of main window.

Once that newly created node is present in the window, it is possible to connect **Out' *from*** *Input **node with** In **from** Smoother **node'***. Similar thing should be done with **Output** node where its **In** connector should be linked with **Out** connector of **Smoother** node.

[![armareforger-new-prop-procedural-animation-adding-smoother.gif](/wikidata/images/e/e8/armareforger-new-prop-procedural-animation-adding-smoother.gif)](/wiki/File:armareforger-new-prop-procedural-animation-adding-smoother.gif)

[![armareforger-new-prop-procedural-animation-smoother-settings.png](/wikidata/images/d/db/armareforger-new-prop-procedural-animation-smoother-settings.png)](/wiki/File:armareforger-new-prop-procedural-animation-smoother-settings.png)

With those nodes being properly connected, now it's time to adjust behavior of **Smoother** node itself. To do so, click on it and navigate to **Item detail** window and go to **Data** section over there.

**Fade In Time & Type** will correspond to **opening of drawer** and **Fade Out Time & Type** to **closing of drawer**. In this case, **Fade In Time** was set to 700 and **Type** was set to **S-curve** (See [Sigmoid function](https://en.wikipedia.org/wiki/Sigmoid_function)) - this should give it quite nice & natural motion. Closing of the drawer should behave more like you slam it so that's why **Power of 2** was selected in **Fade Out Type** drop-down list.

At later stage, **once scripted user action are configured**, it might be wise to **play with those parameters in-game** and observe what kind of results you can get by changing time or type of fade effect.

Once first set of **Input, Smoother & Output** nodes is completed, it is now possible to create rest of the signals for all separate drawers. This could be achieved in two ways:

1. By **repeating whole procedure** and manually adding **Input, Smoother & Output** nodes and then, adjusting content of **Smoother** node
2. By using **copy and paste procedure**

Naturally, since all those signals are supposed to work almost exactly the same, using **copy paste procedure is recommend** way of dealing with rest of the signal.

To copy paste nodes, simply click and then hold **![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")** and select all nodes that you want copy. If everything was done correctly, nodes should have now **golden stroke** around them. After that, use **`Ctrl` + `C`** combination to copy files and then use **`Ctrl` + `V`** combo to paste those nodes under the cursor. Voila, now this action has to be repeated till **signal project has 6 sets of Input, Smoother & Output** nodes.

### Aligning and grouping nodes

ⓘ

Both **Procedural Animation Editor** and [**Audio Editor**](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor") offers way to adjust position of the nodes making it look more pleasant and aesthetical. This is done by **selecting multiple nodes** and then **clicking on of them with RMB** and then, selecting **one of the alignment options from the context menu**. It is also possible to catch few nodes in a single group by selecting **Group** option but again - this option has purely aesthetical function.

[![armareforger-new-prop-procedural-animation-align-nodes.gif](/wikidata/images/7/7b/armareforger-new-prop-procedural-animation-align-nodes.gif)](/wiki/File:armareforger-new-prop-procedural-animation-align-nodes.gif)

Last step in setting up signal will be changing of **Name** property in all **Signal** nodes. This **Name** property will be later used to **link script source to signal**, therefore, some reasonable name should be picked - in case of that project all **Input** nodes were named **Drawer 1** to **Drawer 6**. Names of rest of the nodes is not important but it is usually quite helpful to maintain some reasonable names - especially with **Output** nodes - since it helps with navigation and understanding of project that you are working on.

In the end, you should end up with following setup in **Procedural Animation Signal**

[![armareforger-new-prop-procedural-animation-signal-ready.png](/wikidata/images/0/03/armareforger-new-prop-procedural-animation-signal-ready.png)](/wiki/File:armareforger-new-prop-procedural-animation-signal-ready.png)

## Setting up Procedural Animation Project

After **signal** is configured, it is possible to proceed further with **Procedural Animation Project** itself. To put in motion things, **nodes** are required.

The goal here would be to **make drawers move**. To achieve that goal, let's take a look   Motion of the drawer is quite simple and simple **translation** could be used. In **Procedural Animation Editor**, this translates into following procedure:

* Add [**TranslateMake**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#TranslateMake "Arma Reforger:Procedural Animation Editor: Nodes") node and connect signal from **Signal** node to **Z axis** of that newly created item
  + Select **TranslateMake** node that was just created and in **Item detail** window, navigate to **Z** **axis** parameter in **Data** section - over there change the value to -0.5 *(meters)-* that will be the maximal movement of the bone.
* Create [**TranslateSet**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#TranslateSet "Arma Reforger:Procedural Animation Editor: Nodes") node and connect **TranslateMake** node to translation input on **TranslateSet**
* Create [**Bone**](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes#Bone "Arma Reforger:Procedural Animation Editor: Nodes") node and connect it to **In** input in **TranslateSet**

[![armareforger-new-prop-procedural-animation-setting-pap.gif](/wikidata/images/7/75/armareforger-new-prop-procedural-animation-setting-pap.gif)](/wiki/File:armareforger-new-prop-procedural-animation-setting-pap.gif)

Those actions should be repeated for all 6 drawers present in that model -  **similar to signal setup**, it is possible to utilize **copy paste process** to speed up whole procedure. In the end, Procedural Animation Project should look like that:

[![armareforger-new-prop-procedural-animation-pap-ready.png](/wikidata/images/1/13/armareforger-new-prop-procedural-animation-pap-ready.png)](/wiki/File:armareforger-new-prop-procedural-animation-pap-ready.png)

## Previewing animation

After adding nodes, drawer animations should be already in working state but without any means to test it, it is rather hard to verify if the motion is fully correct. To preview those animations, it will be necessary to set up few things.

⚠

Previewing animations which have smoother node is only working in 0.9.6 version of Workbench and beyond

In **View** window, located by default in right section of **Procedural Animation Editor**, it is possible to quickly preview animation that is currently developed. To do so, follow below steps:

[![armareforger-new-prop-procedural-animation-setting-preview.gif](/wikidata/images/6/65/armareforger-new-prop-procedural-animation-setting-preview.gif)](/wiki/File:armareforger-new-prop-procedural-animation-setting-preview.gif)

* Click on **Select Model** button (*in 0.96 version this button is replaced by icon*) and in pop up window select preview model (in this case it will be **PortableDesk\_01.xob**)
* On all **Bone** nodes that were previously created, go to Item detail window and over there, pick from **Bone name** list correct bone for particular signal
  + For example **Drawer** **1** signal should use **Drawer\_1** bone
* Select **signal** node in the **main window**
* In **Signal Simulation** window change of the inputs (*i.e. Drawer 1)* **-** either with mouse or by typing the value

ⓘ

Please note that it might be necessary to **save & close project after selecting model** in order to have those changes being correctly applied.

With that knowledge, previewing animations in **View** window should not longer be a problem

[![armareforger-new-prop-procedural-animation-preview.gif](/wikidata/images/e/e0/armareforger-new-prop-procedural-animation-preview.gif)](/wiki/File:armareforger-new-prop-procedural-animation-preview.gif)

## Applying procedural animation to prefab

At this stage, it is finally possible to plug in animation to rugged desk prefab. It will be still not possible to animate it in game via some user action but it will be good  base for further endeavors.

First step towards it would be adding two new components to **Rugged Desk** base prefab

* **ProcAnimComponent -** links Procedural Animation Projects to prefab and defines which bones are linked to PAP file
* **SignalsManagerComponent -** enables usage of signals which are either provided by game code or scripts

Those components can be added via **Add Component +** button located in bottom section of **Object Properties** window - whole process of adding new components is described in detail on [**Prefabs Basics**](/wiki/Arma_Reforger:Prefabs_Basics#Adding_new_components_to_entity_instance "Arma Reforger:Prefabs Basics") page.

[![armareforger-new-prop-procedural-adding-component.gif](/wikidata/images/8/8a/armareforger-new-prop-procedural-adding-component.gif)](/wiki/File:armareforger-new-prop-procedural-adding-component.gif)

While **SignalsManagerComponent** basically doesn't need any further tweaking, **ProcAnimComponent** needs some configuration. In this component, locate **Parameters** array and add new element by clicking on **plus symbol** on the right side of that parameter. This should add new element to the array of  **ProcAnimParams** type. Over there, there are two new parameters available:

* **Resource Name** - this parameter is used to link **Procedural Animation Project (.pap)** to the prefab. In this case **drawer\_move.pap** should be selected.
* **Bone Names** - array of bones (list is filled automatically if **MeshObject** has bones/pivots available) which should be animated. This list should match bones order (the stored in .pap file itself!) and sometimes it might be necessary to verify in game which entry in array corresponds to **Bone** node in **Procedural Animation Project.**

If components were added to entity instance of **Rugged Desk base prefab**, then don't forget to use **Apply to prefab** button and then remember to **save current world** to store previously made changes **permanently on the drive**. After that, initial implementation of procedural animations should be completed.

## Adding scripted action

💬

**Overview**

Following topics will be covered in this paragraph

* Creating user action for drawer
* Adding new user action to prefab

ⓘ

Before proceeding further it is recommend to make yourself familiar with [scripting basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") & [example](/wiki/Arma_Reforger:Scripting_Example "Arma Reforger:Scripting Example") or [tutorial](/wiki/Arma_Reforger:Scripting_Modding "Arma Reforger:Scripting Modding").

To add new scripted action for opening and closing of drawers following elements are needed:

* **New script** for drawer action
* Properly configured **Action Contexts** and **Additional Actions** in **ActionsManagerComponent**

Before going any further, first step towards adding new user action would be **adding of ActionsManagerComponent** to **PortableDesk\_01\_Base.et** prefab. Similar as in [**applying procedural animation to prefab**](#Applying_procedural_animation_to_prefab) segment, this can be done in few ways and all those methods are described on [**Prefabs Basics**](/wiki/Arma_Reforger:Prefabs_Basics "Arma Reforger:Prefabs Basics") page.

In this tutorial, it is assumed that user have already some experience with scripts in general. If not, a good starting point might be [Arma Reforger/Modding/Scripting/Guidelines](/wiki/Category:Arma_Reforger/Modding/Scripting/Guidelines "Category:Arma Reforger/Modding/Scripting/Guidelines") and then checking one of the scripting tutorials [Scripting Example](/wiki/Arma_Reforger:Scripting_Example "Arma Reforger:Scripting Example") or  [Scripting Modding](/wiki/Arma_Reforger:Scripting_Modding "Arma Reforger:Scripting Modding").

In [**Script Editor**](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor"), in [**Projects**](/wiki/Arma_Reforger:Script_Editor#Projects "Arma Reforger:Script Editor") tab, it is possible to check some of the already existing **User Actions** in **Game('Scripts/Game')** →  **Scripts → Game → UserActions.**  A good candidate for inspiration might be **[SCR\_DoorUserAction](enfusion://ScriptEditor/Scripts/Game/UserActions/SCR_DoorUserAction.c)** which has some of parts, which could be potentially reused for **drawer open/close** action.

## Creating new user action

Starting with creation of new scripted user action, it will be necessary to prepare correct folder structure first. In this case, new script should be created in **Scripts → Game → UserActions** folder.

By default, game is evaluating scripts located in few predefined locations (*which are defined in [**Resource Manager options** in **Modules** section](/wiki/Arma_Reforger:Resource_Manager:_Options#Modules "Arma Reforger:Resource Manager: Options")), so **Scripts → Game** part of the structure is extremely important - script outside of that folder would not be compiled otherwise. **UserActions** folder is recommendation though since all other vanilla user actions are located over there.*

[![armareforger-new-prop-script-structure.png](/wikidata/images/thumb/6/67/armareforger-new-prop-script-structure.png/900px-armareforger-new-prop-script-structure.png)](/wiki/File:armareforger-new-prop-script-structure.png)

[![armareforger-new-prop-script-rb.png](/wikidata/images/f/f0/armareforger-new-prop-script-rb.png)](/wiki/File:armareforger-new-prop-script-rb.png)

When folder structure is ready, it is time to create a new script called **SCR\_DrawerUserAction.c**. In general, it can be done in at least three ways:

1. Via **Create** button in **Resource Browser** (doesn't matter if its attached to which editor it is attached, **Resource Manager** will work fine though)
   1. To use this method, navigate to **Scripts → Game → UserActions** folder and then either click on **Create** button or use RMB on file list. In both cases, a new menu will appear from which you can select **Script** option. This will
2. Via **Add New Script..**. context menu in **Projects** window
3. Via **Script Wizard**

In principle, to make your new user action selectable in **Additional Actions** inside **ActionsManagerComponent**, you have to inherit from **ScriptedUserAction** class - just like **SCR\_DoorUserACtion** does. This can be simply achieved by writing following code in **SCR\_DrawerUserAction.c**

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
}
```

Next step would be getting reference to **SingalsManagerComponent**. It is done once in init and reference to it is cached in memory.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	//! Signal manager to pass signals into procedural animation
	private SignalsManagerComponent m_SignalsManager;
	//------------------------------------------------------------------------------------------------
	override void Init(IEntity pOwnerEntity, GenericComponent pManagerComponent)
	{
		m_SignalsManager = SignalsManagerComponent.Cast(pOwnerEntity.FindComponent(SignalsManagerComponent));
	}
}
```

Once that is set, it is a proper moment to add new attribute to the action itself. To do so, add [[Attribute()](enfusion://ScriptEditor/scripts/Core/attributes.c;221)] tag above variable that you want to expose. Attributes can be provided in two ways:

* Without parameters names, just values are typed, separated by coma. Example: *[Attribute("hello", UIWidgets.EditBox)]*
* With parameter names, list of parameters can be checked in Script Editor [over there](enfusion://ScriptEditor/scripts/Core/attributes.c;231).

In this tutorial, the 2nd method was used.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[Attribute( defvalue: "Drawer 1", uiwidget: UIWidgets.EditBox, desc: "Signal index of drawer" )]
	private string m_sSignalName; // string for pairing with the user action
}
```

Above code should translate to parameter being visible in **SCR\_DrawerUserAction** as on picture below. This part is detailed more in detail later, so for now we can continue with writing script.

[![armareforger-new-prop-script-action-parameter.png](/wikidata/images/d/d2/armareforger-new-prop-script-action-parameter.png)](/wiki/File:armareforger-new-prop-script-action-parameter.png)

Further step is finding of signal index in **SingalsManagerComponent** using name provided by attribute exposed before. To do so, it is necessary to override two methods:

* **CanBeShownScript** - controls if this action can be performed by the provided user entity
* **CanBePerformedScript** - controls if this action can be shown in the UI by the provided user entity

In case of **CanBeShownScript**, work is quite simple since it is assumed that action should be always visible if it can be performed.

**CanBePerformedScript** does some simple validation of input and verifies if **SingalsManagerComponent** is present on entity, if not, then the script will exit. Afterwards, code is validating  **m\_iSignalIndex** variable. By default it is set to -1 and in such case, code will try to find **m\_sSignalName** in **SingalsManagerComponent** and return correct index. If its not found, then action won't be visible.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[...]
	private int m_iSignalIndex = -1;
	//------------------------------------------------------------------------------------------------
	override bool CanBeShownScript(IEntity user)
	{
		return CanBePerformedScript(user);
	}
	//------------------------------------------------------------------------------------------------
	override bool CanBePerformedScript(IEntity user)
	{
		if(!m_SignalsManager) // Do nothing if there is no signal manager
		return false;
		if (m_iSignalIndex < 0) // Check if signal index is valid
		{
			m_iSignalIndex = m_SignalsManager.FindSignal(m_sSignalName);
			return false;
		}
		return true;
	}
	[...]
}
```

After that, code for sending signal to **Procedural Animation Project** can be added. This is done by finding signal (*via [FindSignal](enfusion://ScriptEditor/Scripts/Game/generated/Components/SignalsManagerComponent.c;35) method*) in **SignalManagerComponent**, which was automatically registered by linked procedural animation in **ProcAnimComponent**.

Since setting **ValRT** in **Procedural Animation Projects** do a full movement of the drawer, **targetValue** variable  switches between 0 and **MAX\_SIGNAL\_VALUE** constants, so the drawer is not fully opened. Of course, this can be changed/optimised and this is mainly a **showcase how to work with signals**. Value of **targetValue** is then passed to **SignalsManagerComponent** through [**SetSignalValue**](enfusion://ScriptEditor/Scripts/Game/generated/Components/SignalsManagerComponent.c;37) method, which is finally passed to procedural animation.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[...]
	const float MAX_SIGNAL_VALUE = 0.25;
	[...]
	//---------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		float targetValue = Math.AbsFloat((m_SignalsManager.GetSignalValue(m_iSignalIndex)) - MAX_SIGNAL_VALUE);
		m_SignalsManager.SetSignalValue(m_iSignalIndex, targetValue);
	}
	[...]
}
```

Last step will be handling of UI part and changing name of the action depending whether drawer is going to be closed or opened. This is done by **overriding GetActionNameScript** method and changing **outName** string. If signal connected to that particular drawer is **equal to 0**, then we can assume that **drawer is closed** and action that should be visible by player should be called "**Open**". Otherwise, **"Close"** action should be presented to player.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[...]
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if((m_SignalsManager.GetSignalValue(m_iSignalIndex)) == 0)
		{
			outName = "Open";
		} else
		{
			outName = "Close";
		}
		return true;
	}
	[...]
}
```

If all described above steps were done correctly, then new, **SCR\_DrawerUserAction** action should be ready to use in **ActionsManagerComponent**. Below you can find full script code.

Full code
Show text

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[Attribute( defvalue: "Drawer 1", uiwidget: UIWidgets.EditBox, desc: "Signal index of drawer" )]
	private string m_sSignalName; // for pairing with the user action
	private int m_iSignalIndex;
	const float MAX_SIGNAL_VALUE = 0.25;
	//! Signal manager to pass signals into procedural animation
	private SignalsManagerComponent m_SignalsManager;
	//------------------------------------------------------------------------------------------------
	override bool CanBeShownScript(IEntity user)
	{
		return CanBePerformedScript(user);
	}
	//------------------------------------------------------------------------------------------------
	override bool CanBePerformedScript(IEntity user)
	{
		if(!m_SignalsManager) // Do nothing if there is no signal manager
		return false;
		if (m_iSignalIndex < 0) // Check if signal index is valid
		{
			m_iSignalIndex = m_SignalsManager.FindSignal(m_sSignalName);
			return false;
		}
		return true;
	}
	//---------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		float targetValue = Math.AbsFloat((m_SignalsManager.GetSignalValue(m_iSignalIndex)) - MAX_SIGNAL_VALUE);
		m_SignalsManager.SetSignalValue(m_iSignalIndex, targetValue);
	}
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if((m_SignalsManager.GetSignalValue(m_iSignalIndex)) == 0)
		{
			outName = "Open";
		} else
		{
			outName = "Close";
		}
		return true;
	}
	//------------------------------------------------------------------------------------------------
	override void Init(IEntity pOwnerEntity, GenericComponent pManagerComponent)
	{
		m_SignalsManager = SignalsManagerComponent.Cast(pOwnerEntity.FindComponent(SignalsManagerComponent));
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63d39aee9a2)

## Adding action contexts

[![armareforger-new-prop-script-action-manager.png](/wikidata/images/5/5e/armareforger-new-prop-script-action-manager.png)](/wiki/File:armareforger-new-prop-script-action-manager.png)

With **SCR\_DrawerUserAction** being properly setup, it is possible to move forward and start adding new **Action Contexts** in **ActionsManagerComponent.**

**Action Contexts** serves as a sort of list of interactable places , where other systems requiring user input can use them.

Each **action context** have following properties:

* **Context Name** - used by other systems to distinguish this action from the rest of action contexts
* **Position -** defines position of the action. Using **PointInfo** class, it is possible to define following things
  + **Pivot ID -** in this drop-down list box it is possible to attach **action context** to **selected bone**. Action will follow the bone if its animated
  + **Offset** - offset from the center of model or (*Pivot ID has some valid bone selected*) from center of the **Pivot ID**
  + **Angles -** orientation of the action - it is especially important when **Omnidirectional** property is set to false
* **Radius** - this parameter defines how far, from the origin defined by **Position** parameter,  action context is visible.
* **Omnidirectional -** defines if context is available from both sides or not.

ⓘ

Some of the extra details can be found on [Arma\_Reforger:Action\_Context\_Setup](/wiki/Arma_Reforger:Action_Context_Setup "Arma Reforger:Action Context Setup") page

New **Action Contexts** can be added to **ActionsManagerComponent** by clicking on **plus + sign (1)** located to the right of the parameter. **ActionsManagerComponent** component has also some interesting options available **which can speed up process of debug and setup of new actions.**

After clicking with RMB on **ActionsManagerComponent** in **Object Properties** windows, a new context menu should appear containing various options for debuging and one action for **assisted creation of new action contexts** - **Create user action context(s) from bones.**

After clicking **Create user action context(s) from bones** option, a new window will appear listing all bones that **don't have action context defined yet**. By clicking on **drawer\_1 with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")** and clicking on **drawer\_6** with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") while holding left `⇧ Shift`, you can select multiple entries at once. It is also possible to select multiple elements by holding left control and then clicking with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button"). Once all drawer related bones (from 1 to 6) are selected, it is possible to confirm selection by clicking on **OK** button.

[![armareforger-new-prop-script-action-manager-create-actions-bones.gif](/wikidata/images/0/05/armareforger-new-prop-script-action-manager-create-actions-bones.gif)](/wiki/File:armareforger-new-prop-script-action-manager-create-actions-bones.gif)

It's also worth to mention that this context menu has few other options, which are quite useful for setting up & debugging of context actions. Below you can see in action transform gizmos (activated via *Toggle context(s) transform gizmo visualization* ) and radius visualization (activated via *Toggle context(s) radius visualization* option). *Toggle context(s) visibility angle visualization* is also quite handy in setting non **Omnidirectional** actions.

[![armareforger-new-prop-script-action-manager-context-menu.png](/wikidata/images/c/c4/armareforger-new-prop-script-action-manager-context-menu.png)](/wiki/File:armareforger-new-prop-script-action-manager-context-menu.png)

Position of the action context can be further tweaked by changing **Offset** values in **Position** object.

[![armareforger-new-prop-script-action-adjustments.gif](/wikidata/images/8/8d/armareforger-new-prop-script-action-adjustments.gif)](/wiki/File:armareforger-new-prop-script-action-adjustments.gif)

## Adding additional actions

[![armareforger-new-prop-script-action-manager-additional-actions.gif](/wikidata/images/c/c0/armareforger-new-prop-script-action-manager-additional-actions.gif)](/wiki/File:armareforger-new-prop-script-action-manager-additional-actions.gif)

Additional actions are actions, which are scripted. Some of the actions like get in, **switch seat or opening of doors are handled** by respective components like **DoorComponent** or **SCR\_BaseCompartmentComponent**. Other more specific actions, which don't necessarily need a whole component to be handled, are handled via **Additional Actions** array. In vanilla game, **flushing toilet** or **opening hatches in BTR-70** are handled via this kind of actions and **drawer open/close action is also a good candidate to use this method**.

New **additional actions** can be added to the asset by clicking on plus icon to the right of **Additional Actions** property inside **ActionsManagerComponent**. Once that plus sign is clicked, a new menu pops up asking for a type of action which should be added to the asset. In this case **SCR\_DrawerUserAction** should be selected from the list.

After adding first **SCR\_DrawerUserAction** element to the array'***, there are** few things that need to be adjusted over there**. First task will be configuring** Parent Context List **parameter. This array contain list of contexts where this action will be visible. It is possible to use multiple contexts for such action but in this case, only single context -** previously created drawer\_1 -'* should be present in **Parent Context List** array.

Next thing will be defining of **Signal Name** parameter. This is custom property which was added by **SCR\_DrawerUserAction** and, as it was mentioned before, it should contain name of the **signal** defined in **Input** node of **Procedural Animation Editor Signal** (.*siga file*). In this case it will be **Drawer 1**

If all steps were properly followed, then it should be already possible to test it in-game and verify if the user action is visible & usable. If action is not visible, verify if bone used as **Pivot ID** is correctly rotated and that radius of action is correct.

## Adding sounds

💬

**Overview**

Following topics will be covered in this paragraph

* Creating audio project
* Assigning it to object

ⓘ

Before proceeding further, it is recommended to make yourself familiar with the [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor") documentation and the [Audio Editor: Getting Started Tutorial](/wiki/Arma_Reforger:Audio_Editor:_Getting_Started_Tutorial "Arma Reforger:Audio Editor: Getting Started Tutorial").

## Creating new audio project

New **audio project** can be created quite easily inside **Audio Editor**. To do so, open **Audio Editor** and either select from **File → New → Project..**. option or use **`Ctrl` + `⇧ Shift` + `N`** shortcut.

[![armareforger-new-prop-sound-new-project.png](/wikidata/images/c/c0/armareforger-new-prop-sound-new-project.png)](/wiki/File:armareforger-new-prop-sound-new-project.png)

## Setting nodes in audio project

For drawer open/close action following things will be required in audio project:

* **2 [BankLocal nodes](/wiki/Arma_Reforger:Audio_Editor:_Nodes#BankLocal "Arma Reforger:Audio Editor: Nodes")** (*aka Bank node)* - one for opening and another for closing of drawer
* **2 [Shader nodes](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Shader "Arma Reforger:Audio Editor: Nodes")** - separate for opening and drawer. Applies panning and attenuation based on the spatial relation between the in-game listener and emitter
* **2 [Sound nodes](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Sound "Arma Reforger:Audio Editor: Nodes")** - same as above. The root node of a signal chain. Must be present in every chain in order for sound to be playable. Name of the **Sound** node can be later to trigger **Sound Event** via scripts
* **[Amplitude node](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Amplitude "Arma Reforger:Audio Editor: Nodes")** - controls amplitude attenuation that is applied by the **Shader -** shared with both **Shader nodes**
* **[Frequency node](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Frequency "Arma Reforger:Audio Editor: Nodes")** - controls frequency attenuation that is applied by the **Shader -** shared with both **Shader nodes**
* **[Spatiality node](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Spatiality "Arma Reforger:Audio Editor: Nodes")** - controls spatiality that is applied by the **Shader** **-** shared with both **Shader nodes**
* **[OutputState node](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Mixer "Arma Reforger:Audio Editor: Nodes")** *(aka Mixer or [FinalMix.afm](enfusion://ResourceManager/~ArmaReforger:Sounds/FinalMix.afm))* - This node serves as a way to group sounds based on type so that they can be processed as a unit before being sent to the final output(s). As with the Signal node, this node contains some internal logic that defines how each sound group is processed. This internal logic can be edited by double-clicking on the node.

### Adding audio banks

Starting configuration of **audio project** with creation of **audio banks**, new **Bank nodes** can be easily added by following below steps:

* Click on **BankLocal** button in the upper section of the editor
* Click on that new **Bank** node and navigate to **Unsorted section** in **Item Detail** window
* Fill in **Samples** array with audio files (*start with Sfx\_RuggedDesk\_Drawer\_Open\_01.wav)* that should be played when this audio bank is used. This can be done in two ways:
  + By drag and dropping registered audio file in wav format on **Bank** node
  + By adding new element to **Samples** array via plus button and then assigning wav file in **Filename** field

This should be enough to have it in working state. Additionally, following steps can be performed to make it more pleasant to look at:

* Rename **Bank** node to f.e. *"Open"* by changing **Name** parameter in **Item Detail** window
* Reposition **Bank** node to make space for another **Bank** node

After that, above steps  can be repeated to create *Close* **Bank** node, which would contain *Sfx\_RuggedDesk\_Drawer\_Close\_01.wav* sample. With two bank nodes in place, it is also possible to already start process of beautification of the audio project by aligning and then grouping those nodes in **Audio Bank**s group.

[![armareforger-new-prop-sound-audio-bank.gif](/wikidata/images/f/f0/armareforger-new-prop-sound-audio-bank.gif)](/wiki/File:armareforger-new-prop-sound-audio-bank.gif)

### Adding shaders

Process of adding **new shaders** to **audio project** is quite similar, if not even simpler, to explained above procedure of adding audio banks.

In principle, those two steps have to be done for both **Close** and **Open** variants:

* Click on **Shader** button in top section of the main window
* Connect **Out** port on **Bank node** with **In port on Shader node**

Once again, it is also recommended to **change names** of the nodes and then **group** them improve readability of the audio project.

[![armareforger-new-prop-sound-adding-shader.gif](/wikidata/images/2/2b/armareforger-new-prop-sound-adding-shader.gif)](/wiki/File:armareforger-new-prop-sound-adding-shader.gif)

### Adding special nodes

Next in audio project configuration will be adding of **Amplitude, Frequency & Spatiality** nodes to the working file. Adding of those nodes should be at this point quite straightforward so in this chapter we will focus more on **configuration of new nodes**.

Beginning with **Amplitude** node, here are few things that has be done in order to have it working correctly:

* In **Item Detail** window, change **Curve** parameter from **None** to **Logarithm**. This action will enable **attenuation of the sound** or in other words, sound will be more **silent the further away you are from the sound source**.
  + After changing **Curve** parameter to **Logarithm**, two new parameters should appear**:**
    - **Inner Range** - this parameter define distance in meters below no attenuation is applied. By default it is set to 1 and for this asset we can leave it as it is
    - **Outer Range** - this parameter define distance in meters beyond sound is muted. For such small drawer as this, **5 meters** would be a sensible number
* **Amplitude Out** port should be connected to **Amplitude** port on both **Close & Open** shaders.

Second on the list is **Frequency** node, which also affects attenuation of the sound & similar to **Amplitude** node, following things have to be adjusted on that node

* In **Item Detail** window, enable **Enable Distance Att** parameter
  + Default values should be fine for this rugged desk - for more information see Audio Editor nodes documentation
* **Frequency Out** port should be connected to **Frequency** port on both **Close & Open** shaders.

Last but not least it is time to configure **Spatiality** node, which controls 3D sound perception.

* In **Item Detail** window, change **Spatial Factor** parameter **to 1.0 -** this will enable 3D like perception of the sound.
* **Spatiality Out** port should be connected to **Spatiality** port on both **Close & Open** shaders.

[![armareforger-new-prop-sound-adding-special-nodes.gif](/wikidata/images/a/aa/armareforger-new-prop-sound-adding-special-nodes.gif)](/wiki/File:armareforger-new-prop-sound-adding-special-nodes.gif)

### Adding sound nodes

With shaders being properly configured, it is time to move to **Sound** nodes. In this case, **name of the Sound node is important**, since it will be later used in script to trigger **playing of the sound event**. Sound nodes can be added by clicking on **Sound** button in top section of the main window. Both **close and open action need a separate Sound node**, so two **Sound** nodes are necessary.

Once both nodes are present in work board, change their name to **SOUND\_DRAWER\_CLOSE** and **SOUND\_DRAWER\_OPEN** and then connect it to respective shaders.

**Sound** nodes have quite a lot of parameters which can be tweaked so at later stage, when whole asset is configured, it is recommended to play with parameters that are available over there.

[![armareforger-new-prop-sound-adding-sound-node.gif](/wikidata/images/7/75/armareforger-new-prop-sound-adding-sound-node.gif)](/wiki/File:armareforger-new-prop-sound-adding-sound-node.gif)

### Adding final mix

Final step will be adding of [FinalMix.afm](enfusion://ResourceManager/~ArmaReforger:Sounds/FinalMix.afm) (*also known as [Mixer](/wiki/Arma_Reforger:Audio_Editor:_Nodes#Mixer "Arma Reforger:Audio Editor: Nodes")*) to the audio project and then connecting outputs of **Sound** nodes to **ENV\_Objects** port on **OutputState** node.

[![armareforger-new-prop-sound-final-mix.gif](/wikidata/images/5/58/armareforger-new-prop-sound-final-mix.gif)](/wiki/File:armareforger-new-prop-sound-final-mix.gif)

### Result

In the end, audio project should look somewhat similar to the picture below. Since **Procedural Animation Editor** & **Audio Editor** are sharing a lot of code, it is also possible to [**align & group nodes**](#Aligning_and_grouping_nodes) here too.

[![armareforger-new-prop-sound-audio-project-result.png](/wikidata/images/5/5d/armareforger-new-prop-sound-audio-project-result.png)](/wiki/File:armareforger-new-prop-sound-audio-project-result.png)

It is also possible to check playback of the sounds inside **Audio Editor** - without switching to play mode - by selecting node and then hitting **space** button. Example of using sound debug can be found on [Weapon Modding](/wiki/Arma_Reforger:Weapon_Modding#Audio_Bank_Content_Change "Arma Reforger:Weapon Modding") tutorial page.

## Applying sound to prefab

[![armareforger-new-prop-sound-adding-component.png](/wikidata/images/6/6e/armareforger-new-prop-sound-adding-component.png)](/wiki/File:armareforger-new-prop-sound-adding-component.png)

In order to apply sounds to prefab, first step towards would be **adding of SoundComponent** to it. As described before, this can be achieved by using **[+ Add Component](/wiki/Arma_Reforger:Prefabs_Basics#Adding_new_components_to_entity_instance "Arma Reforger:Prefabs Basics")** button and selecting **SoundComponent** from list.

Once that component is present and selected, it is possible to add new element to **Filenames** array. Since in this tutorial only single **.acp** file was created, one element in this array should be enough.

After new element in array is present, it is possible to fill that property either by drag and dropping **audio project file** on that resource field or by clicking on button with two dots and the selecting **Rugged Desk .acp file.**

## Playing sound in script

First step towards triggering a sound from the script will be fetching of **SoundComponent** in **SCR\_DrawerUserAction script**. Similar to **SignalManagerComponent**, this can done by using [**FindComponent**](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;536) method.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[...]
	//! Sound component
	private SoundComponent m_SoundComponent;
	//------------------------------------------------------------------------------------------------
	override void Init(IEntity pOwnerEntity, GenericComponent pManagerComponent)
	{
		[...]
		// get Sound component
		m_SoundComponent = SoundComponent.Cast(pOwnerEntity.FindComponent(SoundComponent));
	}
}
```

Next, at the bottom of **PerformAction** method, code for triggering sounds can be added. This is done by calling **SoundEvent** method and passing name of the **Sound** node, which was previously defined in **audio project**, as a parameter.

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	//---------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		[...]
		// Check if sound component exist
		if (m_SoundComponent)
		{
			// If target value is above 0, then trigger SOUND_DRAWER_OPEN event, which will play sound of drawer opening
			// Otherwise drawer closing sound will be played via SOUND_DRAWER_CLOSE sound event
			if(targetValue > 0)
			{
				m_SoundComponent.SoundEvent("SOUND_DRAWER_OPEN");
			}
			else
			{
				m_SoundComponent.SoundEvent("SOUND_DRAWER_CLOSE");
			}
		}
	}
}
```

Once those changes are saved and compiled, this desk should be already in state where closing and opening of drawers plays some sounds! Below you can find full code in case you had some troubles with implementing those extra bits of code

Full code
Show text

```enforce
class SCR_DrawerUserAction : ScriptedUserAction
{
	[Attribute( defvalue: "Drawer 1", uiwidget: UIWidgets.EditBox, desc: "Signal index of drawer" )]
	private string m_sSignalName; // for pairing with the user action
	private int m_iSignalIndex;
	const float MAX_SIGNAL_VALUE = 0.25;
	//! Signal manager to pass signals into procedural animation
	private SignalsManagerComponent m_SignalsManager;
	//! Sound component
	private SoundComponent m_SoundComponent;
	//------------------------------------------------------------------------------------------------
	override bool CanBeShownScript(IEntity user)
	{
		return CanBePerformedScript(user);
	}
	//------------------------------------------------------------------------------------------------
	override bool CanBePerformedScript(IEntity user)
	{
		if(!m_SignalsManager) // Do nothing if there is no signal manager
		return false;
		if (m_iSignalIndex < 0) // Check if signal index is valid
		{
			m_iSignalIndex = m_SignalsManager.FindSignal(m_sSignalName);
			return false;
		}
		return true;
	}
	//---------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		float targetValue = Math.AbsFloat((m_SignalsManager.GetSignalValue(m_iSignalIndex)) - MAX_SIGNAL_VALUE);
		m_SignalsManager.SetSignalValue(m_iSignalIndex, targetValue);
		// Check if sound component exist
		if (m_SoundComponent)
		{
			// If target value is above 0, then trigger SOUND_DRAWER_OPEN event, which will play sound of drawer opening
			// Otherwise drawer closing sound will be played via SOUND_DRAWER_CLOSE sound event
			if(targetValue > 0)
			{
				m_SoundComponent.SoundEvent("SOUND_DRAWER_OPEN");
			}
			else
			{
				m_SoundComponent.SoundEvent("SOUND_DRAWER_CLOSE");
			}
		}
	}
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if((m_SignalsManager.GetSignalValue(m_iSignalIndex)) == 0)
		{
			outName = "Open";
		} else
		{
			outName = "Close";
		}
		return true;
	}
	//------------------------------------------------------------------------------------------------
	override void Init(IEntity pOwnerEntity, GenericComponent pManagerComponent)
	{
		m_SignalsManager = SignalsManagerComponent.Cast(pOwnerEntity.FindComponent(SignalsManagerComponent));
		m_SoundComponent = SoundComponent.Cast(pOwnerEntity.FindComponent(SoundComponent));
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63d39b0030b)

## Editor integration

💬

**Overview**

Following topics will be covered in this paragraph

* Creating placeable entities for in-game editor
* Configuring placeable entities appearance in entity browser

ⓘ

Recommended pages to familirize with: [Arma\_Reforger:Game\_Master:\_Editable\_Entities\_Configuration](/wiki/Arma_Reforger:Game_Master:_Editable_Entities_Configuration "Arma Reforger:Game Master: Editable Entities Configuration"), [Arma\_Reforger:Game\_Master:\_Image\_Generation\_Tutorial](/wiki/Arma_Reforger:Game_Master:_Image_Generation_Tutorial "Arma Reforger:Game Master: Image Generation Tutorial"), [Arma\_Reforger:Mod\_Localisation](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation"), [Arma\_Reforger:Asset\_Browser\_Mod\_Integration](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration "Arma Reforger:Asset Browser Mod Integration")

## Generating placeable entity

Once the configuration of the prefab is done, it is possible to use one of the editor plugins to prepare **Editable Entities**. This can be done by using **Create/Update Selected Editable Prefabs** plugin, which will create **new, editable prefabs** for assets which usually are not replicated (*like props)* and don't have one of the *Editable Components* attached to it.

Usage of that plugin is quite well explained on [Game Master Existing Prefab Configuration](/wiki/Arma_Reforger:Game_Master:_Editable_Entities_Configuration#Existing_Prefab "Arma Reforger:Game Master: Editable Entities Configuration") page and therefore some of the details are skipped here. In case of that set of rugged desk props following steps were performed:

* In **Resource Browser** (*main one, it doesn't work with Resource Browser 2!*)  attached to **Resource Manager**, all prefabs in **PrefabsEditable → Military → Camps** folder were selected. Assets with **\_base,\_Base,\_dst,\_Dst or\_DST suffix are automatically omitted** by the plugin so they can be also selected without any issues - this behavior can be tweaked in [**EditablePrefabsConfig.conf**](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/EditablePrefabs/EditablePrefabsConfig.conf).
* From **Plugins→ Editor** menu **Create/Update Selected Editable Prefabs** plugin was selected. Alternatively **`Ctrl` + `⇧ Shift` + `U`** shortcut can be used

After that, new prefabs should be automatically generated. Those new **editable prefabs** created by the plugin are stored in **Auto** folder of filesystem *(aka addon/mod)* they belong to. Over there, new inherited prefabs with **E\_** prefix are present, which are **inheriting from original, non editable prefabs** and they are **mimicking file structure of the original prefab**. In this case, it means that all editable prefabs should be located in **PrefabsEditable → Auto → Military → Camps** folder.

Additionally, plugin also prepares **placeholder preview images** for all assets - without it, it is impossible to go through next step which is generation of proper preview images.

[![armareforger-new-prop-editor-placeable-entities.gif](/wikidata/images/c/cf/armareforger-new-prop-editor-placeable-entities.gif)](/wiki/File:armareforger-new-prop-editor-placeable-entities.gif)

## Generating preview images

New, proper preview images can be created using special world located within Arma Reforger data ([link](enfusion://WorldEditor/worlds/Editor/Slots/AssetImages/Eden_AssetImages.ent;6454.71,171.752,6435.05;-27.5997,25.4986,0)) . Instructions about usage of that special world can be found on [Game Master: Image Generation Tutorial](/wiki/Arma_Reforger:Game_Master:_Image_Generation_Tutorial "Arma Reforger:Game Master: Image Generation Tutorial") and here you can find a short instruction on how it was done for **Sample New Prop** addon:

* Load [Eden\_AssetImages.ent](enfusion://WorldEditor/worlds/Editor/Slots/AssetImages/Eden_AssetImages.ent;6454.71,171.752,6435.05;-27.5997,25.4986,0)
* Select **SCR\_EditorImageGeneratorEntity** from Hierarchy window
* Adjust viewport to 400x300
* In diagnostic menu, set *Game > Compressed format in WB* to *true*
* Select prefabs in **Resource Browser** attached to **World Editor** which should be **subject of preview image generation**. In this case all **prefabs in PrefabsEditable → Auto → Military → Camps folder were selected**
* Switch to **Play** mode

If editable prefabs were correctly generated in previous step, then new pictures should be ready to use. If not, check **Console Log** for hints about what went wrong.

## Changing display name

[![armareforger-new-prop-editor-display-name.png](/wikidata/images/9/9d/armareforger-new-prop-editor-display-name.png)](/wiki/File:armareforger-new-prop-editor-display-name.png)

Display name of the prefab can be changed in prefabs located in **PrefabsEditable** folder inside **Auto** subfolder which was described in paragraph about generating placeable entities.

Once those editable prefabs are located and present in World Editor (either by placing them or using Edit prefab button), it is time to change their configuration. Bunch of editor related configuration is located in **SCR\_EditableEntityComponent** and among parameters that you can change there you can find **Name** parameter in **Visualization** section of that component.

Over here, change **Name** parameter so it reflects desired display name of that asset in [**Entity Browser**](/wiki/Arma_Reforger:Game_Master#Entity_Browser "Arma Reforger:Game Master") - in this case *Sand Rugged Desk* was picked, and it was made inline with [Arma\_Reforger:Editor\_Entity\_Naming\_Conventions](/wiki/Arma_Reforger:Editor_Entity_Naming_Conventions "Arma Reforger:Editor Entity Naming Conventions")

### Setting up localization table

⚠

Currently, non localised entities cannot be searched in Entity Browser, therefore it is **highly recommend to localize editable prefabs!**

After changing the strings in the prefab, it is possible to use **localization plugin** to add string to localization table. It is also worth to note, that in some case it might be faster to first **localize placeholder strings** and then **change those strings inside** **String Editor -** in any case, both workflows have their advantages and its matter of preference which one to use.

In this example, first method was used - adjusting display names in prefab and then putting those strings to **localization table (.st)** via **Localize Selected Assets** plugin. Before that, it is necessary to create localization table and runtime variant of it and the register those elements in game project. Whole process is described in detail on [Mod Localisation](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation") so here is short version of that procedure:

* Create "**Language**" folder in the root addon file system
* Inside of that folder, following files have to be created:
  + **GUI string table** - this is main file which is used by **String Editor**. It is important to assign to it unique name so it doesn't clash with other mods - in this case file was called *samplenewprop\_localization.st*
    - *After  samplenewprop\_localization.st* was created, click on it with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") and select **Open File in New Tab** option from **Context Menu** and set parameters to **following values*:***
      * *Target Prefix*: **Target\_**
      * *Item Class Name*: **CustomStringTableItem**
  + **StringTableRuntime -** version of localization table used by the game - it is stripped from all **String Editor** related content. Each language needs separate config file. In this tutorial, *samplenewprop\_localization.en\_us.conf* was created

[![armareforger-new-prop-editor-creating-localization.gif](/wikidata/images/7/7c/armareforger-new-prop-editor-creating-localization.gif)](/wiki/File:armareforger-new-prop-editor-creating-localization.gif)

Once files are created, it is time to plug them into game project:

* Open **Workbench options** by clicking on **Options** from **Workbench** top menu.
* Navigate to **Widget Manager Settings → String Tables**
* Added new **StringTableDefinition**
* Assign *samplenewprop\_localization.st* to **String Table Source** property
* Add new element to **Languages** array
  + Change **Code** property to **en\_us**
  + Assign *samplenewprop\_localization.en\_us.conf* to **String Table Runtime** property
* Save changes by clicking on **OK** button

[![armareforger-new-prop-editor-configuring-localization.gif](/wikidata/images/7/71/armareforger-new-prop-editor-configuring-localization.gif)](/wiki/File:armareforger-new-prop-editor-configuring-localization.gif)

### Adding strings to localization table

With string table in place, it is possible to actually fill it with data. This can be done by following below steps (see [Mod Localisation](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation") for more details):

* Select all prefabs inside **PrefabsEditable → Auto → Military → Camps** folder in **Resource Browser** (it can be either **Resource Browser** attached to **Resource Manager** or **World Editor**)
* Click on **Localize Selected Assets** in **Plugins** tab or use `Ctrl` + `⇧ Shift` + `L` shortcut
* In window that appeard make following adjustment:
  + Change **Config Path** config and use
  + Assign *samplenewprop\_localization.st* in **String Table Override** property
  + Type **SampleMod-** in **Prefix Override** property
  + Click on **Localize** button

Plugin now should open *samplenewprop\_localization.st* in **String Editor** and fill all the detected strings. If file is empty, then make sure that **String Table Override** property is correctly assigned and check **Console Log** for errors or warnings.

[![armareforger-new-prop-editor-localizing-assets.gif](/wikidata/images/9/95/armareforger-new-prop-editor-localizing-assets.gif)](/wiki/File:armareforger-new-prop-editor-localizing-assets.gif)

## Adjusting labels

[![armareforger-new-prop-editor-labels.png](/wikidata/images/1/17/armareforger-new-prop-editor-labels.png)](/wiki/File:armareforger-new-prop-editor-labels.png)

Entity labels are used by in-game Editor Entity Browser to [filter](/wiki/Arma_Reforger:Game_Master#Filters "Arma Reforger:Game Master") content. By default, some of those labels are automatically assigned by **Create/Update Selected Editable Prefabs** plugin base on their **type (***is it GenericEntity or Vehicle class?),* **folder in which they are located** (*Prefabs in Prefabs/Military folder will get Theme\_Military* *label)*, **parameters in their prefabs** (*Mass in RigidBody, size of the group by counting elements in Unit Prefab Slots, etc),* **size of bounding box** and so on. Most of those rules can be checked by opening [**EditablePrefabsConfig.conf**](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/EditablePrefabs/EditablePrefabsConfig.conf). It is also possible to add some custom rules as it was for instance described in the [Faction creation guide](/wiki/Arma_Reforger:Faction_Creation#Adjusting_generators_configs "Arma Reforger:Faction Creation").

Beside that, it is also possible to manually add extra labels or even custom ones by changing content of **Authored Labels** array in **SCR\_EditableEntityComponent** . In case of that prop, it is recommended to add **CONTENT\_MODDED** label to **Authored Labels** array (*Auto Labels array shouldn't be touched by the user since content of that array might get overridden by editor plugin!)* for each asset prefab in **PrefabsEditable → Auto → Military → Camps** folder. This can be done by clicking on **button with plus sign** on the right side of the **Authored Labels** property and then select from that new element in array **CONTENT\_MODDED** label from drop down menu.

## Registering asset

The final step in editor integration is adding editable props to register of placeable assets. Whole process is described in detail on [Asset Browser Mod Integration](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration "Arma Reforger:Asset Browser Mod Integration") page so here is simplified list of steps that had to be done:

* Create new **SCR\_PlaceableEntitiesRegistry**  register in *SampleMod\_NewProp\Configs\Editor\PlaceableEntities* called **NewProp\_Props.conf**
* Override in addon [**EditorModeEdit.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)
* Open [**EditorModeEdit.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et) and in **SCR\_PlacingEditorComponent, add new entry to Registries** array by i.e. drag and dropping **NewProp\_Props.conf**
* Save changes in [**EditorModeEdit.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Editor/Modes/EditorModeEdit.et)
* Open**NewProp\_Props.conf**, select all prefabs in **PrefabsEditable → Auto → Military → Camps** inside **Resource Browser** attached to **Resource Manager** and then drop that selection onto **Prefabs** array inside of that config
  + Furthermore, you can also change **Addon** & **Source Directory** property so plugin can in future **semi-automatically fill or update** that **Prefabs** array

[![armareforger-new-prop-editor-register.png](/wikidata/images/3/38/armareforger-new-prop-editor-register.png)](/wiki/File:armareforger-new-prop-editor-register.png)

ⓘ

Please note that GUID of any new file is **generated from filesystem path + file name**. In order to avoid conflicts with other mods, it is recommended to use **unique name for registers by i.e. adding tag as prefix**.

After all those steps, assets should be available in in-game [**Entity Browser!**](/wiki/Arma_Reforger:Game_Master#Entity_Browser "Arma Reforger:Game Master")

## Testing result

💬

**Overview**

In this paragraph you can read about:

* Testing your creation in Game Master
* Testing your creation in new world

In **Workbench**, new prop can be tested either by **launching  [Game Master scenario](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master")** and then placing new assets through **Asset Browser** or by **simply creating a new sub-world in World Editor** and then placing asset in game

## Testing in Game Master

To test new prop in **Game Master**, load [**GM\_Eden.ent**](enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent;4657.23,50.7969,10727.1;-33.3344,23.4751,0;1385091)  world in [**World Editor**](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") and once it's ready, switch to [**Play mode**](/wiki/Arma_Reforger:World_Editor#Play_Game "Arma Reforger:World Editor") by either pressing on green arrow or via **F5 shortcut** (*furthermore, that option is also available in **Game** tab*). This should initiate loading of scenario and once it's completed, regular Game Master interface should be present.

[![armareforger-new-prop-testing-playmode.png](/wikidata/images/6/6e/armareforger-new-prop-testing-playmode.png)](/wiki/File:armareforger-new-prop-testing-playmode.png)

Now, by bringing up [**Entity Browser**](/wiki/Arma_Reforger:Game_Master#Entity_Browser "Arma Reforger:Game Master") either by pressing **Tab** or by clicking on **Place entities** button in bottom right section of the interface

[![armareforger-new-prop-testing-gamemaster.png](/wikidata/images/1/12/armareforger-new-prop-testing-gamemaster.png)](/wiki/File:armareforger-new-prop-testing-gamemaster.png)

## Testing in new world

Simplest & fastest way to do it might be loading [**MpTest**](enfusion://WorldEditor/worlds/MP/MpTest.ent;110.145,5.1562,100.424;-35.2569,46.3466,0) world and then creating a new subworld of it. To do so, follow this instruction:

[![armareforger-new-prop-testing-subworld.png](/wikidata/images/9/90/armareforger-new-prop-testing-subworld.png)](/wiki/File:armareforger-new-prop-testing-subworld.png)

* Click on **File → New World** (***`Ctrl` + `N` shortcut***) or on **New World** icon in the toolbar
* Select **Sub-scene (of current world)** option and confirm it with **OK** button
* Save current world by selecting **File → Save World** option (**`Ctrl` + `S` shortcut**) and save it somewhere inside your addon

[![armareforger-new-prop-testing-subworld-creation.gif](/wikidata/images/0/0e/armareforger-new-prop-testing-subworld-creation.gif)](/wiki/File:armareforger-new-prop-testing-subworld-creation.gif)

After that, you can place prop prefab - i.e. *PortableDesk\_01\_Sand.et* - into the world and switch to **Play mode**. Once in play mode, you should be able to interact with the prop!

[![armareforger-new-prop-testing-subworld-result.png](/wikidata/images/8/85/armareforger-new-prop-testing-subworld-result.png)](/wiki/File:armareforger-new-prop-testing-subworld-result.png)
