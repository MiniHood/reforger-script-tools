# [Workbench Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Workbench_Plugin)

**Workbench Plugins** are script files that can be triggered from within any editor (Resource Browser, World Editor, Script Editor, etc).

Existing plugins are listed in Data\Scripts\WorkbenchGame and are sorted by directories.

| Editor | Directory | API Class (Module Type) |
| --- | --- | --- |
| Common Plugins | Data\Scripts\WorkbenchGame | N/A |
| Resource Manager | Data\Scripts\WorkbenchGame\ResourceManager | [ResourceManager](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/ResourceManager.c;14) |
| World Editor (Tools and Plugins) | Data\Scripts\WorkbenchGame\WorldEditor | [WorldEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/WorldEditor.c;14) |
| Particle Editor | N/A | N/A |
| Animation Editor | N/A | N/A |
| Script Editor | Data\Scripts\WorkbenchGame\ScriptEditor | [ScriptEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/ScriptEditor.c;14) |
| Audio Editor | N/A | N/A |
| Behavior Editor | N/A | N/A |
| String Editor | Data\Scripts\WorkbenchGame\LocalizationEditor | [LocalizationEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/LocalizationEditor.c;14) |
| Procedural Animation Editor | N/A | N/A |

A **Plugin** must be named Classname**Plugin**, and its file too.

A **Tool** must be named Classname**Tool**, and its file too.

## Plugin

A plugin inherits from WorkbenchPlugin and is decorated with a WorkbenchPluginAttribute attribute which signature is as follow:

```enforce
WorkbenchPluginAttribute(string name, string description = "", string shortcut = "", string icon = "", array<string> wbModules = null, string category = "", int awesomeFontCode = 0)
```

* name is mandatory: it is the plugin's display name
* description: optional, it is the on hover description text
* shortcut: in format e.g "Ctrl+Shift+I" for `Ctrl` + `⇧ Shift` + `I` - none (empty string) can be defined, the plugin will then need to be triggered from the Plugin top menu
* icon
* wbModules: to which editors does this plugin apply (e.g wbModules = { "ScriptEditor" })
* category: the plugins menu entry in which this plugin will find itself (e.g Plugins > Text > *Plugin Name*)

  ⓘ

  category accepts forward slash / to create sub-categories.
* awesomeFontCode: the FontAwesome icon associated with the plugin (see <https://fontawesome.com/FontAwesome's> [Free Icons Cheatsheet](https://fontawesome.com/v5/search?o=r&m=free))

A plugin must also override either or both Run or RunCommandLine methods in order to have an impact.
It can also, but is not mandatory, override the Configure method to display a settings entry.

## Tool

A tool is a system that allows for direct manipulation with a config panel available on the side.

ⓘ

For now, Tools are only available for the World Editor, and inherit from the WorldEditorTool class.

A tool inherits from the editor-related class (e.g World Editor: WorldEditorTool) in order to be found in said editor's Tools menu.

It is decorated with a WorkbenchToolAttribute attribute which signature is identical to WorkbenchPluginAttribute (see above).

## Scripting

### Modules

A plugin has access to the currently loaded game/project resources, but in order to be as adaptable as possible it should also be generic.

Each Workbench module (editor) API can be accessed through the following script:

```enforce
ModuleType moduleType = Workbench.GetModule(ModuleType);
```

Where ModuleType can be one of the classes listed [at the beginning of this document](#mw-content-text), all children of the WBModuleDef class).

Each module has obviously a different API - see their classes for more information.

### Plugins

Other plugins can be accessed through caWorkbenchModule.GetPlugin(TAG\_ClassNamePlugin);.

### Common Methods

See [WorkbenchPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorkbenchPlugin.c;14).

#### Run

The Run method is called when clicking Plugins > *Plugin Name* or using its shortcut if any. If this method is not overridden with a non-empty code, the plugin does not appear in this menu.

ⓘ

There is no way to distinguish between clicking the plugin in the menu or using the shortcut.

#### RunCommandline

The RunCommandline method is called when calling the script from [Startup Parameters](/wiki/Arma_Reforger:Startup_Parameters "Arma Reforger:Startup Parameters")'s [plugin](/wiki/Arma_Reforger:Startup_Parameters#plugin "Arma Reforger:Startup Parameters") option, e.g:

```
ArmaReforgerWorkbenchSteam.exe -wbModule=ScriptEditor -plugin=TAG_MyPlugin pluginArguments
```

#### Configure

The Configure method is called when clicking Plugins > Settings > *Plugin Name*. If this method is not overridden with a non-empty code, the entry does not appear in this menu.

ⓘ

Think of the plugin's behaviour: should the end user encounter a configuration popup:

* on each plugin usage (e.g from the cRun method)
* only when reaching for the plugin settings
* and/or should only *some* settings be offered on every usage?

#### OnResourceContextMenu

The OnResourceContextMenu method is called when clicking [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") Resource Browser Context Menu's Plugins > *Plugin Name*.
{{Feature|important|For the plugin to appear, WorkbenchPluginAttribute's resourceTypes parameter must be defined (e.g cresourceTypes: { "fbx", "xob", "et" })

### Generic Modal

The c[Workbench](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Workbench.c;14).Dialog() method can be used to create a modal addressed to the user. It takes a caption (modal title) a text (in-modal description text), and a detailed text if needed.

The end result is a modal with an OK button and a "Show Details" button if a detailed text has been provided.

### Scripted Modal

The c[Workbench](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Workbench.c;14).ScriptDialog() method can be used to create a modal addressed to the user to confirm an action or set values. It can be used in any method (e.g not only Configure).
It takes a caption (modal title) a text (in-modal description text), and a class instance - usually cthis.

The class instance can be anything but null, but it is much more interesting to offer options to the user, at least an OK/Cancel button choice.

ⓘ

If no buttons are defined, the modal can still be closed through the close button (« × ») or using `Alt` + `F4`.

#### Attributes

Same as a [Config Object](/wiki/Arma_Reforger:Scripting:_Config_Object "Arma Reforger:Scripting: Config Object") declaration, the c[[Attribute](enfusion://ScriptEditor/scripts/Core/generated/Attributes/Attribute.c;12)()] decorator is required.

```enforce
[Attribute(defvalue: "1", desc: "Does something, otherwise does nothing")]
protected m_bDoSomething;
```

#### Buttons

A button method is decorated with a c[Button()] decorator which signature is as follow:

```enforce
ButtonAttribute(string label = "ScriptButton", bool focused = false)
```

A button method's return type can be anything, but the value will be treated as a boolean int (1 = true, 0 = false). A void method will return 0.

⚠

Beware, as an empty string result will convert as a 1.

```enforce
[ButtonAttribute("OK", true)] // focused by default
int ButtonOK()
{
	return 42; // Workbench.ScriptDialog will return 1/true
}
[ButtonAttribute("Cancel")]
int ButtonCancel()
{
	return 0; // Workbench.ScriptDialog will return 0/false
}
```

The Workbench.ScriptDialog's return value can then be used to know if the user confirmed or cancelled an interface.

ⓘ

By default, a scripted dialog does not have any buttons and clicking on the close button (« × ») or using `Alt` + `F4` will make the Workbench.ScriptDialog method return 0.

## Tutorials

See [Plugin/Tool tutorials](/wiki/Category:Arma_Reforger/Modding/Scripting/Workbench/Tutorials "Category:Arma Reforger/Modding/Scripting/Workbench/Tutorials").
