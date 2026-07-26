# [Resource Manager Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager_Plugin)

This tutorial teaches how to create a [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager")-specific plugin.

⚠

Please read [Workbench Plugin](/wiki/Arma_Reforger:Workbench_Plugin "Arma Reforger:Workbench Plugin") before following this tutorial.

## Setup

* Open [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")
* In an addon, create a new script in WorkbenchGame/ResourceManager - name it [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin.c (must end with Plugin by convention)
* Double-click the file to open it
* Press `Ctrl` + `T` to use the [Script Template plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
  + In its window, select "Class Type: WorkbenchPlugin", **set the parent class to [ResourceManagerPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/ResourceManagerPlugin.c;14)** and leave the other fields blank/default
  + A Workbench plugin skeleton is inserted.
* In the cRun() method, write cPrint("It works!"); and save the file
* Reload Workbench scripts via **Reload WB Scripts** option located in *Plugins→Settings* menu (default shortcut: `Ctrl` + `⇧ Shift` + `R`)
* The [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialPlugin plugin should appear in the Resource Manager's Plugins list, available in the top bar - click on the plugin entry
* "It works!" gets printed in the output console.

## Contextual Menu Option

The Resource **Browser**'s Contextual Menu can provide a Plugin option allowing to run code on the selected resource(s) of the defined type(s).

ⓘ

The Plugin Contextual Menu is only available in Resource Manager, not any other Workbench module.

In the [WorkbenchPluginAttribute](enfusion://ScriptEditor/scripts/GameLib/workbench/workbench.c;61), the resourceTypes parameter must be filled, e.g cresourceTypes: { "et", "c" }.

The cOnResourceContextMenu method must be overridden to work with said resources.

```enforce
[WorkbenchPluginAttribute(name: "Tutorial Plugin", wbModules: { "ResourceManager" }, resourceTypes: { "et", "c" })]
class TAG_TutorialPlugin : ResourceManagerPlugin
{
	//------------------------------------------------------------------------------------------------
	override void OnResourceContextMenu(notnull array<ResourceName> resources)
	{
		Print("Resource context menu action has been called! Here are the selected resources:");
		foreach (ResourceName resource : resources)
		{
			if (resource.EndsWith("c"))
			Print("- Script File: " + resource);
			else
			Print("- Prefab File: " + resource);
		}
	}
}
```

## ResourceManager Module API

ⓘ

See the [ResourceManager](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Modules/ResourceManager.c;14) class.

The Resource Manager API allows to register resources, rebuild them, get the selected ones or get their meta file content.
In the below code:

* the cGetResourceBrowserSelection method is used to temporarily store Resource Browser-selected resources in cm\_aResourceNames and their file path in cm\_aFilePaths
* the cGetMetaFile method is used to get the current resource's .meta file as a [MetaFile](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/MetaFile.c;14) instance.

```enforce
[WorkbenchPluginAttribute(name: "Tutorial Plugin", wbModules: { "ResourceManager" }, awesomeFontCode: 0xF188)]
class TAG_TutorialPlugin : ResourceManagerPlugin
{
	protected ref array<ResourceName> m_aResourceNames;
	protected ref array<string> m_aFilePaths;
	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
		m_aResourceNames = {};
		m_aFilePaths = {};
		resourceManager.GetResourceBrowserSelection(WorkbenchSearchResourcesCallbackMethod, false); // non-recursive search
		if (m_aFilePaths.IsEmpty())
		{
			Print("No Resources selected in Resource Browser");
			return;
		}
		MetaFile metaFile;
		foreach (int i, ResourceName filePath : m_aFilePaths)
		{
			metaFile = resourceManager.GetMetaFile(filePath);
			if (!metaFile) // e.g directory
			continue;
			PrintFormat(
			"#%1: %3 (dir %2)",
			i,
			metaFile.GetSourceFilePath(), // returns the directory tree, e.g $MyMod:Path/To/
			metaFile.GetResourceID()); // returns the file's ResourceName, equal to 'resName' below
		}
	}
	//------------------------------------------------------------------------------------------------
	// \param[in] resName
	// \param[in] filePath is in format $MyMod:Path/To/File.ext
	protected void WorkbenchSearchResourcesCallbackMethod(ResourceName resName, string filePath = "")
	{
		m_aResourceNames.Insert(resName);
		m_aFilePaths.Insert(filePath);
	}
}
```
