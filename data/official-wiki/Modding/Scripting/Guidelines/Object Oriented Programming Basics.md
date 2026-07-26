# [Object Oriented Programming Basics](https://community.bistudio.com/wiki/Arma_Reforger:Object_Oriented_Programming_Basics)

## Class vs Object

A class is a definition of an object; it can be seen as the blueprint for an object's creation. An object is an ***instance*** of a class, as in an entity created following the class' specifics.

An object can hold values, known as **member variables**, and functions, known as **methods**.

A class can have variables and methods too, known as **static** variables and methods.

## Member Variable

A member variable is a variable scoped to that object instance.
It is usually protected but can be private (see [Visibility](#Visibility)); a public member is usually a bad practice - it is best to use [Getters and Setters](#Getter_and_Setter).

See [Scripting: Values](/wiki/Arma_Reforger:Scripting:_Values "Arma Reforger:Scripting: Values") for naming prefixes.

```enforce
class MyClass
{
	protected int m_iValue = 100;
	void MyClass(int value)
	{
		m_iValue = value;
	}
}
```

## Method

A method is an object's function or a class' (static) function - it can be seen as a "member function".

A method has a **signature**, i.e a return type, a name and a parameters list - e.g:

```enforce
int GetNumber(bool canBeNegative)
```

Two methods can be named identically as long as they differ by their parameters.

```enforce
class MyClass
{
	void MyMethod()
	{
		Print("MyMethod without arguments");
	}
	int MyMethod() // error: only the return type differs
	{
		return 42;
	}
	void MyMethod(int a)
	{
		Print("MyMethod with 1 int argument");
	}
	void MyMethod(int a, int b = 0) // error: "identical" to the above method due to the optional parameter
	{ // both can be called MyMethod(42)
		Print("MyMethod with 1 or 2 int argument(s)");
	}
	void MyMethod(string a)
	{
		Print("MyMethod with 1 string argument");
	}
	void MyMethod(bool a, string b)
	{
		Print("MyMethod with 1 bool, 1 string argument");
	}
	static int MyStaticMethod()
	{
		return 33;
	}
}
MyClass instance = new MyClass(); // creates an instance of MyClass
int result = instance.MyMethod(); // returns 42
instance.MyMethod(10); // prints "MyMethod with 1 int argument"
instance.MyMethod("hello"); // prints "MyMethod with 1 string argument"
instance.MyMethod(false, "hey"); // prints "MyMethod with 1 bool, 1 string argument"
delete instance; // deletes instance - instance variable value is now null
int staticResult = MyClass.MyStaticMethod(); // without instance usage, staticResult is 33
```

### Getter and Setter

A Getter describes a method that **gets** the value of a property;

A Setter describes a method that **sets** it.

```enforce
class MyClass
{
	int Health = 100; // bad - public access
}
class MyClass
{
	protected int m_iHealth; // good - getter and setter allow for evolution
	int GetHealth() // health getter
	{
		return m_iHealth;
	}
	void SetHealth(int health) // health setter
	{
		m_iHealth = health;
	}
	void MyClass() // constructor method
	{
		m_iHealth = 100;
	}
}
// because health could be implemented another way - and if it is, we do not want to have to go through old code to change calls
class MyClass
{
	protected int m_iHeadHealth;
	protected int m_iBodyHealth;
	void MyClass() // constructor method
	{
		m_iHeadHealth = 100;
		m_iBodyHealth = 100;
	}
	int GetHealth() // health getter
	{
		return m_iHeadHealth * 0.75 + m_iBodyHealth * 0.25; // head damage counts thrice
	}
	void SetHealth(int health) // health setter
	{
		if (health > 100 || health < 0)
		{
			return;
		}
		m_iHeadHealth = health * 0.75;
		m_iBodyHealth = health * 0.25;
	}
}
```

### Constructor

A constructor method is a specific one: it is a method that is automatically called on object instanciation, and can be with or without arguments.  
There can be zero to one constructor.

A constructor is declared as a method having the same name as its class.

ⓘ

The parent class constructor (see the [Inheritance](#Inheritance) chapter below) is automatically called.
An inheriting class can only **add** new arguments to those accepted by the base class constructor.

```enforce
class MyClass
{
	int m_iValue;
	void MyClass()
	{
		PrintFormat("Instance created");
		m_iValue = 50;
	}
}
MyClass instance = new MyClass(); // instance.m_iValue = 50
```

```enforce
class MyClass
{
	int m_iValue;
	void MyClass(int value)
	{
		PrintFormat("Instance created with value %1", m_iValue);
		m_iValue = value;
	}
}
MyClass instance = new MyClass(42); // prints "Instance created with value 42"
MyClass instance = new MyClass(); // error: argument is missing
```

### Destructor

A destructor method is a specific one: it is a method that is automatically called on object destruction, and exists only without arguments.  
There can be zero to one destructor.

A destructor is declared as a method having the same name as its class, starting with a tilde ~.

```enforce
class MyClass
{
	void ~MyClass()
	{
		Print("I am being destroyed!");
	}
}
MyClass instance = new MyClass();
delete instance; // prints "I am being destroyed!";
```

## Visibility

Visibility is the accessibility of an object's or class' method/variable from the "outside" of that object/class.

### public

This is the default visibility - the member is accessible from inside as well as outside the object. As the default visibility, it does not need a keyword.

```enforce
class ParentClass
{
	int Health = 100; // while a bad practice, a public member is PascalCase-named
}
class ChildClass : ParentClass
{
	void ChildClass()
	{
		Health = 200; // stronger child
	}
}
ParentClass parent = new ParentClass();
parent.Health = 10; // OK
ChildClass child = new ChildClass();
child.Health = 10; // OK
```

### protected

This is a "hierarchy" visibility - the member is accessible from the object and objects of inheriting classes.
It uses the protected keyword.

⚠

It is important to keep visibility to protected for modding purpose - an inherited class can then expand or reuse these methods.

```enforce
class ParentClass
{
	protected int m_iHealth = 100;
}
class ChildClass : ParentClass
{
	void SetHealth(int health)
	{
		m_iHealth = health;
	}
}
ParentClass parent = new ParentClass();
parent.m_iHealth = 10; // error: cannot access from the outside
parent.SetHealth(10); // error: the SetHealth method is a member of ChildClass, not ParentClass
ChildClass child = new ChildClass();
child.m_iHealth = 10; // error: cannot access from the outside
child.SetHealth(10); // OK - the child can internally edit the inherited member
```

### private

This is the strictest visibility - only the object can access this member.
This is useful to e.g cut code in smaller methods and not clutter the list of available methods on this object from the outside.
It uses the private keyword.

```enforce
class ParentClass
{
	private int m_iHealth = 100;
}
class ChildClass : ParentClass
{
	void SetHealth(int health)
	{
		m_iHealth = health; // error: ChildClass does not know m_iHealth
	}
}
ParentClass parent = new ParentClass();
parent.m_iHealth = 10; // error: cannot access from the outside
parent.SetHealth(10); // error: the SetHealth method is a member of ChildClass, not ParentClass
```

## Inheritance

Inheritance is the transmission of parent properties to a child class. Class inheritance is written with :. A class can only inherit from **one** class.

```enforce
class ParentClass
{
	void ParentMethod()
	{
		Print("Parent Method");
	}
}
class ChildClass : ParentClass
{
	void ChildMethod()
	{
		Print("Child Method");
	}
}
ParentClass parent = new ParentClass();
parent.ParentMethod(); // outputs "Parent Method"
parent.ChildMethod(); // error: ChildMethod is not a member of ParentClass, only ChildClass
ChildClass child = new ChildClass();
child.ParentMethod(); // outputs "Parent Method"
child.ChildMethod(); // outputs "Child Method"
```

### override

A **non-private** (protected or public) inherited method can be overridden thanks to the override keyword:

```enforce
class ParentClass
{
	void TheMethod()
	{
		Print("Parent Method");
	}
}
class ChildClass : ParentClass
{
	override void TheMethod()
	{
		Print("Child Method");
	}
}
ParentClass parent = new ParentClass();
parent.TheMethod(); // outputs "Parent Method"
ChildClass child = new ChildClass();
child.TheMethod(); // outputs "Child Method"
```

⚠

An overridden method must have the **exact same signature** as the overridden method - up to the parameter names.

```enforce
int MyMethod(int value1, int value2);
// overrides
override int MyMethod(int value1, int value2); // works
override int MyMethod(int value1, string value2); // does not work - parameter types mismatch
override int MyMethod(int valueA, int valueB); // does not work - parameter names mismatch
```

### super

An inherited object can call its parent's **non-private** (protected or public) method:

```enforce
class ParentClass
{
	void MethodA()
	{
		Print("Parent MethodA");
	}
}
class ChildClass : ParentClass
{
	void MethodB()
	{
		super.MethodA();
		Print("Child MethodB");
	}
}
ParentClass parent = new ParentClass();
parent.MethodA(); // outputs "Parent MethodA"
ChildClass child = new ChildClass();
child.MethodB(); // outputs "Parent MethodA" then "Child MethodB"
```

It can even call an overridden one:

```enforce
class ParentClass
{
	void TheMethod()
	{
		Print("Parent Method");
	}
}
class ChildClass : ParentClass
{
	override void TheMethod()
	{
		super.TheMethod();
		Print("Child Method");
	}
}
ParentClass parent = new ParentClass();
parent.TheMethod(); // outputs "Parent Method"
ChildClass child = new ChildClass();
child.TheMethod(); // outputs "Parent Method" then "Child Method"
```

## See Also

* [Object Oriented Programming Advanced Usage](/wiki/Arma_Reforger:Object_Oriented_Programming_Advanced_Usage "Arma Reforger:Object Oriented Programming Advanced Usage")
