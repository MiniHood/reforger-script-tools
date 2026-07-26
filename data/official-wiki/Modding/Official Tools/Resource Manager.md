# [Resource Manager](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager)

Welcome to the **Enfusion Workbench - Resource Manager!**

The Resource Manager is an integral part of the main "hub" of the **Enfusion Workbench**. The Resource Manager is its asset manager, which allows for **importing, managing and editing** assets and their related files. It’s also the place from which navigation to other tools or areas of Workbench can be done, as well as accessing project and Workbench **options**, **build data**, **package**, **build project** and more.

### Interface

[![armareforger-resource-manager-main-view.gif](/wikidata/images/thumb/2/20/armareforger-resource-manager-main-view.gif/500px-armareforger-resource-manager-main-view.gif)](/wiki/File:armareforger-resource-manager-main-view.gif)

By default, **Resource Manager** welcomes you with 4 main sections:

1. **Resource Browser** - as its name suggests, this window can be used to browse through available resources from base game and loaded mods
2. **Main Window** - if there are not tabs opened, then **Welcome Tab** will be visible. When resource is opened, it will appear in this main window area in a tab.
3. **Log Console** - window containing live updated logs from current session.
4. **Window Menu** - in top toolbar it is possible to access **all available Editors** , options, plugins and other things. More details can be found below

Beside that, the Workbench version is available in the bottom right corner along to which backend (Production = Stable branch, Submission = Experimental branch) it is connected.

## Window Menu

| Title | Hierarchy |
| --- | --- |
| Workbench | * Save * Save All * Options * Publish Project * Login/Logout * Exit |
| Editors | * [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") * [Particle Editor](/wiki/Arma_Reforger:Particle_Editor "Arma Reforger:Particle Editor") * [Animation Editor](/wiki/Arma_Reforger:Animation_Editor "Arma Reforger:Animation Editor") * [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") * [Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor") * [Behavior Editor](/wiki/Arma_Reforger:Behavior_Editor "Arma Reforger:Behavior Editor") * [Procedural Animation Editor](/wiki/Arma_Reforger:Procedural_Animation_Editor "Arma Reforger:Procedural Animation Editor") * [String Editor](/wiki/Arma_Reforger:String_Editor "Arma Reforger:String Editor") |
| View | * Normal view * Text view * Hex view   + 8 bytes   + 16 bytes   + 32 bytes |
| Edit | * Undo * Redo * Reimport resource * Locate file in resource browser |
| Window | * Next Tab * Prev Tab * Cycle Tabs * Close Current Tab * Undo Close Tab  * Log Console * Resource Browser * Resource Browser 2 |
| Utilities | * Generate GUID * File system sources * Qt introspection tool |
| Plugins | * Settings   + Reload WB Scripts   + Clear All Settings   + List world resources   + Texture Import   + Model Import   + Material Import   + Terrain Import   + Edit Selected Prefab(s)   + SVN Log   + Create/Update Selected Editable Prefabs  * Image Set Generator * Batch resource processor * Batch texture processor * Re-Save Meta Tool * Re-Save Tool * In-game Editor   + Register Placeable Entities...   + Update All Editable Prefabs   + Create/Update Selected Editable Prefabs * SVN   + SVN Log * User Settings   + Edit Engine Settings   + Edit Game Settings |
| Help | * About |

### Workbench

#### Save

Save the file from the currently selected tab.

#### Save All

Save the file from all opened tabs.

#### Options

See [Resource Manager: Options](/wiki/Arma_Reforger:Resource_Manager:_Options "Arma Reforger:Resource Manager: Options")

#### Login/Logout

Log in or out of the Bohemia account. Logging in allows for publishing online to the [Bohemia Workshop](/wiki/Arma_Reforger:Workshop "Arma Reforger:Workshop").

#### Publish Project

[Publish the mod](/wiki/Arma_Reforger:Mod_Publishing_Process "Arma Reforger:Mod Publishing Process") in its current state.

#### Exit

Close the Workbench and all other open editors, if any.

### Editors

This menu allows for navigating through all available editors.

ⓘ

Greyed out editors are either already opened or unopenable as per the script base status (non-compilable code).

### View

This menu allows the switching of the current tab's view mode between:

* **Normal** view
* **Text** view
* **Hex** view (8, 16 or 32B)

ⓘ

**Text** and **Hex** views are **read-only**.

### Edit

#### Undo

Undo the last applied action to the world. Shortcut: `Ctrl` + `Z`.

#### Redo

Reapply the last undone action. Shortcut: `Ctrl` + `Y`.

#### Reimport resource

Reload the current file from storage. Wanted changes must have been saved first! Shortcut: `F5`.

#### Locate file in resource browser

Show the selected file in the Resource Browser tree. Shortcut: `Ctrl` + `B`.

#### Edit prefab

Open the World Editor to edit the currently-opened prefab. Shortcut: `Ctrl` + `E`.

### Window

The first section groups Tab operations.

The second section groups Window operations.

#### Next Tab

Shortcut: `Ctrl` + `↡ PgDown`.

#### Previous Tab

Shortcut: `Ctrl` + `↟ PgUp`.

#### Cycle Tabs

Shortcut: `Ctrl` + `↹ Tab`.

#### Close Current Tab

Shortcut: `Ctrl` + `W`.

#### Undo Close Tab

Shortcut: `Ctrl` + `⇧ Shift` + `T`.

#### Log Console

Hide/show the Log Console and its history. Shortcut: `F1`.

#### Resource Browser

Hide/show the "project tree" view. Shortcut: `F3`.

#### Resource Browser 2

Hide/show a secondary Resource Browser. This one has no keyboard shortcuts.

### Utilities

#### Generate GUID

Generate a new Globally Unique IDentifier.

#### File system dump

Show which directories are used as source, in the form of

1. **<Game>** game data directory
2. **core** ./addons/core/
3. **profile** game user profile directory

#### Qt introspection tool

Show debug information about Qt, the Workbench Graphical User Interface (GUI).

### Plugins

This menu lists Workbench plugins. Each plugin has its own usage and user manual.

#### Settings

##### Reload WB Scripts

Reload Workbench plugins. Shortcut: `Ctrl` + `⇧ Shift` + `R`

##### Clear All Settings

Resets all parameters that have been changed, Workbench parameters included.

##### Texture Import

##### Model Import

##### Material import

##### Terrain Import

##### Edit Selected Prefab(s)

##### Create/Update Selected Editable Prefabs

##### Bookmark #

##### SVN Log

#### Batch resource processor

#### Batch texture processor

#### Re-Save Meta Tool

Re-save the .meta files for files having the provided extensions. Useful if a format has changed and files should be re-saved.

#### Re-Save Tool

Re-save the files having the provided extensions. Useful if a format has changed and files should be re-saved.

Extensions are to be provided **without** the dot (e.g layout and not .layout)

#### Image Set Generator

#### In-game Editor

#### User Settings

##### Edit Engine Settings

##### Edit Game Settings

##### Register Placeable Entities...

##### Update All Editable Prefabs

##### Create/Update Selected Editable Prefabs

#### Bookmarks

#### SVN

##### SVN Log

#### Validate Behavior Trees

#### Search Tool (XOBs)

#### Edit Selected Prefab(s)

Shortcut: `Ctrl` + `⇧ Shift` + `E`

#### Generate Class from Layout

#### Localize Selected Files

Shortcut: `Ctrl` + `Alt` + `L`

#### Find Linked Resources

#### Validate Prefabs

#### Generate Controls Scheme

## Resource Browser

Resource Browser is the main tool for **navigating** and **managing** game data, it can work with "local" data, packed data and both at the same time.

Default layout provides navigation through game data in two windows - one displays **directories** in tree layout and the other displays content (**sub-directories** and **files**) of the selected one.

There is a **Search bar** at the top left which has various filters and options available.

And lastly, over viewport, there are **tabs** with opened resources.

ⓘ

This section also applies to "Resource Browser 2" that is a simple copycat of this window.

### Contextual Menus

In both windows, using ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") on files and folders opens a **context menu**, dependent on the clicked item's type:

| Context | Options |
| --- | --- |
| Tree view | * Show in Explorer  * Register * Edit Properties * Delete * Rename  * Copy Resource Name(s) * Copy Resource GUID(s) * SVN   + Update   + Commit   + Add   + Remove   + Revert |
| File view | * Open File in New Tab  * Show in Explorer * Navigate to Folder  * Register Registers the resource to be used (creates .meta files) * Register and Import **Register** and (re)**import** * Reimport Converts file from source file (tiff, fbx, etc.) to Enfusion file (xob/txo, edds, etc.) * Delete * Rename  * [Navigate to Original](/wiki/Arma_Reforger:Data_Modding_Basics#Navigate_to_Original "Arma Reforger:Data Modding Basics") * [Navigate to Ancestor](/wiki/Arma_Reforger:Data_Modding_Basics#Navigate_to_Ancestor "Arma Reforger:Data Modding Basics") * [Navigate to Override](/wiki/Arma_Reforger:Data_Modding_Basics#Navigate_to_Override "Arma Reforger:Data Modding Basics") * [Override in...](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Override_in....22_function "Arma Reforger:Data Modding Basics") * [Duplicate to...](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Duplicate_to....22_function "Arma Reforger:Data Modding Basics") * [Inherit in...](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Inherit_in....22_function "Arma Reforger:Data Modding Basics") * [Transfer to...](/wiki/Arma_Reforger:Data_Modding_Basics#Using_.22Transfer_to....22_function "Arma Reforger:Data Modding Basics")  * Copy Resource Name(s) Copies the Enfusion path to this resource (format {GUID}path/to/file.ext) to the clipboard * Copy Resource GUID(s) Copies the resource GUID to the clipboard * Copy File Path(s) Copies the absolute path (format C:\path\to\file.ext) to the clipboard * Copy Link Copies an [Enfusion:// link](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") pointing to this resource to the clipboard * SVN   + Update   + Commit   + Add   + Remove   + Revert * Find References **expensive operation** looks through all project files to find a reference to this element. *not present for script files* * Refresh Thumbnail(s) * [Edit Prefab(s)](/wiki/Arma_Reforger:Prefabs_Basics#Prefab_edit_mode "Arma Reforger:Prefabs Basics") [Opens the prefab](/wiki/Arma_Reforger:Prefabs_Basics#Opening_prefab_in_Prefab_Edit_Mode "Arma Reforger:Prefabs Basics") in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") in Prefab Edit Mode to see and edit it * Save changes * Revert changes |

### Usage

#### Search bar

The **Search bar** allows the typing of words/sequences, separated by a space, that should be found in the file name - including keywords.

It is also possible to search by GUID, i.e. A1F2619C082CFDDF will find {A1F2619C082CFDDF}assets/models/weapons/Ammo/Bullets/545x39/Data/5\_45x39\_US\_NMO.edds

e.g. looking for village 1i03 .xob will find and pinpoint House\_Village\_E\_1I03.xob.

##### Special characters

By using those character it is possible to modify the search result

| Character | Function | Example |
| --- | --- | --- |
| # | Search by tag | #myCustomTag - search by tag |
| ! | Exclude from search | chair old !dst - this will search for all assets called **chair old** but it will exclude from the list files having **dst** in the file name |

#### Filters

* Show Runtime files only
* Show unregistered sources only

* **Filter by Category**  
  Filter files by type (See [File Types](/wiki/Arma_Reforger:File_Types "Arma Reforger:File Types"))

#### Options

* Expand All on Search

* Horizontal Layout
* Vertical Layout

* Grid View
* List View

* On with filters
* Always On
* Always Off

* Icons Scale

* Sort by Name
* Sort by Size
* Sort by Type
* Sort by Date

* Ascending sorting order
* Descending sorting order

## Viewport

### Tabs

|  |  |
| --- | --- |
| Right-click | * Close Tab * Close All Tabs * Close All But This * Reopen Closed Tabs  * Navigate to File * Show in Explorer |

A **double-click** on the tab will **Locate** the file **in Resource Browser**, when a middle mouse button click will close it.

### Viewport Types

See [Arma Reforger/Modding/Official Tools/Resource Manager Viewports](/wiki/Category:Arma_Reforger/Modding/Official_Tools/Resource_Manager_Viewports "Category:Arma Reforger/Modding/Official Tools/Resource Manager Viewports").

## Log Console

Log console displays various messages, they can be either **informative**, **warnings** or **errors**, and can be sorted by **type** or **category**.

## Details / Import Settings

**Details** and **Import Settings** tabs are closely related, even dependent on the Viewport type. See Resource Manager: Viewport for more respective details.

### Import Settings

This tab is about import options. Each type of resource has different options available that are detailed in Resource Manager: Viewport.

If any changes are done in this category, it’s required to click on the "**Reimport resource**" button at the top to take effect.

ⓘ

Each platform has its own settings, but all of them inherit from PC.

#### Edit also selected files

It is possible to edit multiple files at once. To do that, you need to check the "Edit also selected files" checkbox and select files in Resource Browser.

#### Parameters

Different per resource type, each described in [Arma Reforger/Modding/Official Tools/Resource Manager Viewports](/wiki/Category:Arma_Reforger/Modding/Official_Tools/Resource_Manager_Viewports "Category:Arma Reforger/Modding/Official Tools/Resource Manager Viewports").

## General Shortcuts

| Shortcut | Function |
| --- | --- |
| `Ctrl` + `S` | Saves resource in active tab |
| `Ctrl` + `⇧ Shift` + `S` | Save all |
| `F5` | Reimports active resource |
| `Ctrl` + `B` | Locates active resource in Resource Browser |
| `F1` | Opens/closes log console |
| `F3` | Opens/closes Resource Browser window |
| `Ctrl` + `Page Up` | Next tab |
| `Ctrl` + `Page Down` | Previous tab |
| `Ctrl` + `↹ Tab` | Cycles between opened tab |
| `Ctrl` + `W` | Closes active tab |
