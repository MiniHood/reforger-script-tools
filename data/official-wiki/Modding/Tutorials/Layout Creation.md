# [Layout Creation](https://community.bistudio.com/wiki/Arma_Reforger:Layout_Creation)

## Definitions

### Display

A display is a layout that only provides information to the player; it does not require any input. The player can control his character normally, movement/cursor are not captured.

ⓘ

Think Head-Up Display (HUD).

### Menu

A menu is an interface that provides choices or functionality to the player.

ⓘ

Think Field Manual, Inventory etc.

### Dialog

A dialog is an interface that requires player's input. There can be multiple dialogs at the same time, they are then ordered by priority. It is displayed on top of a menu, if any is present.

ⓘ

Think OK/Cancel popup window, name input, etc.

### Widget

A widget is an element composing a layout - a button, a text, a scrollbar, any "layout part" is a widget.

## Create a Mod

See [Mod Project Setup](/wiki/Arma_Reforger:Mod_Project_Setup "Arma Reforger:Mod Project Setup").

## Create a Layout

Using [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager"), open the mod's directory and create the directory tree UI\layouts;
in it, **right-click > GUI layout** (or the **Create button > GUI layout**) to create a .layout file - this is our new layout.

Double-click on it to open it, using the [Layout Editor](/wiki/Arma_Reforger:Resource_Manager:_Layout_Editor "Arma Reforger:Resource Manager: Layout Editor") to edit it.

This first example will sport three features: a title, a change text button and a close button.

* Clicking the change button will change the title
* Clicking the close button will close the UI
* Clicking the title will do nothing.

### Add the Close Button

The first widget is an important one; this button will be used to close the opened UI.

Place a [WLib\_ButtonText.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/WidgetLibrary/Buttons/WLib_ButtonText.layout) button by dragging the layout file from the Resource Browser into the opened layout.
In the Hierarchy window, name it ButtonClose.

ⓘ

* Widget names will be used later in the code.
* The WLib prefix means Widget Library and is used to find widget prefabs more easily.
* Buttons are only useful in Menu and Dialog; as a Display is not interactable

To change this button's text, one could be tempted to change ButtonClose > SizeLayout > Overlay > Text's Text property.

However, as this prefab uses a [SCR\_ButtonTextComponent](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/Button/SCR_ButtonTextComponent.c;2) script component, the displayed value is set within this component.

Click on **ButtonClose** (the prefab's topmost widget) and in its properties, under [Script](/wiki/Arma_Reforger:Resource_Manager:_Layout_Editor#Script "Arma Reforger:Resource Manager: Layout Editor"), find this said component to change its Text property.

### Add the Title Text

The second widget is the title.
Place a [RichText\_Heading2.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/WidgetLibrary/RichTextWidgets/RichText_Heading2.layout) and name it "TextTitle".

### Add the Change Text Button

Place another [WLib\_ButtonText.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/WidgetLibrary/Buttons/WLib_ButtonText.layout) button and name it ButtonChange.

## Menu/Dialog Setup

Menus and Dialogs share the same first steps:

* Add an enum constant by [modding](/wiki/Arma_Reforger:Scripting_Modding "Arma Reforger:Scripting Modding") the [ChimeraMenuPreset](enfusion://ScriptEditor/scripts/Game/UI/Menu/ChimeraMenuBase.c;3) enum in a new script file:

  ENFORCECODEMARKER

  ```
  modded enum ChimeraMenuPreset
  {
  	TAG_LayoutTutorialExample,
  }
  ```
* *Override* chimeraMenus.conf in the mod (see [Data Modding Basics - Overriding\_assets](/wiki/Arma_Reforger:Data_Modding_Basics#Overriding_assets "Arma Reforger:Data Modding Basics") - right-click → override in LayoutTutorial)
* Declare this new layout in it by adding an entry (clicking the "+" button):
  + The entry name ("object name" required on new entry) **must** match the enum constant, here TAG\_LayoutTutorialExample, otherwise the engine will not be able to find the menu and will throw an error
  + **Layout** - the path to the created layout
  + **Action Context** - *unused in this tutorial*
  + **Class** - the script class to be used with the menu/dialog, see the next chapter where we will create the TAG\_LayoutTutorialUI class.
  + **Menu Items** - *unused in this tutorial*
  + **Preload Count** - *unused in this tutorial*
  + **Persistent** - *unused in this tutorial*

## Class Setup

The menu/dialog class must inherit from at least [MenuBase](enfusion://ScriptEditor/scripts/GameLib/generated/Menu/MenuBase.c;31), [ChimeraMenuBase](enfusion://ScriptEditor/scripts/Game/UI/Menu/ChimeraMenuBase.c;71) or [MenuRootBase](enfusion://ScriptEditor/scripts/Game/UI/Menu/MenuRootBase.c;10), depending on the required features.

Here is the example adapted to our earlier layout:
Show TAG\_LayoutTutorialUI code

```enforce
class TAG_LayoutTutorialUI : MenuBase
{
	protected static const string TEXT_TITLE = "TextTitle";
	protected static const string BUTTON_CLOSE = "ButtonClose";
	protected static const string BUTTON_CHANGE = "ButtonChange";
	//------------------------------------------------------------------------------------------------
	protected override void OnMenuOpen()
	{
		Print("OnMenuOpen: menu/dialog opened!", LogLevel.NORMAL);
		Widget rootWidget = GetRootWidget();
		if (!rootWidget)
		{
			Print("Error in Layout Tutorial layout creation", LogLevel.ERROR);
			return;
		}
		// Close button
		SCR_ButtonTextComponent buttonClose = SCR_ButtonTextComponent.GetButtonText(BUTTON_CLOSE, rootWidget);
		if (buttonClose)
		buttonClose.m_OnClicked.Insert(Close);
		else
		Print("Button Close not found - won't be able to exit by button", LogLevel.WARNING);
		// Change button
		SCR_ButtonTextComponent buttonChange = SCR_ButtonTextComponent.GetButtonText(BUTTON_CHANGE, rootWidget);
		if (buttonChange)
		buttonChange.m_OnClicked.Insert(ChangeText);
		else
		Print("Button Change not found", LogLevel.WARNING); // the button can be missing without putting the layout in jeopardy
		/*
		ESC/Start listener
		*/
		InputManager inputManager = GetGame().GetInputManager();
		if (inputManager)
		{
			// this is for the menu/dialog to close when pressing ESC
			// an alternative is to have a button with the SCR_NavigationButtonComponent component
			// and its Action Name field set to MenuBack - this would activate the button on ESC press
			inputManager.AddActionListener("MenuOpen", EActionTrigger.DOWN, Close);
			inputManager.AddActionListener("MenuBack", EActionTrigger.DOWN, Close);
			#ifdef WORKBENCH // in Workbench, F10 is used because ESC closes the preview
			inputManager.AddActionListener("MenuOpenWB", EActionTrigger.DOWN, Close);
			inputManager.AddActionListener("MenuBackWB", EActionTrigger.DOWN, Close);
			#endif // WORKBENCH
		}
		else if (!buttonClose)
		{
			Print("Auto-closing the menu that has no exit path", LogLevel.WARNING);
			Close();
			return;
		}
	}
	//------------------------------------------------------------------------------------------------
	protected override void OnMenuClose()
	{
		// here we clean action listeners added above as the good practice wants it
		InputManager inputManager = GetGame().GetInputManager();
		if (inputManager)
		{
			inputManager.RemoveActionListener("MenuOpen", EActionTrigger.DOWN, Close);
			inputManager.RemoveActionListener("MenuBack", EActionTrigger.DOWN, Close);
			#ifdef WORKBENCH
			inputManager.RemoveActionListener("MenuOpenWB", EActionTrigger.DOWN, Close);
			inputManager.RemoveActionListener("MenuBackWB", EActionTrigger.DOWN, Close);
			#endif // WORKBENCH
		}
	}
	//------------------------------------------------------------------------------------------------
	protected void ChangeText()
	{
		Widget rootWidget = GetRootWidget();
		if (!rootWidget)
		return;
		TextWidget textTitle = TextWidget.Cast(rootWidget.FindWidget(TEXT_TITLE));
		if (!textTitle)
		{
			Print("Title as TextWidget could not be found", LogLevel.WARNING);
			return;
		}
		string result;
		switch (Math.RandomInt(1, 6))
		{
			case 1: result = "This is a title"; break;
			case 2: result = "Random text"; break;
			case 3: result = "Third text, actually"; break;
			case 4: result = "Bonjour"; break;
			case 5: result = "I like trains"; break;
		}

		textTitle.SetText(result);
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63d32271272)

## Usage

In order to test more easily, we can place a prefab with an ActionsManagerComponent to add actions that trigger UI creation.
Each action targets a specific type (display, menu, dialog and their respective classes, TAG\_DisplayAction, TAG\_MenuAction and TAG\_DialogAction).

ⓘ

See [Action Context Setup](/wiki/Arma_Reforger:Action_Context_Setup "Arma Reforger:Action Context Setup") for more information.

### Display

```enforce
GetGame().GetWorkspace().CreateWidgets(m_sLayout);
```

### Menu

```enforce
GetGame().GetMenuManager().OpenMenu(ChimeraMenuPreset.TAG_LayoutTutorialExample);
```

### Dialog

```enforce
GetGame().GetMenuManager().OpenDialog(ChimeraMenuPreset.TAG_LayoutTutorialExample, DialogPriority.INFORMATIVE, 0, true);
```

### Code

Show Scripted Action code

```enforce
class TAG_ScriptedUserAction : ScriptedUserAction
{
	[Attribute(defvalue: "{74A4182551CBD740}UI/layouts/TAG_LayoutTutorial.layout")] // setup the created layout here
	protected ResourceName m_sLayout;
	protected ref Widget m_wDisplay;
}
class TAG_DisplayAction : TAG_ScriptedUserAction
{
	//------------------------------------------------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		if (m_wDisplay)
		{
			delete m_wDisplay;
			return;
		}
		m_wDisplay = GetGame().GetWorkspace().CreateWidgets(m_sLayout);
	}
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if (m_wDisplay)
		outName = "Delete display";
		else
		outName = "Create display";
		return true;
	}
}
class TAG_MenuAction : TAG_ScriptedUserAction
{
	//------------------------------------------------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		if (m_wDisplay)
		{
			delete m_wDisplay;
			return;
		}
		GetGame().GetMenuManager().OpenMenu(ChimeraMenuPreset.TAG_LayoutTutorialExample);
	}
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if (m_wDisplay)
		outName = "Delete menu";
		else
		outName = "Create menu";
		return true;
	}
}
class TAG_DialogAction : TAG_ScriptedUserAction
{
	//------------------------------------------------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		if (m_wDisplay)
		{
			delete m_wDisplay;
			return;
		}
		GetGame().GetMenuManager().OpenDialog(ChimeraMenuPreset.TAG_LayoutTutorialExample, DialogPriority.INFORMATIVE, 0, true);
	}
	//------------------------------------------------------------------------------------------------
	override bool GetActionNameScript(out string outName)
	{
		if (m_wDisplay)
		outName = "Delete dialog";
		else
		outName = "Create dialog";
		return true;
	}
}
```

[↑ Back to spoiler's top](#bikisp6a63d3227700e)

Using these actions allow to see the different behaviours of said interface - be sure to set the action's default layout (m\_sLayout either in code or in Action's configuration UI.

## See Also

[MenuManager](enfusion://ScriptEditor/scripts/GameLib/generated/Menu/MenuManager.c;12)'s various methods, including:

* cIsAnyDialogOpen() - check if an existing dialog exists
* cGetTopMenu() - get the topmost menu
* cCloseAllMenus() - close all opened menus
