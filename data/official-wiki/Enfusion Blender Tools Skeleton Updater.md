# [Enfusion Blender Tools: Skeleton Updater](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Skeleton_Updater)

## Description

Skeleton updater tool allows to update existing character armatures on character items or models to a new standard introduced in 1.2.1 version of AR.

Enfusion engine expects that characters and their equipment use exactly same amount of bones and if that condition is not fulfilled, then animations on the gear will cease to work.

Tool itself removes all children bones of head, appends new head bones from template blend file and then also unifies the bone transformation on all remaining bones (some of the bones changed their location too).

⚠

**Do not use this tool for updating animation rig!** For this there is separate tool - [Rig Updater](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Rig_Updater "Arma Reforger:Enfusion Blender Tools: Rig Updater")

## Usage

### Settings

Armature that is used by **Skeleton Updater Tool** is defined in Enfusion Blender Tool settings in TXA subsection. Select [**Character\_Weights\_Template.blend**](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/blob/main/SampleMod_NewCharacter/Assets/Characters/SampleCharacter/Character_Weights_Template.blend) as a template which skeleton updater is going to use

ⓘ

Tip: Disable "Relative path" setting if you have troubles with selecting blend file

[![armareforger-blender-skeleton-updater-settings.png](/wikidata/images/a/af/armareforger-blender-skeleton-updater-settings.png)](/wiki/File:armareforger-blender-skeleton-updater-settings.png)

If this field is empty or invalid and you will try to use **Skeleton Updater Tool**, you will be asked to provide correct path. If its correct, then it wil be stored in EBT settings

### From Blender

1. Select **Armature object** that is supposed to be updated
2. In Enfusion Tools tab, navigate to **Animation Tools** and click on **Update Skeleton** button
   1. If link to blend file containing reference was not defined in Enfusion Tools settings they you will be prompted to select correct blend file (*see settings section*)

### Using batch file

1. Locate **Enfusion Blender Tools** script location in Windows Explorer and find inside txa/bake folder **updateSkeleton.bat** file
   1. Example location: ***%appdata%\Blender Foundation\Blender\4.2\scripts\addons\EnfusionBlenderTools\txa\batch***
2. Drag and drop fbx files to update skeletons
