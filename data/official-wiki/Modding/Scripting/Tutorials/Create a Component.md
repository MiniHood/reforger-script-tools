# [Create a Component](https://community.bistudio.com/wiki/Arma_Reforger:Create_a_Component)

A [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") Component is a code element that can be placed as a child (well, as a *component*) of an [entity](/wiki/Arma_Reforger:Create_an_Entity "Arma Reforger:Create an Entity") from the World Editor's **Add Component** button.

In this example, we will create a Component that teleports humans if they get too close.

## Declaration

### Component

Create a new file and name it as your component - here, we will go with TAG\_TeleportFieldComponent so the file should be TAG\_TeleportFieldComponent.c.

ⓘ

By convention, all Component classnames must end with the Component suffix, here TAG\_TeleportField**Component**.

⚠

A component script file **must** be created in the **Game** module (scripts/Game), otherwise it will not be listed in the Components list!

```enforce
class TAG_TeleportFieldComponent : ScriptComponent // ScriptComponent → GenericComponent
{
}
```

### Component Class

Like an Entity, a Component requires a Component Class declaration. This allows it to be visible in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor").
The name must be **exactly** the Component name suffixed by Class, here TAG\_TeleportFieldComponent**Class**.
A Component Class is usually placed just above the Component definition as such:

```enforce
[ComponentEditorProps(category: "Tutorial/Component", description: "TODO")]
class TAG_TeleportFieldComponentClass : ScriptComponentClass
{
}
class TAG_TeleportFieldComponent : ScriptComponent
{
}
```

The class is decorated using [ComponentEditorProps](enfusion://ScriptEditor/scripts/Core/generated/Attributes/ComponentEditorProps.c;12); the category is where the Component will be found using e.g the **Add Component** button - see [below](#ComponentEditorProps).

#### ComponentEditorProps

category
:   the "Create" tab's category in which the Component can be found

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
:   set the component's icon in World Editor's UI - direct path to a png file, e.g cicon: "WBData/ComponentEditorProps/componentEditor.png"

ⓘ

In order for the component to appear in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor"), scripts **must** be compiled and reloaded *via* `⇧ Shift` + `F7`.

## Filling

The Component is now visible in World Editor's "Add Component" UI, the next step is to make it do something.

### Add Code

Let's use the Component's cOnPostInit() method to call code.

```enforce
[ComponentEditorProps(category: "Tutorial/Component", description: "Teleport humans that are too close to the entity")]
class TAG_TeleportFieldComponentClass : ScriptComponentClass
{
}
class TAG_TeleportFieldComponent : ScriptComponent
{
	protected float m_fCheckDelay;
	protected ref array<IEntity> m_aNearbyCharacters;
	protected static const float CHECK_PERIOD = 0.25;
	protected static const float CHECK_RADIUS = 10;
	//------------------------------------------------------------------------------------------------
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		super.EOnFrame(owner, timeSlice);
		vector ownerPos = owner.GetOrigin();
		m_fCheckDelay -= timeSlice;
		if (m_fCheckDelay <= 0)
		{
			m_fCheckDelay = CHECK_PERIOD;
			m_aNearbyCharacters.Clear();
			owner.GetWorld().QueryEntitiesBySphere(ownerPos, CHECK_RADIUS, QueryEntitiesCallbackMethod, null, EQueryEntitiesFlags.DYNAMIC | EQueryEntitiesFlags.WITH_OBJECT);
			Print("There are " + m_aNearbyCharacters.Count() + " human entities around the teleporter.");
			foreach (IEntity character : m_aNearbyCharacters)
			{
				vector charPos = character.GetOrigin();
				vector vectorDir = vector.Direction(owner.GetOrigin(), charPos).Normalized();
				character.SetOrigin(charPos + 5 * vectorDir);
			}
		}
	}
	//------------------------------------------------------------------------------------------------
	// QueryEntitiesCallback type
	protected bool QueryEntitiesCallbackMethod(IEntity e)
	{
		if (!e || !ChimeraCharacter.Cast(e)) // only humans
		return false;
		m_aNearbyCharacters.Insert(e);
		return true;
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnPostInit(IEntity owner)
	{
		m_aNearbyCharacters = {};
		SetEventMask(owner, EntityEvent.FRAME);
	}
}
```

This code does multiple things:

* in cOnPostInit() the nearby characters array is initialised and the "on frame" event mask is set, allowing cEOnFrame to be executed every frame
* in cEOnFrame() we query entities by sphere (by straight line distance from point to point) and fill the nearby characters array through the cQueryEntitiesCallbackMethod() callback method
* still in cEOnFrame() we move all detected entities 5 metres back using in c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12).SetOrigin()

ⓘ

The cGetOwner() method as well as the c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12) owner (cEOnFrame's parameter), although not notnulled, never return null.
If they do, there is a **much** bigger problem than a null owner, given a component cannot exist without an entity.

### Add Properties

Now, we can declare properties with the Attribute decorator in order to be able to adjust some settings from the World Editor interface. The following code only contains the added attributes:

```enforce
[ComponentEditorProps(category: "Tutorial/Component", description: "Warn then teleport humans that are too close to the entity")]
class TAG_TeleportFieldComponentClass : ScriptComponentClass
{
}
class TAG_TeleportFieldComponent : ScriptComponent
{
	/*
	Teleportation
	*/
	[Attribute(defvalue: "10", desc: "Distance at which the field draws a line to its target to warn it about teleportation", category: "Teleportation")]
	protected float m_fWarningRadius;
	[Attribute(defvalue: "2", desc: "Distance at which the field triggers the teleportation", params: "0.25 10 0.25", category: "Teleportation")]
	protected float m_fTriggerRadius;
	[Attribute(defvalue: "10", desc: "Distance at which the teleportation places the unit from the teleporter", category: "Teleportation")]
	protected float m_fTeleportDistance;
	/*
	Line Drawing
	*/
	[Attribute(defvalue: "1 0.75 0 1", desc: "The line's colour", category: "Line Drawing")]
	protected ref Color m_LineColour;
	[Attribute(defvalue: "1", desc: "Whether or not the line must fade in/out with transparency based on distance", category: "Line Drawing")]
	protected bool m_bLineFadeInOut;
	[Attribute(defvalue: "0 1 0", desc: "The line offset from entities's origins", category: "Line Drawing")]
	protected vector m_vOffset;
	/*
	Performance
	*/
	[Attribute(defvalue: "0.25", desc: "Duration between proximity checks", category: "Performance")]
	protected float m_fCheckPeriod;
	// ...
}
```

These attributes's implementation can be a good exercise. As you may have noticed some additional attributes made their way to the code.

The goal is to draw a line from the object to the entity in order to warn them they *will* be teleported if they keep on getting closer.
You can try to figure out how to do it properly then take a peek at [Final Code](#Final_Code) to see one possible solution.

Now all there is to do is to attach a TAG\_TeleportFieldComponent component to a world entity and see its effects!

## Final Code

The final file content can be found here:
Show File Content

```enforce
[ComponentEditorProps(category: "Tutorial/Component", description: "Warn then teleport humans that are too close to the entity")]
class TAG_TeleportFieldComponentClass : ScriptComponentClass
{
}
class TAG_TeleportFieldComponent : ScriptComponent
{
	/*
	Teleportation
	*/
	[Attribute(defvalue: "10", desc: "Distance at which the field draws a line to its target to warn it about teleportation", category: "Teleportation")]
	protected float m_fWarningRadius;
	[Attribute(defvalue: "2", desc: "Distance at which the field triggers the teleportation", params: "0.25 10 0.25", category: "Teleportation")]
	protected float m_fTriggerRadius;
	[Attribute(defvalue: "10", desc: "Distance at which the teleportation places the unit from the teleporter", category: "Teleportation")]
	protected float m_fTeleportDistance;
	/*
	Line Drawing
	*/
	[Attribute(defvalue: "1 0.75 0 1", desc: "The line's colour", category: "Line Drawing")]
	protected ref Color m_LineColour;
	[Attribute(defvalue: "1", desc: "Whether or not the line must fade in/out with transparency based on distance", category: "Line Drawing")]
	protected bool m_bLineFadeInOut;
	[Attribute(defvalue: "0 1 0", desc: "The line offset from entities's origins", category: "Line Drawing")]
	protected vector m_vOffset;
	/*
	Performance
	*/
	[Attribute(defvalue: "0.25", desc: "Duration between proximity checks", category: "Performance")]
	protected float m_fCheckPeriod;
	protected float m_fCheckDelay;
	protected int m_iTempLineColour;
	protected ref array<ref Shape> m_aShapes;
	protected ref array<IEntity> m_aNearbyCharacters;
	//------------------------------------------------------------------------------------------------
	//! Draw a debug line between two entities
	//! \param[in] from
	//! \param[in] to
	//! \param[in] offset
	protected Shape DrawLine(notnull IEntity from, notnull IEntity to, vector offset)
	{
		vector points[2] = { from.GetOrigin() + offset, to.GetOrigin() + offset };
		float distance = vector.Distance(points[0], points[1]);
		if (m_bLineFadeInOut)
		{
			int alpha255 = 255 * (1 - ((distance - m_fTriggerRadius) / (m_fWarningRadius - m_fTriggerRadius)));
			m_iTempLineColour = m_iTempLineColour & 0x00FFFFFF | (alpha255 << 24);
			return Shape.CreateLines(m_iTempLineColour, ShapeFlags.TRANSP, points, 2);
		}
		else
		{
			return Shape.CreateLines(m_iTempLineColour, 0, points, 2);
		}
	}
	//------------------------------------------------------------------------------------------------
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		super.EOnFrame(owner, timeSlice);
		vector ownerPos = owner.GetOrigin();
		m_fCheckDelay -= timeSlice;
		if (m_fCheckDelay <= 0)
		{
			m_fCheckDelay = m_fCheckPeriod;
			m_aNearbyCharacters.Clear();
			owner.GetWorld().QueryEntitiesBySphere(ownerPos, m_fWarningRadius, QueryEntitiesCallbackMethod, null, EQueryEntitiesFlags.DYNAMIC | EQueryEntitiesFlags.WITH_OBJECT);
		}
		m_aShapes.Clear();
		m_aShapes.Reserve(m_aNearbyCharacters.Count());
		foreach (IEntity character : m_aNearbyCharacters)
		{
			vector characterPos = character.GetOrigin();
			if (vector.Distance(characterPos, ownerPos) > m_fTriggerRadius) // in the warning zone
			{
				m_aShapes.Insert(DrawLine(owner, character, m_vOffset)); // draw line
			}
			else // in the trigger zone
			{
				vector dir = vector.Direction(ownerPos, characterPos).Normalized();
				character.SetOrigin(ownerPos + dir * m_fTeleportDistance); // teleport
			}
		}
	}
	//------------------------------------------------------------------------------------------------
	// QueryEntitiesCallback type
	protected bool QueryEntitiesCallbackMethod(IEntity e)
	{
		if (!e || !ChimeraCharacter.Cast(e)) // only humans
		return false;
		m_aNearbyCharacters.Insert(e);
		return true;
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnPostInit(IEntity owner)
	{
		m_aShapes = {};
		m_aNearbyCharacters = {};
		m_iTempLineColour = m_LineColour.PackToInt();
		SetEventMask(owner, EntityEvent.FRAME);
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63b442aee34)
