# [Resource Manager: Options](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Options)

Access the [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") Options through **Workbench > Options**.

[![armareforger-resourcemanager-options.png](/wikidata/images/2/29/armareforger-resourcemanager-options.png)](/wiki/File:armareforger-resourcemanager-options.png)

## Game Project

## General Parameters

The side dropdown box allows to select between the detected projects. Core is the engine one, whereas ArmaReforger is about Arma Reforger project(s).

ⓘ

The switch happens for all settings present **on this tab only** - other options tabs are independent of this setting.

A project .png image can be set/unset here using Set/Clear buttons on the left.

### Unsorted

#### ID

The project's id

#### GUID

The project's [GUID](/wiki/Arma_Reforger:Workbench_Metadata "Arma Reforger:Workbench Metadata")

#### TITLE

The project's user-friendly title

#### Dependencies

List of all project dependencies. List contains GUIDs of all projects which should be also loaded when engine is tasked with loading given project.

[![](/wikidata/images/thumb/5/5f/armareforger-resourcemanager-options-dependencies.png/586px-armareforger-resourcemanager-options-dependencies.png)](/wiki/File:armareforger-resourcemanager-options-dependencies.png)

To add new dependency, perform the following steps:

* Click on **plus +** **(1)** sign
* In new entry that was created either:
  + Click on **button with two dots (2)** and select **gproj** of your desired dependency
  + Type in GUID of your desired dependency manually in **text field on the left side (3)**. You can obtain GUID by opening gproj in text editor for instance
* Click on **OK button** (4) once you are done to save changes.

ⓘ

Arma Reforger requires core to work properly.  

Any Arma Reforger mod requires Arma Reforger as a dependency.

There is no need to add Arma Reforger **and** core as having Arma Reforger implies having core.

After changes are saved, selected dependencies will be loaded after restart of Workbench.

When using **[Enfusion Workbench Launcher](/wiki/Arma_Reforger:Mod_Project_Setup#Workbench_Launcher_Setup "Arma Reforger:Mod Project Setup")**, make sure that all dependencies are listed in available projects list - otherwise game will fail to launch selected mods and it will fall back to vanilla **Arma Reforger without any mods loaded**.

When using **[startup parameters](/wiki/Arma_Reforger:Startup_Parameters "Arma Reforger:Startup Parameters")**, make sure that all dependencies are located in one of the folders listed by **[addonsDir](/wiki/Arma_Reforger:Startup_Parameters#addonsDir "Arma Reforger:Startup Parameters")** parameter, otherwise it will also fail to launch.

If one of the dependencies no longer exists and the mod cannot be started anymore, open its .gproj file in text editor and carefully remove the faulty dependency and save the file.

**Removal of dependencies** is done by selecting one of the entries in the list and then pressing pressing **minus -** arrow next to the **plus + (1)** button

⚠

Do not try to empty to remove dependencies by removing text from text field! Engine will treat it as an dependency with  *GUID and will fail to load!*

#### Engine Settings Path

Project's engine config name

#### Game Settings Path

Project's game config name

#### Brush Library Path

Project's brush location

#### Welcome Screen Config

Resource path to the Workbench Welcome Screen configuration file (based on WelcomeScreen root config)

## Per platform parameters

Here it is possible to select for which platform the following settings will be applied.

It is also where one can add new platform parameters (by clicking "+") or remove them (by clicking "-").

### General

#### World File

Which world is loaded on project's startup (e.g Main Menu world)

#### SplashScreen

### Engine

#### Platform Hardware

Defines the platform's hardware (e.g HEADLESS has PC hardware)

#### Product ID

Hexadecimal representation of the product ID.

#### Default Settings

Defines the project's default settings.

Based on EngineUserSettings config class, the Default Settings class holds sub-settings classes:

##### Audio Settings

* Volume
* Volume SFX
* Volume Music
* Volume Voice Chat
* Volume Dialogue
* Volume UI
* Stereo Mode

##### Terrain Gen material Settings

* Detail
* Soft Alpha
* Detail Distance
* Geometry Level

##### User Interface Settings

* Use Software Cursor
* Language Code
* System Language Code

##### Resource Manager User Settings

* Texture Detail
* Geometric Detail

##### Postprocess User Settings

* PostProcess

##### Material System User Settings

* Texture Filter
* Max Aniso

##### Video User Settings

* Frames Limit
* Max FPS
* Window Mode
* Screen Width
* Screen Height
* Resolution Scale
* Super Width
* Super Height
* Window Pos X
* Window Pos Y
* Console Mode
* Console Width
* Console Height
* Console Pos X
* Console Pos Y
* Vsynch
* Square Pixel
* Ratio3D
* Fsaa
* Render Format
* Distant Shadow Quality
* Environment Quality
* Gamma
* Contrast
* Brightness
* Adapter
* Device
* Atoc
* Generate Shaders
* FSR Enabled

##### Grass Material Settings

* Distance
* Lod
* Quality

##### Input Device User Settings

* Mouse Speed
* Mouse Invert
* Gamepad Invert
* Gamepad Profile
* Joystick Profile

##### Graphics Quality Settings

* Object Draw Distance Scale

##### Water Pool Material Settings

* Detail

##### Display User Settings

* PP Quality
  + HBAO
  + SSDO
  + SSR
  + Under Water
  + PPAA
  + Rain
* Overall Quality

##### Pipeline User Settings

* Shadow Quality
* Tight Cascades
* Light Shadow
* Light Level
* Reflections
* Linear Depth
* G Buffer

##### Clustered Tiling User Settings

* Clusters X
* Clusters Y
* Clusters Z
* Reflection Texture Size
* Diffuse Texture Size
* Cluster Far Plane
* Cluster Near Plane
* Probe Max Distance
* Probe Priority Distance

### Modules

#### Script Project Manager Settings

* Modules
* Configurations

#### Navmesh Manager

* Project Settings

#### Distant Shadows Quality Profiles

* Low
* Medium
* High

#### Environment Quality Profiles

* Environment Quality Profiles

#### Input Manager Settings

* Default
* User
* Ui Mappings
* Hold Duration
* Click Duration
* Double Click Duration
* Repeat Initial Interval
* Repeat Interval

#### Physics Settings

* Iterations
* Ticks
* Margin
* Gravity
* Grid BP
* Interactions
  + Layers
  + Layer Presets
  + Response Indices
  + Matrix
  + Response Matrix
  + Layers Filter
  + Layer Projectile

#### Wetness Render Settings

* Min Diffuse Coef
* Min Roughness Coef
* Porosity From Roughness Coef
* Albedo Change Start
* Albedo Change End
* Roughness Change Start
* Roughness Change End
* Wetness Normal Limit Min
* Wetness Normal Limit Max
* Roughness Normal Limit Min
* Roughness Normal Limit Max
* Accum Normal Limit Min
* Accum Normal Limit Max
* Flood Sharpness Cracks
* Flood Sharpness Puddles
* Min Obj Radius
* Min Obj Side Size
* Min Tree Scale
* Max Tree Scale
* Max Height Cracks
* Max Height Puddles
* Underwater Fade Start
* Underwater Fade Size

#### Chimera Global Config

* Default Player Controller  
  This defines default player character which is used by various game systems, like [Play from camera position](/wiki/Arma_Reforger:World_Editor#Play_Game "Arma Reforger:World Editor") option. By default it is using [DefaultPlayerController.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Characters/Core/DefaultPlayerController.et). To change character prefab which is used in-game, to change **Default Controlled Entity** parameter
* Maximum View Distance
* Minimum View Distance
* Ballistics
  + Damage Modifier

#### Setting Profiles Config

* Effects
* Overall Low
* Overall Medium
* Overall High
* Overall Ultra

#### Audio Global Config

* Terrain Attenuation
* Terrain LPF Rolloff

#### Resource Manager Settings

* Stream Mode

#### Menu Manager Settings

* Menu Configs

#### Shadow Quality Profiles

* Shadows Quality Profiles

#### Widget Manager Settings

* Widget Styles
* Mouse Cursors
* String Tables
  + StringTableDefinition
    - String Table Source
    - Languages

## Workbench

## Qt Style Sheet

Defines a qss stylesheet to be used over the Workbench.

## Use SVN integration

Uses SVN on file operation (rename, delete etc); requires SVN command line tools to be installed.

ⓘ

A Workbench restart is required for this option to take effect.

## Auto-rebuild scripts

Automatically launches a script rebuild when changes from Script Editor have been detected - otherwise, `Ctrl` + `⇧ Shift` + `R` rebuilds scripts manually.

## FPS limit

Allows to set an FPS limit to save on resources. Recommended value is screen's refresh ratio, e.g 60 or 144 Hz.

## Enable net API

Enables net API where external applications like Blender (with [Enfusion Blender Tools](/wiki/Arma_Reforger:Enfusion_Blender_Tools "Arma Reforger:Enfusion Blender Tools") loaded) can communicate with Workbench.

## Invert mouse

Invert mouse's Y-axis in Workbench.

## Register "enfusion://" protocol

Registers the '**"enfusion://" protocol** in Windows so Enfusion links open the Workbench.

Once Enfusion protocol is registered, clicking on Enfusion link will launch Workbench, open one of the selected Editors.

ⓘ

See [Workbench Links](/wiki/Arma_Reforger:Workbench_Links "Arma Reforger:Workbench Links") for more information.

## Export settings

Export Workbench settings as .ini file.

## World Editor

## General

### Undo Steps

Number of stored `Ctrl` + `Z` undo operations.

## Compass

### Show compass

Defines if the compass is shown in the main viewport.

### Show axis

Defines if the X-Y-Z axis are shown in the main viewport.

## Entity Selection

### Outline Enabled

### Outline Thickness

### Outline Mask enabled

### Outline Mask opacity

### Outline Intersection Opacity

## Animation Editor

## Event Filter

* Event Filter
* Include/Exclude

## Animation export profile directory

## Node Colors

Defines Colours for each type of nodes.

## Script Editor

## Tab Size

Defines the size in spaces a tab has. The tab will remain a tab character, only its display width will be impacted.

## Font Size

Defines the font size.

## Font Family

Defines the font family.

## Append closing bracket

Automatically append the closing bracket (}) when opening one ({).

## Show scroll-bar markers

Shows where code was modified/saved, breakpoints etc on the scrollbar to the right.

## Audio Editor

## Default audio listener model

Defines the audio listener's default 3D model (xob).

## Behavior Editor

## Restore session

Restore the previous session on startup.

## Shortcuts

Defines all the shortcuts for all the following categories:

* Camera
* Common
* Compass Gizmo
* Resource Browser
* Resource Manager
* World Editor
* Particle Editor
* Animation Editor
* Script Editor
* Audio Editor
* Behavior Editor
* Navmesh Generator
* Procedural Animation Editor
* String Editor

A "Reset All" button is made available to reset shortcuts to default, next to the search bar.
