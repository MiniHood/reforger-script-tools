# [Script Editor: Fill From Template Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin)

| Fill from Template |
| --- |
| [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") plugin |
| `Ctrl` + `T` |
| A plugin to help create a script skeleton based on templates |
| **File:** [SCR\_ScriptTemplatePlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_ScriptTemplatePlugin.c) |

**Fill from Template** is a plugin that allows adding content to a file (empty or not) based on the selected template.
This is useful to gain time by automatically preparing a class to be e.g a Workbench plugin and respect standards (e.g ending with "Component" for a component).

## Parameters

* **Class Type** - can be one of:

  + None - a normal class
  + Entity - a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") entity
  + Component - a scripted component
  + WidgetComponent - a widget component
  + ScriptInvoker - a [script invoker](/wiki/Arma_Reforger:ScriptInvoker_Usage "Arma Reforger:ScriptInvoker Usage")
  + ScriptedUserAction - a user action script
  + UIMenu - a UI menu script
  + ConfigRoot - a config
  + WorkbenchPlugin - a [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools") plugin (any editor)
  + WorldEditorTool - a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") tool

:   ⓘ

    This type is defined by the [SCR\_EScriptTemplateType](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_ScriptTemplatePlugin.c;136) enum.

* **Class Name** - if left empty, it will use the file name
* **Parent Name** - if left empty, the class does not inherit from anything unless the template mentions an inheritance

ⓘ

See the Template config ([ScriptTemplateConfig.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Workbench/ScriptTemplatePlugin/ScriptTemplateConfig.conf), an array of [SCR\_ScriptTemplateConfigEntry](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_ScriptTemplatePlugin.c;120)) to know more about Template configuration.

## See Also

* [Script Editor: Create New Script Plugin](/wiki/Arma_Reforger:Script_Editor:_Create_New_Script_Plugin "Arma Reforger:Script Editor: Create New Script Plugin")
* [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor")
