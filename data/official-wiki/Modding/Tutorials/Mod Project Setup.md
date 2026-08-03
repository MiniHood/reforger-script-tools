# [Mod Project Setup](https://community.bistudio.com/wiki/Arma_Reforger:Mod_Project_Setup)

## Prerequisites

* [Arma Reforger](/wiki/Category:Arma_Reforger "Category:Arma Reforger") installed
* [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools") installed

## Workbench Launcher Setup

Before creating a new project, it is necessary to set Enfusion Workbench Launcher so it knows where Reforger data is located. Reforger data is essential for Workbench to function correctly so its important

There are two ways how to prepare Workbench for creation of the addon - automatic & manual - which are described below

### Preparing Data

By default, every time **Arma Reforger (game)** is launched, executable will automatically add hidden link to the data **ArmaReforger.gproj** in Windows Register.

It might happen though, that automatic detection is not working (f.e. installation dir was moved) and because of that, **it is recommended to manually add ArmaReforger.gproj** to Project list

#### Adding Arma Reforger Project

1. Start the Workbench either through Steam (Tools > Arma Reforger Tools > START) or by double-clicking **ArmaReforgerWorkbenchSteam.exe** located in the Workbench installation directory.  
   The following screen will appear:  
   [![armareforger modsetup-launcherwindow-reforgernotfound.png](/wikidata/images/6/6f/armareforger_modsetup-launcherwindow-reforgernotfound.png)](/wiki/File:armareforger_modsetup-launcherwindow-reforgernotfound.png)
2. Click "Add Existing" button
3. Browse to ArmaReforger.gproj (located in <Arma Reforger installation directory>\addons\data\ArmaReforger.gproj) and select it
4. Arma Reforger project is now listed in the Projects window.  
   [![armareforger modsetup-launcherwindow-reforgerfound.png](/wikidata/images/d/df/armareforger_modsetup-launcherwindow-reforgerfound.png)](/wiki/File:armareforger_modsetup-launcherwindow-reforgerfound.png)

#### Adding Other Existing Projects

Other projects, like dependencies, can be added one by one via **Add Existing Project** as described above or by using **Scan for Projects** button.

* Click on **+ Add Project** button
* Select **Scan for Projects** from the list
* Select folder where you have addons located, which you want to use as dependencies
* Confirm selection

[![armareforger-modsetup-scan-for-projects.gif](/wikidata/images/0/09/armareforger-modsetup-scan-for-projects.gif)](/wiki/File:armareforger-modsetup-scan-for-projects.gif)

Now you can either try to launch one of those mods or use them as dependencies

ⓘ

Projects list is stored in user profile. You can use different profile via [-profile](/wiki/Arma_Reforger:Startup_Parameters#profile "Arma Reforger:Startup Parameters") command line parameter, allowing you to have **set of mods** for various occasions or for **different version of game**.
[![armareforger-modsetup-profile-cli.png](/wikidata/images/4/4c/armareforger-modsetup-profile-cli.png)](/wiki/File:armareforger-modsetup-profile-cli.png)

## Project Creation

* Open Enfusion Workbench Launcher if you don't have it already open.  
  [![armareforger modsetup-launcherwindow-reforgerfound.png](/wikidata/images/d/df/armareforger_modsetup-launcherwindow-reforgerfound.png)](/wiki/File:armareforger_modsetup-launcherwindow-reforgerfound.png)  
  This interface displays existing projects known to Workbench and will later display the current creation; an existing project is openable by selecting it and clicking **Open** to edit it.
* Click "**Create New**" to open the project creation interface

|  |  |
| --- | --- |
| Vanilla Arma Reforger | Modded Arma Reforger |

* Enter the project's name
  + the project's name can only contain **letters**, **numbers**, **spaces** and the following symbols: **-** (dash) **\_** (underscore) and **.** (dot).
* Confirm or edit the project's location
* ⚠

  Do not create projects in **OneDrive** directories - such project will fail to load!

  ⚠

  Be sure to pick a location where the current Windows user has write permissions (e.g C:\Users\Username\Documents - **not** C:\Program Files).

  ⓘ

  The default project directory's location is %userProfile%\Documents\My Games\ArmaReforgerWorkbench\addons; the default project name (and directory name) is New Enfusion Project - it can only contain letters, numbers, ampersands, spaces, dashes, dots and underscores.
* Pick the project's dependencies:
  + dependencies are other projects (and/or mods) on which the current project relies to work
  + a project cannot be loaded if a dependency is missing
  + the dependency link is one-way: a dependency does not need the current project in order to be loaded
  + Arma Reforger is a default dependency: an Arma Reforger mod needs Arma Reforger data to run properly
* Click "**OK**" to create the project.

✩

Dependencies of created project can be later changed in [Resource Manager Options](/wiki/Arma_Reforger:Resource_Manager:_Options#Dependencies "Arma Reforger:Resource Manager: Options")

*Et voilà !* The project is created and the Workbench [**Resource Manager**](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") is waiting for input.

ⓘ

The addon.gproj file can be renamed to have a more fitting name; the project will then need to be re-added to the Projects list on Workbench opening.

📖

**Recommended read**:

* [Tools Documentation](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools") - for general knowledge about using Workbench
* [Asset Creation Tutorials](/wiki/Category:Arma_Reforger/Modding/Assets/Tutorials "Category:Arma Reforger/Modding/Assets/Tutorials") - if you intend to create or modify assets, like changing weapon parameters or adding new vehicle
* [Scripting Tutorials](/wiki/Category:Arma_Reforger/Modding/Scripting/Tutorials "Category:Arma Reforger/Modding/Scripting/Tutorials") - this category contains various pages explaining how to create your first script
* [Scenario Creation Tutorials](/wiki/Category:Arma_Reforger/Modding/Scenario/Tutorials "Category:Arma Reforger/Modding/Scenario/Tutorials") - tutorials explaining how to create your first scenario in World Editor
* [Terrain Tutorials](/wiki/Category:Arma_Reforger/Modding/Terrains/Tutorials "Category:Arma Reforger/Modding/Terrains/Tutorials") - tutorials containing information how to create your first terrain

## Managing Projects

### Launching Project

When launching Enfusion Workbench Launcher second and after projects were added to the list, launching of mods can be done in following ways:

* By double clicking with ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") on mod (either tile or element in list view)
* By clicking on it with with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") and selecting option "**Open**" (1) from context menu

[![armareforger-modsetup-open-project.png](/wikidata/images/3/32/armareforger-modsetup-open-project.png)](/wiki/File:armareforger-modsetup-open-project.png)

#### With Mods

[![](/wikidata/images/thumb/3/38/armareforger-modsetup-open-with-presets.png/300px-armareforger-modsetup-open-with-presets.png)](/wiki/File:armareforger-modsetup-open-with-presets.png)

Launching mod with additional mods and preset menu

Launching your project with other addons, which are **not dependencies**, can be done via **Open with Addons** **(2 on picture above)** option which is available in context menu visible after pressing on mod in list with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button"). After selecting this option, a new menu ***- Open project with additional addons*** (similar to one, used for setting dependencies) - will show up and you will be able to select your dependencies.

##### Presets

List of additionally launched mods in ***Open project with additional addons*** menu is saved and restored when using this option. Additionally, selections of those addons is stored in **Presets,** which you can use to switch between specific mod sets. **Presets** are shared for all addons listed in **Enfusion Workbench Launcher.**

**You can select one of the 8 presets** by clicking on **Preset *x*** list box in top right corner of the window**.**

### Removing Projects

Projects can be removed by Enfusion Project List by clicking on them with ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") and then selecting **Remove from List** option. This will remove project **only from the list** and data itself will remain intact.

[![armareforger-modsetup-removing-project.png](/wikidata/images/c/c7/armareforger-modsetup-removing-project.png)](/wiki/File:armareforger-modsetup-removing-project.png)

### Projects View

It is possible to switch between List & Grid view. To do so, click on **cog** icon in top right corner and select one of the views from the context menu.

[![](/wikidata/images/1/17/armareforger-modsetup-list-grid-view.gif)](/wiki/File:armareforger-modsetup-list-grid-view.gif)

## Experimental Branch

It is also possible to create and publish projects using Experimental Branch of the tools. For more info, head out to [**Experimental Branch**](/wiki/Arma_Reforger:Experimental_Branch "Arma Reforger:Experimental Branch") page.

## Troubleshooting

### Arma Reforger project is not found in the Projects window

* You need to add **Arma Reforger game project** (**not your project!**) to the list. See instructions listed in [Adding Arma Reforger Project](/wiki/Arma_Reforger:Mod_Project_Setup#Adding_Arma_Reforger_Project "Arma Reforger:Mod Project Setup") section

⚠

Be sure to use the adequate Workbench to open the corresponding data. Using default Workbench to open e.g Experimental branch data may result in errors!

### Arma Reforger Workbench loads without selected mod

Make sure that all dependencies of your mod are visible in the Workbench project list. This also includes dependencies of dependencies!

Usually you will find following error in **Log Console**

```enforce
INIT : Workbench startup
INIT : Workbench Init Engine
ENGINE (E): Addon 'SampleMod' dependency '5614E48126E3ADF2' can't be added
```

This means, that addon *5614E48126E3ADF2* was not found and due to that, **SampleMod** cannot be loaded. Workbench will still launch but it will skip all mods which couldn't be loaded and quite often only ArmaReforger addon will be loaded. To find name of that name, you can for instance try to find it on workshop, by adding GUID to the end of link <https://reforger.armaplatform.com/workshop/> (f.e. <https://reforger.armaplatform.com/workshop/5614E48126E3ADF2> ). If this is a local mod, then search for that specific GUID in local addons that you have created.

Instruction how to add mod to Enfusion Workbench Launcher project list can be [found above](/wiki/Arma_Reforger:Mod_Project_Setup#Adding_Other_Existing_Projects "Arma Reforger:Mod Project Setup").

⚠

Keep in mind dependencies of dependencies have also to be listed in Projects list - otherwise Workbench will not be able to locate such addon.

##### Empty dependency

In some cases, you might end up with dependency with empty dependency ('') which most likely was done by trying to remove dependency in incorrect way.

```enforce
INIT : Workbench startup
INIT : Workbench Init Engine
ENGINE (E): Addon 'SampleMod' dependency '' can't be added
```

In such scenario it might be necessary to open mentioned mod .gproj file in text editor (like notepad) and manually remove '' dependency from **Dependencies** array.

### Project is read-only

If you see read-only icon in project then ensure that:

* It's not synced via OneDrive (or similar, issue also applies to GDrive)
* Project is not in the same folder as where downloaded Workshop mods are located

If you need to unlock your mod, which was locked by cloud service, use following code in batch file and then execute it in root of your addon.

```enforce
@echo off
setlocal enabledelayedexpansion

echo Removing read-only attributes recursively from all files and folders...
echo Current directory: %CD%
echo.

echo Processing folders...
for /d /r %%D in (*) do (
attrib -r -s "%%D" /S /D
echo Processed: %%D
)

echo.
echo Processing files...
attrib -R *.* /S

echo.
echo Process completed successfully.
pause
```
