# [Scripting: Best Practices](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Best_Practices)

## Getting Started

In the domain of development, any rule is a rule of thumb.
If a rule states for example that it is better that a line of code doesn't go over 80 characters, it doesn't mean that any line ***must not*** go over 80 characters; sometimes, the situation needs it.

If the code has a good structure, **do not** change it to enforce a single arbitrary rule. If many of them are not implemented/not respected, changes should be applied; again, this is according to one's judgement.

With that being said, let's go!

## Best Practices

ⓘ

See [Scripting Conventions](/wiki/Arma_Reforger:Scripting:_Conventions "Arma Reforger:Scripting: Conventions") for all the conventions to date.

### Code Format

* **Reminder:** chosen indentation for Enfusion is [Allman style](https://en.wikipedia.org/wiki/Indentation_style#Allman_style)
* **Reminder:** indentation is done with **tabulations**
* Use empty space. Line return, spaces before and after brackets, if this improves readability, use it: space is free
* *One-lining* (putting everything in one statement) memory improvement is most of the time not worth the headache it gives when trying to read it: don't overuse it
  + it also hinders debugger's usage, e.g in the event of an inlined if

### Variable Format

* Name variables and functions properly: code must be readable by a human being, e.g variables like **u** instead of **uniform** should not exist.
  + **i** is an accepted iteration variable name (e.g in for loops).
* Prefix any public content (classes, global methods, global variables) with a [Creator Tag](/wiki/Scripting_Tags "Scripting Tags") in order to prevent conflicts with other mods.
* Use the closest value type whenever possible; using auto for a known variable type makes code more obscure and prevents autocompletion.

### Code Structuration

#### SOLID

A series of development principles to follow in order to ensure an easy code maintenance and lifetime.

ⓘ

See [SOLID](https://en.wikipedia.org/wiki/SOLID).

#### DRY

**D**on't **R**epeat **Y**ourself. If within the same class, the same code or the same pattern is written in various places, write a protected method and use appropriate parameters.

ⓘ

See [Don't repeat yourself](https://en.wikipedia.org/wiki/Don't_repeat_yourself).

#### Logical Simplifications

If the code has too many repetitions, make a common method as stated above.

If the code has too many levels, it is time to split it and rethink it.

#### Examples

| Improvable | Good |
| --- | --- |
| ENFORCECODEMARKER   ``` auto number = 42; Animal cutePet = new Dog(); ``` | ENFORCECODEMARKER   ``` int number = 42; Dog cutePet = new Dog(); ``` |
| ENFORCECODEMARKER   ``` int i = 0; string result = ""; SCR_MyClass obj = null; ``` | ENFORCECODEMARKER   ``` int i; // default value = 0 - see Values - integer string result; // default value = "" (a string cannot be null) SCR_MyClass obj; // default value = null ``` |
| ENFORCECODEMARKER   ``` // a method call is more expensive than a bool check if (obj.MustBeTreated() || m_bTreatAllObjects) Print(obj); if (obj.MustBeTreated() && m_bTreatAllObjects) Print(obj); ``` | ENFORCECODEMARKER   ``` // cheap checks go first, expensive checks (method calls) go after if (m_bTreatAllObjects || obj.MustBeTreated()) Print(obj); if (m_bTreatAllObjects && obj.MustBeTreated()) Print(obj); ``` |
| ENFORCECODEMARKER   ``` // many identical method calls if (obj.MustBeTreated() && obj.GetObject()) Print("Result: " + obj.GetObject().m_sValue1 + " " + obj.GetObject().m_sValue2); ``` | ENFORCECODEMARKER   ``` // "bigger", non-repetitive code can be beneficial for performance and readability if (obj.MustBeTreated()) { 	SCR_Object subObj = obj.GetObject(); 	if (subObj) 	Print("Result: " + subObj.m_sValue1 + " " + subObj.m_sValue2); } ``` |
| ENFORCECODEMARKER   ``` foreach (SCR_Object obj : list) { 	Method(obj); // one method call per iteration } void Method(SCR_Object obj) { 	if (!obj) 	return; 	Print(obj.m_sName + " has a value of " + obj.m_sValue); } ``` | ENFORCECODEMARKER   ``` foreach (SCR_Object obj : list) // the least method calls, the better { 	if (!obj) 	continue; 	Print(obj.m_sName + " has a value of " + obj.m_sValue); } ``` |
| ENFORCECODEMARKER   ``` bool IsObjectAlive(SCR_Object obj) { 	if (!obj) 	return false; 	if (obj.m_Health > 0) // keep this structure for complex code 	return true; 	else 	return false; } ``` | ENFORCECODEMARKER   ``` bool IsObjectAlive(SCR_Object obj) { 	return obj && obj.m_Health > 0; } ``` |
| ENFORCECODEMARKER   ``` bool IsObjectValid(SCR_Object obj) { 	if (!obj) 	return false; 	return true; } ``` | ENFORCECODEMARKER   ``` bool IsObjectValid(SCR_Object obj) { 	return obj != null; // for readability } ``` |
| ENFORCECODEMARKER   ``` for (int i; i < list.Count(); i++) // list.Count() is called on every iteration { 	// ... } ``` | ENFORCECODEMARKER   ``` for (int i, count = list.Count(); i < count; i++) // only one list.Count() call { 	// ... } ``` |
| ENFORCECODEMARKER   ``` for (int i, count = list.Count(); i < count; i++) { 	if (list[i]) // first 	Print(list[i]); // and second .Get(i) method calls } ``` | ENFORCECODEMARKER   ``` foreach (SCR_Object obj : list) // foreach is faster for start-to-end iterating { 	if (obj) 	Print(obj); // no additional method call } ``` |
| ENFORCECODEMARKER   ``` for (int i, count = list.Count(); i < count; i++) { 	PrintFormat("Object #%1 = %2", i, list[i]); } ``` | ENFORCECODEMARKER   ``` foreach (int i, SCR_Object obj : list) // iteration index is available this way too { 	PrintFormat("Object #%1 = %2", i, obj); } ``` |
| ENFORCECODEMARKER   ``` // declaring an 'obj' every loop generates a pointer release each time foreach (SCR_ParentObject parent : list) { 	SCR_Object obj = parent.m_Object; 	if (obj) 	Print(obj.m_sName); } ``` | ENFORCECODEMARKER   ``` SCR_Object obj; // external declaration = only one release at the end of the scope foreach (SCR_ParentObject parent : list) { 	obj = parent.m_Object; 	if (obj) 	Print(obj.m_sName); } ``` |
| ENFORCECODEMARKER   ``` array<SCR_Object> toRemove = {}; foreach (SCR_Object obj : bigArray) { 	if (obj.m_bShouldBeRemoved) 	toRemove.Insert(obj); } foreach (SCR_Object obj : toRemove) { 	bigArray.RemoveItem(obj); // or RemoveItemOrdered if order is important } ``` | ENFORCECODEMARKER   ``` for (int i = bigArray.Count() - 1; i >= 0; i--) // reverse iterating { 	if (bigArray[i].m_bShouldBeRemoved) 	bigArray.Remove(i); // or RemoveItemOrdered if order is important } ``` |
| ENFORCECODEMARKER   ``` if (a) { 	if (b) 	{ 		if (c) // also known as Hadouken code 		Method(true); 		else 		Method(false); 	} } ``` | ENFORCECODEMARKER   ``` if (a && b) Method(c); ``` |
| ENFORCECODEMARKER   ``` if (a) { 	Method(a); 	if (b) 	{ 		Method(b); 		if (c) // another Hadouken code, with complications 		{ 			Method(c); 			return 42; 		} 		else 		{ 			return -1; 		} 	} 	else 	{ 		return -1; 	} } else { 	return -1; } ``` | ENFORCECODEMARKER   ``` if (!a) return -1; // this is called early return and helps funnel down the code Method(a); if (!b) return -1; Method(b); if (!c) return -1; Method(c); return 42; ``` |
| ENFORCECODEMARKER   ``` int i; if (a) i++; if (b) i++; if (c) i++; // etc ``` | ENFORCECODEMARKER   ``` int i; array<bool> conditions = { a, b, c, /* etc */ }; foreach (bool condition : conditions) { 	if (condition) 	i++; } ``` |
| ENFORCECODEMARKER   ``` Initialise(player1, 1); Initialise(player2, 2); Initialise(player3, 3); Initialise(player4, 4); Initialise(player5, 5); Initialise(player6, 6); ``` | ENFORCECODEMARKER   ``` array<IEntity> list = { player1, player2, player3, player4, player5, player6 }; foreach (int i, IEntity item : list) { 	Initialise(item, i + 1); } ```     ENFORCECODEMARKER   ``` // or, better, one method call that initialises all of them Initialise(list); // numbering is then done inside the method, if possible Initialise(list, 1); // otherwise the starting number can be provided ``` |

#### Code Comments

Code comments are surprisingly **not** a must-have for inside code; code organisation combined to variable names should be enough to be read by a human, **then** comment can be used:

* a comment should explain ***why*** the code is written this way
* a comment should not tell ***what*** the code does; code should be self-explanatory
* as a last resort in the event of a complex piece of code, a comment can be used to describe what the code actually does - or at least its intention

On the other hand, *documentation* is more than welcome as it provides information from the outside without having to read the code. Enfusion uses [Doxygen](/wiki/Doxygen "Doxygen").

#### Files Organisation

ⓘ

See [Directory Structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure") to know how/where to organise script files (Scripts\GameCode).

* Have one class/enum per file
  + Small classes/enums can always be grouped together in the same file, provided they are part of the same system or only used there
* Use (sub-)directories to group related classes together
