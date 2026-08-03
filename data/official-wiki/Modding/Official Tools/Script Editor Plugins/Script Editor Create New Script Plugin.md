# [Script Editor: Create New Script Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_Create_New_Script_Plugin)

| Create New Script |
| --- |
| [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") plugin |
| `Ctrl` + `N` |
| A plugin to help create a new file |
| **File:** [SCR\_NewScriptPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_NewScriptPlugin.c) |

**Create New Script** is a plugin that fulfills the "Create New Script" feature for the [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor").

## Parameters

* **Addon** - in which addon the file will be created
* **Destination Directory** - in which *directory tree* the file will be created:  
  It is possible to select an "ArmaReforger" directory to create the file and the directory tree in the selected addon, e.g

  ```
  Addon: MyAddon
  Selected directory: $ArmaReforger:scripts/Game/Building
  End destination: $MyAddon:scripts/Game/Building
  ```
* **Parent Class Name** - the parent class name, if any - it may be overridden by config's parent class name
* **Type** - the script type (same as [Fill From Template's Type](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin#Parameters "Arma Reforger:Script Editor: Fill From Template Plugin"))
* **Template Config File** - the template config to use, storing the different types's templates

## See Also

* [Script Editor: Fill From Template Plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
* [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor")
