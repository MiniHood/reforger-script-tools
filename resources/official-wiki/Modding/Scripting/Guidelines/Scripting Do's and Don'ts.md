# [Scripting: Do's and Don'ts](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Do%27s_and_Don%27ts)

### Don't

```enforce
class ExampleClass {
	int LProcLgh;
	int stringlength( string Value ){
		LProcLgh= Value.Length();
		Print ( "String length obtained: "+LProcLgh );
		return LProcLgh;
	}
}
```

### Do

```enforce
class ExampleClass
{
	protected int m_iLastProcessedLength;
	int GetStringLength(string value)
	{
		m_iLastProcessedLength = value.Length();
		Print("String length obtained: " + m_iLastProcessedLength);
		return m_iLastProcessedLength;
	}
	int GetLastProcessedLength()
	{
		return m_iLastProcessedLength;
	}
}
```

### Don't

```enforce
class ExampleClass
{
	protected int m_iLength;
	int GetStringLength(string name)
	{
		m_iLength = name.Length();
		Print("String length obtained: " + m_iLength);
		return m_iLength;
	}
}
```

### Do

```enforce
class ExampleClass
{
	// keeping the variable into method scope and away from the instance
	int GetStringLength(string name)
	{
		int length = name.Length();
		Print("String length obtained: " + length);
		return length;
	}
}
```

### Don't

```enforce
switch (value)
{
	case 0:
	{
		Print("0");
		break;
	}
	default:
	{
		Print("default");
		break;
	}
}
```

### Do

```enforce
switch (value)
{
	case 0:
	Print("0");
	break;
	default: // brackets are needed only if variables are declared in the 'case' code
	{ // otherwise scoping is superfluous (and very slightly impacts performance)
		string message = "default";
		Print(message);
		break;
	}
}
```

### Don't

```enforce
// this array only lists pointers but does not increase the reference count
array<ExampleClass> classArray = {};
ExampleClass newInstance;
for (int i; i < 10; i++)
{
	newInstance = new ExampleClass();
	classArray.Insert(newInstance);
	// newInstance will be deleted at the end of the scope
	// as there are no references to it
}
```

### Do

```enforce
// this array keeps a strong reference to its items
array<ref ExampleClass> classArray = {};
ExampleClass newInstance;
for (int i; i < 10; i++)
{
	newInstance = new ExampleClass();
	classArray.Insert(newInstance);
	// classArray keeps a strong reference to newInstance - it will not be cleared
}
```

### Don't

ⓘ

Here, both ParentClass and ChildClass have a strong reference to each other, keeping the reference count above zero - creating an "island of isolation" (see [Scripting: Automatic Reference Counting](/wiki/Arma_Reforger:Scripting:_Automatic_Reference_Counting "Arma Reforger:Scripting: Automatic Reference Counting") for more information).

```enforce
class MainClass
{
	ref SubClass m_SubClass;
	void MainClass()
	{
		m_SubClass = new SubClass(this);
	}
}
class SubClass
{
	ref MainClass m_Parent;
	void SubClass(MainClass parent)
	{
		m_Parent = parent;
	}
	void DoSomething()
	{
		Print(m_Parent);
	}
}
```

### Do

ⓘ

Here, the MainClass needs the SubClass (it creates it in its constructor meaning it needs it to work) but the subClass does not require MainClass - if the MainClass reference does not exist, it will simply not use it.

```enforce
class MainClass
{
	ref SubClass m_SubClass;
	void MainClass()
	{
		m_SubClass = new SubClass(this);
	}
}
class SubClass
{
	MainClass m_Parent; // ref removed
	void SubClass(MainClass parent)
	{
		m_Parent = parent;
	}
	void DoSomething()
	{
		if (!m_Parent) // null safety check
		return;
		Print(m_Parent);
	}
}
```
