# [Class Template Example](https://community.bistudio.com/wiki/Arma_Reforger:Class_Template_Example)

A **Class Template** is a class definition that allows managing various types (separately) with only one declaration.

A good example that may not appear as obvious is the c[array](enfusion://ScriptEditor/scripts/Core/proto/Types.c;154) class; as you may now realise, an array is simply a generic list of items where we declare the managed type on instanciation (like a set or a map), e.g:

```enforce
array<string> arrayOfStrings = {};
array<bool> arrayOfBools = {};
set<int> setOfInts = new set<int>();
map<string, vector> mapOfStringsAndVectors = new map<string, vector>();
```

| Pros | Cons |
| --- | --- |
| * Not repeating generic code * Not repeating generic code * Not repeating generic code | * [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") cannot detect an error in code unless the template is used somewhere * Some silent errors can occur (e.g cif (!value) check on a class instance is different from an int, etc) * Too generic code may not always fit the encountered scenarios - its usage is up to the user |

## Examples

Below is a raw example of how to use a template, here used for casting:

```enforce
class TAG_EntityCastHelper<Class T>
{
	static T Get(Managed something)
	{
		return T.Cast(something);
	}
	static bool IsValid(Managed something)
	{
		return Get(something) != null;
	}
	static void Output(T anInstance)
	{
		if (anInstance)
		Print(anInstance.ToString());
	}
}
```

```enforce
array<bool> anArray = {};
IEntity anEntity = EntityUtils.GetPlayer(); // provided anEntity is not null like in e.g Script Editor
TAG_EntityCastHelper<IEntity> entityCastHelper = new TAG_EntityCastHelper<IEntity>();
Print("IEntity = " + entityCastHelper.IsValid(anEntity)); // IEntity = true
Print("Managed = " + entityCastHelper.IsValid(anArray)); // Managed = false
TAG_EntityCastHelper<Managed> managedCastHelper = new TAG_EntityCastHelper<Managed>();
Print("IEntity = " + managedCastHelper.IsValid(anEntity)); // IEntity = true
Print("Managed = " + managedCastHelper.IsValid(anArray)); // Managed = true
TAG_EntityCastHelper<array<bool>> boolArrayCastHelper = new TAG_EntityCastHelper<array<bool>>();
Print("IEntity = " + boolArrayCastHelper.IsValid(anEntity)); // IEntity = false
Print("Managed = " + boolArrayCastHelper.IsValid(anArray)); // Managed = true
TAG_EntityCastHelper<array<string>> stringArrayCastHelper = new TAG_EntityCastHelper<array<string>>();
Print("IEntity = " + stringArrayCastHelper.IsValid(anEntity)); // IEntity = false
Print("Managed = " + stringArrayCastHelper.IsValid(anArray)); // Managed = false - not a string array
```

⚠

Methods used on cT **must** be generic enough; for example, cT.Cast() is possible on many classes/types but e.g **not** on an c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) type - trying to use this type here raises an error.

It is also possible to use multiple types at once:

```enforce
class TAG_EntityCastHelper<Class T, Class U, Class V>
{
	protected ref array<T> m_aT = {};
	protected ref array<U> m_aU = {};
	protected ref array<V> m_aV = {};
	static void Insert(T item1, U item2, V item3)
	{
		m_aT.Insert(item1);
		m_aU.Insert(item2);
		m_aV.Insert(item3);
	}
	static bool GetByItem1(T item1, out U item2, out V item3)
	{
		int index = m_aT.Find(item1);
		if (index < 0)
		return false;

		item2 = m_aU[index];
		item3 = m_aV[index];
		return true;
	}
}
```
