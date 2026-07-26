# [From SQF to Enforce Script](https://community.bistudio.com/wiki/Arma_Reforger:From_SQF_to_Enforce_Script)

Welcome to **Enfusion**!

While **[Arma 3](/wiki/Category:Arma_3 "Category:Arma 3")** is powered by **[SQF](/wiki/SQF_Syntax "SQF Syntax")**, **[Arma Reforger](/wiki/Category:Arma_Reforger "Category:Arma Reforger")** uses **[Enforce Script](/wiki/Category:Arma_Reforger/Modding/Scripting/Guidelines "Category:Arma Reforger/Modding/Scripting/Guidelines")** as language, which is syntactically close to C#.

**SQF** is a succession of [Scripting Commands](/wiki/Category:Scripting_Commands "Category:Scripting Commands") processed in a certain order to give you a certain access or reference to the engine objects themselves (e.g [player](/wiki/player "player")).

It is easy to access, read and process at the cost of performance.

**Enforce Script** is an **Object-Oriented Programming** **language** (OOP language) which as this designation suggests is based on objects.

ⓘ

See [Object Oriented Programming Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") for an introduction to the OOP approach.

The language is way closer to the engine and is inspired by C++, which allows scripters **more possibilities**, but also **more responsibilities** - a script can make or break one's game experience.

It is a stricter language that allows for a wider, stronger interaction with the engine.

## Similarities

Let's start with the similarities between the languages… because they are very different, the list will actually be a short one.

| SQF | Enforce Script |
| --- | --- |
| Copy // inline comment /\* comment block \*/ [private](/wiki/private) \_booleanValue [=](/wiki/a_=_b) [true](/wiki/true); [private](/wiki/private) \_floatValue1 [=](/wiki/a_=_b) 5; [private](/wiki/private) \_floatValue2 [=](/wiki/a_=_b) 5.5; [private](/wiki/private) \_stringValue [=](/wiki/a_=_b) "Hello there"; | ENFORCECODEMARKER   ``` // inline comment /* comment block */ bool booleanValue = true; float floatValue1 = 5; float floatValue2 = 5.5; string stringValue = "Hello there"; ``` |

And that's it! As this example already shows, even the variable declaration syntax has changed. Now, let's see the main differences on first sight!

## Main Differences

### If-Then-Else

Definitely the most important point of this document. SQF being keyword-based, it needs to "bridge" the "if" condition to the "then" value, using the [then](/wiki/then "then") keyword. Enforce Script uses if-else structure like any recent language.

| SQF | Enforce Script |
| --- | --- |
| Copy [if](/wiki/if) ([alive](/wiki/alive) [player](/wiki/player)) [then](/wiki/then) { [hint](/wiki/hint) "I am alive!"; }; | ENFORCECODEMARKER   ``` if (myPlayer.GetHealth() > 0) // no more 'then'! { 	Print("I am alive!"); } if (myPlayer.GetHealth() > 0) Print("I am alive!"); // a short version is possible ``` |

The same applies for other code structures (while-do, switch-do, for-do, etc), see [Code Flow](#Code_Flow) below.

The **else** part supports else if (note: not elseif) statements:

| SQF | Enforce Script |
| --- | --- |
| Copy [if](/wiki/if) ([alive](/wiki/alive) [player](/wiki/player)) [then](/wiki/then) { [hint](/wiki/hint) "I am alive!"; } [else](/wiki/else) { [if](/wiki/if) ([damage](/wiki/damage) [player](/wiki/player) [>](/wiki/a_greater_b) 0.9) [then](/wiki/then) { [hint](/wiki/hint) "That's a lot of damage"; }; }; | ENFORCECODEMARKER   ``` if (myPlayer.GetHealth() > 0) { 	Print("I am alive!"); } else if (myPlayer.GetHealth() < 10) // no more "else { if }" structure { 	Print("That's a lot of damage"); } ``` |

### Code Flow

#### for

| SQF | Enforce Script |
| --- | --- |
| Copy // There are two ways to write a for-loop in SQF: [for](/wiki/for) "\_i" [from](/wiki/from) 0 [to](/wiki/to) 10 [do](/wiki/do) { }; [for](/wiki/for) [{ \_i [=](/wiki/a_=_b) 0 }, { \_i [<](/wiki/a_less_b) 10 }, { \_i [=](/wiki/a_=_b) \_i [+](/wiki/+) 1 }] [do](/wiki/do) { }; | ENFORCECODEMARKER   ``` // Enforce Script has only one type of for loop // statement, condition and step are separated by semicolons for (int i; i < 5; i++) // i is automatically initialised to 0, but "int i = 0" could also be used { } ``` |

#### foreach

| SQF | Enforce Script |
| --- | --- |
| Copy // forEach loop in SQF is very convenient by selecting the current index and element automatically { [hint](/wiki/hint) [str](/wiki/str) [\_forEachIndex](/wiki/Magic_Variables#forEachIndex); // \_forEachIndex is the current array index [hint](/wiki/hint) [str](/wiki/str) [\_x](/wiki/Magic_Variables#x); // \_x represents the current element } [forEach](/wiki/forEach) \_items; | ENFORCECODEMARKER   ``` // foreach's syntax is changed and follows today's standards // the code to be run is placed after the keyword // beware of the syntax: currentIndex and currentElement are separated with "," // whereas currentElement and the array being iterated over are separated with ":" foreach (int currentIndex, string currentElement : items) { 	Print(currentIndex); 	Print(currentElement); } // or, if the index is not required foreach (string currentElement : items) { 	Print(currentElement); } ``` |

#### while

| SQF | Enforce Script |
| --- | --- |
| Copy // Unlike other flow structures, the "while" condition is written between {} brackets [while](/wiki/while) { \_i [<](/wiki/a_less_b) 10 } [do](/wiki/do) { \_i [=](/wiki/a_=_b) \_i [+](/wiki/+) 1; // only way to increment i by 1 as i++ does not exist in SQF }; | ENFORCECODEMARKER   ``` // Condition check in while loop is done in regular () brackets while (i < 10) { 	i++; // this will increment i by 1 } ``` |

#### switch

| SQF | Enforce Script |
| --- | --- |
| Copy [switch](/wiki/switch) (i) [do](/wiki/do) { [case](/wiki/case) 0[:](/wiki/a_:_b) { [systemChat](/wiki/systemChat) "case 0" }; // if i is 0, code in brackets is executed and switch is exited automatically [case](/wiki/case) 1[:](/wiki/a_:_b) { [systemChat](/wiki/systemChat) "case 1" }; [default](/wiki/default) { [systemChat](/wiki/systemChat) "default" }; // default does -not- use ":" like case does }; | ENFORCECODEMARKER   ``` switch (i) { 	case 0: 	Print("case 0"); 	break; 	case 1: 	Print("case 1"); 	break; 	default: // default -does- use ":" just like case does 	Print("default"); 	break; } ``` |

#### Others

| SQF | Enforce Script |
| --- | --- |
| Copy // SPECIAL CASES i [=](/wiki/a_=_b) [if](/wiki/if) (isScript) [then](/wiki/then) { 1 } [else](/wiki/else) { 0 }; // if-then-else returning value, not possible in Enforce Script [if](/wiki/if) (isScript) [exitWith](/wiki/exitWith) {}; // exitWith breaks from the current -scope- // it will exit the script only if at its root // lazy evaluation: by default, all statements in condition field are evaluated // to prevent this behaviour the second statement should be wrapped in code brackets {} // if (!isNil "\_value" && \_value > 10) then // wrong as "\_value > 10" will be evaluated [if](/wiki/if) ([!](/wiki/!_a)[isNil](/wiki/isNil) "\_value" [&&](/wiki/a_&&_b) { \_value [>](/wiki/a_greater_b) 10 }) [then](/wiki/then) { [hint](/wiki/hint) "\_value exists and is greater than 10"; }; | ENFORCECODEMARKER   ``` // SPECIAL CASES // an if statement can avoid brackets for only one line of code if (isScript) break; // this will exit a while loop if (isScript) return 0; // this will exit the current method and return 0 // "value.GetValue() > 10" will not be evaluated if "value != null" returns false if (value != null && value.GetValue() > 10) { 	Print("value exists and is greater than 10"); } ``` |

### Case Sensitivity

Everything is case-sensitive in Enforce Script, unlike SQF:

| SQF | Enforce Script |
| --- | --- |
| Copy [private](/wiki/private) \_myValue [=](/wiki/a_=_b) "Hello there"; [hint](/wiki/hint) \_MYVALUE; // hints "Hello there" IF ([true](/wiki/true)) THEN { [hint](/wiki/hint) "it's true!"; }; [if](/wiki/if) (\_myValue [==](/wiki/a_==_b) "HELLO THERE") [then](/wiki/then) { // will work }; | ENFORCECODEMARKER   ``` string myValue = "Hello there"; Print(MYVALUE); // error: MYVALUE is undefined IF (true) // error: "IF" is an unknown operator. "if" is the proper casing { 	Print("it's true!"); } if (myValue == "HELLO THERE") { 	// will NOT work - "Hello there" is different from "HELLO THERE" } ``` |

### Typed Variables

In SQF, a variable can change its type on the go according to value assignation.
In Enforce Script, a variable has one type and this type cannot be changed during the variable's lifetime.

| SQF | Enforce Script |
| --- | --- |
| Copy [private](/wiki/private) \_myValue [=](/wiki/a_=_b) "Hello there"; // \_myValue is a string \_myValue [=](/wiki/a_=_b) 5; // \_myValue is now a floating point number \_myValue [=](/wiki/a_=_b) [true](/wiki/true); // \_myValue is now a boolean, etc | ENFORCECODEMARKER   ``` string myValue = "Hello there"; // myValue is a string myValue = 5; // error: cannot convert an int to string myValue = true; // error: cannot convert a bool to string ``` |

### Position

A position is the location of an object. A position is composed of X, Y and Z values, X being the West → East axis, Y being the Floor → Sky axis, and Z being the South → North axis. A position is called origin in Enfusion.

Real Virtuality uses the [X, Z, Y] format (a vector pointing up being [0, 0, 1]) whereas Enfusion uses { X, Y, Z } (a vector pointing up being { 0, 1, 0 }).

⚠

Note that in Enfusion the coordinate system is **left-handed** - see [Wikipedia](https://en.wikipedia.org/wiki/Cartesian_coordinate_system#Orientation_and_handedness).

| SQF | Enforce Script |
| --- | --- |
| Copy [private](/wiki/private) \_northVector [=](/wiki/a_=_b) [0, 1, 0]; [private](/wiki/private) \_westVector [=](/wiki/a_=_b) [-1, 0, 0]; [private](/wiki/private) \_upVector [=](/wiki/a_=_b) [0, 0, 1]; | ENFORCECODEMARKER   ``` vector northVector = "0 0 1"; vector westVector = "-1 0 0"; vector upVector = "0 1 0"; ``` |

### Array

In SQF, an array is a list that can contain any type of data including sub-arrays, and can be expanded at will.
In Enforce Script, there are two types of array: static (extremely fast but of static size) and dynamic (size can change); also, an array can only contain **one** type of data.

| SQF | Enforce Script |
| --- | --- |
| Copy [private](/wiki/private) \_array [=](/wiki/a_=_b) []; \_array [pushBack](/wiki/pushBack) 0; // \_array is [0] \_array [pushBack](/wiki/pushBack) [1]; // \_array is [0, [1]] \_array [pushBack](/wiki/pushBack) "2"; // \_array is [0, [1], "2"], etc \_array [set](/wiki/set) [0, 1]; // sets the first element to 1 // \_array is [1, [1], "2"] // array duplication [private](/wiki/private) \_array2 [=](/wiki/a_=_b) [+](/wiki/+)\_array; | ENFORCECODEMARKER   ``` array<int> myArray = {}; // dynamic array myArray.Insert(0); // array is { 0 } myArray.Insert(true); // array is { 0, 1 } - true is 1 myArray.Insert("2"); // error: an array cannot contain a different type myArray[0] = 1; // sets the first element to 1 // array duplication array<int> duplicate = {}; duplicate.Copy(myArray); // does not work for ref items // ref array duplication array<ref SCR_Class> refDuplicate = {}; foreach (SCR_Class item : myArray) { 	refDuplicate.Insert(item); } // a helper is provided array<ref SCR_Class> refDuplicate = SCR_ArrayHelper<SCR_Class>.GetCopy(myArray); // new array type: static (size) array int myArray[3]; // automatically initialised to the type's default value (here { 0, 0, 0 }) myArray[0] = 1; // { 1, 0, 0 } myArray[1] = 2; // { 1, 2, 0 } myArray[2] = 3; // { 1, 2, 3 } ``` |

### Data Types

Including **vectors**, **enums**, **sets**, **classes** of course and others! Find all the new data types on their [dedicated page](/wiki/Arma_Reforger:Scripting:_Values "Arma Reforger:Scripting: Values").

| SQF | Enforce Script |
| --- | --- |
| Copy [private](/wiki/private) \_position [=](/wiki/a_=_b) [50, 50, 0]; // an array [private](/wiki/private) \_number1 [=](/wiki/a_=_b) 42; // a Number, a.k.a float [private](/wiki/private) \_number2 [=](/wiki/a_=_b) 5.5; // a Number, a.k.a float \_number1 [=](/wiki/a_=_b) \_number1 [+](/wiki/+) 1; \_number2 [=](/wiki/a_=_b) \_number2 [-](/wiki/-) 1; [private](/wiki/private) \_hashmap [=](/wiki/a_=_b) [createHashMap](/wiki/createHashMap); \_hashmap [set](/wiki/set) [1, "oops"]; \_hashmap [set](/wiki/set) [2, "two"]; \_hashmap [set](/wiki/set) ["two", [true](/wiki/true)]; \_hashmap [set](/wiki/set) [1, "one"]; [private](/wiki/private) \_value [=](/wiki/a_=_b) [nil](/wiki/nil); [if](/wiki/if) ([isNil](/wiki/isNil) "\_value") [then](/wiki/then) { [hint](/wiki/hint) "\_value is not defined" }; [private](/wiki/private) \_value [=](/wiki/a_=_b) [objectParent](/wiki/objectParent) [player](/wiki/player); [if](/wiki/if) ([isNull](/wiki/isNull) \_value) [then](/wiki/then) { [hint](/wiki/hint) "player has vehicle" }; | ENFORCECODEMARKER   ``` vector position = { 50, 0, 50 }; int number1 = 42; float number2 = 5.5; number1++; number2--; map<int, string> hashmap = new map<int, string>(); // types are set in stone hashmap.Insert(1, "oops"); hashmap.Insert(2, "two"); // no Enforce Script equivalent hashmap.Set(1, "one"); // Set() is to update a value - it can create it too, but Insert() is faster set<string> stringSet = new set<string>(); stringSet.Insert("value1"); stringSet.Insert("value2"); stringSet.Insert("value1"); // returns false as value1 already exists // there is no isNil equivalent in Enforce Script as a variable is either defined or not // its content can still be null as seen below MyClass myInstance; if (myInstance == null) Print("myInstance is null"); // alternatively if (!myInstance) Print("myInstance is null"); ``` |

### Object-Oriented Programming

SQF is a scripting language based on sequences of expressions set in scripts. Enforce Script is Object-Oriented Programming (OOP) meaning that the code is placed inside declared objects.

ⓘ

The basics of OOP can be found in [Object Oriented Programming Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics"); see also [Wikipedia](https://en.wikipedia.org/wiki/Object-oriented_programming).

| SQF | Enforce Script |
| --- | --- |
| Copy // in-script code declaration - normally one would have to declare it in CfgFunctions [private](/wiki/private) \_sum [=](/wiki/a_=_b) { [params](/wiki/params) [ ["\_value1", 0, [0]], ["\_value2", 0, [0]] ]; \_value1 [+](/wiki/+) \_value2; }; [private](/wiki/private) \_result [=](/wiki/a_=_b) [5, 3] [call](/wiki/call) \_sum; | ENFORCECODEMARKER   ``` // a method is a class' function class MyClass { 	int Sum(int value1, int value2) 	{ 		return value1 + value2; 	} 	// a different method signature, yet named the same 	string Sum(string value1, string value2) 	{ 		return value1 + value2; 	} } // further in code MyClass myInstance = new MyClass(); int result = myInstance.Sum(5, 3); string result = myInstance.Sum("Hello ", "there"); string result = myInstance.Sum("Hello ", 3); // does not work - no such string/int signature, only string/string ``` |

⚠

It is worth noting that Enforce Script does **not** benefit from previous titles' [Scheduler](/wiki/Scheduler "Scheduler"), therefore wildly creating new threads is not recommended!
See Enforce Script's example below for Callqueue's usage.

| SQF | Enforce Script |
| --- | --- |
| Copy [hint](/wiki/hint) "Thread A 1/2"; 0 [spawn](/wiki/spawn) { [hint](/wiki/hint) "Thread B 1/2"; [sleep](/wiki/sleep) 1; [hint](/wiki/hint) "Thread B 2/2"; }; [hint](/wiki/hint) "Thread A 2/2"; | ENFORCECODEMARKER   ``` class MyClass { 	void PrintMessage() 	{ 		Print("Thread A 1/2"); 		GetGame().GetCallqueue().CallLater(PrintOtherMessage, 1000); // in milliseconds 		// thread Thread_PrintOtherMessage(); // thread usage is not recommended 		Print("Thread A 2/2"); 	} 	void PrintOtherMessage() 	{ 		Print("CallQueue later"); 	} 	void Thread_PrintOtherMessage() 	{ 		Print("Thread B 1/2"); 		Sleep(1000); 		Print("Thread B 2/2"); 	} } // further in code MyClass myInstance = new MyClass(); myInstance.PrintMessage(); ``` |

## See Also

* [Scripting: Conventions](/wiki/Arma_Reforger:Scripting:_Conventions "Arma Reforger:Scripting: Conventions")
* [Scripting Guidelines](/wiki/Category:Arma_Reforger/Modding/Scripting/Guidelines "Category:Arma Reforger/Modding/Scripting/Guidelines")
* [Scripting Tutorials](/wiki/Category:Arma_Reforger/Modding/Scripting/Tutorials "Category:Arma Reforger/Modding/Scripting/Tutorials")
