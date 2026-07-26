# [Create an Entity](https://community.bistudio.com/wiki/Arma_Reforger:Create_an_Entity)

A [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") entity is a scripted entity that can be placed from the World Editor's [**Create** tab](/wiki/Arma_Reforger:World_Editor#Create "Arma Reforger:World Editor").

In this example, we will create an Entity that once placed in the world, will print the player's position with a certain frequency.

## Declaration

### Entity

Create a new file and name it as your entity - here, we will go with [TAG\_](/wiki/Scripting_Tags "Scripting Tags")PrintPlayerPositionEntity so the file should be [TAG\_](/wiki/Scripting_Tags "Scripting Tags")PrintPlayerPositionEntity.c.

ⓘ

By convention, all Entity classnames must end with the Entity suffix, here [TAG\_](/wiki/Scripting_Tags "Scripting Tags")PrintPlayerPosition**Entity**.

⚠

An entity script file **must** be created in the **Game** module (scripts/Game), otherwise it will not be listed in the Entities list!

```enforce
class TAG_PrintPlayerPositionEntity : GenericEntity
{
}
```

### Entity Class

An Entity requires an Entity Class declaration. This allows it to be visible in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor").
The name must be **exactly** the Entity name suffixed by Class, here TAG\_PrintPlayerPosition*Entity***Class**.
An Entity Class is usually placed just above the Entity definition as such:

```enforce
[EntityEditorProps(category: "Tutorial/Entities", description: "Prints player's position regularly")]
class TAG_PrintPlayerPositionEntityClass : GenericEntityClass
{
}
class TAG_PrintPlayerPositionEntity : GenericEntity
{
}
```

The class is decorated using [EntityEditorProps](enfusion://ScriptEditor/scripts/Core/generated/Attributes/EntityEditorProps.c;12); the category is where the Entity will be found in World Editor's **Create** tab - see [below](#EntityEditorProps).

#### EntityEditorProps

category
:   the "Create" tab's category in which the Entity can be found

description
:   **unused** (for now)

color
:   the bounding box's unselected line colour - useful only when visible is set to true

visible
:   have the bounding box always visible - drawn in color

insertable

configRoot
:   **unused**

icon
:   **unused**: direct path to a png file, e.g WBData/EntityEditorProps/entityEditor.png

style
:   can be "none", "box", "sphere", "cylinder", "capsule", "pyramid", "diamond"

sizeMin
:   bounding box's lower dimensions

sizeMax
:   bounding box's higher dimensions

color2
:   the bounding box's surface colour

dynamicBox
:   enables the entity visualiser using custom dimensions (provided by c\_WB\_GetBoundBox())

## Filling

The Entity is now visible in World Editor, the next step is to make it do something.

### Add Code

Let's use the IEntity's constructor to set flags and call code.

```enforce
class TAG_PrintPlayerPositionEntity : GenericEntity
{
	protected float m_fWaitingTime = float.INFINITY; // trigger Print on start
	protected int m_iCycleDuration = 10; // in seconds
	//------------------------------------------------------------------------------------------------
	protected void PrintPlayerPosition()
	{
		PlayerController playerController = GetGame().GetPlayerController();
		if (!playerController)
		return;
		IEntity player = playerController.GetControlledEntity();
		if (!player)
		{
			Print("Player entity position: no player", LogLevel.NORMAL);
			return;
		}
		Print("Player entity position: " + player.GetOrigin(), LogLevel.NORMAL);
	}
	//------------------------------------------------------------------------------------------------
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		m_fWaitingTime += timeSlice;
		if (m_fWaitingTime < m_iCycleDuration)
		return;
		m_fWaitingTime = 0;
		PrintPlayerPosition();
	}
	//------------------------------------------------------------------------------------------------
	void TAG_PrintPlayerPositionEntity(IEntitySource src, IEntity parent)
	{
		SetEventMask(EntityEvent.FRAME);
	}
}
```

### Make It Unique

Let's assume we do not want the Print to be displayed multiple times in the case someone placed multiple Entities in the world.

ⓘ

A singleton is an entity that can only be instanciated once. See [Singleton pattern](https://en.wikipedia.org/wiki/Singleton_pattern).

For that we will use the [static](/wiki/Arma_Reforger:Scripting:_Keywords#static_2 "Arma Reforger:Scripting: Keywords") keyword to keep a single reference:

```enforce
class TAG_PrintPlayerPositionEntity : GenericEntity
{
	// other properties
	protected static TAG_PrintPlayerPositionEntity s_Instance;
	// other methods
	//------------------------------------------------------------------------------------------------
	void TAG_PrintPlayerPositionEntity(IEntitySource src, IEntity parent)
	{
		if (s_Instance)
		{
			Print("Only one instance of TAG_PrintPlayerPositionEntity is allowed in the world!", LogLevel.WARNING);
			delete this;
			return;
		}
		s_Instance = this;
		// rest of the init code
	}
}
```

### Add Properties

Now, we can declare properties with the Attribute in order to be able to adjust some settings from the World Editor interface. The following code only contains the added attributes:

```enforce
class TAG_PrintPlayerPositionEntity : GenericEntity
{
	[Attribute(defvalue: "1", desc: "Print player position")]
	protected bool m_bPrintPlayerPosition;
	[Attribute(defvalue: "1", desc: "Print a message when player is null")]
	protected bool m_bPrintWhenPlayerIsNull;
	[Attribute(defvalue: "1", uiwidget: UIWidgets.Slider, desc: "Print cycle period (in seconds)", params: "1 30 1")]
	protected int m_iCycleDuration;
}
```

The following code contains code with the implemented attributes:

```enforce
class TAG_PrintPlayerPositionEntity : GenericEntity
{
	[Attribute(defvalue: "1", desc: "Print player position")]
	protected bool m_bPrintPlayerPosition;
	[Attribute(defvalue: "1", desc: "Print a message when player is null")]
	protected bool m_bPrintWhenPlayerIsNull;
	[Attribute(defvalue: "1", uiwidget: UIWidgets.Slider, desc: "Print cycle period (in seconds)", params: "1 30 1")]
	protected int m_iCycleDuration;
	// other properties
	//------------------------------------------------------------------------------------------------
	protected void PrintPlayerPosition()
	{
		PlayerController playerController = GetGame().GetPlayerController();
		if (!playerController)
		return;
		IEntity player = playerController.GetControlledEntity();
		if (!player)
		{
			if (m_bPrintWhenPlayerIsNull)
			Print("Player entity position: no player", LogLevel.NORMAL);
			return;
		}
		Print("Player entity position: " + player.GetOrigin(), LogLevel.NORMAL);
	}
	// other methods
	//------------------------------------------------------------------------------------------------
	void TAG_PrintPlayerPositionEntity(IEntitySource src, IEntity parent)
	{
		if (!m_bPrintPlayerPosition)
		{
			delete this;
			return;
		}
		// rest of the init code
	}
}
```

Now all there is to do is to place one TAG\_PrintPlayerPositionEntity entity in the world and see the player's position printed in logs!

ⓘ

In order for the entity to appear in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor"), scripts must be compiled and reloaded *via* [**Compile & Reload Scripts**](/wiki/Arma_Reforger:Script_Editor#Menu_Bar "Arma Reforger:Script Editor") function in **Build** tab ( `⇧ Shift` + `F7` ).

## Final Code

The final file content can be found here:
Show File Content

```enforce
[EntityEditorProps(category: "Tutorial/Entities", description: "Prints player's position regularly", color: "0 255 0 0")]
class TAG_PrintPlayerPositionEntityClass : GenericEntityClass
{
}
class TAG_PrintPlayerPositionEntity : GenericEntity
{
	[Attribute(defvalue: "1", desc: "Print player position")]
	protected bool m_bPrintPlayerPosition;
	[Attribute(defvalue: "1", desc: "Print a message when player is null")]
	protected bool m_bPrintWhenPlayerIsNull;
	[Attribute(defvalue: "1", uiwidget: UIWidgets.Slider, desc: "Print cycle period (in seconds)", params: "1 30 1")]
	protected int m_iCycleDuration;
	protected float m_fWaitingTime = float.INFINITY; // trigger Print on start
	protected static TAG_PrintPlayerPositionEntity s_Instance;
	//------------------------------------------------------------------------------------------------
	protected void PrintPlayerPosition()
	{
		PlayerController playerController = GetGame().GetPlayerController();
		if (!playerController)
		return;
		IEntity player = playerController.GetControlledEntity();
		if (!player)
		{
			if (m_bPrintWhenPlayerIsNull)
			Print("Player entity position: no player", LogLevel.NORMAL);
			return;
		}
		Print("Player entity position: " + player.GetOrigin(), LogLevel.NORMAL);
	}
	//------------------------------------------------------------------------------------------------
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		m_fWaitingTime += timeSlice;
		if (m_fWaitingTime < m_iCycleDuration)
		return;
		m_fWaitingTime = 0;
		PrintPlayerPosition();
	}
	//------------------------------------------------------------------------------------------------
	void TAG_PrintPlayerPositionEntity(IEntitySource src, IEntity parent)
	{
		if (s_Instance)
		{
			Print("Only one instance of TAG_PrintPlayerPositionEntity is allowed in the world!", LogLevel.WARNING);
			delete this;
			return;
		}
		if (!m_bPrintPlayerPosition)
		{
			delete this;
			return;
		}
		SetEventMask(EntityEvent.FRAME);
		s_Instance = this;
	}
}
```

[↑ Back to spoiler's top](#bikisp6a631b7f46be9)
