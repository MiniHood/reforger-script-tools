# [Behavior Editor](https://community.bistudio.com/wiki/Arma_Reforger:Behavior_Editor)

## Editor Basics

[![armareforger behavioreditor-interface.jpg](/wikidata/images/thumb/4/42/armareforger_behavioreditor-interface.jpg/600px-armareforger_behavioreditor-interface.jpg)](/wiki/File:armareforger_behavioreditor-interface.jpg)

| N° | Description |
| --- | --- |
| 1 | Node area |
| 2 | Nodes palette |
| 3 | Node parameters |
| 4 | Variables |
| 5 | Debug |
| 6 | Console |

### Shortcuts

| Shortcut | Description |
| --- | --- |
| `F9` | Create/delete breakpoint |
| `Ctrl` + `O` | Open |
| `Ctrl` + `S` | Save |
| `Ctrl` + `⇧ Shift` + `S` | Save As |
| `Ctrl` + `C` | Copy |
| `Ctrl` + `X` | Cut |
| `Ctrl` + `V` | Paste |
| `Del` | Delete node, variable or connection |

### Controls

| Shortcut | Description |
| --- | --- |
| Move around behavior tree | Hold Left Mouse Button in free space |
| Connect nodes | Press Left Mouse Button and drag from a black bar in the lower or upper part of the node, connect it to the black bar in another node. Connecting variables is analogous but with pins. |
| Select multiple nodes | `⇧ Shift` + Left Mouse Button drag. `Ctrl` + Left Mouse Button on different nodes. |
| Zoom in/out | Mousewheel up/down. |
| Disconnect node | Select connection and press delete. |
| Organise nodes | When multiple nodes are selected, they can be aligned or grouped. |
| Open scripted node script | Double click on a scripted node |

## Behavior Tree

Editing behavior trees have a similar basis as FSM. Nodes are being executed from the Root node according to the rules of connection. The biggest difference is that BT is not limited to one state at one time, but can run on multiple branches at one time. There are conditions and effects in nodes, but also more advanced flow control mechanisms, that make BTs powerful, but also more complex. Each node returns Success, Failure or Running to its parent node, which can decide the following steps according to the information.

Behavior trees take a slightly different approach from the usual script flow, but similarities can be found:
an AND logic can be found in the Sequence node (checking all direct sub-nodes return Success) and an OR logic in the Selector node (checking at least one sub-node returns a Success, sub-nodes being numbered and evaluated from left to right).
Behavior trees allow writing complex behaviors with multiple states which cannot be defined by simpler methods, like FSM.

### Flow Control

* **Root** – basic node which serves as entry point to which other nodes are directed.
* **Sequence** – all children nodes will be executed in sequence unless children returns Failure. Children nodes are assigned numbers to show the execution order.
* **Selector** – opposite of sequence: After one of the executed children returns Success, other children will not be executed. Order of execution is marked with numbers above the node.
* **Parallel** – run all children nodes simultaneously.
* **Repeater** – run child multiple times.
* **Run BT** – launch behavior tree.

### Node

A node is a "step" in the BT.

#### Events

Events from [AITaskScripted](enfusion://ScriptEditor/scripts/Game/generated/AI/AITaskScripted.c;12) class:

* **OnInit** – runs once when the tree is loaded. Note that the tree can be reloaded after its entire completion
* **OnEnter** – runs when the node is entered for the first time - **before** EOnTaskSimulate.
* **EOnTaskSimulate** – runs when the node is evaluated
* **OnAbort** – runs when the node gets aborted - see [Abort Type](#Abort_Type)

### Decorator

Decorators are nodes with execution conditions, and depending on the evaluation's result, they may execute their node and branches.
There are several types of conditions decorators are testing, and different flow effects decorators can have.

Example tests and effects on flow:

* Test validity of given entity (does it exist, is it alive?)
* Test scripted condition by creating your own scripted decorators
* Interrupt the flow in the other parts of the BT (with parameter [AbortType](#Abort_Type)) in case the evaluation result is false

  ⓘ

  A decorator evaluating to false can this way force a result (Success or Failure only) before even reaching the node's evaluation.

More complex operations can be done by decorators. For example executing a child node but returning an arbitrary value (either Success or Failure) so the rest of the tree reacts on it in different ways.

⚠

Unless **AbortType** is defined, decorators are evaluated only **once** pre tree execution.

### Tasks

Nodes that are executing functions inside Behavior Trees. Any action that AI should provide is task: Move to position, run, play surrender gesture. Even waiting is task. There are 2 types of tasks:

* **Immediate** – tasks that are done immediately and continue with execution of BT.
* **Running** – tasks, like move, that take time to complete. At the point in which character is running, the node's branch waits for the node to be either Success or Failure (not evaluating nodes to the right) - then the next time BT is called, will continue from this point. If there is no flow running in parallel, they block execution of anything else in BT until task is done.

  ⓘ

  A "Running" task must see its CanReturnRunning method overridden in script to return true.

Tasks cannot have children, and usually they are organised in sequences to complete more complex operations. For example: Stop, Raise Weapon, Aim, Fire.

Tasks can perform very simple operations, like assigning value to variable, to more complex functionality, like Land (with aircraft).

## Behavior Tree Flow

As explained at the beginning, behavior trees are executed from root folder, and through flow control and decorators, pass through the graph.
BTs can be called from engine, script or other BT, so it's possible to consider it as a function, which can call other functions, and keep the graph relatively clean, not having all AI functionality in one gigantic graph.

### Returning Values to Parent Nodes

Each executed node will return a value to parent node, which can react to it, in two ways:

* Flow control nodes like selector and sequence will either execute another child page, or return value up to its parent node.
* There are 3 return value types: Success, Failure or Running
  + **Success** means the check is successful
  + **Failure** means the check did not match the wanted conditions.

    ⓘ

    "Failure" may sound very negative but it is only a "false" result (e.g "does this unit have a weapon"). It is not an error/exception in the tree.
  + **Running** means the calculation is still happening - an ongoing behavior like movement, which will prevent execution of the rest of the BT (unless using parallel node).

    ⚠

    Running state will return through the BT up to the Root node, so the state of the whole behavior tree becomes "Running".

### Force Node Result

Force node's output to the set value, whatever the actual result is - e.g a sub-branch resulting in Failure will still return Success if a parent node says so; said node will still run all sub-nodes.

### Abort Type

When you need to interrupt running behavior or prevent execution of other nodes, you can use the decorators' **AbortType** parameter.
If decorator's evaluation changes and Abort is used, if will stop BT's execution depending on the Abort Type.

**Types of Abort:**

* Abort Children Branch – the node's decorator is evaluated every time its sub-tree is executed. If the decorator's condition becomes false, it aborts all the node's children (calling their OnAbort event) and returns Failure.
* Abort Parent Node Further Children – the node's decorator is evaluated every time the parent's sub-tree is evaluated and aborts its siblings to its right as soon as the decorator's evaluation becomes true.
* Abort Children Branch And Parent Further Children – the node's decorator is evaluated every time the parent's sub-tree is evaluated. If this decorator's condition becomes true and its sub-tree is not running, it kills all the parent's children to the right of it and runs. If it becomes false while its sub-tree is running, it aborts its own sub-tree and lets other parent's children to the right to run.

ⓘ

If no **Abort Type** is set, the decorator's evaluation is tested only **once** per tree execution.

## In-Game Setup

You need a world with character prefab, which is set up with all important AI components. If you want to prepare AI in your own world, follow these steps:

1. To have character with working AI, it has to have at least AI Control Component, and other AI components enabling AI functionality: AI Character Aiming Component, AI Character Movement Component, Perception Component. Working AI character prefab is ChimeraCharacterAI.
2. This character has all required AI components. AIControlComponent creates agent, which is separate entity ordering actions to character.
3. In AIControlComponent:
   * set OverrideAIBehaviorData with your Behavior tree (will be created below)
   * tick "Enable AI" (activates AIagent)
4. Run the game with BT editor opened.
5. There is list of running BTs in right panel. Click on BT of your soldier and you will see his behavior tree and states, in which it is now (Note: **No modifications of this debug tree will be saved, you have to modify the original BT**).

## Basic Behavior Trees

### Simple Action

Most basic behavior tree possible is to attach Task to Root node. This this will make object on which BT is called crouch every time BT is called.

### Follow Player

Basic version of follow player is to first select the entity to follow, and then executing the move command.

⚠

There is one complication: the Get Controlled Entity output cannot be directly connected to Move to Entity input. It has to be stored in variable and then loaded from variable.

1. Create variable with + symbol at variables tab on left panel
2. Select type (GenericEntity or IEntity) and name
3. Drag the variable into graph, it will be represented as node
4. Connect its input and output with correct nodes.

Now character on which BT is running, will follow player. See parameters of the node to adjust specifics, like obstacle avoidance, precision, etc.

This BT has one minor problem that can be optimised: When the unit is at player, every time that BT will tick, it will always perform left branch for the tree: Getting controlled entity and overwriting the variable.

This is optimised in next graph, which runs as follows:

* Entity decorator tests validity of input, and return negative result.
* Negative test + negative result will return Success, so node connected under it (Get Controlled Entity) will be executed.
* It will set player variable,.
* Next time decorator Entity will be called, it will return Failure, and its branch will not be called.

### Guard Player

This behavior tree is switching between 2 states: If unit is more than 10 meters from player, move to him. If unit is closer, crouch and look around with weapon raised. The flow of the tree goes in this way:

1. Fist select most left branch, because player entity variable is not assigned.
2. Variable player is assigned, and BT exited.
3. Second run, test for player variable validity returns negative, and distance check of unit from player is performed.
4. Let's say player is further than 10 meters, so test returns Success, and sequence on the right is performed: Stand up, Lower weapon (performed with Raise weapon node, and with uncheck parameter raise), move to player, with tolerance of 4 meters.
5. Until movement is completed, behavior tree will not check any other condition, like distance decorator in the middle.
6. When unit gets close to player, distance decorator will return Success, and its children branch is executed: Crouch, Raise Weapon, and create a randomised position, in which the unit will look (this is how we get random direction). Orient node will mean unit will look into specified direction, and idle for short time (0.5 second).
7. Once this sequence is completed, tree will be executed again. If player's distance is still below 10 meters, children sequence will be executed again, if not, return to point 4.

### Guard Player and Shoot Enemies

To create an Engage functionality, there should be a third branch, handling the engagement. This is most primitive version of the engagement node, which searches for predefined enemy (searched by variable):

To be able to recognise enemy, we can script our own Task, called Find Enemy. It will find characters nearby except player and guard, and mark them as target. Current target is selected automatically.

**Scripted Task:** Find Target

* If a valid target is found, then BT will continue with the branch, which includes aiming and shooting at the enemy.

This is how task scripted can look. For clarity, the calculation of target is not included.

ⓘ

Scripted node gives great possibilities for Behavior Trees, but it comes at a performance cost.  

Native behavior trees, which are not running any scripted nodes, can much faster thanks to parallelization.

Any behavior tree with scripted nodes is running in a single thread and is therefore much slower.

Scripted Task code

```enforce
class AITaskFindTarget : AITaskScripted
{
	// Member variables
	// Exposing variable m_Radius as attribute will make it visible as parameter inside BT Editor.
	// Note: Functionality not completed now
	[Attribute("10", "editbox", "Radius")]
	float m_Radius;
	IEntity m_Player;
	// Constructor can be used for initializing default values,
	// but for any other operation, use EOnTaskSimulate
	void Init()
	{
		m_Radius = 100;
	}
	// Make scripted node visible or hidden in nodes palette
	bool VisibleInPalette()
	{
		return true;
	}
	// Sets up input variables, as array of strings
	override TStringArray GetVariablesIn()
	{
		// Make sure variables are initialised only once
		if (!m_aVariablesIn)
		{
			m_aVariablesIn = new TStringArray;
			m_aVariablesIn.Insert("Distance");
		}
		return m_aVariablesIn;
	}
	// Sets up output variables
	override TStringArray GetVariablesOut()
	{
		if (!m_aVariablesOut)
		{
			m_aVariablesOut = new TStringArray;
			m_aVariablesOut.Insert("Target");
		}
		return m_aVariablesOut;
	}
	// Main method to perform all operations handled by task
	// This is just example, to see important parts, without any actual calculations
	ENodeResult EOnTaskSimulate(IEntity owner, float dt)
	{
		// This is engine method to read variables from Behavior tree
		GetVariableIn("Distance",m_Radius);
		IEntity target = GetValidTarget(m_Radius);
		// This is method to output variable into Behavior Tree
		SetVariableOut("Target", target);
		if (target)
		return ENodeResult.Fail;
		else
		return ENodeResult.Success;
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63c5d6d5974)

#### Scripted Decorator

Scripted Decorator is basically the same, it must be inherited from DecoratorScripted, but it uses the same methods as Scripted Task.

Scripted Decorator code

```enforce
class ExampleScriptedDecorator : DecoratorScripted
{
	[Attribute("10", "editbox", "Radius")]
	float m_Distance;
	bool VisibleInPalette()
	{
		return true;
	}
	override TStringArray GetVariablesIn()
	{
		if (!m_aVariablesIn)
		{
			m_aVariablesIn = new TStringArray();
			m_aVariablesIn.Insert("Distance");
		}
	}
	override TStringArray GetVariablesOut()
	{
		if (!m_aVariablesOut)
		{
			m_aVariablesOut = new TStringArray();
			m_aVariablesOut.Insert("Target");
		}
		return m_aVariablesOut;
	}
	override TStringArray GetVariableType()
	{
		if (!m_aVariablesOut)
		{
			m_aVariablesOut = new TStringArray();
			m_aVariablesOut.Insert("Target");
		}
		return m_aVariablesOut;
	}
	// Instead of EOnTaskSimulate, this method has different return type parameters:
	override bool TestFunction(IEntity owner)
	{
		// You can check the type of variable you are handling:
		GetVariableIn("Distance", m_Radius);
		SetVariableOut("Target", null);
		return GetVariableType(true, "Distance") == "float";
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63c5d6d882b)

## Debugging

To check AI behavior in runtime, all AI entities are listed in the Debug section, which will open the root behavior tree of the unit. From there it's possible to traverse to any other connected behavior.

It's not possible to change any behavior during the runtime or variable, the functionality is read-only. You can read the states of nodes and what values are in variables.

You can put a breakpoint on any node, and the behavior tree will stop its execution. But this will not stop the game (as in the case of script debugger), and if there is any parallel node above breakpoint, other branches of it will keep executing.

Colors in debug:

* Green – node returned Success (variable is assigned)
* Red – node returned Fail (variable is unassigned)
* Blue – node returned Running
* Dark red – node is suspended by breakpoint

## Further Reading

* [Behavior Editor: Nodes](/wiki/Arma_Reforger:Behavior_Editor:_Nodes "Arma Reforger:Behavior Editor: Nodes")
* [Intro into Behavior Trees in Gamasutra](http://gamasutra.com/blogs/ChrisSimpson/20140717/221339/Behavior_trees_for_AI_How_they_work.php): Overview on Behavior Trees with useful examples
* [Artificial Intelligence for Games, Second Edition: The big book on game AI development](http://lecturer.ukdw.ac.id/~mahas/dossier/gameng_AIFG.pdf)
* [The Behavior Tree Starter Kit](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter06_The_Behavior_Tree_Starter_Kit.pdf) on <http://www.gameaipro.com/> by Alex J. Champandard and Philip Dunstan: The first section paints the big picture for behavior trees and explains how to build BTs and how to use them for making AI decisions. The second section dives into the implementation. Finally, it explains the principles of second-generation BTs, and the improvements they offer, e.g. about memory optimizations and event-driven implementations that scale up significantly better on modern hardware.
