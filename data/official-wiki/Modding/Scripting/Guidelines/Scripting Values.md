# [Scripting: Values](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Values)

A **value** describes a *data holder*; a value is either a **variable** (that can be changed) or a **constant** (that cannot be changed).

Values are declared in Enfusion with a type, in format type identifier = value (the = value part being optional).
Enforce Script uses strong types, which means a value's type cannot be changed along its lifetime.

## Identifier

An **identifier** is the actual name of a value; it *identifies* the value. The naming rules are:

* an identifier can be composed of ASCII letters, numbers and underscores
* an identifier **must** start with an underscore or a letter (**cannot** start with a number)
* an identifier is **case-sensitive**
* an identifier **cannot be identical to a keyword**

The regular expression to be respected is: ^[a-zA-Z\_][a-zA-Z0-9\_]\*$

It is *recommended* to write variable identifiers in camelCase, (e.g aNewVariable) and constants' in UPPER\_CASE (e.g A\_NEW\_CONSTANT).
See [Scripting: Conventions - Variable](/wiki/Arma_Reforger:Scripting:_Conventions#Variable "Arma Reforger:Scripting: Conventions") for Arma Reforger naming conventions.

```enforce
int myNumber = 10;
int _myNumber = 10;
int myNUMBER = 10; // different from myNumber
int 1number = 1; // wrong - starts with a number
int my#variable = 1; // wrong - contains a character that is not letter/number/underscore
int auto = 1; // wrong - the "auto" keyword exists and cannot be replaced
```

## Value Declaration

Declaring a value is done by using a type and an identifier:

```enforce
int myNumber; // variable myNumber is declared and auto-initialised to 0

myNumber = 10; // myNumber is now 10
```

It can also be directly created with a value:

```enforce
int myNumber = 10;
```

### const

A value can be made **constant**, meaning that it will remain the same along its lifetime.

```enforce
string value1 = "value"; // value1's value is "value"
const string CONST_VALUE = "value";

value1 = "new value"; // value1 is now "new value"
CONST_VALUE = "new value"; // error: cannot modify a const value
```

An object (array, set, map, other class instance) can be const yet still have its values changed - const here only keeps the reference and does not "freeze" the object in place.

```enforce
const ref array<string> STRINGS = {};
STRINGS.Insert("Hello there"); // OK: the array remains the same, its content is changed
STRINGS = null; // error: cannot modify a const value
```

## Passing a Value

### By Content

```enforce
// integers (whole numbers) are passed by content
int myVar1 = 33;
int myVar2 = myVar1; // myVar2 is 33 - it copied myValue1's content
myVar2 = 42; // myVar2 is now 42, myVar1 is still 33 - they are two different values
```

### By Reference

```enforce
// arrays (list of values) are passed by reference
array<int> myArray1 = { 0, 1, 2, 3 };
array<int> myArray2 = myArray1; // myArray2 targets myArray1 object - value is not copied but referenced
myArray2[0] = 99; // myArray1 is now { 99, 1, 2, 3 };
```

## Types

Prefix cheat sheet

|  |
| --- |
| ENFORCECODEMARKER   ``` // root prefixes bool m_bValue; int m_iValue; float m_fValue; string m_sValue; SCR_EEnum m_eValue; vector m_vValue; array<x> m_aValue; map<x, y> m_mValue; // children use parent's prefix ResourceName m_sResourceName; LocalizedString m_sLocalisedString; Curve m_aCurve; // no prefix SCR_Class m_ClassInstance; typename m_ClassTypename; set<x> m_Set; Color m_Color; ``` |

Values can be of various types, listed below.

Notions:

* Passed by: content or reference - see above
* Naming prefix: used in **Object**'s value convention (See [conventions](/wiki/Arma_Reforger:Scripting:_Conventions#Variable "Arma Reforger:Scripting: Conventions")): m\_ or s\_ + **prefix** + VariableName (e.g: m\_**b**PlayerIsAlive = true)

  ⓘ

  No prefix is used for constants in conventions.
* Default value: the default value given when a value is declared without content (e.g: int value - the variable here is 0, integer's default value)

| Incorrect | Correct |
| --- | --- |
| ENFORCECODEMARKER   ``` bool variable = true; if (variable) { 	variable = "it works!"; // not - variable is and remains a bool 	Print(variable); } ``` | ENFORCECODEMARKER   ``` bool variable = true; if (variable) { 	string display = "it works!"; // correct 	Print(display); } ``` |

### Primitive Types

| Type name | C++ equivalent | Range | Default value | Size (Bytes) |
| --- | --- | --- | --- | --- |
| int | int32 | -2,147,483,648 through +2,147,483,647 | 0 | 4 |
| float | float | ±1.18E-38 through ±3.402823E+38 | 0.0 | 4 |
| bool | bool | *true* or *false* | false | 4 |
| string | char\* | - | "" (empty string) | 8 (pointer) + (length × 1 byte) |
| vector | float[3] | like float | { 0.0, 0.0, 0.0 } | 8 (pointer) + (3 × float) 12\* |
| void | void | - | - | - |
| class | Instance\* | pointer to any script object | null | 8 |
| typename | VarType\* | pointer to type structure | null | 8 |

\*the asterisk in C++ means a pointer.

### Bool

Passed by: **value**

Naming prefix: **b**

Default value: **false**

A **bool** is a value that can be either **true** or **false** . For example, c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) result = 10 > 5; result can either be true or false (here is true, as 10 is greater than 5).

Example:

```enforce
bool myValue; // myValue is false
myValue = 10 > 0; // myValue is true
```

### Integer

Passed by: **value**

Naming prefix: **i**

Default value: **0**

Range: -2,147,483,648 to 2,147,483,647

Description: An **integer** (or int) is a whole number, meaning a value without decimals. It can be positive or negative.

Example:

```enforce
int myValue; // myValue is 0
myValue = 20; // myValue is 20
myValue /= 3; // myValue is 6: 20 / 3 = 6.666… which gets floored to 6 (and not rounded)
```

### Float

Passed by: **value**

Naming prefix: **f**

Default value: **0.0**

Range: 1.18 E-38 to 3.40 E+38

A **float**, or its full name **floating-point number** is a number that can have decimals.

**Precision** is a matter at hand when working with floats; do not expect exact calculations: 0.1 + 0.1 + 0.1 != 0.3 - rounding would be needed to have the result one can expect.

ⓘ

The further away from 0 the float value is, the less precision it has.  
For "almost equal" comparison, see float.AlmostEqual; the default accepted difference (epsilon) is 0.0001 (1/10000) and can be set for each operation as an optional third argument.

Example:

```enforce
float myValue; // myValue is 0 (or 0.0)
myValue = 1; // myValue is 1 (or 1.0)
myValue /= 3; // myValue is 0.333333…
float originalValue = 1;
float divisionResult = originalValue/3333;
float result = divisionResult * 3333;
originalValue == result; // returns false due to floating point precision
float.AlmostEqual(originalValue, result); // returns true
```

### String

Passed by: **value**

Naming prefix: **s**

Default value: **""** (empty string)

Maximum length: 2,147,483,647 characters

Description: a **string** is a sequence of characters; it supports the following escape characters: \n (line return) \r (caret return) \t (horizontal tabulation) \\ (antislash) \" (double quote).

ⓘ

* A string **cannot** be null
* String comparison is **case-sensitive**; e.g c"abc" != "ABC"

Example:

```enforce
string username; // username is "" (empty string)
username = "Player 1"; // username is "Player 1";
```

### Enum

Passed by: **value**

Naming prefix: **e**

Default value: 0 (which may **not** exist in the enum)

An **enum** is a value that offers a choice between defined options.

"Behind the curtains" it is also an integer that can be assigned to an int variable.

Examples:

```enforce
enum EHealthState
{
	ALIVE,
	INJURED,
	UNCONSCIOUS,
	DEAD,
}
EHealthState healthState; // healthState is ALIVE (0)
healthState = EHealthState.UNCONSCIOUS; // healthState is UNCONSCIOUS (4)
int myValue = EHealthState.ALIVE; // valid
```

By default, the first value is equal to 0 and the next values are incremented by 1.

```enforce
enum EHealthState
{
	ALIVE = 42,
	INJURED, // equals 43
	UNCONSCIOUS = 50,
	DEAD, // equals 51
}
EHealthState healthState; // healthState is 0 (no corresponding enum)
healthState = EHealthState.INJURED; // healthState is 43 (INJURED)
int myValue = EHealthState.ALIVE; // 42
```

```enforce
// an enum value can also be defined through bit shifting for e.g flag usage
enum ELifeStatus
{
	HEALTHY = 1 << 0, // 1
	HAS_HOUSE = 1 << 1, // 2
	HAS_FOOD = 1 << 2, // 4
}
```

### Vector

Passed by: **value**

Naming prefix: **v**

Default value: **{ 0, 0, 0 }**

A **vector** is a type that holds **three** float values. Two vectors can be compared with == for **value** comparison.

Example:

```enforce
// there are three ways to create a vector
vector myVector0; // myVector0 is { 0, 0, 0 }
vector myVector1 = { 0, 1.5, 2 };
vector myVector2 = "0 1.5 2";

myVector1 == myVector2; // true
// editing one of the vector values
vector myVector3 = myVector1; // copies the value
myVector1 == myVector3; // true
myVector1[1] = 42; // myVector1 is now { 0, 42, 2 }
myVector1 == myVector3; // false
// editing the whole vector
myVector3 = { 0, 42, 2 }; // syntax "0 42 2" works too
myVector1 == myVector3; // true
// operators work
myVector1 += "0 1 0";
myVector1 -= "0 1 0";
myVector1 *= 3.33;
myVector1 /= 3.33;
// cross product and dot product in one operation
vector crossProduct = vector1 * vector2;
float dotProduct = vector1 * vector2;
auto product = vector1 * vector2; // product is by default a vector
```

### Array

Passed by: **reference** (dynamic array) or **value** (static array)

Naming prefix: **a** for both

Default value: null (**not** an empty array) or static array filled with default item type value (e.g 0 for int, null for object, etc)

Maximum size: int.MAX

An **array** is a list of values. In Enforce Script, an array can only hold one type of data (defined in its declaration).

There are two types of array in Enfusion:

* dynamic array: "list" that will extend the more items are added
* static array: fixed-size list of items - items can be changed, but the array size cannot

Example:

```enforce
array<string> dynamicArray = {};
string staticArray[2] = { "Hello", "" };

dynamicArray.Insert("Hello"); // dynamicArray = { "Hello" };
dynamicArray.Insert(""); // dynamicArray = { "Hello", "" };

dynamicArray[1] = "there"; // dynamicArray = { "Hello", "there" };
staticArray[1] = "there"; // staticArray = { "Hello", "there" };
// dynamic/static arrays cannot be directly compared for values; == would compare pointers (a.k.a is it the same array, not the same values)
dynamicArray == staticArray; // false
dynamicArray[0] == staticArray[0]; // true
dynamicArray[1] == staticArray[1]; // true
```

### Set

Passed by: **reference**

Naming prefix: none

Default value: null (**not** an empty set)

A **set** is a list that ensures uniqueness of values. Due to this uniqueness check, item insertion is slower than for an Array; however, checking if an item is contained is faster.

Example:

```enforce
set<string> setInstance = new set<string>();

setInstance.Insert("Hello there");
setInstance.Insert("General Kenobi");
setInstance.Insert("Hello there"); // setInstance will still only contain "Hello there" and "General Kenobi"

setInstance.Get(0); // the values order is never guaranteed as it can change on value insertion!
```

### Map

Passed by: **reference**

Naming prefix: **m**

Default value: null (**not** an empty map)

A **map** is a list of key-value pairs, also known as a dictionary. Two keys cannot be identical as they are used to index the values.

A float **cannot** be a map key.

Example:

```enforce
map<int, string> mapInstance = new map<int, string>();

mapInstance.Set(5712, "Hello there");
mapInstance.Set(5716, "General Kenobi");

mapInstance.Get(5712); // returns "Hello there"
mapInstance.Get(5715); // /!\ returns "" (default string)
mapInstance.Get(5716); // returns "General Kenobi"

mapInstance.GetKeyByValue("Hello there"); // returns 5712
mapInstance.GetKeyByValue("test"); // /!\ returns 0 (default int)

mapInstance.Contains(5716); // true
mapInstance.ReplaceKey(5716, 5715); // replaces "General Kenobi" key
mapInstance.Contains(5716); // false

mapInstance.Contains(5715); // true
mapInstance.Remove(5715); // removes "General Kenobi" from the map
mapInstance.Contains(5715); // false
```

### Class

Passed by: **reference**

Naming prefix: none

Default value: **null**

A **class** is what defines an object's structure - said from the other end, an object is an **instance** of a class.

ⓘ

For more information on classes and Object Oriented Programming, see [Object Oriented Programming Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") and [Object Oriented Programming Advanced Usage](/wiki/Arma_Reforger:Object_Oriented_Programming_Advanced_Usage "Arma Reforger:Object Oriented Programming Advanced Usage").

Example:

```enforce
class ObjectClass
{
	protected int m_iHealth = 100;
	int GetHealth()
	{
		return m_iHealth;
	}
	bool SetHealth(int health)
	{
		if (health < 0 || health > 100)
		return false;
		m_iHealth = health;
		return true;
	}
}
void HealthMethod()
{
	ObjectClass myObjectInstance; // myObjectInstance is null
	myObjectInstance = new ObjectClass();
	int objectHealth = myObjectInstance.GetHealth();
	Print(objectHealth);
}
```

### Typename

Passed by: **reference**

Naming prefix: none

Default value: **typename.Empty**

A **typename** is a "class information" type allowing to obtain values and other instance details through [reflection](https://en.wikipedia.org/wiki/Reflective_programming).

Example:

```enforce
class ObjectClass
{
	int m_Health = 100;
	void MyMethod()
	{
		typename myTypename = ObjectClass;
		string myClassName = myTypename.ToString(); // returns "ObjectClass" string
		myTypename = myClassName.ToType(); // returns ObjectClass typename
		int localHealth;
		myTypename.GetVariableValue(this, 0, localHealth); // localHealth now has m_Health's value
		//		typename cannotAssignNull = null;					// throws an error - cannot assign null
		typename canAssignEmpty = typename.Empty; // good
		string nonExistingClass = "ThisClassDoesNotExist";
		typename wrongTypename = nonExistingClass.ToType(); // returns typename.Empty
		typename uninitialisedTypename; // is typename.Empty
	}
}
```

## Scope

A value has a lifetime, whether it is the game instance, mission or script duration; but it also has a scope that defines its existence and its accessibility.

```enforce
void SetDammage(int dammage)
{
	int newHealth; // newHealth variable is declared
	newHealth = m_iHealth - dammage;
	if (newHealth < 0)
	{
		newHealth = 0;
	}
	else if (newHealth > 100)
	{
		int difference = newHealth - 100; // difference variable is declared
		Print("health overflow: " + difference);
		newHealth = 100; // difference variable is destroyed after leaving the "else" scope and doesn't exist outside of it
	}
	// difference variable does not exist in this scope and cannot be used
	m_iHealth = newHealth; // newHealth variable's last usage - the variable still exists
	Print("Health has been set"); // newHealth is destroyed after this line (on closing the "SetDammage" scope)
}
```

| Incorrect | Correct | Shorter but more expensive |
| --- | --- | --- |
| ENFORCECODEMARKER   ``` if (soldiersHealth > 75) { 	string message = "I'm doing well"; } else { 	// no conflict with the previous variable as scopes are different 	string message = "I don't feel so good"; } Print(message); // error: message variable is undefined here ``` | ENFORCECODEMARKER   ``` string message; if (soldiersHealth > 75) { 	message = "I'm doing well"; } else { 	message = "I don't feel so good"; } Print(message); // OK ``` | ENFORCECODEMARKER   ``` string message = "I don't feel so good"; if (soldiersHealth > 75) { 	message = "I'm doing well"; } Print(message); // OK ``` |

## Casting

A value can sometimes be returned as one of its inherited classes or interfaced type - it can be **casted** ("forced" into a type) if the underlying type is known.

⚠

A wrong cast will return a **null** value - no exception will be thrown.

```enforce
class Soldier_Base
{
	int m_iScope = 0;
}
class B_Soldier_F : Soldier_Base
{
	int m_iScope = 2;
	void SayHello()
	{
		Print("Hello there");
	}
}
Soldier_Base aSoldier = new B_Soldier_F(); // valid
aSoldier.SayHello(); // invalid - Soldier_Base does not have the SayHello method
B_Soldier_F mySoldierF = B_Soldier_F.Cast(aSoldier);
mySoldierF.SayHello(); // valid
```
