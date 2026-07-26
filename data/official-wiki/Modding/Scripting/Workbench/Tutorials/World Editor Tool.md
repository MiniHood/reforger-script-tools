# [World Editor Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor_Tool)

ⓘ *Not to be confused with [World Editor Plugin](/wiki/Arma_Reforger:World_Editor_Plugin "Arma Reforger:World Editor Plugin").*

---

This tutorial teaches how to create a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor")-specific **tool**.

A World Editor tool appears in the Tools bar unlike plugins that appear in the plugins list.

⚠

* Please read [Workbench Plugin](/wiki/Arma_Reforger:Workbench_Plugin "Arma Reforger:Workbench Plugin") and [World Editor Plugin](/wiki/Arma_Reforger:World_Editor_Plugin "Arma Reforger:World Editor Plugin") before following this tutorial.
* Tools do not support shortcut definition - the icon must be clicked.
  + Tools do not run on icon click; they require button definition (see [Setup](#Setup)) for them to trigger an action.

## Setup

* Open Script Editor
* In an addon, create a new script in WorkbenchGame/WorldEditor - name it [TAG\_](/wiki/Scripting_Tags "Scripting Tags")TutorialTool.c (must end with Tool by convention)
* Double-click the file to open it
* Press `Ctrl` + `T` to use the [Script Template plugin](/wiki/Arma_Reforger:Script_Editor:_Fill_From_Template_Plugin "Arma Reforger:Script Editor: Fill From Template Plugin")
  + In its window, select "Class Type: **WorldEditorTool**", leave the other fields blank/default
  + A World Editor tool skeleton is inserted.
* In the WorkbenchToolAttribute, fill the description field with "Tool description and quick usage guide"
* Make the plugin inherit from c[WorldEditorPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorldEditorPlugin.c;14) instead of c[WorkbenchPlugin](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorkbenchPlugin.c;14)
* Uncomment the cRun() method and write cPrint("It works!"); in it, then save the file (default `Ctrl` + `S`)
* Reload Workbench scripts via **Reload Scripts** option located in **Plugins → Settings** (default shortcut: `Ctrl` + `⇧ Shift` + `R`)
* The TAG\_TutorialTool tool should appear in the World Editor's Tools list, available in the top bar - click on the defined icon
* The tool's panel appears, with Tool description and quick usage guide text and the "Run" button - click on the Run button
* "It works!" gets printed in the output console.

## WorldEditorTool API

ⓘ

See the [WorldEditorTool](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorldEditorTool.c;17) class.

The WorldEditorTool API offers some interesting things:

* the m\_API variable is a direct access to the WorldEditorAPI instance
* the cGetModifierKeyState() and the On\* events methods allow to intercept user input
* the cUpdatePropertyPanel() method allows to refresh the tool's panel in case its values are changed by script.

## WorldEditorAPI API

ⓘ

See the [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) class as well as [WorldEditorAPI Usage](/wiki/Arma_Reforger:WorldEditorAPI_Usage "Arma Reforger:WorldEditorAPI Usage").

## Example

In this example, we will use [BaseWorld](enfusion://ScriptEditor/scripts/Core/generated/World/BaseWorld.c;12), [WorldEditorTool](enfusion://ScriptEditor/scripts/GameLib/generated/WorkbenchAPI/Plugins/WorldEditorTool.c;17) and [WorldEditorAPI](enfusion://ScriptEditor/scripts/Core/generated/WorkbenchAPI/WorldEditorAPI.c;12) methods to:

* use `Ctrl` + ![Mouse Scrollwheel](/wikidata/images/thumb/c/c5/mouse-button-scrollwheel.png/32px-mouse-button-scrollwheel.png "Mouse Scrollwheel") to set the detection radius
* detect entities in a radius around the cursor's world position
* draw lines from the cursor to them
* turn them upside-down on double-click ![Double Left Mouse Button](/wikidata/images/thumb/a/af/mouse-button-left-double.png/32px-mouse-button-left-double.png "Double Left Mouse Button")
* delete them on `⟵ Backspace` keypress:

⚠

Editing entities implies to wrap the action(s) (on potentially multiple entities) with cm\_API.BeginEntityAction() / cm\_API.EndEntityAction();

this is mandatory and also allows to have multiple actions into one "step", that undo/redo (default `Ctrl` + `Z` / `Ctrl` + `Y`) can process.

See [WorldEditorAPI Usage](/wiki/Arma_Reforger:WorldEditorAPI_Usage "Arma Reforger:WorldEditorAPI Usage").

```enforce
#ifdef WORKBENCH
[WorkbenchToolAttribute(
name: "TAG_TutorialTool",
description: "Set the wanted radius, move the mouse to highlight found entities, then:"
+ "\n- double-click to turn them upside-down"
+ "\n- press BACKSPACE to delete them",
awesomeFontCode: 0xF188)]
class TAG_TutorialTool : WorldEditorTool
{
	[Attribute(defvalue: "30", uiwidget: UIWidgets.Slider, params: MIN_RADIUS.ToString() + " " + MAX_RADIUS + " 0.1")]
	protected float m_fRadius;
	[Attribute(defvalue: "0", desc: "If checked, the cursor stops on the hovered entity; otherwise it only considers the ground as an obstacle")]
	protected bool m_bCursorOnObject;
	[Attribute(defvalue: "0.5 1 0.5 0.25", desc: "Sphere colour")]
	protected ref Color m_SphereColour;
	[Attribute(defvalue: "1 1 0 1", desc: "Line colour")]
	protected ref Color m_LineColour;
	protected ref array<IEntity> m_aNearbyEntities;
	float m_cursorX;
	float m_cursorY;
	protected BaseWorld m_World;
	protected ref SCR_DebugShapeManager m_DebugShapeManager;
	protected static const float MIN_RADIUS = 1;
	protected static const float MAX_RADIUS = 100;
	//------------------------------------------------------------------------------------------------
	protected override void OnMouseMoveEvent(float x, float y)
	{
		m_World = m_API.GetWorld();
		if (!m_World)
		return;
		m_cursorX = x;
		m_cursorY = y;
		RefreshShapes();
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnKeyPressEvent(KeyCode key, bool isAutoRepeat)
	{
		if (key == KeyCode.KC_BACK)
		DeleteEntities();
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnMouseDoubleClickEvent(float x, float y, WETMouseButtonFlag buttons)
	{
		if (buttons == WETMouseButtonFlag.LEFT)
		TurnEntitiesUpsideDown();
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnWheelEvent(int delta)
	{
		if (GetModifierKeyState(ModifierKey.CONTROL))
		{
			m_fRadius += delta / 120;
			if (m_fRadius < MIN_RADIUS)
			m_fRadius = MIN_RADIUS;
			else if (m_fRadius > MAX_RADIUS)
			m_fRadius = MAX_RADIUS;
			RefreshShapes();
			UpdatePropertyPanel();
		}
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnActivate()
	{
		m_DebugShapeManager = new SCR_DebugShapeManager();
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnDeActivate()
	{
		m_aNearbyEntities = null;
		m_DebugShapeManager = null;
	}
	//------------------------------------------------------------------------------------------------
	protected void RefreshShapes()
	{
		m_DebugShapeManager.Clear();
		vector cursorWorldPos;
		if (!GetCursorWorldPosition(m_cursorX, m_cursorY, cursorWorldPos))
		return; // not on screen
		FillNearbyEntitiesArray(cursorWorldPos, m_fRadius);
		m_DebugShapeManager.AddSphere(cursorWorldPos, m_fRadius, m_SphereColour.PackToInt(), ShapeFlags.NOOUTLINE | ShapeFlags.DEPTH_DITHER);
		DrawLinesFromPosToEntities(cursorWorldPos, m_aNearbyEntities);
	}
	//------------------------------------------------------------------------------------------------
	protected bool GetCursorWorldPosition(float x, float y, out vector cursorWorldPos)
	{
		vector traceStart, traceDir;
		TraceFlags flags = TraceFlags.WORLD;
		if (m_bCursorOnObject)
		flags |= TraceFlags.ENTS;
		return m_API.TraceWorldPos(x, y, flags, traceStart, cursorWorldPos, traceDir) && cursorWorldPos != vector.Zero;
	}
	//------------------------------------------------------------------------------------------------
	protected void FillNearbyEntitiesArray(vector centre, float radius)
	{
		m_aNearbyEntities = {};
		m_World.QueryEntitiesBySphere(
		centre,
		radius,
		QueryEntitiesCallbackMethod,
		queryFlags: EQueryEntitiesFlags.STATIC | EQueryEntitiesFlags.DYNAMIC | EQueryEntitiesFlags.WITH_OBJECT);
		// as QueryEntitiesBySphere gets entities by sphere-AABB intersection, we shave by origin distance
		for (int i = m_aNearbyEntities.Count() - 1; i >= 0; --i)
		{
			if (vector.Distance(m_aNearbyEntities[i].GetOrigin(), centre) > radius)
			m_aNearbyEntities.Remove(i);
		}
	}
	//------------------------------------------------------------------------------------------------
	protected bool QueryEntitiesCallbackMethod(IEntity e)
	{
		if (!e)
		return false;
		m_aNearbyEntities.Insert(e);
		return true;
	}
	//------------------------------------------------------------------------------------------------
	protected void DrawLinesFromPosToEntities(vector worldPos, notnull array<IEntity> entities)
	{
		foreach (IEntity entity : entities)
		{
			m_DebugShapeManager.AddLine(worldPos, entity.GetOrigin(), m_LineColour.PackToInt());
		}
	}
	//------------------------------------------------------------------------------------------------
	protected void TurnEntitiesUpsideDown()
	{
		if (!m_aNearbyEntities || m_aNearbyEntities.IsEmpty())
		return;
		m_API.BeginEntityAction();
		IEntitySource entitySource;
		foreach (IEntity nearbyEntity : m_aNearbyEntities)
		{
			entitySource = m_API.EntityToSource(nearbyEntity);
			if (!entitySource)
			continue;
			vector angles;
			if (!entitySource.Get("angles", angles))
			continue;

			angles[2] = angles[2] + 180;
			angles[2] = Math.Repeat(angles[2] + 180, 360) - 180;
			m_API.SetVariableValue(entitySource, null, "angles", angles.ToString());
		}
		m_API.EndEntityAction();
		RefreshShapes();
	}
	//------------------------------------------------------------------------------------------------
	protected void DeleteEntities()
	{
		if (!m_aNearbyEntities || m_aNearbyEntities.IsEmpty())
		return;
		m_API.BeginEntityAction();
		foreach (IEntity nearbyEntity : m_aNearbyEntities)
		{
			m_API.DeleteEntity(m_API.EntityToSource(nearbyEntity));
		}
		m_API.EndEntityAction();
		RefreshShapes();
	}
}
#endif
```
