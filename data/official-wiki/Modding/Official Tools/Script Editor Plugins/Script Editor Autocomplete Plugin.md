# [Script Editor: Autocomplete Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_Autocomplete_Plugin)

| Autocomplete |
| --- |
| [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") plugin |
| `Ctrl` + `Return ↵` |
| A plugin to autocomplete/autofix the current line |
| **File:** [SCR\_AutocompletePlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_AutocompletePlugin.c) |

The **Autocomplete** [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") plugin helps script authors by providing autocompletion and smart insertions for e.g common code structures, code snippets, boilerplate code etc. It also assists with attribute decorators, log level consistency, null and validity checks, and more.

## Features

* Script snippets on keyword (see [Default Keywords](#Default_Keywords) below for a full list of default keywords)
* Instantiation help (see [Autocomplete](#Autocomplete) below)
* Safety checks: adds a null check after a cast (if not present), adds a validity check after c[Resource](enfusion://ScriptEditor/scripts/Core/generated/Resources/Resource.c;24).Load (if not present)
* Attribute decorators: automatically suggests/adds appropriate c[[Attribute](enfusion://ScriptEditor/scripts/Core/generated/Attributes/Attribute.c;12)(/\* ... \*/)] decorators for variables
* Log level consistency: ensures cPrint and cPrintFormat method calls have explicit log levels
* User customisation: supports user-defined keywords/snippets and attribute decorators; defaults can be edited or reset.

## How to Use

In Script Editor, type a supported keyword or code pattern then press `Ctrl` + `Return ↵` to execute the plugin.
The keyword gets replaced with the corresponding snippet, or the line will be auto-fixed/augmented as needed.
Many behaviours are configurable in the plugin settings via Workbench and custom keywords can be added.

## Default Keywords

| Keyword | Description | Result |
| --- | --- | --- |
| if | if structure | Show text ENFORCECODEMARKER   ``` if (condition) { } ``` |
| ife | if-else structure | Show text ENFORCECODEMARKER   ``` if (condition) { } else { } ``` |
| for | for loop temporarily storing the iteratedclength | Show text ENFORCECODEMARKER   ``` for (int i, count = arr.Count(); i < count; i++) { } ``` |
| forr | Reversed for loop (from end to start) | Show text ENFORCECODEMARKER   ``` for (int i = arr.Count() - 1; i >= 0; i--) { } ``` |
| foreach | foreach loop (item type to be replaced) | Show text ENFORCECODEMARKER   ``` foreach (SCR_Class item : items) { } ``` |
| foreachi | Indexed foreach loop | Show text ENFORCECODEMARKER   ``` foreach (int i, SCR_Class item : items) { } ``` |
| switch | switch-case structure | Show text ENFORCECODEMARKER   ``` switch (value) { 	case 0: 	break; 	default: 	break; } ``` |
| while | while structure | Show text ENFORCECODEMARKER   ``` while (condition) { } ``` |
| class | class basic structure | Show text ENFORCECODEMARKER   ``` class SCR_MyClass { 	protected string m_sValue = "Generated class"; 	//------------------------------------------------------------------------------------------------ 	//! constructor 	void SCR_MyClass() 	{ 	} } ``` |
| method/func | Method/Function structure (indentation depending on keyword's indentation) | Show text ENFORCECODEMARKER   ``` //------------------------------------------------------------------------------------------------ //! \param[in] parameter protected void Method(int parameter) { } ``` |
| ctor/dtor | Current class's constructor/destructor c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) constructor if class name ends with Entity | Show text ENFORCECODEMARKER   ``` //------------------------------------------------------------------------------------------------ // constructor void SCR_CurrentClassName() { } ```   ENFORCECODEMARKER   ``` //------------------------------------------------------------------------------------------------ // constructor void SCR_CurrentClassNameEntity(IEntitySource src, IEntity parent) { } ```   ENFORCECODEMARKER   ``` //------------------------------------------------------------------------------------------------ // destructor void ~SCR_CurrentClassName() { } ``` |
| findcomp | Script component-finding structure with null check | Show text ENFORCECODEMARKER   ``` SCR_ComponentClass component = SCR_ComponentClass.Cast(entity.FindComponent(SCR_ComponentClass)); if (!comp) return; ``` |
| print | A simple cPrint line with log level | Show text ENFORCECODEMARKER   ``` Print("fill", LogLevel.NORMAL); ``` |

## Autocompletion

| Type | Examples |
| --- | --- |
| Instantiation | Show text ENFORCECODEMARKER   ``` SCR_Class instance = // or SCR_Class instance = new // or SCR_Class instance = new SCR_Class; // becomes SCR_Class instance = new SCR_Class(); ```   ENFORCECODEMARKER   ``` map<string, ref SCR_MyLongClassName> myMap = // becomes map<string, ref SCR_MyLongClassName> myMap = new map<string, ref SCR_MyLongClassName>(); ```   ENFORCECODEMARKER   ``` array<string> list = new array<string>(); // becomes array<string> list = {}; ``` |
| Attribute decorator creation | Show text ENFORCECODEMARKER   ``` protected bool m_bShowWhenEmpty; // becomes [Attribute(defvalue: "1", desc: "Show When Empty")] protected bool m_bShowWhenEmpty; ```   ENFORCECODEMARKER   ``` protected SCR_EBoxSide m_eBoxSide; // becomes [Attribute(defvalue: "0", desc: "Side To Show", uiwidget: UIWidgets.ComboBox, enumType: SCR_EBoxSide)] protected SCR_EBoxSide m_eSideToShow; ``` |
| Log level check | Show text ENFORCECODEMARKER   ``` Print("Hello there"); // becomes Print("Hello there", LogLevel.NORMAL); ```   ENFORCECODEMARKER   ``` PrintFormat("General %1!", "Kenobi"); // or PrintFormat("General %1!", "Kenobi", LogLevel.NORMAL); // becomes PrintFormat("General %1!", "Kenobi", level: LogLevel.NORMAL); ```   ENFORCECODEMARKER   ``` PrintFormat("You are a %1 one", "bold", LogLevel.WARNING); // becomes PrintFormat("You are a %1 one", "bold", level: LogLevel.WARNING); ``` |
| Cast null check | Show text ENFORCECODEMARKER   ``` SCR_Class casted = SCR_Class.Cast(instance); Print(casted.m_Value); // becomes SCR_Class casted = SCR_Class.Cast(instance); if (!casted) return; Print(casted.m_Value); ``` |
| Resource validity check | Show text ENFORCECODEMARKER   ``` Resource resource = Resource.Load(resourceName); BaseContainer baseContainer = resource.GetResource().ToBaseContainer(); // becomes Resource resource = Resource.Load(resourceName); if (!resource.IsValid()) return; BaseContainer baseContainer = resource.GetResource().ToBaseContainer(); ``` |

## Settings Overview

* Enable/Disable constructor/destructor/autoinstantiate/fix features
* Choose when instantiation assistance triggers (on "= new", "=" only or both)
* Configure default cPrint/cPrintFormat log level
* Edit tool/user keyword pairs and attribute decorator presets
* All settings available via plugin configuration dialog.

## Customisation

* Custom keywords/snippets and attribute templates are supported via settings.
* Tool-provided (default) and user-defined lists are kept separate.

## See Also

* [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")
* [Script Editor Plugins](/wiki/Category:Arma_Reforger/Modding/Official_Tools/Script_Editor_Plugins "Category:Arma Reforger/Modding/Official Tools/Script Editor Plugins")
* [Script Editor: Fill From Template Plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
