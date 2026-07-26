# [World Editor Plugin](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor_Plugin)

ⓘ *Not to be confused with [World Editor Tool](/wiki/Arma_Reforger:World_Editor_Tool "Arma Reforger:World Editor Tool").*

---

This tutorial teaches how to create a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor")-specific plugin.

⚠

Please read [Workbench Plugin](/wiki/Arma_Reforger:Workbench_Plugin "Arma Reforger:Workbench Plugin") before following this tutorial.

## Setup

* Open Script Editor
* In an addon, create a new script in WorkbenchGame/WorldEditor - name it [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin.c (must end with Plugin by convention)
* Double-click the file to open it
* Press `Ctrl` + `T` to use the [Script Template plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
  + In its window, select "Class Type: WorkbenchPlugin", leave the other fields blank/default
  + A Workbench plugin skeleton is inserted.
* In the WorkbenchPluginAttribute, replace cwbModules: { "ResourceManager" } by cwbModules: { "WorldEditor" }
* Make the plugin inherit from c[WorldEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorldEditorPlugin.c;14) instead of c[WorkbenchPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorkbenchPlugin.c;14)
* In the cRun() method, write cPrint("It works!"); and save the file
* Unlike other plugins, **Rebuild Scripts** using the `⇧ Shift` + `F7` default shortcut
* Reload Workbench scripts via **Reload Scripts** option located in **Plugins → Settings** (default shortcut: `Ctrl` + `⇧ Shift` + `R`)
* The TAG\_TutorialPlugin plugin should appear in the World Editor's Plugins list, available in the top bar - click on the plugin entry
* "It works!" gets printed in the output console.

## WorldEditorPlugin API

ⓘ

See the [WorldEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorldEditorPlugin.c;14) class.

The WorldEditorPlugin class only exposes some events on which it is possible to subscribe (e.g cOnGameModeEnded()).

## WorldEditor Module API

ⓘ

See the [WorldEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/WorldEditor.c;14) class.

The WorldEditor module allows to do some minor operations (e.g get Resource Browser's selection, get current world's boundaries).

```enforce
vector min, max;
WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
if (!worldEditor)
return;

worldEditor.GetWorldBounds(min, max);
Print("Min = " + min);
Print("Max = " + max);
```

The meat of the matter is happening in c[WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12), available through the c[WorldEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/WorldEditor.c;14).GetApi() method - see [below](#WorldEditorAPI_API).

## WorldEditorAPI API

ⓘ

See the [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) class as well as [WorldEditorAPI Usage](/wiki/Arma_Reforger:WorldEditorAPI_Usage "Arma Reforger:WorldEditorAPI Usage").

## Example

In this example, we will use c[WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) methods to hide/show all selected entities (it will hide a shown entity and vice versa):

```enforce
#ifdef WORKBENCH
[WorkbenchPluginAttribute(name: "Tutorial Plugin", shortcut: "Ctrl+Shift+H", wbModules: { "WorldEditor" }, awesomeFontCode: 0xF188)]
class TAG_TutorialPlugin : WorldEditorPlugin
{
	override void Run()
	{
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		return;
		WorldEditorAPI worldEditorAPI = worldEditor.GetApi();
		IEntitySource entitySource;

		worldEditorAPI.BeginEntityAction();
		for (int i = worldEditorAPI.GetSelectedEntitiesCount() - 1; i >= 0; --i)
		{
			entitySource = worldEditorAPI.GetSelectedEntity(i);
			if (!entitySource)
			continue;

			worldEditorAPI.SetEntityVisible(entitySource, !worldEditorAPI.IsEntityVisible(entitySource), false);
		}
		worldEditorAPI.EndEntityAction();
	}
}
#endif
```

ⓘ

For a more advanced c[WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) usage, see [World Editor Tool](/wiki/Arma_Reforger:World_Editor_Tool "Arma Reforger:World Editor Tool").
