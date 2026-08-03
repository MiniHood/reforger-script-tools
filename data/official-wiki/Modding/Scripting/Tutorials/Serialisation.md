# [Serialisation](https://community.bistudio.com/wiki/Arma_Reforger:Serialisation)

This page explains serialisation using [SCR\_JsonSaveContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;30)/[SCR\_JsonLoadContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;40) and [SCR\_BinSaveContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;24)/[SCR\_BinLoadContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;27).

## JSON

ⓘ

* Passing an empty string as the name parameter into c[BaseSerializationSaveContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;7).WriteValue() or c[BaseSerializationLoadContext](enfusion://ScriptEditor/scripts/Game/Plugins/Serialization/BackwardsCompatiblity.c;4).ReadValue() allows for a complex top-level struct to be written/read.
* See also [JsonApiStruct Usage](/wiki/Arma_Reforger:JsonApiStruct_Usage "Arma Reforger:JsonApiStruct Usage").

### Serialisation

```enforce
SCR_JsonSaveContext saveContext = new SCR_JsonSaveContext();
string stringValue = "data";
int integerValue = 123;

saveContext.WriteValue("key1", stringValue);
saveContext.WriteValue("key2", integerValue);
// process saved data (export, send, save...), in this case data are exported as json string
string dataString = saveContext.ExportToString();
```

### Deserialisation

```enforce
SCR_JsonLoadContext loadContext = new SCR_JsonLoadContext();
loadContext.ImportFromString(dataString);
string stringValue;
int integerValue;
// order does not matter for JSON as it uses key names
loadContext.ReadValue("key2", integerValue);
loadContext.ReadValue("key1", stringValue);
```

## Binary

### Serialisation

```enforce
SCR_BinSaveContext saveContext = new SCR_BinSaveContext();
string stringValue = "data";
int integerValue = 123;

saveContext.WriteValue("key1", stringValue);
saveContext.WriteValue("key2", integerValue);
// process saved data (export, send, save...), in this case data are saved to "file.bin"
saveContext.SaveToFile("file.bin");
```

### Deserialisation

```enforce
SCR_BinLoadContext loadContext = new SCR_BinLoadContext();
loadContext.LoadFromFile("file.bin");
string stringValue;
int integerValue;
// order matters for Binary serialisation, as Binary ignores names
loadContext.ReadValue("key1", stringValue);
loadContext.ReadValue("key2", integerValue);
```

## Object Serialisation

### Simple

The following class set to serialise will serialise all its properties.

⚠

Data structures that shall be serialised are not allowed to have parameters in their constructor, otherwise they can not be read back.

```enforce
class MyClass : Managed
{
	protected int m_iVariable = 42;
	protected string m_sVariable;
	protected float m_fVariable = 33.3;
}
```

#### NonSerialized

Adding the NonSerialized() decorator to a field will make the serialisation ignore it.

```enforce
class MyClass : Managed
{
	protected int m_iVariable = 42;
	protected string m_sVariable = "Hello there";
	[NonSerialized()]
	protected float m_fVariable = 33.3;
}
```

### Advanced

The following methods allow to define a custom serialisation per class. This is useful to avoid saving lengthy yet useless information for loading as well as load values in a certain order.

ⓘ

The [NonSerialized](#NonSerialized) decorator is only useful with the simple object serialisation - [SerializationSave](#SerializationSave)/[SerializationLoad](#SerializationLoad) will ignore it.

#### SerializationSave

If an object has the SerializationSave method defined, the SaveContext will use it and not process object's properties automatically at all.

```enforce
class MyClass : Managed
{
	protected int m_iVariable = 42;
	protected string m_sVariable = "Hello there";
	protected float m_fVariable = 33.3;
	bool SerializationSave(BaseSerializationSaveContext context)
	{
		if (!context.IsValid())
		return false;

		context.WriteValue("theString", m_sVariable);
		context.WriteValue("integer", m_iVariable);
		context.WriteValue("floatingpoint", m_fVariable);
		return true;
	}
}
```

#### SerializationLoad

If an object has the SerializationLoad method defined, the SaveContext will use it and not process object's properties automatically at all.

```enforce
class MyClass : Managed
{
	protected int m_iVariable = 42;
	protected string m_sVariable = "Hello there";
	protected float m_fVariable = 33.3;
	bool SerializationLoad(BaseSerializationLoadContext context)
	{
		if (!context.IsValid())
		return false;

		context.ReadValue("theString", m_sVariable);
		context.ReadValue("integer", m_iVariable);
		context.ReadValue("floatingpoint", m_fVariable);
		return true;
	}
}
```
