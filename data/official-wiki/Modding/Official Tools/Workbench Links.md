# [Workbench Links](https://community.bistudio.com/wiki/Arma_Reforger:Workbench_Links)

A [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools") link is a protocol link that allows to share a direct point to a resource/script/game world location.

⚠

Using a Workbench link requires registering the enfusion:// protocol in Windows, clicking the Register enfusion:// protocol button button in [Workbench Options > Workbench tab](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options").

## Format

A Workbench link is composed of the enfusion:// protocol followed by the Module name to open; eventually an addon ID, the relative file path and eventual parameters separated by a semicolon:

* enfusion://ResourceManager**/**~ArmaReforger:Configs/Factions/BLUFOR.conf
* enfusion://ScriptEditor**/**scripts/Game/Editor/Containers/UIInfo/SCR\_UIInfo.c**;**8
* enfusion://WorldEditor**/**worlds/arland/arland.ent**;**3458.4,34.5587,2820.21**;**-15.107,297.881,0**;**0.4375,1989,8,20**;**46247

| Module | Module Name | Base Link | Parameters |
| --- | --- | --- | --- |
| [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") | ResourceManager | enfusion://ResourceManager | N/A |
| [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") | ScriptEditor | enfusion://ScriptEditor | * file line number |
| [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") | WorldEditor | enfusion://WorldEditor | * camera's world position (x, y, z) * camera's angles (x, y, z) - angles in -180..+180 range, z is always 0 * daytime (in range 0..1), year, month, day * entity ID - to select a specific entity |
| The following modules do not support a file parameter - a link can only be used to open the associated editor. | | | |
| [Particle Editor](/wiki/Arma_Reforger:Particle_Editor "Arma Reforger:Particle Editor") | ParticleEditor | enfusion://ParticleEditor | N/A |
| [Animation Editor](/wiki/Arma_Reforger:Animation_Editor "Arma Reforger:Animation Editor") | AnimEditor | enfusion://AnimEditor |
| [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor") | AudioEditor | enfusion://AudioEditor |
| [Behavior Editor](/wiki/Arma_Reforger:Behavior_Editor "Arma Reforger:Behavior Editor") | BehaviorEditor | enfusion://BehaviorEditor |
| [String Editor](/wiki/Arma_Reforger:String_Editor "Arma Reforger:String Editor") | *Localization*Editor | enfusion://LocalizationEditor |
| [Procedural Animation Editor](/wiki/Arma_Reforger:Procedural_Animation_Editor "Arma Reforger:Procedural Animation Editor") | ProcAnimEditor | enfusion://ProcAnimEditor |

ⓘ

A link can be found prefixed with https://enfusionengine.com/api/redirect?to=;
the [Enfusion Engine website](https://enfusionengine.com) provides a redirection for platforms that do not see enfusion:// as a valid protocol (e.g Discord).

## Link Creation

### [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager")

Create a link by clicking on any resource in **Resource Browser** with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") then selecting **Copy Link** option from the context menu.

[![](/wikidata/images/thumb/7/70/armareforger-resource-manager-options-rm-link-create.jpg/300px-armareforger-resource-manager-options-rm-link-create.jpg)](/wiki/File:armareforger-resource-manager-options-rm-link-create.jpg)

Creating a link in Resource Browser

[![](/wikidata/images/thumb/9/98/armareforger-resource-manager-options-rm-link.jpg/800px-armareforger-resource-manager-options-rm-link.jpg)](/wiki/File:armareforger-resource-manager-options-rm-link.jpg)

Example: <enfusion://ResourceManager/~ArmaReforger:Assets/Props/Fabric/Flags/Flag_1_2.xob>  
 [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") opens the [Flag\_1\_2.xob](enfusion://ResourceManager/~ArmaReforger:Assets/Props/Fabric/Flags/Flag_1_2.xob) file

### [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")

Create a link by selecting from the top menu *Edit →* **Copy link option** or by using the `Ctrl` + `⇧ Shift` + `L` shortcut (can be changed in the [shortcuts options section](/wiki/Arma_Reforger:Resource_Manager:_Options#Shortcuts "Arma Reforger:Resource Manager: Options"))

[![](/wikidata/images/f/f0/armareforger-resource-manager-options-se-link-create2.jpg)](/wiki/File:armareforger-resource-manager-options-se-link-create2.jpg)

Creating a link in Script Editor

[![](/wikidata/images/thumb/b/b5/armareforger-resource-manager-options-se-link.jpg/800px-armareforger-resource-manager-options-se-link.jpg)](/wiki/File:armareforger-resource-manager-options-se-link.jpg)

Example: <enfusion://ScriptEditor/scripts/Core/proto/EnWorld.c;17>  
 [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") opens the [EnWorld.c](enfusion://ScriptEditor/scripts/Core/proto/EnWorld.c) file at line [17](enfusion://ScriptEditor/scripts/Core/proto/EnWorld.c;17)

### [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor")

Create a link by selecting from the top menu *Game →* **Copy view link** option or by using the `Ctrl` + `⇧ Shift` + `L` shortcut (can be changed in the [shortcuts options section](/wiki/Arma_Reforger:Resource_Manager:_Options#Shortcuts "Arma Reforger:Resource Manager: Options"))

[![](/wikidata/images/thumb/1/1c/armareforger-resource-manager-options-we-link-create2.jpg/300px-armareforger-resource-manager-options-we-link-create2.jpg)](/wiki/File:armareforger-resource-manager-options-we-link-create2.jpg)

Creating a link in World Editor

[![](/wikidata/images/thumb/8/89/armareforger-resource-manager-options-we-link.jpg/800px-armareforger-resource-manager-options-we-link.jpg)](/wiki/File:armareforger-resource-manager-options-we-link.jpg)

Example: <enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent;5275.56,81.3831,6383.2;-4.2668,-283.292,0>  
 [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") opens the [GM\_Eden.ent](enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent) file at world coordinates [5275.56,81.3831,6383.2](enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent;5275.56,81.3831,6383.2) and camera angles [-4.2668,-283.292,0](enfusion://WorldEditor/worlds/GameMaster/GM_Eden.ent;5275.56,81.3831,6383.2;-4.2668,-283.292,0)
