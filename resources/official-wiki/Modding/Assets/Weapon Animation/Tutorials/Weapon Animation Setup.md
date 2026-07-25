# [Weapon Animation/Setup](https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Animation_Setup)

This part of the tutorial covers the basics of setting up the animation workspace. You might expect to find information on how to create an animation workspace using one of the existing workspaces as a base.
This means that this tutorial will not go into depth on creating new custom graphs, creating new nodes, and so on. In any case, following these instructions, you should be able to have a fully animated weapon in the game.

## Animation Workspace Preparation

### Workspace Creation

📖

**Recommended read:** Before going any further, it is recommended to make yourself familiar with [**Animation Editor documentation**](/wiki/Arma_Reforger:Animation_Editor "Arma Reforger:Animation Editor").

First step in creating animation for weapons is preparing new **animation workspace** for asset that is being worked on. Alternatively, it is also to **duplicate one of the existing workspaces** which can save a lot of time.

#### Duplicating existing workspace

If you choose to duplicate some existing workspace, start by opening workspace (via **File → Open Workspace** or **`Ctrl` + `O`** shortcut) which should be duplicated - in this case it is **[ak74.aw](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.aw)**.

[![armareforger-new-weapon-animation-duplicate-workspace-option.png](/wikidata/images/2/28/armareforger-new-weapon-animation-duplicate-workspace-option.png)](/wiki/File:armareforger-new-weapon-animation-duplicate-workspace-option.png)

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

After AK74 workspace is loaded, use **File → Duplicate Workspace** option. This should open new window like one below

[![armareforger-new-weapon-animation-duplicate-workspace.png](/wikidata/images/9/99/armareforger-new-weapon-animation-duplicate-workspace.png)](/wiki/File:armareforger-new-weapon-animation-duplicate-workspace.png)

Over there, select the **location and name of new workspace** in **Target workspace**. In Reforger, workspaces files for multiple similar assets are placed in asset folder one level higher than the asset is located in - f.e. **Assets/Weapons/Rifles/Workspaces**.
In this tutorial, the **sampleweapon\_01.aw** workspace was created in **Assets/Weapons/Rifles/Workspaces** inside **SampleMod\_NewWeapon** addon.

**Duplicate column** contains names of the new files, which will be created - change it so it matches the name of the weapon you are creating.

If you are not using [SVN repository](https://en.wikipedia.org/wiki/Apache_Subversion) , then uncheck **Add files to SVN** option and after that, click on **Duplicate** option. If everything went fine, new files should be created and the **new workspace should be now opened** in Animation Editor with exactly same data as source workspace.

##### Clearing duplicated workspace

[![](/wikidata/images/thumb/b/ba/armareforger-new-weapon-animation-incorrect-anim.png/300px-armareforger-new-weapon-animation-incorrect-anim.png)](/wiki/File:armareforger-new-weapon-animation-incorrect-anim.png)

M16 using AK74 animation - notice where the bolt is moved

Weapon animations (like bolt or trigger movement), such as those copied from **ak74\_weapon.asi** are unique to each weapon. If an asset doesn't use **exactly the same skeleton** as that weapon, then animations may misplace different parts of the weapon. Therefore it is recommended to **unassign animation**s from weapon animation instance - this way risk of some **vanilla animation breaking your weapon is avoided**.

It is possible to select multiple entries in animation instance and then clicking on that selection with **Right Mouse Button**. After that, select from the context menu option **Unassign animations -** this should clear assingment in all selected fields.

[![armareforger-new-weapon-animation-unassign-animations.png](/wikidata/images/1/1f/armareforger-new-weapon-animation-unassign-animations.png)](/wiki/File:armareforger-new-weapon-animation-unassign-animations.png)

After all those actions were completed, you can safely **skip to [Setting up preview models](/wiki/Arma_Reforger:Weapon_Animation/Setup#Setting_up_preview_models "Arma Reforger:Weapon Animation/Setup")** segment.

#### Creating new workspace

In case you want to start from more empty state, navigate to **File → New Workspace** in top bar or use the **`Ctrl` + `N`** shortcut.
Once that is pressed, **Create New Workspace** window where it is necessary to fill location where new workspace will be created. It is also important to check **Create empty** option - otherwise new workspace will create bunch of files which you cannot change anymore.

[![armareforger-new-weapon-animation-workspace-creation.png](/wikidata/images/0/08/armareforger-new-weapon-animation-workspace-creation.png)](/wiki/File:armareforger-new-weapon-animation-workspace-creation.png)

In Reforger, workspaces files for multiple similar assets are placed in asset folder one level higher than the asset is located in - f.e. **Assets/Weapons/Rifles/Workspaces**.
In this tutorial, the **sampleweapon\_01.aw** workspace was created in **Assets/Weapons/Rifles/Workspaces** inside **SampleMod\_NewWeapon** addon.

ⓘ

Vanilla Reforger assets are using **name of the weapon** as **name of various animation files**. For instance:

* [ak74.aw](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.aw) - Animation Workspace for AK74
* [ak74.ast](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.ast) - Animation Template for AK74
* [ak74.agr](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.agr) - Animation Graph File
* [ak74.agf](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.agf) - Animation Graph
* [ak74\_weapon.asi](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74_weapon.asi) - Animation Instance - Weapon part
* [ak74\_player.asi](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74_player.asi) - Animation Instance - Character part

Once location & name of the file are confirmed, an empty **sampleweapon\_01.aw** workspace should be automatically opened in **Animation Editor**.

[![armareforger-new-weapon-animation-animation-editor.png](/wikidata/images/thumb/0/0f/armareforger-new-weapon-animation-animation-editor.png/800px-armareforger-new-weapon-animation-animation-editor.png)](/wiki/File:armareforger-new-weapon-animation-animation-editor.png)

### Creating animation instance template (or reusing existing)

📖

**Recommended read**: [Animation Editor: Templates and Instances Tutorial](/wiki/Arma_Reforger:Animation_Editor:_Templates_and_Instances_Tutorial "Arma Reforger:Animation Editor: Templates and Instances Tutorial")

Next thing on the list will be configuration of **Animation Template**. That file contains [template animation set](/wiki/Arma_Reforger:Animation_Editor#Anim_Set "Arma Reforger:Animation Editor") - **animation groups, columns and rows** which are then used by all **Animation Instances** in current **Animation Workspace**.

Now, there are few possibilities when it comes configuring **Animation Template -** it is possible to **use existing template**, create **duplicate of it** or **create new template from scratch**. Both have its pros and cons and it all depends on various factors. If for instance new weapon is yet another variant of AK74, it might be possible to reuse existing animation template without any problems.

#### Creating new template

* ![Unchecked](/wikidata/images/thumb/f/f6/Ico_none.png/24px-Ico_none.png "Unchecked") All entries in animation template has to be set from scratch
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") New animation groups & rows can be added or deleted
* ![Warning](/wikidata/images/thumb/3/3b/Ico_warning.png/24px-Ico_warning.png "Warning") Need to be manually updated in case there is some change in vanilla date

[![armareforger-new-weapon-animation-creating-new-template.png](/wikidata/images/9/98/armareforger-new-weapon-animation-creating-new-template.png)](/wiki/File:armareforger-new-weapon-animation-creating-new-template.png)

To create new template, click with **Right Mouse Button** on **Animation Template** line in **Workspace** window and then select **New Template...** option from the menu. After that, you will be asked where new file should be created.

#### Using existing template

* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") Minimal amount of work required to get it working
* ![Unchecked](/wikidata/images/thumb/f/f6/Ico_none.png/24px-Ico_none.png "Unchecked") Cannot be modified - it is not possible to add new groups or rows. Might be problematic if weapon is quite non standard
* ![Warning](/wikidata/images/thumb/3/3b/Ico_warning.png/24px-Ico_warning.png "Warning") Creates dependency on vanilla content - if it gets updated it can break your animation workspace but sometimes it might help you updating your content to latest standards

[![armareforger-new-weapon-animation-using-existing-template.png](/wikidata/images/1/18/armareforger-new-weapon-animation-using-existing-template.png)](/wiki/File:armareforger-new-weapon-animation-using-existing-template.png)

To use existing Animation Template, click with Right Mouse Button on Animation Template line in Workspace window and then pick **Assign Existing Template...** option from the context menu. After picking this option, you will be asked to locate **Animation Template** file (*files using .ast extension*)

#### Using duplicated template

* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") Relatively easy to set up but requires few more steps compared to using existing Animation Template
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") New animation groups & rows can be added or deleted
* ![Warning](/wikidata/images/thumb/3/3b/Ico_warning.png/24px-Ico_warning.png "Warning") Need to be manually updated in case there is some change in vanilla date

In this tutorial, a **duplicated animation template is used**, since **Sample New Weapon** is going to use logic similar to already existing in-game weapon - AK74. To do so, follow below steps:

1. Navigate to **Animation Template** that you want to duplicated. **Animation Templates** are using .ast extension. In this case [ak74.ast](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.ast) was used
2. Use [**Duplicate to addon**](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function_Duplicate_to_addon "Arma Reforger:Data Modding Basics") functionality on that file and type your desired name - i.e. *sampleweapon\_01*

[![armareforger-new-weapon-animation-duplicate-template.gif](/wikidata/images/f/f7/armareforger-new-weapon-animation-duplicate-template.gif)](/wiki/File:armareforger-new-weapon-animation-duplicate-template.gif)

After those steps are completed, it is possible to use this template in **sampleweapon\_01.aw** workspace. To do so, follow the instructions mentioned in **using existing template paragraph** and select **sampleweapon\_01.ast** when asked for existing template.

### Creating animation instance for player & weapon

Once animation template is configured, it is right time to move to preparing **Animation Instances**. As mentioned in [Animation Editor](/wiki/Arma_Reforger:Animation_Editor#Workspace_Animation_Editor "Arma Reforger:Animation Editor") documentation, **Animation Instances** are unique sets of animations which share same logic.

Weapons are using two animation instances - one for **player** and one for **weapon**. **Player animation instance** is applied to character which is holding the weapon and **weapon animation instance** is applied to the weapon itself. When character is holding a weapon, both of those sets are played simultaneously, therefore it is quite important to keep both animations with same length so they stay synchronised. Some more info about power instances can be found on [Animation Editor: Templates and Instances Tutorial](/wiki/Arma_Reforger:Animation_Editor:_Templates_and_Instances_Tutorial "Arma Reforger:Animation Editor: Templates and Instances Tutorial") page (it is a recommended read nevertheless!)

⚠

Please note that not all animations have to be synchronised between instances but those are **exceptions!**

[![](/wikidata/images/8/8b/armareforger-new-weapon-animation-new-animation-instance.png)](/wiki/File:armareforger-new-weapon-animation-new-animation-instance.png)

Adding new animation instance to workspace

Creating new animation instances is fairly straightforward and involves the following actions:

* Click with **Right Mouse Button** on **Animation Instances** field in the **Workspaces** window
* Select **New Animation Instance...** option from the context menu
* Select new animation instance **name & location** and then confirm with **Ok** button
  + It is recommended to use \_*player & \_weapon* suffixes for animation instances to keep data readable (it is not mandatory though)

As it was mentioned before, weapon requires two animation sets and in case of sample weapon, following animation instances were created:

* **sampleweapon\_01\_player.asi**  - animation instance containing character related animations
* **sampleweapon\_01\_weapon.asi**  - animation instance containing weapon related animations

Both sets of animations are empty at this stage, and we will come back to fill them in as soon as we have some animations ready to use.

[![](/wikidata/images/7/76/armareforger-new-weapon-animation-anim-sets-window.png)](/wiki/File:armareforger-new-weapon-animation-anim-sets-window.png)

**Anim Sets** inside **Sample New Weapon** animation workspace

### Setting animation graph

Animation Graph consist of **Graph (.agr)** and **Graph File (.agf)**.
**Graph** itself contains all **Variables**, **Commands** and **IK Chains** that would be available to all animations graph plus link to **Graph File(s)**.

**Graph File** contain **Sheets** and those sheets contain actual **animation graph** with [Animation Nodes](/wiki/Arma_Reforger:Animation_Editor:_Nodes "Arma Reforger:Animation Editor: Nodes").

Similar as with animation template, there are few possibilities when it comes to graph creation and again, same as with **Animation Template,** there are some pros and cons of such solution. If you intend to create a weapon that is similar to one of the existing weapons, then duplicating the animation graph is the suggested approach - unless the weapon you are preparing doesn't require any tweaks in the graph compared to the vanilla data.

Of course, if weapon is sort of **unique** (let's say bolt action rifle or some clip loaded rifles) then **new graph might be necessary** - in such scenario though it would be still wise to use existing weapon graph as reference.

#### Creating new graph

[![armareforger-new-weapon-animation-new-graph.png](/wikidata/images/c/c1/armareforger-new-weapon-animation-new-graph.png)](/wiki/File:armareforger-new-weapon-animation-new-graph.png)

To create new graph, click with **Right Mouse Button** on **Graph** line in **Workspace** window and then select **New Graph...** option from the menu. After that, you will be asked where new file should be created.

#### Using existing graph

[![armareforger-new-weapon-animation-existing-graph.png](/wikidata/images/3/36/armareforger-new-weapon-animation-existing-graph.png)](/wiki/File:armareforger-new-weapon-animation-existing-graph.png)

To create new graph, click with **Right Mouse Button** on **Graph** line in **Workspace** window and then select **Assign Existing Graph...** option from the menu. After picking this option, you will be asked to locate **Graph** file (*files using .agr extension*)

#### Duplicating existing graph

Process of duplicating **Graph** is exactly same as with **Animation Template** described before. Similar as with animation template, **duplication of existing graph was chosen** - mainly to save time required to define **variables** and mandatory **commands**.
Even if you are doing some custom weapon In this case, [ak74.agr](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.agr) was picked to be duplicated into **sampleweapon\_01.agr**.

Once duplication is completed and new graph is assigned to **Graph** property, it will be necessary to remove reference to old **Graph File** and this can be done by clicking with **Right Mouse Button** on **Files** entry ([ak74.agf](enfusion://ResourceManager/~ArmaReforger:Assets/Weapons/Rifles/workspaces/ak74.agf) in this case*) in **Graph** section of the Workspace and then selecting **Remove** option from the context menu.*

⚠

Proceed with caution when doing changes in text editor!

After duplication of .agr file, it will be necessary to change **AnimSetTemplate** property inside of that new .agr via **text editor** of your choice - it can be even notepad. Open **Animation Workspace (.aw)** in text editor, locate **AnimSetTemplate** and copy it. Next, open .agr file in text editor and replace **AnimSetTemplate** line by the one copied from animation workspace. Once its done, save file and restart workbench.

[![armareforger-new-weapon-animation-edit-graph.png](/wikidata/images/d/d6/armareforger-new-weapon-animation-edit-graph.png)](/wiki/File:armareforger-new-weapon-animation-edit-graph.png)

### Creating Graph File

[![armareforger-new-weapon-animation-creating-graph.png](/wikidata/images/c/ca/armareforger-new-weapon-animation-creating-graph.png)](/wiki/File:armareforger-new-weapon-animation-creating-graph.png)

Process of creating **Graph File** is almost exactly the same as with graph or template. In most cases, **duplication** is still the most recommended method.

In case you want to make new file from scratch, new **Graph File** can be created through context menu which is available after clicking with **Right Mouse Button** on **Files** property in **Graph** section. Over there, you can either select **Create Graph File..**. or **Add Existing Graph File...** options. Keep in mind

[![](/wikidata/images/6/6e/armareforger-new-weapon-animation-new-sheet.png)](/wiki/File:armareforger-new-weapon-animation-new-sheet.png)

New sheet creation

ⓘ

In case of creating new **Graph File**, it is necessary to **create new sheet** - this can be done by clicking on **Graph File** with Right Mouse Button and then selecting **New Sheet..**. option from the context menu. After that, a new pop will appear asking for sheet name. In this case, it is recommend to use "Master" for main sheet

#### Setting graph file

**Duplication** approach can be also used to create a **Graph File** and since it is in many cases the **easiest and fastest way**, this solution was used to configure **Sample Weapon** animation workspace.

Perhaps it is worth mentioning here, that it is also possible to copy and paste animation nodes between Animation Graphs. To do so, open second instance of **Animation Editor** and then open **Animation Workspace** of your choice. Once it is loaded, select all nodes in **Animation Graph** window with mouse by holding ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button")which you want to copy paste, press **`Ctrl` + `C`** to copy those nodes**,** select again **Animation Editor** containing your weapon, click once with **Left Mouse Button** in **Animation Graph** inside of that **Animation Editor** and finally, press **`Ctrl` + `V`** to paste all those nodes.

[![armareforger-new-weapon-animation-copy-paste-graph.gif](/wikidata/images/thumb/a/a8/armareforger-new-weapon-animation-copy-paste-graph.gif/1800px-armareforger-new-weapon-animation-copy-paste-graph.gif)](/wiki/File:armareforger-new-weapon-animation-copy-paste-graph.gif)

### Setting up preview models

It might be also quite handy to set **Preview Models** in current animation workspace - this will allow to quickly verify if animations are playing correctly and see if interpolations between nodes are working as intended in graph debug mode. Only **one preview model** can be active in **Anim Editor Preview** window**,** so it will be necessary to set two sets of preview models - one for debugging of **weapon animation instance** and second for debug of **player animation instance.**

#### Weapon preview models

Beginning with setting preview model for weapon, following steps has to be performed to add:\* Click with **Right Mouse Button on Preview Models** line in **Workspace** window and then select **Add Preview Model...** option.  After that a proper model (.xob) has to be selected. In this case [SampleWeapon\_01.xob](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/SampleWeapon_01/SampleWeapon_01.xob) was selected.[![armareforger-new-weapon-animation-add-preview-model.png](/wikidata/images/f/f1/armareforger-new-weapon-animation-add-preview-model.png)](/wiki/File:armareforger-new-weapon-animation-add-preview-model.png)

✩

**Tip**: It is possible to disable background ground model in **Anim Editor Preview** window by clicking on **Options** in top right corner of the view port and then unchecking **Ground property** in **Scene settings** tab.

Next, it might be nice to have (but not essential) to add magazine to the weapon by adding child model, to do so, click with **Right Mouse Button** on **entry** in **Preview Models** list (*f.e. [SampleWeapon\_01.xob](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/SampleWeapon_01/SampleWeapon_01.xob)),* then select from context menu option **Add Child Model...** and finally, pick model of the magazine ([Magazine\_65x39c\_SampleWeapon\_01\_30rnd.xob](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Magazines/SampleWeapon_01/Magazine_65x39c_SampleWeapon_01_30rnd.xob)) from the list.

[![armareforger-new-weapon-animation-add-child-model.png](/wikidata/images/0/0b/armareforger-new-weapon-animation-add-child-model.png)](/wiki/File:armareforger-new-weapon-animation-add-child-model.png)

By default, child model will be created at 0,0,0 and most likely such state in desired. It is possible to attach such child model to some bone in parent bone by changing **Bone** parameter in **Properties** window. In case of the magazine, it is possible to snap it to **slot\_magazine** bone, which should result in model nicely sitting in magazine well.

[![armareforger-new-weapon-animation-set-child-bone-preview.gif](/wikidata/images/thumb/b/b4/armareforger-new-weapon-animation-set-child-bone-preview.gif/1800px-armareforger-new-weapon-animation-set-child-bone-preview.gif)](/wiki/File:armareforger-new-weapon-animation-set-child-bone-preview.gif)

✩

**Tip**: Magazine model visibility can be toggled clicking on **Enabled** property in **Properties** window when magazine in **Preview Models** list is selected or by clicking on little **eye icon** which is visible when mouse is hovering above **child preview model**.

#### Character preview model

Adding of character preview is using basically same principle as weapon but this time there will be two child models attached to it. Starting with character model itself, click with **Right Mouse Button** on **Preview Models** field and then select **Add Preview Model...** option. After that, you can pick one of the characters models in xob model. Vanilla data contains two models with some equipment merged in:

* [AnimTestChar\_USSR\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Animation/AnimTestChar_USSR_01.xob) - character with USSR uniform
* [AnimTestChar\_US\_01.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Characters/Animation/AnimTestChar_US_01.xob) -  character with US Army uniform

Once character model is present in Preview Models list, it is possible to add two child models:

* Weapon ([SampleWeapon\_01.xob](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/SampleWeapon_01/SampleWeapon_01.xob)) attached to **Right Hand Prop**
* Magazine ([Magazine\_65x39c\_SampleWeapon\_01\_30rnd.xob](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Magazines/SampleWeapon_01/Magazine_65x39c_SampleWeapon_01_30rnd.xob)) attached to **Left Hand Prop -** this should allow for previewing reload magazine animations.

Currently active preview models can be either switched in two ways:\* Through context menu which appears after clicking on it with **Right Mouse Button** entry in **Preview Models**list and then selecting **Set as current** option

* By double clicking on it

Active preview model is marked with **bold text**.

[![armareforger-new-weapon-animation-set-preview-active.gif](/wikidata/images/2/2e/armareforger-new-weapon-animation-set-preview-active.gif)](/wiki/File:armareforger-new-weapon-animation-set-preview-active.gif)

Above set of preview models should be enough to preview all necessary weapon animations (both character and weapon itself) once **Animation Workspace** is filled with actual animations.

## Assigning animation workspace to prefab

[![](/wikidata/images/thumb/3/37/armareforger-new-weapon-animation-component-setup.png/328px-armareforger-new-weapon-animation-component-setup.png)](/wiki/File:armareforger-new-weapon-animation-component-setup.png)

Content of **WeaponAnimationComponent**

If everything went fine, the new weapon animation workspace should be ready to be plugged into the weapon.

Most of the weapon animation configuration is located in **WeaponAnimationComponent.** Unless weapon prefab is duplicate of some existing weapon, such component might not exist in a prefab which just inherits from [Rifle\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/Core/Rifle_Base.et) prefab. In this case, it will be necessary to add such component to the prefab first. This can be done in few steps:\* Load weapon prefab ([SampleWeapon\_01\_base.et](enfusion://ResourceManager/~SampleMod_NewWeapon:Prefabs/Weapons/Rifles/SampleWeapon_01_base.et)) in World Editor - either by using **Edit Prefab** button in **Resource Manager** or by drag and dropping weapon prefab into World Editor view port

* Locate **WeaponComponent** in **Object Properties** window of the weapon
* Click on **WeaponComponent** with **Right Mouse Button** and select **Add child component** option from the context menu
* Select **WeaponAnimationComponent** from the list of components

After that, **WeaponAnimationComponent** should be ready for further configuration. If component

### Setting weapon animation instance

First, let's begin with general weapon animations and weapon graph & animation instance. Those two things are controlled by following parameters which can be found in **WeaponAnimationComponentn**\* **Anim Graph** →  This parameter expects **animation graph** used in animation workspace. In this case [sampleweapon\_01.agr](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/Workspaces/sampleweapon_01.agr) was used

* **Anim Instance** → This parameter expects **animation instance** of the **weapon**. Over here [sampleweapon\_01\_weapon.asi](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/Workspaces/sampleweapon_01_weapon.asi) should be assigned

It is also necessary to switch those two parameters to on, to ensure proper working of the component\* **Always Active**

* **Bind With Injection**

### Setting player animation instance

Next, we can move to character animations configuration and to do so, click on **set class** button next to **Anim Injection** property.
This will add new class to the component where you can configure following options:

* **Anim Graph**  → Same as before, this parameter expects **animation graph** used in animation workspace. In this case [sampleweapon\_01.agr](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/Workspaces/sampleweapon_01.agr) was used again
* **Animation Instance**  → This parameter expects **animation instance** of the **character**. Over here [sampleweapon\_01\_player.asi](enfusion://ResourceManager/~SampleMod_NewWeapon:Assets/Weapons/Rifles/Workspaces/sampleweapon_01_player.asi) should be assigned
* **Binding Name**  → Binding name in root graph. Type here **Weapon**

That's it for now! Now we are going to leave the **Animation Editor** for a while and move on to the **creation of the actual animations**.

## Continuation

ⓘ

**Next part** - [Weapon Animation Basic Creation](/wiki/Arma_Reforger:Weapon_Animation_Basic_Creation "Arma Reforger:Weapon Animation Basic Creation").
