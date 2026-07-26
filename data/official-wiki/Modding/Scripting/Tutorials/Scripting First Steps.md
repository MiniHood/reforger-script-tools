# [Scripting First Steps](https://community.bistudio.com/wiki/Arma_Reforger:Scripting_First_Steps)

Welcome to Enforce Script! This guide will get you writing and testing your first lines of code in 5 minutes.

ⓘ

This is a hands-on quick start. Skip the theory, start coding immediately!

## Requirements

* **Arma Reforger Tools** ([Arma Reforger Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools")) installed via Steam
* That's it!

## Open Workbench and Remote Console

1. Launch **Enfusion Workbench** from Steam (Arma Reforger Tools)
2. Click on the **Script Editor** icon or select it from the menu  

   [![](/wikidata/images/thumb/0/01/WE_UI.png/500px-WE_UI.png)](/wiki/File:WE_UI.png)

   Launch Script Editor from Workbench
3. If the **Remote Console** window is not visible, open it from the menu
4. Click on the **Remote Console** and **Output** tabs at the bottom to switch to the appropriate view  

   [![](/wikidata/images/thumb/8/84/SE_UI_Default.png/500px-SE_UI_Default.png)](/wiki/File:SE_UI_Default.png)

   Script Editor default UI - click the tabs at the bottom

   [![](/wikidata/images/thumb/d/d6/SE_UI_RC_and_output.png/500px-SE_UI_RC_and_output.png)](/wiki/File:SE_UI_RC_and_output.png)

   Script Editor with Remote Console and Output panels visible
5. You are ready to start coding!

ⓘ

For these basic exercises, you do not need to load a world or run the game. The Remote Console can execute simple code directly.

## First Line of Code

Type the following in the **Remote Console** and click the **Run** button at the top of the remote console panel:

The cPrint method prints (as its name states) the provided arguments into the log console:

```enforce
Print("Hello there!"); // displays "Hello there!" in the log console
```

**You did it!** You just wrote your first Enforce Script code. The text appears in the console output.

cPrintFormat allows for printing a string with arguments:

```enforce
PrintFormat("Hello %1, welcome to %2!", "there", "Arma Reforger"); // %1 is replaced by "there" (the first argument), %2 by "Arma Reforger
PrintFormat("Hello %1, welcome to %2!", "you", "Enfusion"); // result here is "Hello you, welcome to Enfusion!"
```

⚠

A percent sign is printed without issue using cPrint, however it requires to be doubled with cPrintFormat, being a special character to this function:

```enforce
Print("5%"); // prints "5%"
Print("%%"); // prints "%%"
PrintFormat("%1", 5); // prints "5"
PrintFormat("5%"); // prints "5"
PrintFormat("5%%"); // prints "5%"
PrintFormat("%1%%", 5); // prints "5%"
```

## Basic Maths

Try these commands one by one in the Remote Console:

```enforce
Print(10 + 5);
Print(20 * 3);
Print(100 / 4);
```

The console shows: c15, c60, c25.

## Variables

Variables let you store and reuse values:

```enforce
int myAge = 25;
Print("I am " + myAge + " years old");
```

Try changing the number and run it again!

**Common variable types:**

* c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) - whole numbers (1, 42, -5)
* c[float](enfusion://ScriptEditor/scripts/Core/generated/Types/float.c;12) - decimal numbers (3.14, 10.5)
* c[string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) - text ("Hello", "Arma")
* c[bool](enfusion://ScriptEditor/scripts/Core/generated/Types/bool.c;12) - true or false

ⓘ

For a complete list of all data types, see [Scripting Values](/wiki/Arma_Reforger:Scripting:_Values "Arma Reforger:Scripting: Values").

```enforce
float distance = 150.5;
string playerName = "Soldier";
PrintFormat("%1 is at %2m", playerName, distance);
```

## Decisions

```enforce
int health = 75;
if (health > 50)
Print("Healthy!");
else
Print("Need medical attention");
```

Try changing chealth to different numbers!

## Arrays

Arrays can be seen as lists that store naught to multiple values **of the same type**:

```enforce
array<string> soldiers = { "Alpha", "Bravo", "Charlie" };
Print(soldiers[0]); // Prints "Alpha"
Print(soldiers[1]); // Prints "Bravo"

soldiers.Insert("Delta"); // soldiers is now { "Alpha", "Bravo", "Charlie", "Delta" }
Print("Squad size: " + soldiers.Count()); // Squad size: 4
```

## Loops

### For

A cfor loop repeats instructions a specific number of times:

```enforce
for (int i; i < 5; i++)
{
	Print("Count: " + i); // prints ("Count: ") 0, 1, 2, 3, 4 then leaves (i < 5)
}
```

### Foreach

A cforeach loop goes through all array items:

```enforce
array<string> weapons = { "Rifle", "Pistol", "Grenade" };
foreach (string weapon : weapons)
{
	Print("Weapon: " + weapon); // prints "Rifle", "Pistol", "Grenade"
}
```

The index can also be obtained this way:

```enforce
foreach (int i, string weapon : weapons)
```

## A First Mini-Program

Combine what you learned! Copy this entire block and run it:

```enforce
// Soldier health checker
array<string> soldierNames = { "Alpha", "Bravo", "Charlie", "Delta" };
array<int> healthValues = { 100, 45, 80, 20 };
Print("=== Squad Status Report ===");
foreach (int i, string soldierName : soldierNames)
{
	int health = healthValues[i];
	if (health > 70)
	PrintFormat("%1: Healthy (%2%%)", soldierName, health);
	else if (health > 30)
	PrintFormat("%1: Injured (%2%%)", soldierName, health);
	else
	PrintFormat("%1: Critical! (%2%%)", soldierName, health);
}
Print("=== End of Report ===");
```

ⓘ

Lines starting with c// are **comments** - they are ignored by the code but help explain what is happening.

## Quick Reference Card

| Category | Code | Description |
| --- | --- | --- |
| **Output** | ENFORCECODEMARKER   ``` Print("text"); ``` | Shows text in console |
| **Variables** | ENFORCECODEMARKER   ``` int number = 42; ``` | Stores a whole number |
| ENFORCECODEMARKER   ``` float decimal = 3.14; ``` | Stores a decimal number |
| ENFORCECODEMARKER   ``` string text = "Hi"; ``` | Stores text |
| ENFORCECODEMARKER   ``` bool flag = true; ``` | Stores true/false |
| **Arrays** | ENFORCECODEMARKER   ``` array<int> numbers = { 1, 2, 3 }; ``` | Creates a list |
| ENFORCECODEMARKER   ``` myArray.Insert(value); ``` | Adds item to list |
| ENFORCECODEMARKER   ``` myArray.Count(); ``` | Gets list size |
| **If/Else** | ENFORCECODEMARKER   ``` if (condition) { } ``` | Do something if true |
| ENFORCECODEMARKER   ``` else { } ``` | Do something if false |
| **Loops** | ENFORCECODEMARKER   ``` for (int i; i < 10; i++) { } ``` | Repeat 10 times |
| ENFORCECODEMARKER   ``` foreach (int item : myArray) { } ``` | Execute for each item |
| **Math** | c+ | Add |
| c- | Subtract |
| c\* | Multiply |
| c/ | Divide |
| **Compare** | c== | Equal to |
| c!= | Not equal to |
| c> | Greater than |
| c< | Less than |
| c>= | Greater or equal |
| c<= | Less or equal |

## Common Mistakes

| Incorrect | Correct | Explanation |
| --- | --- | --- |
| cprint("hello"); | cPrint("hello"); | Capital P in Print |
| c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) x = 5 | c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) x = 5; | Missing semicolon |
| cif (x = 5) | cif (x == 5) | Use == for comparison, = for assignment |
| c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) firstElement = myArray[1]; | c[int](enfusion://ScriptEditor/scripts/Core/generated/Types/int.c;12) firstElement = myArray[0]; | Arrays start at 0, not 1 |
| c[string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) name = John; | c[string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) name = "John"; | Text needs quotes |

ⓘ

Follow professional coding standards: [Scripting Conventions](/wiki/Arma_Reforger:Scripting:_Conventions "Arma Reforger:Scripting: Conventions").

## Exercises

The following challenges help in practicing and understanding scripting.

### Temperature Converter

Create a program that converts 25°C to Fahrenheit.
Show solution

```enforce
// Convert Celsius to Fahrenheit
float celsius = 25;
float fahrenheit = (celsius * 9 / 5) + 32;
PrintFormat("%1°C = %2°F", celsius, fahrenheit);
```

### Countdown

Create a countdown from 10 to 0, then print "Launch!".
Show solution

```enforce
// Count down from 10 to 0
for (int i = 10; i >= 0; i--)
{
	Print(i);
}
Print("Launch!");
```

### Squad Filter

Print only the soldiers with health above 50 from two arrays (names and health values).
Show solution

```enforce
// Only print soldiers with health above 50
array<string> names = { "Alpha", "Bravo", "Charlie" };
array<int> health = { 80, 30, 90 };
foreach (int i, string name : names)
{
	if (health[i] > 50)
	Print(name + " is combat ready");
}
```

## Troubleshooting

**Console will not open?**

* Make sure the **Script Editor** is opened in Workbench
* Look for the **Remote Console** panel - if hidden, open it from the menu
* Ensure Workbench is properly installed via Steam (Arma Reforger Tools)

**Code does not work?**

* Check for missing semicolons c;
* Make sure quotes match: c"text"
* Check spelling and capitalisation (casing matters: print("ok"); is different from Print("ok");)
* Look for error messages in red

**Getting errors?**

* Read the error message - it often tells you what is wrong
* Click the error message and check reported line number(s)
* Make sure all open brackets c{ } are closed

## See Also

**Reference Documentation:**

* [Scripting Values](/wiki/Arma_Reforger:Scripting:_Values "Arma Reforger:Scripting: Values") - All data types in detail
* [Scripting Conventions](/wiki/Arma_Reforger:Scripting:_Conventions "Arma Reforger:Scripting: Conventions") - Bohemia Interactive coding standards
* [OOP Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics") - Object-oriented programming fundamentals
