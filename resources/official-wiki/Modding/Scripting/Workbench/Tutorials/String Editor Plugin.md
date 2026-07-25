# [String Editor Plugin](https://community.bistudio.com/wiki/Arma_Reforger:String_Editor_Plugin)

This tutorial teaches how to create a [String Editor](/wiki/Arma_Reforger:String_Editor "Arma Reforger:String Editor")-specific plugin.

⚠

Please read [Workbench Plugin](/wiki/Arma_Reforger:Workbench_Plugin "Arma Reforger:Workbench Plugin") before following this tutorial.

## Setup

* Open [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")
* In an addon, create a new script in WorkbenchGame/LocalizationEditor - name it [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin.c (must end with Plugin by convention)
* Double-click the file to open it
* Press `Ctrl` + `T` to use the [Script Template plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
  + In its window, select "Class Type: WorkbenchPlugin", **set the parent class to [LocalizationEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/LocalizationEditorPlugin.c;14)** and leave the other fields blank/default
  + A Workbench plugin skeleton is inserted.
* In the WorkbenchPluginAttribute, replace cwbModules: { "ResourceManager" } by cwbModules: { "LocalizationEditor" }
* Make the plugin inherit from c[LocalizationEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/LocalizationEditorPlugin.c;14) instead of c[WorkbenchPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorkbenchPlugin.c;14)
* In the cRun() method, write c[Workbench](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Workbench.c;14).Dialog("Does it?", "It works!"); (the String Editor not having a log console) and save the file
* Reload Workbench scripts via **Reload WB Scripts** option located in **Plugins → Settings** (default shortcut: `Ctrl` + `⇧ Shift` + `R`)
* The [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin plugin should appear in the String Editor's Plugins list, available in the top bar - click on the plugin entry
* The "It works!" modal appears on screen.

## LocalizationEditorPlugin API

ⓘ

See the [LocalizationEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/LocalizationEditorPlugin.c;14) class.

## LocalizationEditor Module API

ⓘ

See the [LocalizationEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/LocalizationEditor.c;14) class.

The String Editor API allows
In the below code:

* the cGetTable method is used to grab the table as a [BaseContainerList](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainerList.c;12)
* the cOnSelectionChanged method is used to Print the currently selected row IDs.

```enforce
[WorkbenchPluginAttribute(name: "Tutorial Plugin", shortcut: "Ctrl+Shift+H", wbModules: { "LocalizationEditor" }, awesomeFontCode: 0xF188)]
class TAG_TutorialPlugin : LocalizationEditorPlugin
{
	[Attribute(defvalue: "", desc: "If no username is provided, the Windows username will be used")]
	protected string m_sAuthorName;
	protected ref array<ResourceName> m_aResourceNames;
	protected ref array<string> m_aFilePaths;
	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		LocalizationEditor stringEditor = Workbench.GetModule(LocalizationEditor);
		array<int> rows = {};
		stringEditor.GetSelectedRows(rows);
		if (rows.IsEmpty())
		{
			Workbench.Dialog("Input Error", "Please enter a valid author name");
			return;
		}
		BaseContainer stringTable = stringEditor.GetTable();
		if (!stringTable)
		{
			Print("No string table opened", LogLevel.WARNING);
			return;
		}
		BaseContainerList items = stringTable.GetObjectArray("Items"); // entries - cannot be empty since rows are selected
		if (Workbench.ScriptDialog("Author", "Please input the wanted author's name for these entries.", this) == 0)
		return; // on Cancel
		m_sAuthorName.Trim();
		if (m_sAuthorName.IsEmpty())
		{
			Workbench.GetUserName(m_sAuthorName);
			if (m_sAuthorName.IsEmpty())
			{
				Workbench.Dialog("Input Error", "Please enter a valid author name");
				return;
			}
		}
		BaseContainer item;
		string itemId, itemAuthor;

		stringEditor.BeginModify("RowEdit"); // prepare the Undo operation
		foreach (int row : rows)
		{
			item = items.Get(row);
			item.Get("Id", itemId);
			item.Get("Author", itemAuthor);
			Print("Changing #" + itemId + "'s Author from '" + itemAuthor + "' to '" + m_sAuthorName + "'", LogLevel.NORMAL); // log for manual Undo if needed
			stringEditor.ModifyProperty(item, item.GetVarIndex("Author"), m_sAuthorName);
		}

		stringEditor.EndModify(); // wrap the Undo operation
		// stringEditor.Save();					// we allow the user to save, let's not override data ourselves
	}
	//------------------------------------------------------------------------------------------------
	override void OnSelectionChanged()
	{
		LocalizationEditor stringEditor = Workbench.GetModule(LocalizationEditor);
		array<int> rows = {};
		stringEditor.GetSelectedRows(rows);
		Print("Selected rows: " + rows, LogLevel.NORMAL);
	}
	//------------------------------------------------------------------------------------------------
	[ButtonAttribute("OK", true)]
	bool ButtonOK()
	{
		return true;
	}
	//------------------------------------------------------------------------------------------------
	[ButtonAttribute("Cancel")]
	bool ButtonCancel()
	{
		return false;
	}
}
```
