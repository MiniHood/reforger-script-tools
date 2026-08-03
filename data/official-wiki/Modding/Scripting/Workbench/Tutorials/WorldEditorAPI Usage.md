# [WorldEditorAPI Usage](https://community.bistudio.com/wiki/Arma_Reforger:WorldEditorAPI_Usage)

**World Editor API** allows for plugin/tool world operations like obtaining terrain resolution, but mostly for Prefab operations such as editing and saving a Prefab (called **Entity Template** in the API for historical reason – hence the .et extension) using [BaseContainer](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainer.c;12) references.

ⓘ

See the [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) class for in-code information.

## General Usage

Game classes such as WorldEditorAPI, ScriptEditor etc must **not** be a strong ref script-side; on scripts reload, these references are not nulled and could point to anything.

| Do | Don't |
| --- | --- |
| Get WorldEditorAPI as a **method** variable where needed:  1. ENFORCECODEMARKER      ```    WorldEditor worldEditor = Workbench.GetModule(WorldEditor);    if (!worldEditor)    return;    WorldEditorAPI worldEditorAPI = worldEditor.GetApi();    ```     or one-lined (no nullcheck) ENFORCECODEMARKER      ```    WorldEditorAPI worldEditorAPI = ((WorldEditor)Workbench.GetModule(WorldEditor)).GetApi();    ``` 2. ENFORCECODEMARKER      ```    WorldEditorAPI worldEditorAPI = SCR_WorldEditorToolHelper.GetWorldEditorAPI();    ``` 3. ENFORCECODEMARKER      ```    WorldEditorAPI worldEditorAPI = _WB_GetEditorAPI(); // in GenericEntity children    WorldEditorAPI worldEditorAPIFromEntity = anEntity._WB_GetEditorAPI(); // it is public    ``` 4. ENFORCECODEMARKER      ```    m_API; // in WorldEditorTool children    ``` | Store WorldEditorAPI as a **class** pointer:   ENFORCECODEMARKER   ``` class TAG_MyClass { 	WorldEditorAPI m_WorldEditorAPI; // wrong } ``` |

## Prefab Actions

Prefab actions allow editing Prefabs automatically, with or without user input.

⚠

Editing a [BaseContainer](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainer.c;12) using .Set\* methods does **not** modify the Prefab itself; it only modifies the current instance in memory.  

Proper Prefab editing **always** happens in [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) methods, such as:

```enforce
WorldEditorAPI.SetVariableValue()
WorldEditorAPI.ClearVariableValue()
WorldEditorAPI.CreateObjectVariableMember()
WorldEditorAPI.CreateObjectArrayVariableMember()
// etc
```

### Get Prefab

```enforce
//! Load ResourceName into a Resource
//! \param[in] resourceName the resourceName to load
//! \param[out] resource it is important to keep a strong reference to the loaded resource,
//!				otherwise its BaseContainer/IEntitySource will be dropped by memory management
//! \return the provided resourceName's IEntitySource
protected IEntitySource GetPrefabFromResourceName(ResourceName resourceName, out Resource resource)
{
	resource = Resource.Load(resourceName);
	if (!resource.IsValid())
	{
		resource = null;
		return null;
	}
	return resource.GetResource().ToEntitySource();
}
```

### Set Value

```enforce
[Attribute()]
```

values are set as string through WorldEditorAPI in .et files.
For example, a true bool value will be represented as 1 in a .et file, therefore "1" must be provided (see bool.ToString(true) for more information).

⚠

Prefab actions **must** be wrapped between **EntityAction** begin/end methods below – the Workbench will remind you of that anyway with a VME.

```enforce
worldEditorAPI.BeginEntityAction();
// actions here
worldEditorAPI.EndEntityAction();
```

#### Direct Value

The SetVariableValue method is used:

```enforce
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_bVariable", bValue.ToString(true)); // .ToString(true) to write it as "1"
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_fVariable", fValue.ToString());
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_iVariable", iValue.ToString());
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_sVariable", sValue); // no .ToString() for string
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_vVariable", vValue.ToString(false)); // .ToString(false) to write it as "0 1 2" and not "<0, 1, 2>"
```

#### Native Type Array

The SetVariableValue method is used, setting values with separating commas:

```enforce
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_aBoolArray", "0,1,0,0,1,1,0,0,0,1,1,0,1,1,1,1,0,1,1,1,0,1,0,1");
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_aFloatArray", "1.1,2,3.14159");
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_aIntArray", "1,2,3,4,5,6,-10");
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_aStringArray", "value1,value2,value3"); // note: commas cannot be escaped, making string with commas NOT supported
worldEditorAPI.SetVariableValue(myEntitySource, null, "m_aVectorArray", "0 1 0,0 0 1,1 0 0");
```

#### Object Creation

Two cases are identified: the object can either be a direct object property or an array item.

In the first case (direct object property), the CreateObjectVariableMember method must be used:

```enforce
// assuming an object is myEntitySource.m_SubObject
// public, protected or private, WorldEditorAPI can write any Attribute
worldEditorAPI.CreateObjectVariableMember(myEntitySource, null, "m_SubObject", "SubObjectClassName"); // returns true if type is valid and the object was created
```

In the second case (array element), the CreateObjectArrayVariableMember method is used:

```enforce
// assuming an object is an item of the myEntitySource.m_aSubObjects array
worldEditorAPI.CreateObjectArrayVariableMember(myEntitySource, null, "m_aSubObjects", "SubObjectClassName", 0); // returns true if type and index are valid and the object was created
// but... how to get the existing index?
// we can use the BaseContainer.Get method on the entitySource
// using BaseContainer methods is fine as long as it is for reading information, not set them
BaseContainerList subObjectsArray = myEntitySource.GetObjectArray("m_aSubObjects");
int nextIndex = subObjectsArray.Count();
```

#### Object Value

To set an object's value, the path parameter, an array of [ContainerIdPathEntry](enfusion://ScriptEditor/scripts/Core/worldEditor.c;25) must be used:

```enforce
// let's reuse the myEntitySource.m_SubObject situation
// m_SubObject has a "m_sMessage" property we want to set
// m_SubObject cannot be set as SetVariableValue's first argument, as it is not the root container (myEntitySource being its parent)
worldEditorAPI.SetVariableValue(myEntitySource, { new ContainerIdPathEntry("m_SubObject") }, "m_sMessage", "Hello there");
```

ⓘ

For performance reason, the path can be cached in order to create array and objects for each property.  
Here for the sake of examples and teaching, arrays are created on the fly.

An array item is accessed using the ContainerIdPathEntry's *index* parameter, to determine which item is targeted

```enforce
// let's reuse the myEntitySource.m_aSubObjects array situation
worldEditorAPI.SetVariableValue(myEntitySource, { new ContainerIdPathEntry("m_aSubObjects", 3) }, "m_sMessage", "Hello there"); // changes the fourth array item
```

A **component** is accessed the same way - using the "components" path:

```enforce
worldEditorAPI.SetVariableValue(myEntitySource, { new ContainerIdPathEntry("components", 0) }, "m_sMessage", "Hello there"); // if the first component must be edited
```

### Delete Value

```enforce
worldEditorAPI.ClearVariableValue(myEntitySource, null, "m_sName"); // erases "m_sName" from this Prefab's definition (not from parents or children)
```

An array **cannot** be deleted this way. All its items should be removed through RemoveObjectArrayVariableMember usage instead:

```enforce
BaseContainerList subObjectsArray = myEntitySource.GetObjectArray("m_aSubObjects");
for (int i = subObjectsArray.Count() - 1; i >= 0; --i)
{
	worldEditorAPI.RemoveObjectArrayVariableMember(myEntitySource, null, "m_aSubObjects", i);
}
```

### Save Changes

Simply use the following:

```enforce
worldEditorAPI.SaveEntityTemplate(myEntitySource);
```

## Helpers

See also:

* **[SCR\_WorldEditorToolHelper](enfusion://ScriptEditor/scripts/WorkbenchGame/WorldEditor/Helpers/SCR_WorldEditorToolHelper.c;2)** - a helper for WorldEditor/WorldEditorAPI usage
* [SCR\_PrefabHelper](enfusion://ScriptEditor/scripts/WorkbenchGame/WorldEditor/Helpers/SCR_PrefabHelper.c;3) - a helper for various tools's Prefab management
