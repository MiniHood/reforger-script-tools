# [Data Modding Basics](https://community.bistudio.com/wiki/Arma_Reforger:Data_Modding_Basics)

## Content Moddability Overview

## Can Be Replaced

This means that certain resource can be fully replaced. Usually such resources are created manually by creating same asset with same GUID

ⓘ

**Example**:
Variant of *Model.xob* from ArmaReforger is created in Addon1 via ***Override in "Addon1" addon...***. This new file share same GUID of original file and can contain completely different mesh

## Can Be Modified

Certain elements of the resource can be modified while rest of them is kept

ⓘ

**Example**:
Vehicle\_Base.et is copied from ArmaReforger to Addon1 with exactly same GUID as original file via ***Override in "Addon1"*** action. Copy of Vehicle\_Base.et, located in Addon1, contain only changes introduced in this addon and those changes are later applied to original prefab.

## Can Be Inherited From

A new resource can inherit from an existing file:

ⓘ

**Example:**
Inherited version of ArsenalConfig\_BLUEFOR.conf is created *via* ***Create Inherited File*** or ***Inherit in "Addon1"*** option. This new file has unique ID and inherits all data from its parent.

## Moddability table

| Extension | File Type | Can be replaced | Can be modified | Can be inherited from |
| --- | --- | --- | --- | --- |
| **.et** | Prefab | Warning[[1]](#cite_note-note_table_override-1) | Checked | Checked |
| **.conf** | Config | Warning[[1]](#cite_note-note_table_override-1) | Checked | Checked |
| **.xob** | Model | Checked | Unchecked | Unchecked |
| **.emat** | Material Definition | Warning[[1]](#cite_note-note_table_override-1) | Checked | Checked |
| **.edds** | Enfusion DDS Texture | Checked | Unchecked | Unchecked |
| **.c** | Enforce Script Source | Warning[[2]](#cite_note-note_table_1-2) | Warning[[3]](#cite_note-note_table_2-3) | Checked |
| **.layout** | Layout Definition | Warning[[1]](#cite_note-note_table_override-1) | Checked | Checked |
| **.ptc** | Particle System Definition | Warning[[1]](#cite_note-note_table_override-1) | Checked | Unchecked |
| **.ent** | World Scene | Checked | Unchecked | Unchecked |
| **.layer** | World Layer | Unchecked | Unchecked | Unchecked |
| **.bt** | Behavior tree | Checked | Warning[[4]](#cite_note-note_table_3-4) | Unchecked |
| **.anm** | Animation file | Checked | Unchecked | Unchecked |
| **.agf** | Animation graph file | Checked | Unchecked | Unchecked |
| **.agr** | Animation graph | Checked | Checked | Unchecked |
| **.aw** | Animation Workspace | Checked | Unchecked | Unchecked |
| **.asi** | Animation Set Template | Checked | Unchecked | Unchecked |
| **.pap** | Procedural Animation | Checked | Unchecked | Unchecked |
| **.siga** | Procedural Animation Signal | Checked | Unchecked | Unchecked |
| **.st** | String Table | Checked | Unchecked | Unchecked |
| **.acp** | Audio component | Checked | Unchecked | Unchecked |
| **.sig** | Signal (Audio) | Checked | Unchecked | Unchecked |
| **.wav** | Supported audio file format | Checked | Unchecked | Unchecked |

### Footnotes

1. ↑ [Jump up to: 1.0](#cite_ref-note_table_override_1-0) [1.1](#cite_ref-note_table_override_1-1) [1.2](#cite_ref-note_table_override_1-2) [1.3](#cite_ref-note_table_override_1-3) [1.4](#cite_ref-note_table_override_1-4) Following assets are always overridden and cannot be fully replaced
2. [↑](#cite_ref-note_table_1_2-0 "Jump up") Script files are fully replaced if script in other addon (*filesystem)* has same path as the one which is supposed to be replaced. For example **AIAutotest.c** located in *$ArmaReforger:scripts\Game\AI\* will be replaced by variant from **Addon1** if it is placed in same location and same name in **Addon1** file system - *$Addon1:scripts\Game\AI\ AIAutotest.c*
3. [↑](#cite_ref-note_table_2_3-0 "Jump up") Usage of *modded* keyword is required + few other restrictions apply like method in class must not be private in order to allow overloading it
4. [↑](#cite_ref-note_table_3_4-0 "Jump up") Behavior trees attempts to override each other, instead of replace

## Data manipulation

## Basics

In principle, any file with meta file and unique **[GUID](/wiki/Arma_Reforger:Workbench_Metadata#GUID "Arma Reforger:Workbench Metadata") -** *global unique identifier -* can be be overridden or replaced . This means however, that files without meta file have to be modified in different way (scripts ) or cannot be replaced at all.

That means that for instance **models, textures, layout, prefabs, configs or animations can be replaced using this method.**

ⓘ

When replacing assets, engine cares **only about [GUID](/wiki/Arma_Reforger:Workbench_Metadata#GUID "Arma Reforger:Workbench Metadata") -** that means neither **file name** or **location** doesn't have to be same as source file.

Creating overrides, duplicates or inherited files can be performed from context menu which is available in **Resource Browser**. This menu is visible after clicking with **Right Mouse Button** on selected asset. Availability of the actions depends on the type of the assets, so for instance models (*XOBs)* have only *Navigate to ...* options.

[![armareforger-data-modding-context-menu.png](/wikidata/images/e/ef/armareforger-data-modding-context-menu.png)](/wiki/File:armareforger-data-modding-context-menu.png)

|  |  |  |  |  |
| --- | --- | --- | --- | --- |
|  | **Override** | **Replacement** | **Duplicate** | **Inherit** |
| **Description** | Override creates a new file which shares same [GUID](/wiki/Arma_Reforger:Workbench_Metadata#GUID "Arma Reforger:Workbench Metadata") as original asset. This action allows to selectively replaces or add new elements to selected asset. For instance it is possible to change value  **It is not possible to remove elements from the overridden asset this way.**  This action can be performed by selecting **"Override in '...'"** action from the **Addons** menu. | Replacement, as name suggest, completely replace original file with the new file. [armareforger-data-modding-resource-variant.png](/wiki/File:armareforger-data-modding-resource-variant.png) In **Particle Editor**, it is possible to use "**Create variant for '...' addon..."** action but beside that, in all other cases it user has to create duplicates on his own by ensuring that replacing file has same GUID as original one.  This can be done by either copy pasting meta file (if we have access to it) or changing GUID of asset in text editor, so it matches the original file GUID.  **Prefabs**, **configs** and **layouts cannot be replaced** - they will be always in override mode. | Duplication creates a new file, with new, unique GUID. Duplicated file contains all the data from the original file (including inheritance) but doesn't modify in any way original file. This action can be performed by selecting **"Duplicate in '...'"** action from the **Addons** menu. | Inherit action creates a new child file which inherits all the data from the parent. This action can be performed by selecting **"Inherit in '...'"** action from the **Addons** menu.  Inheritance of **prefabs** can be only performed in **Resource Browser** attached to **World Editor** |
| **Where it can be used** | Prefabs, configs, materials and layouts | Sounds, models, textures, animations, particle effects, world files or behavior trees | Prefabs, configs, materials or layouts | Prefabs**\***, configs or layouts |

ⓘ

When using [CLI parameter](/wiki/Arma_Reforger:Startup_Parameters#addons "Arma Reforger:Startup Parameters") to load addons, **working addon** is equal to the last addon listed in addons list.

*Example:*

```enforce
-addons addon2, addon1, NewAddon
```

*In above example **NewAddon** is current work addon*

## Overriding assets

### Using "Override in..." function

"**Override to..."** function can be used to modify (override) existing assets like configs, layouyts or prefabs.

[![armareforger-data-modding-override-action.gif](/wikidata/images/d/dd/armareforger-data-modding-override-action.gif)](/wiki/File:armareforger-data-modding-override-action.gif)

ⓘ

**Override to "..."** option is available only for prefabs, layouts or configs. If there is already override in **working addon**, this option will be missing from the context menu.
After this option is clicked, **Resource Browser** will create new overridden file in **working addon** (retaining original folder structure) and then navigate to **overridden instance** of selected asset.

### Using "Navigate to..." function

Once you have file in addon which is overriding some data, you can quickly navigate

#### Navigate to Original

[![armareforger-data-modding-navigate-original-action.gif](/wikidata/images/e/ea/armareforger-data-modding-navigate-original-action.gif)](/wiki/File:armareforger-data-modding-navigate-original-action.gif)

ⓘ

***Navigate to Original*** button is available in file context menu only if selected file overridden or replaced in some addon. If selected asset is not overridden in any currently loaded addon then this option will appear greyed out.
After this option is clicked, **Resource Browser** will navigate to **original instance** of selected asset.

#### Navigate to Override

[![armareforger-data-modding-navigate-override-action.gif](/wikidata/images/3/31/armareforger-data-modding-navigate-override-action.gif)](/wiki/File:armareforger-data-modding-navigate-override-action.gif)

ⓘ

***Navigate to Override*** button is available in file context menu only if selected file overridden or replaced in some addon. If selected asset is not overridden in any currently loaded addon then this option will appear greyed out.

After clicking that option, **Resource Browser** will navigate to the first **overridden instance** of selected asset.

In scenario where multiple addons are present and they are overriding same file, **Navigate to Override** can be used multiple times until last override is reached.

#### Navigate to Ancestor

[![armareforger-data-modding-navigate-ancestor-action.gif](/wikidata/images/2/25/armareforger-data-modding-navigate-ancestor-action.gif)](/wiki/File:armareforger-data-modding-navigate-ancestor-action.gif)

ⓘ

***Navigate to Ancestor*** button is available in file context menu only if selected file overridden or replaced in some addon. If selected asset is not overridden in any currently loaded addon then this option will appear greyed out.

After clicking that option, **Resource Browser** will navigate to the first **ancestor of overridden instance** of selected asset.

In the scenario where multiple addons are present and they are overriding the same file, **Navigate to Override** can be used multiple times until the last override is reached.

### Scripts

In Enfusion it's possible to modify already existing scripts by using some of special keyword

* **[modded](/wiki/Arma_Reforger:Object_Oriented_Programming_Advanced_Usage#Modding "Arma Reforger:Object Oriented Programming Advanced Usage") -** keyword used to modify existing scripting class
* **[override](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics#override "Arma Reforger:Object Oriented Programming Basics") -** keyword to override methods in modded classes
* **[super](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics#super "Arma Reforger:Object Oriented Programming Basics") -** allows you to invoke content of overridden method

#### Modded keyword

```enforce
modded class SCR_PlayerScoreInfo
{
}
```

#### Override keyword

```enforce
modded class SCR_ScoringSystemComponent : SCR_BaseGameModeComponent
{
	override void AddSuicide(int playerID) {
		{
		}
	}
```

#### Super keyword

```enforce
modded class SCR_ScoringSystemComponent : SCR_BaseGameModeComponent
{
	override void AddSuicide(int playerID)
	{
		super.AddSuicide(playerID);
		// Your example code below
		AudioSystem.PlaySound("{9BB653FF9E065943}Sounds/Animals/Bos_Taurus/Samples/bawl/bawl_0.wav");
	}
}
```

### Using "Duplicate to..." function

In **Resource Browser** you can quickly create new, duplicated file in project you are currently working on.
This file is exact copy of original file and will be used from that point when any part of the game will try use asset with this **GUID**.
Any changes that you are going to do to that file are going to be applied to that copy are not going to be propagated to it.

[![armareforger-data-modding-duplicate-action.gif](/wikidata/images/f/fc/armareforger-data-modding-duplicate-action.gif)](/wiki/File:armareforger-data-modding-duplicate-action.gif)

ⓘ

**Duplicate to "..."** option is available only for prefabs, layouts or configs.
After this option is clicked, **Resource Browser** will create new duplicated file - with name selected by the user - in **working addon** (retaining original folder structure) and then navigate to that duplicate.

## Inheriting

### Using "Inherit in..." function

Inheriting from the prefabs, config files or layouts can be performed via "**Inherit in...**" function. After clicking with **Right Mouse Button** on appropriate file and then selecting Inherit in function, a new pop up window will appear asking for a new file name. Once name is confirmed, a new file will be created in **working addon** with same folder structure as original file. As expected, all of the attributes will be **inherited from parent file**.

[![armareforger-data-modding-inheriting.png](/wikidata/images/8/8a/armareforger-data-modding-inheriting.png)](/wiki/File:armareforger-data-modding-inheriting.png)

### Using "Transfer to..." function

Transfer to function is a functionality for moving data & merging partially overridden files (like prefabs or configs) from current working addon to parent addon. Parent addon need to be unpacked in order to have this functionality working.

## Replacing assets

Replacing assets means that i.e. AK-74 model can be with a simple gray box and all prefabs which are referencing that AK-74 are going to use that new gray box from now. Such replacement can be performed with sounds, models, textures, animations, particle effects, world files or behavior trees.

[![armareforger-data-modding-resource-variant.png](/wikidata/images/6/67/armareforger-data-modding-resource-variant.png)](/wiki/File:armareforger-data-modding-resource-variant.png)

In **Particle Editor** replacement can be performed quite easily by using "**Create variant for "..." addon..**." function. This action creates new **ptc** file with same **GUID** and **content** as **original file**.

⚠

Extreme caution is advised when doing such replacement - tinkering with Workbench generated files in text editor might break those files if you don't know what you are doing

In all other cases, replacing files is slightly more complicated and requires operations outside of Workbench.

#### Getting file GUID

[![armareforger-data-modding-getting-guid.png](/wikidata/images/3/37/armareforger-data-modding-getting-guid.png)](/wiki/File:armareforger-data-modding-getting-guid.png)

First step towards creating such replacement would be obtaining GUID - this can be done via **Copy Resource GUID(s)** action, available in **context menu** after clicking on file with **Right Mouse Button.**

Once **GUID** is obtained, it can be manually typed into meta file of some newly created model. After that action is completed, restart of the **Workbench** might be necessary.

## Replacing script

Scripts can be replace each other simply when they are in same relative location with same name.

For instance, to replace [PickupAction.c](enfusion://ResourceManager/~ArmaReforger:scripts/Game/UserActions/PickupAction.c) located in base **ArmaReforger** data, you would need to create a new script in your addon in same location and with exact name.

```
MyAddon\scripts\Game\UserActions\PickupAction.c
```

If multiple addons are trying to replace same file, then the last one in the loading chain will be used.
