# [Workbench Metadata](https://community.bistudio.com/wiki/Arma_Reforger:Workbench_Metadata)

ⓘ

See the [Metadata](https://en.wikipedia.org/wiki/Metadata) Wikipedia article.

## .meta File

A .meta file is created whenever a resource (not scripts) is registered or created in the Workbench. This file holds metadata such as:

* the resource's [GUID](https://en.wikipedia.org/wiki/Universally_unique_identifier) - a unique identifier for the engine to track
* the original file's path (informational, only the GUID is relevant to the engine)
* platform build configuration:
  + file: import configuration - the one that is set in the Resource Manager's [Import Settings](/wiki/Arma_Reforger:Resource_Manager#Import_Settings "Arma Reforger:Resource Manager") tab (e.g file's build presence, texture import quality, etc)
  + directory: properties set by editing a registered directory using "Edit Properties" (e.g directory's build presence, build tag)

ⓘ

A runtime file (.ent, .et, .json, .edds, but not .fbx, .tiff, .tga) without a .meta file will throw a warning in the Log Console about missing it.

### GUID

* The GUID is generated based on the file's path and name (using them as [seed](https://en.wikipedia.org/wiki/Random_seed) - regenerating the metadata of a file with identical path and name will generate the same GUID)
  + If file with same GUID already exist in one of the currently loaded addons, Workbench will generate a new random GUID for that file
* Once the meta file is generated, moving or renaming the file will not change its GUID (the meta file obviously **must** be renamed along the main file - this is done automatically when renamed from the Workbench)

#### GUID Change

⚠

Changing a GUID manually is not a trivial operation - be careful doing it, and make backups.

* Two files with the same GUID in two different addons will overwrite each other (see [Data Modding Basics - Can Be Modified](/wiki/Arma_Reforger:Data_Modding_Basics#Can_Be_Modified "Arma Reforger:Data Modding Basics"))
* Two files with the same GUID in a single addon will throw a warning message in the Log Console and one of the files will not work properly.

A resource's GUID can be obtained can be obtained via the "Copy Resource GUID(s)" action [Resource Manager - Contextual menus](/wiki/Arma_Reforger:Resource_Manager#Contextual_menus "Arma Reforger:Resource Manager").

A GUID should normally not be changed, but a conflict can, in rare cases, happen. There are two ways to change a GUID:

* rename the resource file, delete the .meta file, reimport the resource and rename the file back to the original name (safer, but any import configuration will be lost)
* generate a new GUID with the Resource Browser utility (see [Resource Manager - Generate GUID](/wiki/Arma_Reforger:Resource_Manager#Generate_GUID "Arma Reforger:Resource Manager")) and replace it manually in the .meta file

## resourceDatabase.rdb File

This binary file is a database storing information of all resources available in the provided project (base game, addon).
Its content changes when a resource is added or removed, and the file is verified and refreshed/re-created on Workbench opening and closing.
