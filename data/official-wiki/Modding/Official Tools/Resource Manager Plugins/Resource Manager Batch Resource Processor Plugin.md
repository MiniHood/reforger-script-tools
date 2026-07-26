# [Resource Manager: Batch Resource Processor Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Batch_Resource_Processor_Plugin)

| Batch Resource Processor |
| --- |
| [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") plugin |
| Perform simple checks and fixes on resource files |
| **File:** [ResourceProcessorTool.c](enfusion://ScriptEditor/scripts/WorkbenchCommon/ResourceProcessorTool.c) |

The **Batch Resource Processor** plugin reprocesses resources (fitting the provided filters) and fixes Prefab/MeshObject/Sound/ParticleEffect meta files.

## Usage

Use it with Plugins > Batch resource processor; a window appears allowing to set all the below settings.

## Parameters

* **File Types** - types (file extensions) to process
* **Path Starts With** - only check resources whose path starts with this value
* **Path Contains** - only check resources whose path contains this value
* **Path Ends With** - only check resources whose path ends with this value
* **Fix Meta File** - change configuration inheritance, etc - note that meta file can still be re-saved by other actions
* **Force Resave Meta File** - resaves meta file even without changes
* **Report Missing Meta File** - report missing meta file
* **Report Missing Configurations** - report missing configurations
* **Report Custom Property Values** - report custom property values
* **Fix Custom Property Values** - fix custom property values (apply all PC custom changes to all other platforms)

## Command Line

This plugin can be used through command line usage:

```
ArmaReforgerWorkbenchSteam.exe -wbModule=ResourceManager -plugin=ResourceProcessorPlugin pluginArguments
```

It uses the following arguments:

* -FileTypes to specify **File Types** as defined above, separated by a comma (e.g -FileTypes="xob,edds")
* path filters (e.g -PathStartsWith="$ArmaReforger:Assets/Structures/Houses"):
  + -PathStartsWith
  + -PathContains
  + -PathEndsWith
