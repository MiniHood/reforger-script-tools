# [Scripting: Conventions](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Conventions)

ⓘ

Format reminder:

* **camelCase** is writing by gluing words together and making their first letter capital, but for the first one, e.g namingLikeThis.
* **PascalCase** is writing in **camelCase** but with the first letter set uppercase, e.g NamingLikeThis.
* **snake casing** (naming\_like\_this) is **not** a convention used in Enfusion.  
  Only const values should be NAMED\_LIKE\_THIS.

## Tag

To prevent conflict with other scripts all classes, and global functions must be distinguished by a [Tag](/wiki/Scripting_Tags "Scripting Tags").

Bohemia Interactive scripts have been prefixed with SCR\_. A mod developer must choose their own tag and not use one already taken by Bohemia Interactive or another developer.

When [modding](/wiki/Arma_Reforger:Scripting_Modding "Arma Reforger:Scripting Modding") any class, all methods and variables within that modded class must also be distinguished by the same unique tag to prevent conflict.

## File/Class

* File should be called TAG\_MyObject.c
  + A component must end with "component": TAG\_ExampleComponent.c
  + An entity must end with "entity': TAG\_ExampleEntity.c
* Class should be called the same (without file extension):
  + class TAG\_MyObject
  + class TAG\_ExampleComponent
  + class TAG\_ExampleEntity
* Enum:
  + name must be prefixed with a capital E, e.g TAG\_EMyEnum
  + values use all capital letters with words separated by underscores, e.g TAG\_EMyEnum.VALUE\_1
* The file should be located in the scripts\Game directory
* The use of plural is prohibited at the end of combined keywords: e.g TAG\_Notification**s**Component → TAG\_NotificationComponent

## Method

* **Functions**/**Methods** naming uses **PascalCase**, e.g:

  ENFORCECODEMARKER

  ```
  int ReturnNumber()
  ```
* Parameters are named with **camelCase**, e.g:

  ENFORCECODEMARKER

  ```
  int ReturnNumber(bool canBeNegative)
  ```

## Variable

Prefix cheat sheet

|  |
| --- |
| ENFORCECODEMARKER   ``` // root prefixes bool m_bValue; int m_iValue; float m_fValue; string m_sValue; SCR_EEnum m_eValue; vector m_vValue; array<x> m_aValue; map<x, y> m_mValue; // children use parent's prefix ResourceName m_sResourceName; LocalizedString m_sLocalisedString; Curve m_aCurve; // no prefix SCR_Class m_ClassInstance; typename m_ClassTypename; set<x> m_Set; Color m_Color; ``` |

ⓘ

See [Scripting: Values - Identifier](/wiki/Arma_Reforger:Scripting:_Values#Identifier "Arma Reforger:Scripting: Values") for more information.

* **Member** variables are prefixed with m\_, a one-letter type prefix for specific types (see [Value types](/wiki/Arma_Reforger:Scripting:_Values#Types "Arma Reforger:Scripting: Values")), and use **PascalCase**, e.g:

  ENFORCECODEMARKER

  ```
  IEntity m_Entity;
  int m_iHealth;
  bool m_bIsEnemy;
  ```
* **Global** variables are prefixed with g\_.

  ⚠

  Global variables are bad practice and must not be used outside of absolute necessity!
* **Static** variables are prefixed with s\_ (**constants** are not), eventually a one-letter type prefix, and use **PascalCase**, e.g

  ENFORCECODEMARKER

  ```
  protected static int s_iUnitsCount;
  ```
* **Local** variables and **arguments** (method parameters) use **camelCase**, e.g:

  ENFORCECODEMARKER

  ```
  void MethodA()
  {
  	int value = 42;
  	string name = "John";
  	float result = MethodB(value, name);
  }
  ```
* **Constant** values use **all capital letters** with words separated by **underscores** (uppercase snake casing), e.g:

  ENFORCECODEMARKER

  ```
  const int MAX_VALUE = 9999; // no 'm_' prefix nor 'i' type prefix
  static const int TOTAL = 10; // no 's_' prefix either
  ```

### Order

* First go **Attributes** (if any),
* then public, protected, private **member variables**
* then public, protected, private **static variables**
* then public, protected, private **constants**

```enforce
[Attribute()]
protected int m_iAttribute;
int m_iPublic;
protected int m_iProtected;
private int m_iPrivate;
static int s_iPublic;
protected static int s_iProtected;
private static int s_iPrivate;
static const int PUBLIC;
protected static const int PROTECTED;
private static const int PRIVATE;
```

## Script

### General

* Curly braces must always be on a new line - Enforce Script uses **[Allman style](https://en.wikipedia.org/wiki/Indentation_style#Allman_style)**
* Variables and functions should be **protected** whenever possible (respecting OOP black box principle) unless they are **intended** to be exposed
* **Getters**/**Setters**: variables should be made protected and accessed through getters and setters (entry methods getting/setting the value)

```enforce
class SCR_HumanComponent : ScriptComponent
{
	protected int m_iAge;
	void SetAge(int age)
	{
		m_iAge = age;
		PrintFormat("Age of instance %1 is now %2", this, m_iAge);
	}
	int GetAge()
	{
		return m_iAge;
	}
}
```

### Spacing

* **Tabs** are used for indentation - they are set to a size of **4 spaces** in [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor")
* A space is used before and after:
  + a binary [operator](/wiki/Arma_Reforger:Scripting:_Operators "Arma Reforger:Scripting: Operators")
  + a foreach colon
  + a class' inheritance colon
* A space is used after:
  + if, for, foreach, switch, while keywords
  + a for semicolon
* Spaces are used inside parentheses but not around their content

```enforce
class SCR_HumanComponent : ScriptComponent
{
	if (true)
	{
	}
	for (int i = 0; i < 10; i++)
	{
	}
	foreach (string item : stringArray)
	{
	}
	switch (value)
	{
		case 42:
		break;
	}
	while (true)
	{
		break; // in case of a hasty copy-paste :)
	}
}
```

### Method

* All methods must be separated using this sequence of characters: two slashes followed by 96 dashes (see [Example](#Example))

  ENFORCECODEMARKER

  ```
  //------------------------------------------------------------------------------------------------
  ```
* **Documentation** must be done with [Doxygen](/wiki/Doxygen "Doxygen") support in mind, using the //! comment syntax (see [Example](#Example))
* Methods should be **sorted** in the following order (top to bottom):
  + General methods
  + EOnFrame
  + EOnInit
  + Constructor
  + Destructor

```enforce
//! A scripted entity
class SCR_ScriptedEntity : GenericEntity
{
	//------------------------------------------------------------------------------------------------
	//! Get the normalized direction vector at position A pointing to B
	//! \param vectorA First position, direction origin
	//! \param vectorB Second position, direction goal
	//! \return The direction from A to B as a normalized vector
	protected vector GetNormalizedDirection(vector vectorA, vector vectorB)
	{
		vector dir = vectorB - vectorA;
		return dir.Normalized();
	}
	//------------------------------------------------------------------------------------------------
	//! Frame
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		vector direction = GetNormalizedDirection(owner.GetOrigin(), vector.Zero);
		Print("OnFrame was called! Direction: " + direction);
	}
	//------------------------------------------------------------------------------------------------
	//! Init
	override void EOnInit(IEntity owner)
	{
		Print("Init was called!");
	}
	//------------------------------------------------------------------------------------------------
	// constructor
	void SCR_ScriptedEntity(IEntitySource src, IEntity parent)
	{
		SetEventMask(EntityEvent.INIT | EntityEvent.FRAME | EntityEvent.CONTACT);
	}
	//------------------------------------------------------------------------------------------------
	// destructor
	void ~SCR_ScriptedEntity()
	{
		Print("Destructing SCR_ScriptedEntity");
	}
}
```

### Miscellaneous

* class instanciation with the new keyword **must** use parentheses:

  ENFORCECODEMARKER

  ```
  SCR_Class myClass = new SCR_Class(); // correct
  SCR_Class myClass = new SCR_Class; // wrong
  ```
* arrays can be initialised directly:

  ENFORCECODEMARKER

  ```
  array<string> myArray = {}; // correct
  array<string> myArray = new array<string>(); // tolerable
  array<string> myArray = new array<string>; // wrong
  ```

## Moddability

It is important to keep moddability in mind when scripting to ensure.
Modded classes work very similar to inherited ones and come with the same restrictions:

* **Constructor/Destructor**: a modded class has its own const/destructor and cannot modify the parent one
* **Private variables & methods**: a modded class cannot override parent's private members - use cprotected instead of cprivate
* **Static variables & methods**: same as above - do not use unless absolutely necessary
* **Global methods**: no classes to mod - do not use unless absolutely necessary.

## Example

```enforce
[EntityEditorProps("GameScripted/SomeFolder", "Description of this component", "255 0 0 255", true, true, "", "box", "-0.25 -0.25 -0.25", "0.25 0.25 0.25", "0 0 0 0")]
class SCR_SomeComponentClass
{
}
SCR_SomeComponentClass SCR_SomeComponentSource;
//! Flags used for an entity to define its currently active components.
enum SomeFlags
{
	MESH = 1,
	BODY = 2,
	HIERARCHY = 4,
	NET = 8,
}
//! A brief explanation of what this component does.
//! The explanation can be spread across multiple lines.
//! This should help with quickly understanding the script's purpose.
class SCR_SomeComponent : ScriptComponent
{
	//! Defines the maximum distance at which this object will be rendered in metres.
	[Attribute("30.0", UIWidgets.Slider, "The maximum distance at which this object will be rendered in metres.", "0 120 0.1")]
	protected float m_fRenderDistance;
	//! Maximum count of children that can be spawned at any time. If the limit is exceeded no more children are spawned.
	[Attribute("100", desc: "Maximum count of children that can be spawned at any time. If the limit is exceeded no more children are spawned.", "0 500")]
	protected int m_iMaximumChildCount;
	//! The offset of this object in metres.
	protected vector m_vPositionOffset = "0 0 0";
	//! A public variable
	float m_fSomethingPublic = 3.2;
	//! A public vector
	vector m_vOtherPublic = "1 2 3";
	//! Defines the minimum distance (in metres) for this object to render. If below this value, object will be culled.
	const float RENDER_DISTANCE_MINIMUM = 10;
	//! Defines the maximum distance (in metres) for this object to render. If above this value, object will be culled.
	const float RENDER_DISTANCE_MAXIMUM = 100;
	//------------------------------------------------------------------------------------------------
	//! Returns the render distance of this object (metres).
	float GetRenderDistance()
	{
		return m_fRenderDistance;
	}
	//------------------------------------------------------------------------------------------------
	//! Set the render distance of this object.
	//! \param renderDistance distance in metres. Is clamped between RENDER_DISTANCE_MINIMUM and RENDER_DISTANCE_MAXIMUM.
	void SetRenderDistance(float renderDistance)
	{
		m_fRenderDistance = Math.Clamp(renderDistance, RENDER_DISTANCE_MINIMUM, RENDER_DISTANCE_MAXIMUM);
	}
	//------------------------------------------------------------------------------------------------
	//! Prints hello to the debug console. Protected method, not exposed to outside classes but available to child classes.
	protected void SayHello()
	{
		string localString = "Hello!";
		Print(localString);
	}
	//------------------------------------------------------------------------------------------------
	//! Compare two integers, return the larger one. (Don't mind the silly documentation, mind the syntax)
	//! \param a first parameter to be compared with the second one
	//! \param b second parameter to be compared with the first one
	//! \return true if a is equal to b, false otherwise.
	bool AreEqual(int a, int b)
	{
		// can be simplified to "return a == b;"
		if (a == b)
		return true;
		else
		return false;
	}
	//------------------------------------------------------------------------------------------------
	override void EOnInit(IEntity owner)
	{
		Print("Initialized some component!");
	}
	//------------------------------------------------------------------------------------------------
	// constructor - remove if empty
	void SCR_SomeComponent(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		ent.SetEventMask(EntityEvent.INIT);
		// If offset is 0, no need to update
		if (m_vPositionOffset != "0 0 0")
		{
			// Get current transformation matrix, add the position offset and update transformation.
			vector mat[4];
			ent.GetTransform(mat);
			mat[3] = mat[3] + m_vPositionOffset;
			ent.SetTransform(mat);
		}
	}
	//------------------------------------------------------------------------------------------------
	// destructor - remove if left empty
	void ~SCR_SomeComponent()
	{
		// ...
	}
}
```
