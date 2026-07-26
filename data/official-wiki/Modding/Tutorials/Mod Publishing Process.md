# [Mod Publishing Process](https://community.bistudio.com/wiki/Arma_Reforger:Mod_Publishing_Process)

## Publication

### Prerequisites

To publish mod on the Workshop, following things are necessary:

* **Working mod** (if it was published before, you have be **owner** of that mod)
* **Bohemia Account**

If you already have **Bohemia Account**, then you can login to Bohemia backend by using **Workbench > Link option.** This option should open new pop up window where it is possible to type email & password associated with **Bohemia Account**. After that was typed in, you can click on **Login** button - if password and account are correct, this will log into your **Bohemia Account** and link it with **currently logged Steam account**.

[![armareforger-publishing-mod-window.png](/wikidata/images/5/5e/armareforger-publishing-mod-window.png)](/wiki/File:armareforger-publishing-mod-window.png) [![armareforger-publishing-mod-login.png](/wikidata/images/b/b0/armareforger-publishing-mod-login.png)](/wiki/File:armareforger-publishing-mod-login.png)

If you don't have account or you don't remember password to it, it is possible to click on **Forgot Password?** link, to initiate password recovery or use **Create Account** link, to create a new **Bohemia Account**.

⚠

For collaboration on mod it is recommended to use **Contributors** feature described in [interface](#Interface) segment.
Otherwise, due to Bohemia Account being linked to Steam account after first login, you would need to share credentials for **Bohemia & Steam account** which were used to upload the mod.

#### Experimental version

When using [Arma Reforger Experimental Tools](/wiki/Arma_Reforger:Experimental_Branch "Arma Reforger:Experimental Branch"), it is necessary to create separate [account on experimental backend](https://accounts-sub-ar.bistudio.com/auth/register). Same email address as for regular tools can be added. Once account is created, you can login into it as described above.

Experimental Workshop is separate from stable (non experimental) - mods published to experimental workshop are not visible in stable version of the game and vice versa (mods published to stable Workshop are not visible experimental version of the game)

### Interface

Publishing a mod is the process of making the mod available on the Workshop. A bundled mod is required.

In order to do so, click on **Workbench > Publish Project**.

This will open the following interface:

[![](/wikidata/images/c/c6/armareforger-publishing-mod-interface.png)](/wiki/File:armareforger-publishing-mod-interface.png)

Publish project window

[![](/wikidata/images/thumb/d/d9/armareforger-publishing-mod-workshop-game-interface.png/600px-armareforger-publishing-mod-workshop-game-interface.png)](/wiki/File:armareforger-publishing-mod-workshop-game-interface.png)

Workshop in-game interface

The following fields are offered:

[![](/wikidata/images/thumb/6/6a/armareforger-publishing-mod-workshop-categories.png/600px-armareforger-publishing-mod-workshop-categories.png)](/wiki/File:armareforger-publishing-mod-workshop-categories.png)

Workshop categories

| Field Name | Description | Additional Notes |
| --- | --- | --- |
| **Project Name** | Project's workshop name. | A maximum of 30 characters is allowed. |
| **Working Dir** | Directory where bundled version of the project will be stored. **Do not use same directory as where your addon is located!!! It best to leave it to default location in most cases.** | By default it is *%userprofile%\Documents\My Games\ArmaReforger\publish\addonName*. |
| **Preview Image** | The project's workshop preview image. | Maximum allowed size of the image is 2MB,. It can be JPG or PNG. |
| **Screenshots** | A gallery of pictures for the users to browse. | Maximum allowed size of the image is 2MB. It can be JPG or PNG. |
| **Contributors** | List of contributors which can publish updates of the mod. Only original author of the mod is able to edit list of contributors and remove it completely from Workshop. List should contain **emails** used in Bohemia Account **separated by coma, space or a new line.**  Contributor needs to **accept invitation** via **Workbench>Check** **Pending Invitation** option in the toolbar. |  |
| **Category** | One or multiple categories in which the mod fit. | At **least one category** has to be selected. |
| **Tags** | Search tags used to get a more precise search result (e.g WW2, CTF, Survival etc). | Tags are separated by space. |
| **License** | License which is applied to the mod. Can either use one of 3 [existing licenses](/wiki/Licenses "Licenses") or custom one  * [Arma Public License (APL)](https://www.bohemia.net/community/licenses/arma-public-license) * [Arma Public License Share Alike (APL-SA)](https://www.bohemia.net/community/licenses/arma-public-license-share-alike) * [Arma Public License Non Derivative (APL-ND)](https://www.bohemia.net/community/licenses/arma-public-license-nd) * Custom license - if this option is picked, a full license of the addons has to be provided in **license.txt** file inside root of the addon. | Currently there is no way to check license of the addon as 3rd party user. |
| **Version** | The project's version. In the format <major>.<minor>.<bugfix>, should ideally be incremented on every update, with the exception of workshop information fix. | Version number can go up 32000 |
| **Visibility** | * **Public:** Makes the project visible to everyone. * **Private:** Makes the project visible only to the uploader. * **Unlisted:** Hides the project in search results. Mod can be still used on dedicated server and downloaded by other users. * **Test:** Makes the project visible only in **Test** mods category. |
| **Summary** | The project's workshop **short** text preview. | Maximum **1024 characters are allowed** |
| **Description** | The project's workshop **full** description. It is possible to open larger, pop up window, for longer descriptions. | Maximum **5000 characters are allowed** |
| **Change Notes** | A summary of all changes introduced in this version of the update. | Maximum **30000 characters are allowed** |

### Publishing Process

When everything needed is filled and bundle was prepared, click on **Publish** to publish the package to the workshop.

⚠

Before proceeding, it is worth making yourself familiar with [Arma Reforger Workshop Terms of Service](https://reforger.armaplatform.com/workshop-terms) and make sure that you **hold all the required rights** to the uploaded mod's content.  
If you are not sure about this aspect, **check the [Intellectual Property](/wiki/Intellectual_Property "Intellectual Property") page**.

After pressing **Publish** button, bundling process starts, which creates **packed data**, where any unneeded source files are removed, **manifest.json** - where backend related data is stored, **copies pictures** (*preview image, screenshots, mission previews*) and **prepares zip** **file,** with all previously generated data**,** which is ready to be uploaded to **Workshop**.
The editable data set is called the **source files**. Once packaged, this is called **packaged files** or **package** – the data is compressed, indexed and encrypted.

* Packaged files are meant for public distribution
* Packaged files cannot be extracted back to source files – so **be sure to keep them safe and with backups**!
* **Source files are ignored**:
  + .meta
  + .txa (text animation)
  + .txo (text model) see [File Types](/wiki/Arma_Reforger:File_Types "Arma Reforger:File Types") page for full list
* Deflate compression is applied to the resulting files
  + .anm, .data, .edds, .et, .nmn, .wav, and .xob are excluded from compression
* Encryption

Depending on the size of the project, bundling process can take from few seconds to couple minutes.

After bundling is completed, a new pop up window will appear showing how much data is going to be uploaded to Workshop and asking you if you want to continue. After clicking yes, uploading process itself begins.

## Update

To update an already published mod, open the mod project again with the Workbench. Apply any changes needed (data, metadata), fill the new **Change Notes** field with the changes applied, and press **Publish button** to update the data.

Any change to description, visibility, pictures, etc has to be **bundled first** and then **published as new version**.

## Removing mod

[![armareforger-publishing-mod-removing-mod.png](/wikidata/images/thumb/0/0e/armareforger-publishing-mod-removing-mod.png/300px-armareforger-publishing-mod-removing-mod.png)](/wiki/File:armareforger-publishing-mod-removing-mod.png)

Mods can be removed from by selecting ***Workbench →* Remove from Workshop** option in **Resource Manager** top bar.

⚠

Warning, removing mods is **irreversible and permanent!** Proceed with caution!

ⓘ

If you lost your source addon you can still delete it by either downloading that mod in game Workshop, opening it in Workbench and then using Remove from Workshop or just by creating new addon, grabbing GUID of your addon from web version of the Workshop and then using that GUID in your addon: Create an empty folder that just gets an addon.gproj file with the well known structure and this GUID. Add this as a project, open it and then remove it from the workshop.

## CLI Parameters

Mods can be bundled and packed using command line parameters documented at [Startup Parameters - Workbench](/wiki/Arma_Reforger:Startup_Parameters#Workbench "Arma Reforger:Startup Parameters").

## Troubleshooting

If problems arise with logging to Bohemia Account or publishing mod, the first thing to do is to look at the **Console Log**.

### Failed to load metafile of mission config image

```
RESOURCES    : Packaging project successful
RESOURCES (E): Failed to load metafile of mission config image:
DEFAULT   (E): Cannot copy image:
RESOURCES (E): Creating bundle failed
```

Make sure that all mission configs in your mod are pointing to .edds files which have sources (like png, tiff or similar) available.

### You are not the owner of the asset

Make sure that the **Steam** **account** you are using is the same as the one you used for the initial upload.

### Sound map/Topology map/Navmesh is missing

```
DEFAULT   (W):     Sound map is missing
DEFAULT   (W):     Topology map is missing
DEFAULT   (W):     Navmesh is missing
```

This issue can be ignored if the published mod is not a terrain. Otherwise, make sure the terrain has all the listed things - see [2D Map Creation](/wiki/Arma_Reforger:2D_Map_Creation "Arma Reforger:2D Map Creation") and [Navmesh Tutorial](/wiki/Arma_Reforger:Navmesh_Tutorial "Arma Reforger:Navmesh Tutorial").

### Addon processing failed

```
DEFAULT (E): Addon processing on Workshop side failed! Error UID: "" Error Message : "Unknown Error"
```

* Make sure that your mod contains some content - it is not allowed to publish empty mods. See <https://feedback.bistudio.com/T193526>

### Workshop timeout

```
BACKEND   (E): [RestApi] ID:[54] TYPE:[EBREQ_WORKSHOP_UploadAssetFile] Error Code:524 - Unimplemented 5xx, apiCode="", uid="", message=""
```

Try publishing again later - Workshop is timing out
