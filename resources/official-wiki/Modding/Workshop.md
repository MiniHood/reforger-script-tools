# [Workshop](https://community.bistudio.com/wiki/Arma_Reforger:Workshop)

The **Workshop** is where [Arma Reforger](/wiki/Category:Arma_Reforger "Category:Arma Reforger")'s mods are uploaded and can get obtained.
It can be browsed and interacted directly in-game or on the [Workshop website](https://reforger.armaplatform.com/workshop).

## Usage

* Authors can upload mods on the Workshop:
  + Using the [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools")
  + The Workshop also stores the previous mod versions.

    ⚠

    Only the last **50** versions are kept!
* Users can download mods from the Workshop:
  + by browsing the Workshop and subscribing to mods
  + by joining a server that requires mods
  + Removed mods (by author, by version, by administrator, etc) are deleted on user's system when synchronised

## Interface

* [![Default tile](/wikidata/images/3/35/armareforger_workshop-mod-tile-subscribe.png)](/wiki/File:armareforger_workshop-mod-tile-subscribe.png "Default tile")

  Default tile
* [![Download](/wikidata/images/c/c8/armareforger_workshop-mod-tile-download.png)](/wiki/File:armareforger_workshop-mod-tile-download.png "Download")

  Download
* [![Downloading](/wikidata/images/c/c1/armareforger_workshop-mod-tile-downloading.png)](/wiki/File:armareforger_workshop-mod-tile-downloading.png "Downloading")

  Downloading
* [![Cancel download](/wikidata/images/7/75/armareforger_workshop-mod-tile-canceldownload.png)](/wiki/File:armareforger_workshop-mod-tile-canceldownload.png "Cancel download")

  Cancel download
* [![Enabled](/wikidata/images/2/26/armareforger_workshop-mod-tile-enabled.png)](/wiki/File:armareforger_workshop-mod-tile-enabled.png "Enabled")

  Enabled
* [![Favourited](/wikidata/images/1/13/armareforger_workshop-mod-tile-favourited.png)](/wiki/File:armareforger_workshop-mod-tile-favourited.png "Favourited")

  Favourited
* [![Disabled](/wikidata/images/e/ee/armareforger_workshop-mod-tile-disabled.png)](/wiki/File:armareforger_workshop-mod-tile-disabled.png "Disabled")

  Disabled

### Workshop

The Workshop is accessed through the game's main menu ("Workshop" tile). The main page lists pages of mods available on the workshop.

Top buttons allow to sort results by Popularity, Subscribers count, Rating, Recently added and Name.

#### Navigation

Go to left/right pages by clicking the left-right buttons, pressing `Z`/`C`, or pressing ↚/↛ on gamepad.

#### Filter

Access to filters by clicking on the the top-right Filter icon, pressing `X`, or pressing ↤ on gamepad.

### Mod Details

Access the Mod Details by clicking on the tile (out of the centre's Download button) or using ↧ on gamepad.

## Actions

### Download

Click on the tile's centre, enter the Mod Details page and click "Download", or press ↥ on gamepad.

ⓘ

A download can be cancelled by using the same action as to download it.

␼

Due to console restrictions, download speed is limited to around 25 Mbps / **~3.125 MB/s**
(see e.g [XR-133: Local Storage Write Limitations](https://learn.microsoft.com/en-us/gaming/gdk/_content/gc/policies/console/certification-requirements#xr-133-local-storage-write-limitations-)).

Storage is also limited to **25 GB** on consoles.

### Enable/Disable

Click on the on/off button, press `T`, or press ↻ on gamepad.

### Like/Dislike

**Mod Details** action - use the "Like"/"Dislike" buttons to add/remove a like/dislike (positive/negative vote regarding this mod).

### Add to Favorites

Click the star top-left of the tile, press `F`, or press ↤ on gamepad.

### Remove

**Mod Details** action - use the "Delete" button to remove the Mod from your computer entirely.

### Report

**Mod Details** action - use the "Report" button to report the Mod to Bohemia Interactive. The report reason can be one of:

* Inappropriate content - e.g purposely offensive, warcrime promotion content etc.
* Offensive language
* Misleading/non-functional item - e.g game-crashing, or a vehicle mod only adding music and no vehicles
* Other - e.g ripped content

⚠

Do **not** report a mod for a wrong reason (e.g "I don't like it")! Misusing the report function on purpose may lead to Workshop usage sanctions.

⚠

* Reporting a mod removes it from the local machine and hides it from Workshop view.
* It is important to provide as much valid information as possible.

:   | Correct feedback | Incorrect feedback |
    | --- | --- |
    | * This 3D model has been ripped from X game (or Y website) * This mod pretends to add AR-15 but it actually adds a chicken * This mod crashes my game when I place it from Game Master | * It's a stolen mod * It doesn't work * I don't like it |

### View Downloads

Press `↹ Tab`, or press ↥ on gamepad to open Downloads; this window displays past and current downloads.

### Downloaded Tab

Click the wanted tab, press `Q`/`E`, or press ↜/↝ to change tabs.

This tab allows to manage downloaded mods the same way as other mods.

## Development

### Installation Directory

On PC, by default mods are downloaded to

```
%userprofile%\Documents\My Games\ArmaReforger\addons
```

This can be changed by using the [addonDownloadDir](/wiki/Arma_Reforger:Startup_Parameters#addonDownloadDir "Arma Reforger:Startup Parameters") startup parameter.

␼

On consoles it is **impossible** to change the download location. The user is limited to **25 GB** of total storage for mods.

### Debug Mode

Using the WORKSHOP\_DEBUG define through the [scrDefine](/wiki/Arma_Reforger:Startup_Parameters#scrDefine "Arma Reforger:Startup Parameters") startup parameter adds a combo box allowing a mod creator to select a specific Workshop mod version.

Example

```
ArmaReforgerSteam.exe -scrDefine WORKSHOP_DEBUG
```
