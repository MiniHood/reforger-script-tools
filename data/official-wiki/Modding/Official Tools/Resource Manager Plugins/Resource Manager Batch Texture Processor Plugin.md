# [Resource Manager: Batch Texture Processor Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Batch_Texture_Processor_Plugin)

| Batch Texture Processor |
| --- |
| [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") plugin |
| Perform simple checks and fixes on textures |
| **File:** [TextureImportTool.c](enfusion://ScriptEditor/scripts/WorkbenchGameCommon/TextureImportTool.c;866) |

The **Batch Texture Processor** plugin reprocesses textures (fitting the provided filters) and fixes them.

## Usage

Use it with Plugins > Batch texture processor; a window appears allowing to set all the below settings.

## Parameters

* **Path Starts With** - only check textures whose path starts with this value
* **Path Contains** - only check textures whose path contains this value
* **Path Ends With** - only check textures whose path ends with this value
* **Report Missing Meta File** - report when a texture exists without a meta file
* **Check Configurations** - check whether configurations are present and have correct classes
* **Check Ancestors** - check whether configurations have correct ancestors
* **Unnecessary Setting In Pc** - check whether PC configuration is directly setting same value as inherited from ancestor
* **Check Suspicious Non Pc Setting** - non-PC configuration property having different value than what is directly set in PC
* **Check Wrong Property Values** - check whether properties have values matching presets
* **Reimport Changed** - reimport changed textures
