# [Weapon Suppressor Creation](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Suppressor_Creation)

💬

**Overview**

This tutorial covers following topics:

* Preparing the 3D model and ensuring correct orientation.
* Configuring the suppressor as an accessory in the game.
* Creating the suppressor prefab and configuring its components.
* Integrating the suppressor with a weapon.

ⓘ

This tutorial guides you through the process of creating a custom suppressor (muzzle attachment) for a weapon in Arma Reforger . The steps are structured similarly to the Weapon Optic Creation tutorial and cover asset preparation, configuration, and integration with a weapon.

📥

Sources files for this tutorial can be found on
[**Arma Reforger Samples Github repository**](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/tree/main/SampleMod_NewWeapon)

---

## Prerequisites

* Basic understanding of **Enfusion Engine** and **Arma Reforger** modding.
* Access to a 3D modeling tool that can export **FBX** files.
* Familiarity with scripting in **Enforce Script** .
* A mod tag or identifier (e.g., `SampleMod_NewWeapon`) for naming conventions.
* Required tools and software:
  + **Enfusion Workbench** .
  + **Enfusion Blender Tools (EBT)** (optional but recommended for Blender users).

---

## Structure Preparation

While sticking to official structure is not mandatory and there are no engine restrictions asset wise about it, it is recommended to follow guidelines listed here - [Data (file) structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure") - to ensure that all automation plugins are parsing your assets correctly and make it later easy to navigate.

Therefore, your first task will be preparing following file structure

## Preparing the Suppressor Asset

### Model Orientation

Ensuring proper model orientation is crucial for correct attachment and in-game behavior. According to the [Arma Reforger: FBX Import](/wiki/Arma_Reforger:FBX_Import#Object_Orientation "Arma Reforger:FBX Import") guidelines:

* **Blender and 3ds Max Users** :
  + The model should point along the **positive Y-axis** .
* **Maya Users** :
  + The model should point along the **positive Z-axis** .

If importing a model from another source (e.g., Arma 3), you may need to rotate it by 90 degrees to align correctly.

[![armareforger-new-weapon-suppressor-model-orientation.png](/wikidata/images/thumb/4/4d/armareforger-new-weapon-suppressor-model-orientation.png/600px-armareforger-new-weapon-suppressor-model-orientation.png)](/wiki/File:armareforger-new-weapon-suppressor-model-orientation.png)

### Object Naming

Follow these naming conventions to ensure proper import and functioning:

* **Level of Detail (LOD) Suffixes** :
  + Use `_LOD0`, `_LOD1`, etc., to indicate different LODs.
  + Example: `Suppressor_LOD0`, `Suppressor_LOD1`.

Proper naming ensures that the engine correctly interprets your model's components during the import process. For a full list of naming conventions and detailed explanations, refer to the [Arma Reforger: FBX Import - Naming Conventions](/wiki/Arma_Reforger:FBX_Import#Naming_Conventions "Arma Reforger:FBX Import") page.

### Adding Snap Points

[![](/wikidata/images/2/2f/armareforger-new-weapon-empty-create.png)](/wiki/File:armareforger-new-weapon-empty-create.png)

Adding Plain Axes aka empty objects to Blender scene

Snap points are essential for correctly attaching the suppressor to weapons. For suppressor, it will be necessary to only add one empty object - **snap\_weapon.** This can be achieved in those few steps:

1. **Create an Empty Object** :
   * In Blender, add an **Empty** object of type **Plain Axes** .
   * Place it at the point where the suppressor will attach to the weapon.
2. **Name the Empty Object** :
   * Set the name to `snap_weapon`.
3. **Align the Empty Object** :
   * Ensure the empty object's orientation matches the weapon's attachment point.
   * The forward direction (front) should align along:
     + **Positive Y-axis** in Blender/3ds Max.
     + **Positive Z-axis** in Maya.
4. **Check Orientation** :
   * Verify that the `snap_weapon` aligns accurately with the suppressor's model.

[![armareforger-new-weapon-suppressor-snap-orientation.png](/wikidata/images/thumb/8/8d/armareforger-new-weapon-suppressor-snap-orientation.png/600px-armareforger-new-weapon-suppressor-snap-orientation.png)](/wiki/File:armareforger-new-weapon-suppressor-snap-orientation.png)

For detailed guidance, see [Weapon Slots and Bones](/wiki/Arma_Reforger:Weapon_Slots_And_Bones "Arma Reforger:Weapon Slots And Bones") .

📖

**Recommended read:** Slot/snap points concept is described in [**Weapon Creation**](/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Add_Slots/Snap_Points "Arma Reforger:Weapon Creation/Asset Preparation") documentation

### Collision Mesh

Creating an efficient collision mesh optimizes performance and ensures accurate physical interactions.

[![](/wikidata/images/thumb/8/8c/armareforger-new-weapon-suppressor-collider.png/600px-armareforger-new-weapon-suppressor-collider.png)](/wiki/File:armareforger-new-weapon-suppressor-collider.png)

In this case convex collider from A3 was used

#### Choosing Collider Shape

Select the appropriate collider shape based on your suppressor's geometry. The collision mesh doesn't have to be a specific type; you can choose from various collider shapes depending on what best fits your suppressor:

* **Common Collider Types** :
  + **Box Collider**
  + **Convex Collider**
  + **Cylinder Collider**
  + **Sphere Collider**
  + **Capsule Collider**
  + **TriMesh Collider**
* **Naming Colliders** :
  + Name your colliders with appropriate prefixes to indicate their type.
  + While specific prefixes like `UBX_`, `UCX_`, etc., are typically used, focus on selecting the collider that best suits your model's shape

📖

**Recommended read:** More details about Inventory Configuration can be found on [**Weapon Optic Creation**](/wiki/Arma_Reforger:Weapon_Optic_Creation#Inventory_system_configuration "Arma Reforger:Weapon Optic Creation") documentation

#### Simplifying the Collider

* **Keep It Simple** : Use the simplest shape that accurately represents the suppressor.
* **Performance Consideration**: Simplified colliders reduce computational overhead.
* **Avoid Complex Meshes** : Complex collision meshes can negatively impact performance.

#### Assigning Layer Preset and Game Material

Assign appropriate properties to your collider:

* **Layer Preset** : `ItemFireView`
  + This layer preset is appropriate for weapon attachments and ensures correct interactions in the game.
* **Game Material** : `weapon_metal.gamemat`
  + This material provides appropriate physical properties and sound effects for metal weapon components.

##### How to Assign

1. **In Your 3D Software** :
   * **Blender with EBT (Optional)** :
     + Use the **EBT Object Tools** to assign properties directly in Blender.
     + Navigate to **Object Properties** > **EBT Object Properties** .
     + Set **Layer Preset** and **Game Material** . Reference: [Enfusion Blender Tools: Object Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools "Arma Reforger:Enfusion Blender Tools: Objects Tools") .
   * **Other 3D Software** :
     + Add custom properties or user-defined attributes to colliders.
     + Refer to [Setting Layer Preset on Colliders (Usage Parameter)](/wiki/Arma_Reforger:FBX_Import#Setting_Layer_Preset_on_colliders_(usage_parameter) "Arma Reforger:FBX Import") .
2. **In Enfusion Workbench Import Settings** :
   * If not set in the 3D software, you can assign them during import.
   * In the **Import Settings** , select your collider.
     + Set **Usage** to `ItemFireView`.
     + Assign **Game Material** to `weapon_metal.gamemat`. Reference: [FBX Import - Setting Layer Preset on Colliders](/wiki/Arma_Reforger:FBX_Import#Setting_Layer_Preset_on_colliders_(usage_parameter) "Arma Reforger:FBX Import") .

### Exporting the Model

[![](/wikidata/images/8/81/armareforger-new-weapon-suppressor-export-ebt.png)](/wiki/File:armareforger-new-weapon-suppressor-export-ebt.png)

Exporting using Enfusion Blender Tools

Export your model to the **FBX** format suitable for Enfusion.

* **Using Enfusion Blender Toolkit (EBT) FBX Exporter** (Optional):
  + Simplifies export and ensures compatibility.
  + **Automatically imports & registers** FBX in Workbench and enables "**Export Scene Hierarchy**".
* **Without EBT** :
  + Manually register **[Import & Register](/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Model_Import_&_Registration "Arma Reforger:Weapon Creation/Asset Preparation")** file
  + Manually enable **"[Export Scene Hierarchy](/wiki/Arma_Reforger:Weapon_Creation/Asset_Preparation#Skeleton_&_Hierarchy "Arma Reforger:Weapon Creation/Asset Preparation")"** in export settings.
  + Ensure export settings align with Enfusion requirements.

[![](/wikidata/images/thumb/e/e8/armareforger-new-weapon-suppressor-imported-model.png/792px-armareforger-new-weapon-suppressor-imported-model.png)](/wiki/File:armareforger-new-weapon-suppressor-imported-model.png)

Sample Suppressor imported into Workbench. When using Blender, you can verify if settings were correctly passed by inspecting **Details** or **Import Settings**.

## Texturing the Suppressor

Apply textures to give your suppressor the desired appearance.

#### Enfusion Materials Creation

* If the import of the model went fine, new Enfusion Materials (`.emat` files) should be created in the `Data` folder next to the model.
* These new materials are named based on the material names in your 3D software.
* By default, they use the `MatPBRMaterial` shader.

[![](/wikidata/images/e/e2/armareforger-new-weapon-suppressor-material-setup.png)](/wiki/File:armareforger-new-weapon-suppressor-material-setup.png)

Suppressor material setup

#### Assigning Textures

1. **Prepare Textures** :
   * Assuming you have already created PBR textures (Base Color, Normal Map, etc.) using a tool like Substance Painter.
   * You can use the **BCR+NMO** export preset shared on GitHub:
     + GitHub Repository: [Arma Reforger Misc - Substance Export Profiles](https://github.com/BohemiaInteractive/Arma-Reforger-Misc/tree/main/Art/Substance%20Export%20Profiles)
     + *This preset ensures that your textures are in the correct format for Arma Reforger*.
2. **Import Textures into the Game** :
   * Place your exported textures (e.g., `Suppressor_BCR.tga`, `Suppressor_NMO.tga`) into the appropriate folder within your mod's `Data` directory.
3. **Assign Textures to Materials** :
   * In the Enfusion Workbench, navigate to the `.emat` files created during the import.
   * Open each material and assign the corresponding textures to the appropriate fields:
     + **BCR Map** :
       - Assign your Base Color Roughness texture (`_BCR`).
     + **NMO Map** :
       - Assign your Normal Map (`_NMO`).
   * For detailed information on texture types and how to use them, refer to [Arma Reforger: Textures](/wiki/Arma_Reforger:Textures "Arma Reforger:Textures") .
4. **Verify Material Settings** :
   * Ensure that all material properties are set correctly.
   * Adjust parameters like **Metallic** , **Roughness** , and **Specular** as needed.

### Testing the Textures

* **Preview the Model** :
  + Use Enfusion Workbench to ensure textures appear correctly.
* **Check for Issues** :
  + Look for texture stretching, UV mapping errors, or material issues.
* **Adjust as Necessary** :
  + Make corrections in your 3D software or texture editor if needed.

## Creating the Attachment Script Class

Attachment config classes in Arma Reforger establish compatibility between weapons and their attachments. Any attachment using a specific attachment class will be compatible with any weapon that references that class. This system ensures that only appropriate attachments can be mounted on specific weapons, maintaining realism and game balance.

In many cases, existing attachment classes can be used. For example, muzzle attachment classes for 5.45 mm or 5.56 mm calibers are already defined in the game's vanilla data. However, since our **Sample Weapon** uses a 6.5 mm barrel, and this caliber doesn't exist in the vanilla data, we need to create a new attachment class specifically for it. This ensures that only attachments intended for 6.5 mm barrels can be mounted on the weapon.

If you intend to make i.e. suppressor suitable for M16, then you could skip that part and move to Creating the Suppressor Prefab part.

ⓘ

Existing vanilla muzzle attachment classes can be found in the **[Attachments\_muzzle.c](enfusion://ScriptEditor/Scripts/Game/Weapon/Attachments/Attachments_muzzle.c;110)** script file.

### Setting Up the Attachment Config Class

To add a new attachment config class, you'll create a new script file in one of your mod's game script folders. For example, you can use the `Scripts/Game/Attachments/Muzzles` folder. If this folder doesn't exist, you'll need to create it manually.

#### Naming Conventions

Scripts in Arma Reforger don't use metafiles and can be overwritten if another mod uses the same script name and path. To prevent conflicts, it's recommended to use a unique naming convention for your script files and classes, such as prefixing them with your mod's tag.

* **Use the `TAG_` Prefix:**
  + Prefix your script file names and class names with a unique tag to prevent naming conflicts.
  + **In this case, the tag is `SampleMod_NewWeapon`.**
* **Script File:**
  + Use a clear and descriptive filename that includes your mod's tag.
  + **Example:** `SampleMod_NewWeapon_Attachments_Muzzle.c`
* **Class Names:**
  + **Attachment Config Class:** Defines the configuration class for your muzzle attachment.
    - **Example:** `AttachmentMuzzle65_39Class`
  + **Attachment Class:** Inherits from `AttachmentMuzzle`.
    - **Example:** `AttachmentMuzzle65_39`

### Creating the Script File

#### Create the Script File

1. **Navigate to Your Script Directory:**
   * Go to `/Scripts/Game/Attachments/Muzzles/` within your mod folder.
   * If these folders don't exist, create them accordingly.
2. **Create a New Script File:**
   * Right-click in the `Muzzles` folder and select **New Script** .
   * Name the file `SampleMod_NewWeapon_Attachments_Muzzle.c`.
   * Ensure the file extension is `.c`.



#### Define the Attachment Config Class

Open the newly created script file and define your classes exactly as follows:



```enforce
class AttachmentMuzzle65_39Class {}
AttachmentMuzzle65_39Class AttachmentMuzzle65_39Source;
class AttachmentMuzzle65_39 : AttachmentMuzzle
{
};
```

### Recompiling Scripts

After creating the script, recompile the game scripts:

* **In Script Editor** :
  + Use **Compile and Reload Scripts** (`⇧ Shift + F7`).
* **In World Editor** :
  + Use **Reload Game Scripts** (`Ctrl + R`).

By following these steps, you've successfully created a new attachment config class for your 6.5 mm suppressor. This class will ensure proper compatibility between your suppressor and the **Sample Weapon** , while preventing incompatible attachments from being mounted.

📖

**Recommended read:** For more information on how attachment classes and inheritance affect compatibility, refer to the [Weapon Optic Creation: Attachments Configuration](/wiki/Arma_Reforger:Weapon_Optic_Creation#Attachments_configuration "Arma Reforger:Weapon Optic Creation") documentation

## Creating the Suppressor Prefab

### Create Suppressor Prefab

[![](/wikidata/images/thumb/2/21/armareforger-new-weapon-suppressor-prefab-structure.png/300px-armareforger-new-weapon-suppressor-prefab-structure.png)](/wiki/File:armareforger-new-weapon-suppressor-prefab-structure.png)

Prefab structure

When creating a suppressor prefab in Arma Reforger, it is recommended to inherit from the existing base suppressor prefab. The base suppressor prefab can be found at **Prefabs\Weapons\Core\[Suppressor\_base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/Core/Suppressor_base.et)**. This prefab contains most of the necessary configurations for suppressor attachments.

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used

To begin, navigate to **Prefabs\Weapons\Core** in the **Resource Browser** and locate the **Suppressor\_base.et** prefab. Right-click on this prefab and select **[Inherit in...](/wiki/Arma_Reforger:Data_Modding_Basics#Using_"Inherit_in..."_function "Arma Reforger:Data Modding Basics")** from the context menu.

✩

Other method involves duplicating some already existing prefab - this might be handy if you are **creating something similar to already existing weapons or don't want to add manually components**.
For instance if you are making some new AK74 suppressor variant, you could use **[Duplicate to "addon name"](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics")** function on **[Suppressor\_PBS4\_base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/Attachments/Muzzle/Suppressor_PBS4/Suppressor_PBS4_base.et)** and then modify parameters of such new prefab.

When prompted to name your new prefab, consider adding a `_base` suffix to maintain consistency and clarity, especially if you plan to have multiple variants of the suppressor. For example, if your suppressor is named "*Suppressor\_SampleSuppressor\_01*," you might name the prefab **Suppressor\_SampleSuppressor\_01\_base.et** . After creating the inherited prefab, move it to a more appropriate location, such as the **Prefabs\Weapons\Attachments\Muzzle\*Suppressor\_SampleSuppressor\_01*** directory.

ⓘ

Inheriting from the base suppressor prefab ensures that essential components and configurations are included, saving you time and reducing the potential for errors.

### Creating a Child Prefab

In addition to the base prefab, it's advisable to create a child prefab without the `_base` suffix. This child prefab will inherit from your base suppressor prefab (**SampleSuppressor\_01\_base.et** ) and will be named **SampleSuppressor\_01.et** .

To create this child prefab:

1. **Right-click** on your base suppressor prefab (**Suppressor\_SampleSuppressor\_01\_base.et** ) and select **Inherit** .
2. **Name** the new prefab without the `_base` suffix (e.g., **Suppressor\_SampleSuppressor\_01.et** ).
3. **Move** this prefab to the appropriate directory alongside your base prefab.

This child prefab will initially be empty and won't hold any unique information. However, it serves an important purpose, especially when reskins or additional variants are present. For example, you might create another variant called **Suppressor\_SampleSuppressor\_01\_camo.et** for a camouflage version of the suppressor. **Suppressor\_SampleSuppressor\_01\_camo** should inherit from **Suppressor\_SampleSuppressor\_01\_base.et** as well. By making changes to the base prefab (**SampleSuppressor\_01\_base.et** ), those changes automatically propagate to all child prefabs, including **Suppressor\_SampleSuppressor\_01.et** and **Suppressor\_SampleSuppressor\_01\_camo.et** . This organizational structure makes it easier to manage and update your assets consistently.

Once your prefabs are set up, you can start editing the base prefab to configure your suppressor. Open the prefab in **[Prefab Edit Mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** by selecting it and clicking the **Prefab Edit Mode** button.

### Edit the Suppressor Prefab

Configure components within your new prefab.

[![](/wikidata/images/f/f7/armareforger-new-weapon-suppressor-inventory-configuration.png)](/wiki/File:armareforger-new-weapon-suppressor-inventory-configuration.png)

**InventoryItemComponent** configuration

#### Configure MeshObject

* Assign your suppressor's mesh to the `MeshObject` component.

#### Configure InventoryItemComponent

Next, configure the suppressor's inventory properties:

1. Select the **InventoryItemComponent** .
2. Modify the **Item Display Name** to change how the suppressor appears in the inventory.
3. Adjust the **Item Phys Attributes** to change storage properties like weight and size.
4. Under **Custom Attributes** , adjust the **PreviewRenderAttributes** to change the suppressor's visibility in the inventory preview

📖

**Recommended read:** More details about Inventory Configuration can be found on [**Weapon Optic Creation**](/wiki/Arma_Reforger:Weapon_Optic_Creation#Inventory_system_configuration "Arma Reforger:Weapon Optic Creation") documentation

#### Configure SCR\_WeaponAttachmentSuppressorAttributes

#### [armareforger-new-weapon-suppressor-change-class.gif](/wiki/File:armareforger-new-weapon-suppressor-change-class.gif)

Now, configure the suppressor's attachment properties:

1. Inside **InventoryItemComponent** find and select the **SCR\_WeaponAttachmentSuppresorAttributes** in **Custom Attributes** list.
2. **Attachment Type** : Set this to the class you created earlier for the suppressor attachment point.
   1. Default, inherited selection can be changed by clicking on **Attachment Type** class with **Right Mouse Button** and selecting **Change Class** option from the context menu
3. **Muzzle Speed Coefficient** : Adjust this value to reflect the effect of the suppressor on bullet velocity. Suppressors often increase bullet speed due to the extended barrel effect.
4. **Muzzle Dispersion Factor** : Modify this to account for changes in bullet spread when the suppressor is attached.
5. **Extra Obstruction Length** : Increase this value to simulate the added length of the suppressor, affecting the [**weapon's obstruction values**](/wiki/Arma_Reforger:Weapon_Components#Obstruction_Test_Added_Length "Arma Reforger:Weapon Components") in the **WeaponComponent** .
6. **Recoil Coefficients** :
   * **Linear Factors**: Adjust to simulate changes in linear recoil.
   * **Angular Factors**: Adjust to simulate changes in angular recoil.
   * **Turn Factors**: Adjust to simulate changes in weapon turn dynamics due to the suppressor's weigh

### Configuring the SCR\_MuzzleEffectComponent

Set up the muzzle effect to represent the suppressor's visual effects when firing:

1. Select the **SCR\_MuzzleEffectComponent**
2. **Particle Effect** : Assign a suitable particle effect (e.g., **[Suppressor\_Carbine.ptc](enfusion://ResourceManager/$ArmaReforger:Particles/Weapon/Attachments/Suppressor_Carbine.ptc)**) to represent the suppressed muzzle flash.
3. **Effect Position** :
   * Click on the **Set Class** button and select **EntitySlotInfo** .
   * Name it appropriately (e.g., **SuppressorEnd** ).
   * Adjust the **Offset** so the effect is positioned at the suppressor's muzzle end.
4. **Reset On Fire** : Disable this parameter to prevent the effect from resetting after each shot, creating a continuous smoke effect.

#### Configure Attachment Obstruction (Optional)

Configuring attachment obstruction ensures that certain attachments cannot be used simultaneously and that some attachments require others to be present. The **SCR\_WeaponAttachmentObstructionAttributes** is already inherited from the parent suppressor prefab, so you do not need to add it manually.

To configure obstruction attributes:

1. **Navigate to SCR\_WeaponAttachmentObstructionAttributes:**
   * In your prefab's **Custom Attributes** , locate the **SCR\_WeaponAttachmentObstructionAttributes** entry.
2. **Adjust Parameters as Needed:**
   * **Attachment Type**
     + Class of attachment which following rules applies to
   * **Obstructed Attachment Types:**
     + List attachment types that cannot be mounted at the same time as this suppressor.
     + For example, you might list muzzle devices or bayonets that are incompatible with the suppressor.
   * **Required Attachment Types:**
     + List attachment types that must be present for this suppressor to be attachable.
     + For instance, if your suppressor requires a specific muzzle adapter or flash hider, include that attachment type here.
     + *Example:* An **M9 Bayonet** requires the **A2 Flash Hider** to be mounted first, as it attaches to the flash hider rather than directly to the barrel.

By configuring these attributes, you control which attachments can be used together and establish dependencies between attachments. This ensures that weapon customization behaves realistically and prevents players from combining incompatible attachments.

[![](/wikidata/images/7/79/armareforger-new-weapon-suppressor-m16-suppressor-obstruction.png)](/wiki/File:armareforger-new-weapon-suppressor-m16-suppressor-obstruction.png)

Example M16 suppressor obstruction setup

#### Adjust ActionsManagerComponent (Optional)

Depending on how your suppressor is positioned, you might need to adjust the positions of user and muzzle actions:

1. Select the **ActionsManagerComponent** .
2. Within the **Action Context** , locate the user and muzzle actions.
3. Modify their **Offset** in the **Position** property to align them correctly with the suppressor's position

[![armareforger-new-weapon-suppressor-action-context-adjustment.gif](/wikidata/images/thumb/2/2e/armareforger-new-weapon-suppressor-action-context-adjustment.gif/913px-armareforger-new-weapon-suppressor-action-context-adjustment.gif)](/wiki/File:armareforger-new-weapon-suppressor-action-context-adjustment.gif)

📖

**Recommended read:** [**Action Context Setup**](/wiki/Arma_Reforger:Action_Context_Setup "Arma Reforger:Action Context Setup") documentation

### Localization

Lastly, it's recommended to localize your suppressor asset:

* This ensures the suppressor's name is displayed correctly in the inspection menu and other UI elements.
* Localize the **Item Display Name** using instruction listed on [Arma Reforger:Mod Localisation](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation") page.

[![](/wikidata/images/9/97/armareforger-new-weapon-suppressor-missing-localisation.png)](/wiki/File:armareforger-new-weapon-suppressor-missing-localisation.png)

Typical **#AR-UserAction\_Dettach** error when accessory is not localized

## Integrating the Suppressor with a Weapon

### Configure the Weapon's Muzzle Component

To enable your weapon to accept and properly function with your newly created suppressor, you need to configure the weapon's components accordingly.

### Configuring the Weapon's Muzzle Component

First, open your weapon prefab (**SampleWeapon\_01\_base.et** ) in **[Prefab Edit Mode](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics")** .

#### Add an AttachmentSlotComponent

1. **Select the MuzzleComponent:**
   * In the **Entity Tree** , expand your weapon entity and locate the **MuzzleComponent**.
2. **[Add AttachmentSlotComponent as Child Component of Muzzle Component](/wiki/Arma_Reforger:Prefabs_Basics#Adding_child_component "Arma Reforger:Prefabs Basics")**

#### Configure the AttachmentSlotComponent

With the **AttachmentSlotComponent** selected, configure its properties:

1. **Pivot ID:**
   * Set the **Pivot ID** to the weapon's muzzle slot.
     + This is typically `slot_barrel_muzzle`.
   * This ensures the attachment point aligns with the muzzle end of the weapon.
2. **Child Pivot ID:**
   * Set the **Child Pivot ID** to `snap_weapon`.
   * This specifies where the attachment (suppressor) will snap onto the weapon. This bone/empty object should exist in attachment itself. If its not prevent, then Scene Root will be used
3. **Attachment Point Type:**
   * Assign the attachment point type to match your suppressor's attachment class.
     + Click on the **Attachment Point Type** property.
     + Select **Change Class** from the context menu.
     + Search for muzzle class appropriate for the weapon - in this case it is`AttachmentMuzzle65_39` (the class you created earlier).
     + Select it to assign it to the attachment point.
4. **Show in Inspection:**
   * Enable the **Show in Inspection** option.
   * This allows the attachment slot to be visible when inspecting the weapon in-game.

#### Add SCR\_WeaponStatsManagerComponent

To ensure the weapon correctly handles parameter adjustments and sound switching when the suppressor is attached, you need to add the **SCR\_WeaponStatsManagerComponent**.

1. **[Add SCR\_WeaponStatsManagerComponent Component](/wiki/Arma_Reforger:Prefabs_Basics#Adding_new_components_to_entity_instance "Arma Reforger:Prefabs Basics")**
2. **No Additional Configuration Needed:**
   * The **SCR\_WeaponStatsManagerComponent** doesn't have any parameters other than **Enabled** .
   * Simply adding this component is sufficient.
   * This component is crucial for handling parameter adjustments and sound switching when attachments like suppressors are used.

ⓘ

The **SCR\_WeaponStatsManagerComponent** manages changes to the weapon's stats and sounds when attachments are added or removed. For more detailed information, you can refer to the [Weapon Stats Modifying Attachments](/wiki/Arma_Reforger:Weapon_Stats-Modifing_Attachments "Arma Reforger:Weapon Stats-Modifing Attachments") documentation.

### Updating the WeaponSoundComponent

To ensure that the weapon produces the correct sounds when the suppressor is attached, you need to update the **WeaponSoundComponent**.

1. **Select the WeaponSoundComponent:**
   * In the **Entity Tree**, locate and select the **WeaponSoundComponent**.
2. **Add Suppressed Sounds:**
   * In the **Properties** panel, find the **Filenames** property.
   * **Add the Suppressed Firing Sound:**
     + Click on the **Add Element** button (often a **+** icon) next to the **Filenames** array to add a new sound file.
     + Assign the suppressed firing sound file to this new element.
       - Choose an appropriate sound file (e.g., **[Weapons\_Rifles\_AK-74\_Shot\_Suppressed\_SuperSonic.acp](enfusion://ResourceManager/$ArmaReforger:Sounds/Weapons/Rifles/AK-74/Weapons_Rifles_AK-74_Shot_Suppressed_SuperSonic.acp)**).
   * **Configure the Filenames:**
     + Ensure that both the normal and suppressed firing sounds are included in the **Filenames** array.
     + The **WeaponSoundComponent** will automatically handle the sound selection based on the weapon's state and attachments.

ⓘ

The **WeaponSoundComponent** uses the **Filenames** array to determine which sounds to play when the weapon is fired. It will automatically switch between the normal and suppressed firing sounds depending on whether the suppressor is attached. The order of the sound files in the **Filenames** array does not affect which sound is played. What matters, is presence of **SOUND\_SUPPRESSED\_SHOT** node.

## Making the Suppressor Available in the Arsenal

To make your suppressor more accessible to players in-game, you need to add it to the arsenal. This involves including the suppressor prefab in the **Entity Catalog** so that it appears in the game's inventory and supply systems.

* **Include in Entity Catalog:**
  + Ensure that your suppressor prefab is added to the **Weapon Attachments** **Entities** list within the **Entity Catalog** of selected faction that you want to expose your weapon.
    - **Item Type** should be set to **WEAPON\_ATTACHMENT**
    - **Item Mode** should be set to **ATTACHMENT**
    - Adjust **Supply Cost** in line with vanilla
  + This allows the game to recognize the suppressor as available equipment.
* **Access in Game:**
  + After adding it to the **Entity Catalog** , the suppressor will be available in-game for players to acquire and use.
  + Players can find the suppressor in arsenals or supply crates of given faction that draw from the **Entity Catalog** .

[![armareforger-new-weapon-suppressor-arsenal-setup.png](/wikidata/images/4/46/armareforger-new-weapon-suppressor-arsenal-setup.png)](/wiki/File:armareforger-new-weapon-suppressor-arsenal-setup.png)

For detailed instructions on how to add items to the **Entity Catalog** and configure arsenals, please refer to the [Crate Filling](/wiki/Arma_Reforger:Weapon_Creation/Prefab_Configuration#Crate_Filling "Arma Reforger:Weapon Creation/Prefab Configuration") section of the **Weapon Creation** documentation.

## Finalizing the Integration

After configuring the components and making the suppressor available in the arsenal:

* **Save All Prefabs:**
  + Ensure that all changes to prefabs are saved.
* **Test Thoroughly:**
  + Load your mod in the game or editor.
  + Verify that:
    - The suppressor appears in the arsenal.
    - It can be added to the player's inventory.
    - It attaches correctly to the weapon.
    - The weapon functions correctly with and without the suppressor.
* **Troubleshoot If Necessary:**
  + If you encounter issues, double-check the steps above to ensure all configurations are correct

* [![armareforger-new-weapon-suppressor-final-result-1.png](/wikidata/images/thumb/9/94/armareforger-new-weapon-suppressor-final-result-1.png/1168px-armareforger-new-weapon-suppressor-final-result-1.png)](/wiki/File:armareforger-new-weapon-suppressor-final-result-1.png)
* [![armareforger-new-weapon-suppressor-final-results-2.png](/wikidata/images/thumb/d/dc/armareforger-new-weapon-suppressor-final-results-2.png/1203px-armareforger-new-weapon-suppressor-final-results-2.png)](/wiki/File:armareforger-new-weapon-suppressor-final-results-2.png)
