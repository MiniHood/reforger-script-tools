# [Prefab Data](https://community.bistudio.com/wiki/Arma_Reforger:Prefab_Data)

A **Prefab Data** is an object storing variables which are shared among all of the instances of the given prefab.

ⓘ

This concept works for both entities and components, where we then talk about **Component Data** and use the cGetComponentData() method instead of the cGetPrefabData() one.

* Example 1: set a value that works for all instances of this Prefab (e.g max health, fuel capacity, etc.)
* Example 2: precalculate a value to not set it manually. The calculation will happen within a script callback that instanciates the variable when the first Prefab instance is created and store the value in the Prefab Data. e.g kinetic energy in a given velocity: Parameter velocity is 40 km/h, another parameter is object's mass, 1050 kg. These two variables allow to precalculate the kinetic energy threshold above which damage will be dealt. This result is the same for all the Prefab instances.

## Requirements

Before moving instance variables to prefab data, please consider the following:

* How many instances of the object (entity/component) will there be in the map and will the saved memory be worth the increased performance consumption?
  + Are there going to be 10 instances of this object? If so, this is probably not worth it, unless it has a 100 variables which can be shared.
  + Are there going to be 100+ instances of this object with multiple variables which can be shared? Totally worth it to use prefab data.
* How often do I access these variables, do I need these values every frame? If so, maybe then the performance consumption of getting and casting prefab data is not worth it.
  + I need these variables only for occasional calculations happening on, for example, collisions, user clicking a button or on periodic check. No worries here, for the performance difference is not noticeable
  + I need these variables each frame on each of my 30 instances. In such case consider storing the prefab data as a member variable (8 bytes for pointer), but only if more memory than that is saved by moving other member variables there - moving 3 and more (non-bool) variables should be enough to compensate for it.

## Code Example

Making a prefab data stored variable from a member variable is really easy.

All that is needed is to put the variable in SCR\_MyEntity**Class** insted of SCR\_MyEntity.

\***Class** classes are required for all Entities and Components, so no more preparation is required.

| Before | After |
| --- | --- |
| ENFORCECODEMARKER   ``` // prefab data class class SCR_AIGroupClass : ChimeraAIGroupClass { } // regular functional class class SCR_AIGroup : ChimeraAIGroup { 	[Attribute("42")] 	protected int m_iValue; 	int GetValue() 	{ 		return m_iValue; 	} } ``` | ENFORCECODEMARKER   ``` // prefab data class class SCR_AIGroupClass : ChimeraAIGroupClass { 	[Attribute("42")] 	int m_iValue; } // regular functional class class SCR_AIGroup : ChimeraAIGroup { 	int GetValue() 	{ 		SCR_AIGroupClass prefabData = SCR_AIGroupClass.Cast(GetPrefabData()); 		if (!prefabData) 		return -1; // assuming -1 is an invalid value 		return prefabData.m_iValue; 	} } ``` |
