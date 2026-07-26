# [BaseContainer Usage](https://community.bistudio.com/wiki/Arma_Reforger:BaseContainer_Usage)

A [BaseContainer](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainer.c;12) is a data holder for many kinds of data: Prefab, Config, [IEntitySource](enfusion://ScriptEditor/scripts/Core/generated/Containers/IEntitySource.c;12), [IEntityComponentSource](enfusion://ScriptEditor/scripts/Core/generated/Containers/IEntityComponentSource.c;12), [WidgetSource](enfusion://ScriptEditor/scripts/Core/generated/Containers/WidgetSource.c;12), [UserSettings](enfusion://ScriptEditor/scripts/Core/generated/Containers/UserSettings.c;12), [MetaFile](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/MetaFile.c;14) are all BaseContainers.

## Structure

Let's take a look at the [Tree\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vegetation/Core/Tree_Base.et) Prefab to understand a BaseContainer's structure:

```
Tree {
 // ...
 Flags 524291
 MaxHealth 10000
 // ...
}
```

* [Tree](enfusion://ScriptEditor/scripts/GameCode/Vegetation/Tree.c;10) is the BaseContainer's class
* there is no inheritance, this is the topmost Prefab; inheritance (e.g in [Tree\_Coniferous\_Base.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vegetation/Tree/Tree_Coniferous_Base.et)) looks like this:

  ```
  Tree : "{388AE316D09D0680}Prefabs/Vegetation/Core/Tree_Base.et" {
   // ...
  }
  ```

  + Tree being the class (a Prefab can have a class that is different from its ancestor)
  + "{388AE316D09D0680}Prefabs/Vegetation/Core/Tree\_Base.et" being the ancestor, the inherited Prefab
* its Flags value is 524291, which is the General > Flags value of Traceable, Visible and Relative Y flags together
* its MaxHealth value is 10000, which is a GameCode-side property (not defined in script)

### Inheritance

In case of inheritance:

* every level can set a property's value
* if the value is not defined in a level, the value is read from its ancestor where it is defined (making it a "default value" for its children)
* if a property is not set anywhere, the default value is the type's default value (see [Scripting: Values - Types](/wiki/Arma_Reforger:Scripting:_Values#Types "Arma Reforger:Scripting: Values"))

### Object Array

A property can be an array of objects, represented as a [BaseContainerList](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainerList.c;12).

## Usage

a BaseContainer **cannot** be strongly referenced in script (cref [BaseContainer](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainer.c;12) m\_BaseContainer; // not possible).

See [BaseContainerTools](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainerTools.c;12) and [SCR\_BaseContainerTools](enfusion://ScriptEditor/scripts/Game/Global/SCR_BaseContainerTools.c;3) classes for useful methods.

ⓘ

For Resource-related BaseContainers, see also [Resource Usage](/wiki/Arma_Reforger:Resource_Usage "Arma Reforger:Resource Usage").

### Create

```enforce
Resource resource = BaseContainerTools.CreateContainer("GenericEntity");
BaseContainer baseContainer = resource.GetResource().ToBaseContainer();
```

### Obtain

### Read

```enforce
int value;
if (baseContainer.Get("m_iValue", value)) // the Get method must be provided a variable of the good type as second argument
Print("m_iValue has been properly read"); // e.g be sure to provide a ResourceName when required and not a string
else
Print("m_iValue cannot be read");
```

Iterating a BaseContainerList:

```enforce
BaseContainer baseContainer;
for (int i, count = baseContainerList.Count(); i < count; ++i)
{
	baseContainer = baseContainerList.Get(i); // BaseContainerList does not support foreach
}
```

#### Config

```enforce
// provided SCR_ConfigClass is decorated with [BaseContainerProps(configRoot: true)]
SCR_ConfigClass configInstance = SCR_ConfigHelperT<SCR_ConfigClass>.GetConfigObject(configResourceName);
```

#### IEntitySource

An IEntitySource can be obtained from [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) usage:

```enforce
IEntitySource entitySource;
entitySource = worldEditorAPI.CreateEntity(prefab, string.Empty, worldEditorAPI.GetCurrentEntityLayerId(), null, vector.Zero, vector.Zero);
entitySource = worldEditorAPI.EntityToSource(entity);
entitySource = worldEditorAPI.FindEntityByName("entityName");
entitySource = worldEditorAPI.GetEntityUnderCursor();
// etc
```

```enforce
array<IEntitySource> entitySources = {};
for (int i, count = worldEditorAPI.GetEditorEntityCount(); i < count; ++i)
{
	entitySources.Insert(worldEditorAPI.GetEditorEntity(i));
}
```

#### Localization

```enforce
LocalizationEditor locEditor = Workbench.GetModule(LocalizationEditor);
BaseContainer table = locEditor.GetTable();
if (!table)
return;
BaseContainerList items = table.GetObjectArray("Items");
```

#### Meta File

```enforce
ResourceManager resourceManager = Workbench.GetModule(ResourceManager);
MetaFile metaFile = resourceManager.GetMetaFile(absoluteFilePath); // MetaFile inherits from BaseContainer
```

### Update

Editing:

```enforce
if (baseContainer.Set("m_iValue", 42)) // m_iValue HAS to be defined in the class - random properties cannot be added
Print("m_iValue has been properly set");
else
Print("m_iValue cannot be set");
```

Saving:

```enforce
BaseContainerTools.SaveContainer(baseContainer, resourceName); // saving using a ResourceName
BaseContainerTools.SaveContainer(baseContainer, ResourceName.Empty, absoluteFilePath); // alternatively, for e.g a MetaFile
```

#### WorldEditorAPI

⚠

IEntitySource **must** be edited with World Editor API for the changes to stay in the saved world/Prefab - otherwise, only the BaseContainer loaded in memory is edited and no data is changed on save.

```enforce
worldEditorAPI.SetVariableValue(entitySource, null, "m_iValue", "42"); // value is set by string in WorldEditorAPI
```

For a value that is not at root level, the path must be defined:

```enforce
array<ref ContainerIdPathEntry> path = { new ContainerIdPathEntry("m_SubObject") };
worldEditorAPI.SetVariableValue(entitySource, path, "m_iValue", "42");
```

An IEntitySource component's value can be set the same way:

```enforce
array<ref ContainerIdPathEntry> path = { new ContainerIdPathEntry("Components", 0) }; // assuming this is the first component
worldEditorAPI.SetVariableValue(entitySource, path, "m_iValue", "42"); // the targeted source is always the parent entitySource
```
