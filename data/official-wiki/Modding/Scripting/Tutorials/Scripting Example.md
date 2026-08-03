# [Scripting Example](https://community.bistudio.com/wiki/Arma_Reforger:Scripting_Example)

This tutorial aims to provide scripting logic basics. The abstract project is to find certain values (smallest, biggest, average, etc) from a list of numbers.

For starters, we are aiming at "get the smallest value from the array" functionality.

## Create the Script

### New File

* [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager"): **Resource Browser** > "Create" button > Script
* [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor"): ![Right Mouse Button](/wikidata/images/thumb/8/84/mouse-button-right.png/32px-mouse-button-right.png "Right Mouse Button") (on a directory in the Projects window) > "Add New Script..."
* [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor"): `Ctrl` + `N` or Plugins > Script Wizard:

⚠

**Always** prefix the filename and the classname with a [modder tag](/wiki/Scripting_Tags "Scripting Tags") (e.g ABC\_, AR15\_, etc) in order to avoid class conflicts:

* **TAG\_**MyFile.c
* class **TAG\_**MyFile / class **TAG\_**MyFile : ParentFile

Methods and values are not concerned by this.

⚠

Scripting modding can only happen in **Modules**, defined in Arma Reforger's .gproj (Enfusion Workbench -> Workbench -> Options -> Game Project -> Script Project Manager Settings > Modules):

| Module | core | gameLib | game | workbench | workbenchGame |
| --- | --- | --- | --- | --- | --- |
| Directories | * Core | * GameLib | * Game * GameCode | * Workbench | * WorkbenchGame |

Scripts placed **outside of those folders** will be simply **ignored!**

## Get the Smallest Value

### Method Setup

#### Creation

Let's begin by creating the object and its method that will host the code.

```enforce
class TAG_TutorialExample
{
	void Method()
	{
	}
}
```

The method exists now. It is important to name objects, methods and variables properly, therefore it has to be more precise than "Method":

```enforce
class TAG_TutorialExample
{
	void GetMinValue()
	{
	}
}
```

#### Parameters

The method is empty, takes no parameters and returns nothing. Since we want to provide a list of numbers from which to extract a result, let's add a parameter:

```enforce
class TAG_TutorialExample
{
	void GetMinValue(array<int> numbers)
	{
	}
}
```

Here, one could pass a null array to the method, which could create a NULL pointer exception on code run; as an author it is our task to cover such case. There are two ways to do such check:

```enforce
class TAG_TutorialExample
{
	void GetMinValue(array<int> numbers) // the numbers array can be null
	{
		if (!numbers) // or if (numbers == null)
		return; // a null numbers is intercepted and stops the code
	}
}
```

```enforce
class TAG_TutorialExample
{
	void GetMinValue(notnull array<int> numbers) // the numbers array cannot be null
	{
	}
}
```

The latter version would make the engine throw an exception before the method is actually called. This is the version with which we are going in this tutorial's scope.

#### Return Value

The method is supposed to return the array's smallest value: it should then return a number.

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
	}
}
```

ⓘ

Some methods use an out parameter to provide the result to an already-assigned variable for memory management purpose. This is not going to be the case here for the sake of clarity.

### Code Setup

#### Conception

The goal here, as decided earlier, is to get the smallest value from the provided array of numbers.
To begin, let's start with writing human code before writing machine code, to lay the logic properly:

* have a "current best result" value
* read each element of the list
  + if an element is a better candidate (smaller) than the stored one, use this element instead
* when the end of the list is reached, return the best value

This is fine. But what happens if the list has no values?
In that case, we must decide for a default value. Depending on the method's role, default values can change:

* methods will return -1 to indicate an error or a "not found"
* methods will return 0 to state "no value = default value"

We are going for the latter here as we are returning a value.

ⓘ

Note that this method could be returning the index of the best candidate's element, and returning -1 (an invalid index) would be a perfectly acceptable answer.

The end logic:

* have a "current best result" value **initialised to zero**
* **if the list is empty, return the current best value**
* read each element of the list
  + if an element is a better candidate (smaller) than the stored one, use this element instead
* when the end of the list is reached, return the best value

**But!**

What if the array has only one element that is 10? The return value would be 0 here, which is an invalid result. Back to the drawing board:

* **if the list is empty, return 0**
* have a "current best result" **initialised to the array first element's value**
* read each element of the list
  + if an element is a better candidate (smaller) than the stored one, use this element instead
* when the end of the list is reached, return the best value

No more pitfalls in sight, code can be started!

### Writing

Following the earlier code conception, here is how it would translate to code:

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int result = numbers[0];
		foreach (int number : numbers)
		{
			if (number < result)
			result = number;
		}
		return result;
	}
}
```

### Commenting

One can note that the above code does not contain any comments.

In the past, commenting has been taught to be extensive, to cover the maximum number of lines of code; on the other hand, some people consider commenting as the last resort.

The balance is somewhere in-between, somewhat closer to the latter:

* a comment should explain ***why*** the code is written this way
* a comment should not tell ***what*** the code does; code should be self-explanatory
* as a last resort in the event of a complex piece of code, a comment can be used to describe what the code actually does - or at least its intention

| Bad Example | Good Example |
| --- | --- |
| ENFORCECODEMARKER   ``` int i = 0; // initialises i to 0 i++; // increments i by 1 ``` | ENFORCECODEMARKER   ``` // int i = 0; // i++; for (int i = 1; i < 10; i++) // starts from 1 to not have 0-based index miscalculation { 	Print(i); } ``` |

### Optimisation

«

« Premature optimization is the root of all evil. » – [Donald Knuth](https://en.wikipedia.org/wiki/Donald_Knuth)

The basic rules of coding are as follow:

* Make it work
* Make it readable
* Optimise then

**Once the code works**, optimisation can be accessed.

What is wrong with this code; it takes the wanted arguments, it returns the expected result, it is fast?  
*Almost* nothing. If we want to care about performance impact, go "straight to the point", **avoid useless operations** (thus saving cycles):

Remaining issue:

*result* is set to the array's first value, but foreach then goes and compares every array element to it, including itself

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int result = numbers[0];
		for (int i = 1; i < numbers.Count(); i++) // change has been made here
		{
			if (numbers[i] < result) // and here (number → numbers[i])
			result = numbers[i]; // and here (number → numbers[i])
		}
		return result;
	}
}
```

So, are we done now?

**No!**

As improvement has been done, another loss has been introduced in the process: numbers.Count().  
The Count method is called on every for loop; since the array is not changing its size between each loops, it is considered wasting resource.

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int result = numbers[0];
		for (int i = 1, count = numbers.Count(); i < count; i++) // count is defined once
		{
			if (numbers[i] < result)
			result = numbers[i];
		}
		return result;
	}
}
```

But: technically, foreach is still faster.

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int result = numbers[0];
		foreach (int i, int number : numbers) // we use foreach's index
		{
			if (i > 0 && number < result) // here - one bool check is faster than two results[] .Get method call
			result = number;
		}
		return result;
	}
}
```

The code flow is now optimal, and there is nothing more to be done.

Except perhaps an int.MIN check, but… this would be overkill and not worth it!

An optimisation should be a balance between time spent and performance gained.

Something 50% optimal but working will always be better than something 98% optimal but unreleased!

ⓘ

For further performance advices, see [Scripting Performance](/wiki/Arma_Reforger:Scripting_Performance "Arma Reforger:Scripting Performance").

## Get the Average Value

### Method Setup

In this section the GetMinValue method code above will be reused.

```enforce
class TAG_TutorialExample
{
	int GetAverageValue(notnull array<int> numbers)
	{
	}
}
```

But an average is most likely a **floating point** number, so float will be used here.

```enforce
class TAG_TutorialExample
{
	float GetAverageValue(notnull array<int> numbers)
	{
	}
}
```

### Code Setup

#### Conception

* have a "sum" variable set to zero
* read each element of the list
* add each element to the sum variable
* when the end of the list is reached, return the sum divided by the array size

**But!**

What if the array has no elements? "sum" would remain a valid value (zero), but then the division could be by zero, which is a forbidden operation in computer science.

⚠

Every time a division is used, the reflex must be to check if the divisor (the right-hand division element) value can be zero, as this is a crash reason.

* **if the list is empty, return 0**
* have a "sum" variable set to zero
* read each element of the list
  + add each element to the sum variable
* when the end of the list is reached, return the sum divided by the array size

Code writing can be started.

### Writing

Here is the code translation:

```enforce
class TAG_TutorialExample
{
	float GetAverageValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int sum = 0;
		foreach (int number : numbers)
		{
			sum += number;
		}
		return sum / numbers.Count(); // numbers.Count() cannot be zero here as it is checked line 5
	}
}
```

### Testing

For demonstration purpose, both methods will be setup inside the TutorialExample class:

```enforce
class TAG_TutorialExample
{
	int GetMinValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int result = numbers[0];
		foreach (int i, int number : numbers)
		{
			if (i > 0 && number < result)
			result = number;
		}
		return result;
	}
	float GetAverageValue(notnull array<int> numbers)
	{
		if (numbers.IsEmpty())
		return 0;
		int sum = 0;
		foreach (int number : numbers)
		{
			sum += number;
		}
		return sum / numbers.Count();
	}
}
```

## Debug(Remote) Console

The following code can be used in the Debug(Remote) Console to try and see the result of previous methods - be sure to set the Console's running environment to "Game":

```enforce
array<int> values = { 18, 10, 5, 4, 5, 10, 33, 42, 666 }; // setup any wanted integer value here
TutorialExample tutorialExample = new TutorialExample();
int minValue = tutorialExample.GetMinValue(values);
float avgValue = tutorialExample.GetAverageValue(values);
PrintFormat("Array %1: min value is %2, average is %3", values, minValue, avgValue);
```

ⓘ

Note that for the debug code to run, it is important to have the Console set to "Game" and either:

* have the game running in World Editor and press "Run"
* press "Run" then compile (if no World Editor is running)

The code logs to the Output window something like this:

```
Array 0x000001C150656B48 {18,10,5,4,5,10,33,42,666}: min value is 4, average is 88.1111
```
