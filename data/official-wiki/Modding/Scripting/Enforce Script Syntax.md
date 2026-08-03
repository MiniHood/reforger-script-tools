# [Enforce Script Syntax](https://community.bistudio.com/wiki/Arma_Reforger:Enforce_Script_Syntax)

**Enforce Script** is the language that is used by the [Enfusion](/wiki/Enfusion "Enfusion") engine first introduced in [DayZ](/wiki/Category:DayZ "Category:DayZ") Standalone.
It is an Object-Oriented scripting language that works with objects and classes and is similar to the [C#](https://en.wikipedia.org/wiki/C_Sharp_(programming_language)) programming language.

[![DayZ](/wikidata/images/thumb/a/ac/dayz_logo_white.png/48px-dayz_logo_white.png)](/wiki/Category:DayZ "DayZ")

For DayZ, see [DayZ:Enforce Script Syntax](/wiki/DayZ:Enforce_Script_Syntax "DayZ:Enforce Script Syntax").

## Data Types

ⓘ

See [Scripting: Values - Types](/wiki/Arma_Reforger:Scripting:_Values#Types "Arma Reforger:Scripting: Values").

There are many types of data, the most common being:

| Type | Description | Example | Wikipedia |
| --- | --- | --- | --- |
| Native types | | | |
| ENFORCECODEMARKER   ``` bool ``` | a bool, a.k.a ctrue or cfalse (and nothing else!) | ENFORCECODEMARKER   ``` bool value = true; ``` | see [Bool](https://en.wikipedia.org/wiki/Boolean_data_type) |
| ENFORCECODEMARKER   ``` int ``` | an integer, a.k.a a whole number e.g 1, 42, -10, etc. | ENFORCECODEMARKER   ``` int value = 42; ``` | see [Integer](https://en.wikipedia.org/wiki/Integer_(computer_science)) |
| ENFORCECODEMARKER   ``` float ``` | a floating point value, a.k.a a partial number e.g 1.0, 4.2, -0.1, etc | ENFORCECODEMARKER   ``` float value = 0.333; ``` | see [Float](https://en.wikipedia.org/wiki/Floating-point_arithmetic) |
| ENFORCECODEMARKER   ``` string ``` | a text value, a.k.a a sequence of characters, e.g "Hello there" | ENFORCECODEMARKER   ``` string owk = "Hello" + " " + "there"; ``` | see [String](https://en.wikipedia.org/wiki/String_(computer_science)) |
| ENFORCECODEMARKER   ``` vector ``` | an array of three float values e.g c{ 1.0, 42.2, 66.6 } | ENFORCECODEMARKER   ``` vector position = { 5, 1.5, 10 }; ``` | see [Vector](https://en.wikipedia.org/wiki/Euclidean_vector) |
| Objects | | | |
| ENFORCECODEMARKER   ``` class ``` | an object able to hold properties and methods. | see [Object Oriented Programming Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") | see [Class](https://en.wikipedia.org/wiki/Class_(programming)) |
| ENFORCECODEMARKER   ``` enum ``` | a structure having static values "listed" as its properties. ⓘ  An enum is not an object *per se*. | ENFORCECODEMARKER   ``` SCR_EBloodType myBloodType = SCR_EBloodType.O_POSITIVE; ``` | see [Enum](https://en.wikipedia.org/wiki/Enumerated_type) |
| static array | a **static** array of elements (of *a* type) - it cannot contain a mix of types. ⚠  Arrays (dynamic or static) are the only list type that can be initialised in line. | ENFORCECODEMARKER   ``` string helloThereWords[] = { "Hello", "there" }; ``` | see [Array](https://en.wikipedia.org/wiki/Array_(data_type)) |
| ENFORCECODEMARKER   ``` array<x> ``` | a **dynamic** array of elements (of *a* type) - it cannot contain a mix of types. ⚠  Arrays (dynamic or static) are the only list type that can be initialised in line. | ENFORCECODEMARKER   ``` array<string> helloThereWords = { "Hello", "there" }; ``` | see [Array](https://en.wikipedia.org/wiki/Array_(data_type)) |
| ENFORCECODEMARKER   ``` set<x> ``` | a set of ***unique*** elements, it can only contain one type of elements. ⚠  Values are **not** stored in insertion order. | ENFORCECODEMARKER   ``` set<string> helloThereWords = new set<string>(); helloThereWords.Insert("Hello"); helloThereWords.Insert("there"); helloThereWords.Insert("Hello"); // ignored Print(helloThereWords.Count()); // prints '2' ``` | see [Set](https://en.wikipedia.org/wiki/Set_(abstract_data_type)) |
| ENFORCECODEMARKER   ``` map<x, y> ``` | a map constituted of key-value pairs, where the key aspect is used to obtain the value. ⚠  Values are **not** stored in insertion order. | ENFORCECODEMARKER   ``` map<string, int> wordsAndLength = new map<string, int>(); wordsAndLength.Insert("General", 7); wordsAndLength.Insert("Kenobi", 6); Print(replyWordsAndLength.Get("Kenobi")); // prints '6' ``` | see [Map](https://en.wikipedia.org/wiki/Map_%28higher-order_function%29) |

## Object-Oriented Programming

See [Object Oriented Programming Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") and [Object Oriented Programming Advanced Usage](/wiki/Arma_Reforger:Object_Oriented_Programming_Advanced_Usage "Arma Reforger:Object Oriented Programming Advanced Usage").

## Operations

### Assignation

Assignation operations convert the value to the expected type; see below:

```enforce
// adding int and int
int result = 1 + 2; // result == 3
// adding float and float
float result = 1.25 + 2.75; // result == 4
// adding int and float
int result = 1 + 0.9; // result == 1 - int truncates a float result
int result = 0.9 + 0.9 + 0.9; // result == 2 - 2.7 truncated to 2
float result = 0.9 + 0.9 + 0.9; // result == 2.7
float result = 1 + 1.9; // result == 2.9
float result = 1.9 + 1; // result == 2.9
bool invalid = new SCR_Ray(); // error: Types 'SCR_Ray' and 'bool' are unrelated
SCR_Ray instance = new SCR_Ray();
bool valid = instance; // valid == true
bool result = 42; // result == true
```

### Logic

```enforce
SCR_Ray instance = new SCR_Ray();
if (instance) // identical to "instance != null"
Print("Instance exists");
else
Print("Instance does not exists");
string text;
if (text) // identical to "!text.IsEmpty()"
Print("Text is not empty");
else
Print("Text is empty");
int val = 42; // same with float
if (val) // identical to "val != 0"
Print("Val is not zero");
else
Print("Val is zero");
```

⚠

cif (intValue) and cif (stringValue) are **not** recommended by [Scripting: Best Practices](/wiki/Arma_Reforger:Scripting:_Best_Practices "Arma Reforger:Scripting: Best Practices").

🏗

This article is a **[work in progress](/wiki/Category:WIP "Category:WIP")**!
