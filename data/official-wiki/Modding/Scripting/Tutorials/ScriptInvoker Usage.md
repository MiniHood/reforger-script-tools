# [ScriptInvoker Usage](https://community.bistudio.com/wiki/Arma_Reforger:ScriptInvoker_Usage)

ⓘ

See [Event Handlers](/wiki/Arma_Reforger:Event_Handlers "Arma Reforger:Event Handlers") for [ScriptInvoker](enfusion://ScriptEditor/scripts/GameLib/tools.c;134) guidelines.

A [ScriptInvoker](enfusion://ScriptEditor/scripts/GameLib/tools.c;134) is an object that allows other objects to subscribe to the event it represents by registering a method.
Such event can provide arguments and it is important for the subscribing methods to fit the proper method signature.

The interest of using Events is that the subscribed method is be executed only *if* and *when* a specific event occurs (e.g a fired weapon, someone leaving a vehicle, etc) - no periodical check is done, saving CPU cycles and executing exactly on event.

## Usage

⚠

Using the [ScriptInvoker](enfusion://ScriptEditor/scripts/GameLib/tools.c;134) class directly is considered bad practice and should be avoided!

Show example

```enforce
class TAG_Dog
{
	protected ref ScriptInvokerVoid m_OnBark; // initialised on request - this allows saving RAM as ScriptInvokers are not small objects
	// IN GENERAL, do NOT expose the ScriptInvoker variable directly (as anyone could delete it)
	// Arma Reforger code keeps Component events public for performance reason
	//! This method provides the ScriptInvoker for outsiders to subscribe to it
	ScriptInvokerVoid GetOnBark()
	{
		if (!m_OnBark)
		m_OnBark = new ScriptInvokerVoid(); // the invoker is only created on external request
		return m_OnBark;
	}
	void Bark()
	{
		if (m_OnBark) // the invoker can be null if nobody subscribed to it - no need to create it
		m_OnBark.Invoke();
		Print("Dog barked", LogLevel.DEBUG);
	}
}
class TAG_Neighbour
{
	protected TAG_Dog m_Dog; // no strong ref
	protected int m_iBarkCount;
	protected void OnDogBark()
	{
		if (m_iBarkCount < 1)
		Print("The dog barked!", LogLevel.NORMAL);
		else if (m_iBarkCount < 10)
		Print("Oh, it barked again!", LogLevel.NORMAL);
		else
		Print("MAKE IT STOP", LogLevel.WARNING);
		m_iBarkCount++;
	}
	//! Constructor
	void TAG_Neighbour(notnull TAG_Dog dog)
	{
		m_Dog = dog;
		m_Dog.GetOnBark().Insert(OnDogBark); // GetOnBark() creates the invoker
		// Insert() adds OnDogBark to the list of methods to be called
	}
	//! Destructor
	void ~TAG_Neighbour()
	{
		// do not forget to unsubscribe for good practice, clean code aaand to avoid potential memory leaks
		if (m_Dog)
		m_Dog.GetOnBark().Remove(OnDogBark); // removes OnDogBark from the list of methods to be called
	}
}
```

[↑ Back to spoiler's top](#bikisp6a629b2a99f37)

## Signature Declaration

The c[ScriptInvokerVoid](enfusion://ScriptEditor/scripts/Game/Helpers/SCR_ScriptInvokerHelper.c;9) class used above is the "default" one used for methods without arguments.
In order to provide arguments to the subscribed methods, a generic [ScriptInvokerBase](enfusion://ScriptEditor/scripts/GameLib/tools.c;117) type must be used, e.g c[ScriptInvokerBase](enfusion://ScriptEditor/scripts/GameLib/tools.c;117)<[func](enfusion://ScriptEditor/scripts/Core/proto/Types.c;143)>.

A method signature can be declared using typedef:

```enforce
void ScriptInvokerIntMethod(int i);
typedef func ScriptInvokerIntMethod;
typedef ScriptInvokerBase<ScriptInvokerIntMethod> ScriptInvokerInt;
```

And used like this:

```enforce
protected ref ScriptInvokerInt m_OnEventWithIntParameter = new ScriptInvokerInt();
```

Show full example

```enforce
class TAG_EventHolder
{
	protected int m_iScore;
	protected ref ScriptInvokerInt m_OnScoreChanged; // still created on request
	ScriptInvokerInt GetOnScoreChanged()
	{
		if (!m_OnScoreChanged)
		m_OnScoreChanged = new ScriptInvokerInt();
		return m_OnScoreChanged;
	}
	void AddScore(int points)
	{
		m_iScore += points;
		if (m_OnScoreChanged)
		m_OnScoreChanged.Invoke(m_iScore);
	}
}
class TAG_Subscriber
{
	protected ref TAG_EventHolder m_EventHolder;
	protected void WarnOfScoreChange(int newScore) // note the similarity with ScriptInvokerIntMethod's signature
	{
		Print("The score is now " + newScore + "!", LogLevel.NORMAL);
	}
	void Subscribe(notnull TAG_EventHolder eventHolder)
	{
		if (m_EventHolder)
		UnSubscribe();

		eventHolder.GetOnScoreChanged().Insert(WarnOfScoreChange); // if the method signature were not matching, the compiler would warn here
		m_EventHolder = eventHolder;
	}
	void UnSubscribe()
	{
		if (!m_EventHolder)
		return;
		m_EventHolder.GetOnScoreChanged().Remove(WarnOfScoreChange);
		m_EventHolder = null;
	}
	void ~TAG_Subscriber()
	{
		UnSubscribe();
	}
}
```

[↑ Back to spoiler's top](#bikisp6a629b2aa9e5b)

That's it! Remember that ScriptInvokers/Event Handlers are very powerful, but dangerous tools. Be sure not to fall into any trap or loop!
