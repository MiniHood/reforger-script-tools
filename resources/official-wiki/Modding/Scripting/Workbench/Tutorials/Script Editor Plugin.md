# [Script Editor Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor_Plugin)

This tutorial teaches how to create a [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")-specific plugin.

⚠

Please read [Workbench Plugin](/wiki/Arma_Reforger:Workbench_Plugin "Arma Reforger:Workbench Plugin") before following this tutorial.

## Setup

* Open Script Editor
* In an addon, create a new script in WorkbenchGame/ScriptEditor - name it [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin.c (must end with Plugin by convention)
* Double-click the file to open it
* Press `Ctrl` + `T` to use the [Script Template plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
  + In its window, select "Class Type: WorkbenchPlugin", leave the other fields blank/default
  + A Workbench plugin skeleton is inserted.
* In the WorkbenchPluginAttribute, replace cwbModules: { "ResourceManager" } by cwbModules: { "ScriptEditor" }
* In the cRun() method, write cPrint("It works!"); and save the file
* Reload Workbench scripts via **Reload WB Scripts** option located in *Plugins→Settings* menu (default shortcut: `Ctrl` + `⇧ Shift` + `R`)
* The TAG\_TutorialPlugin plugin should appear in the Script Editor's Plugins list, available in the top bar - click on the plugin entry
* "It works!" gets printed in the output console.

## ScriptEditor Module API

ⓘ

See the [ScriptEditor](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/ScriptEditor.c;14) class.

In the below code, the GetCurrentLine method is used to get the cursor location's line number (0-based index, first line = 0, second line = 1, etc).
The GetLineText/SetLineText methods are used to obtain and change the line's value by adding an inline comment indicating the current line number ("hardcoded" into the comment).

```enforce
[WorkbenchPluginAttribute(name: "Tutorial Plugin", description: "This tutorial plugin does something.", shortcut: "Ctrl+Shift+H", wbModules: { "ScriptEditor" }, awesomeFontCode: 0xF188)]
class TAG_TutorialPlugin : WorkbenchPlugin
{
	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		ScriptEditor scriptEditor = Workbench.GetModule(ScriptEditor);
		int lineNumber = scriptEditor.GetCurrentLine();
		string lineContent;
		scriptEditor.GetLineText(lineContent);
		lineContent += " // current line is " + (lineNumber + 1);
		scriptEditor.SetLineText(lineContent);
	}
}
```

⚠

Each text operation goes into one `Ctrl` + `Z` **each**, making reverting a multi-line text operation successive Ctrl+Zs.

## Configuration

Displayed parameters are declared values decorated with an Attribute; a full setup is done as follow:

```enforce
[WorkbenchPluginAttribute(name: "Tutorial Plugin", description: "This tutorial plugin does something.", shortcut: "Ctrl+Shift+H", wbModules: { "ScriptEditor" }, awesomeFontCode: 0xF188)]
class TAG_TutorialPlugin : WorkbenchPlugin
{
	[Attribute(desc: "Only display the line number without any other text")]
	protected bool m_bOnlyLineNumber;
	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		ScriptEditor scriptEditor = Workbench.GetModule(ScriptEditor);
		int lineNumber = scriptEditor.GetCurrentLine();
		string lineContent;
		scriptEditor.GetLineText(lineContent);
		if (lineContent.IsEmpty())
		{
			Workbench.Dialog("Empty line", "You cannot add a comment to a completely empty line."); // absolutely arbitrary tutorial decision :)
			return;
		}
		if (m_bOnlyLineNumber)
		lineContent += " // " + (lineNumber + 1).ToString();
		else
		lineContent += " // current line is " + (lineNumber + 1).ToString();

		scriptEditor.SetLineText(lineContent);
	}
	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		Workbench.ScriptDialog("Tutorial Plugin Configuration", "Usage:\nTick the checkbox or not, depending on if the line number should be written or not", this);
	}
}
```
