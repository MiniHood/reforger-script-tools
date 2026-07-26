# [Hint Usage](https://community.bistudio.com/wiki/Arma_Reforger:Hint_Usage)

This tutorial explains the Hint system.

## Config Creation

Create a config that inherits from [SCR\_HintUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_HintUIInfo.c;1); this config has a lot of customisation options.

### Name

Hint's title - should be as short as possible.

### Description

The text displayed in the hint.

⚠

Use either this or the [Description Blocks](#Description_Blocks) - do not use both.

### Icon

This will be shown right below the title. If an imageset is provided, the desired icon from the set needs to be set in [Icon Set Name](#Icon_Set_Name) below.

### Icon Set Name

The icon name if an imageset has been provided in [Icon](#Icon).

### Description Blocks

The text displayed in the hint; like [Description](#Description), but allows for more customisation.

⚠

Use either this or the [Description](#Description) - do not use both.

#### Details

Description blocks can be seen like building blocks with which you build your Hint.

There are different types of description blocks that each have their own purpose.

| Block | Description | Properties |
| --- | --- | --- |
| ActionBlockUIName | This is used to display key binds. Automatically changes between keyboard and gamepad. | * Name: this will be displayed above the Key to tell the user what the button does. * Device: only show the block if the user uses the selected Input Device. * Action Name: defines what action will be shown - the action name is defined in [chimeraInputCommon.conf](enfusion://ResourceManager/~ArmaReforger:Configs/System/chimeraInputCommon.conf) |
| BulletPointUIName | This is just a Text that has a Bullet point in front of it. | * Name: text displayed with the bullet point in front of it. |
| DeviceBlockUIName | This is a normal description block that can be shown when selected Input device is used. | * Name: text displayed. * Device: what input device needs to be used in order for this to be shown. |
| ImageBlockUIName | This allows you to insert an Image into the Text. Other than the Icon, this can be anywhere in the text. Can be either from an imageset or a normal image. Size and color can be changed. | * Name: text shown above the Image. * Image Set: path to the Image or Imageset * Image: if 'Image Set' is a Imageset, define the icon you want to use from the Imageset here. * Scale: defines the scale of the Icon. * Color: sets the color of the Icon. |
| KeyBlockUIName | This works like the '**ActionBlockUIName**' with the difference, that you are not limited to an action but can show custom key binds. | * Name: Text shown above the Keys. * Device: which device should be used in order for this block to be shown. * Key: array of key binds that will be shown. |
| SimpleTextBlockUIName | This can be used as a sub header. An easy way to make the text bold, italic, etc. Can be formatted into "H1, H2, P, B, I". | * Name: text that will be formatted. * Tags: selected tags change the format of the text defined in 'Name". |
| SubBlockUIName | This is just a normal text with no special stuff | * Name: text shown in this block. |
| TipBlockUIName | This can be used as a small Tip. Gets a tip icon in front of it. Has lower opacity and is slightly grey. | * Name: text shown in this block with a tip icon in front of it. |

### Type

When defined, the hint can be shown only a certain amount of times (defined by the [Show Limit](#Show_Limit) property below)

### Show Limit

ⓘ

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.0.0 "Category:Arma Reforger/Version 1.0.0") [1.0.0](/wiki?title=Category:Arma_Reforger/Version_1.0.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.0.0 (page does not exist)") This property has currently no effect.

Determines how often the hint will be shown; if set to 1 the hint is only shown once and then never again, even after game restart.

### Priority

Sets the priority of the hint over other hints:

* If the priority if higher or equal to the currently shown hint, this hint will be shown.
* If the priority is less than the currently shown hint, the hint will only be shown once the current hint disappears.

### Duration

Defines how long the hint will be visible; if set to 0, set time from HintManager will be used, if set to -1 the hint will stay on screen until closed manually.

### Highlight Widgets Names

Highlights widgets with the provided names - useful for explaining widgets usage in a menu.

### Is Timer Visible

If enabled, a timer bar will appear at the top of the hint:

* If a duration is set the bar will behave like a timer.
* When the bar finishes its animation, the hint is hidden automatically.
* If no duration is set, the bar stays static and does not move.

### Field Manual Link

If set, an option to open the Field Manual to the selected page will be added to the hint.

## Script Creation

To create a Hint in script create a new script or use an existing one. In there create a new variable for [SCR\_HintUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_HintUIInfo.c;1).

Then using the c[SCR\_HintUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_HintUIInfo.c;1).CreateInfo() method you can create a hint with your own parameters.

⚠

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.0.0 "Category:Arma Reforger/Version 1.0.0") [1.0.0](/wiki?title=Category:Arma_Reforger/Version_1.0.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.0.0 (page does not exist)") A Hint created in script does not support Description blocks and as of right now only works with title and description.

This is a example on how to create a hint in script using a [SCR\_ScriptedWidgetComponent](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/SCR_ScriptedWidgetComponent.c;7):

```enforce
protected SCR_HintUIInfo TAG_HintInfo
{
	override void HandlerAttached(Widget w)
	{
		string description = "#AR-LocalisedDescription"; // see Description - this text is localised
		string name = "Hint's title"; // see Name
		float duration = 10; // see Duration
		EHint type = EHint.UNDEFINED; // see Type
		bool isTimerVisible = true; // see Is Timer Visible
		// each entry in the EFieldManualEntryId enum represents a specific FieldManual entry
		// if set, a Field Manual button will appear in the hint, leading to said entry
		EFieldManualEntryId fieldManualEntry = EFieldManualEntryId.NONE; // see Field Manual Link
		// create the hint with the information set above
		// this will only create the Hint but not show it - to show it, a call to SCR_HintManagerComponent.ShowHint() is required
		// SCR_HintUIInfo is an object with all the set information stored inside.
		TAG_HintInfo = SCR_HintUIInfo.CreateInfo(description, name , duration, type, fieldManualEntry, isTimerVisible);
	}
}
```

And to show the hint:

```enforce
// this will show the Hint immediately.
// if there is another hint currently shown it will be replaced with this one.
SCR_HintManagerComponent.ShowHint(TAG_HintInfo);
```

## Config Modification

To edit a hint that was created in a config you first need to get a reference to it.
The easiest way to do that is to create a variable in your script, make it an attribute and assign your [SCR\_HintUIInfo](enfusion://ScriptEditor/scripts/Game/Editor/Containers/UIInfo/SCR_HintUIInfo.c;1) config to it.
With this done you are able to edit the following things in your hint:

* Description
* Title
* Text in your description blocks

```enforce
[Attribute("{AE7B95B82AC8B864}TAG_TestHint.conf")]
protected ref SCR_HintUIInfo m_Hint;
{
	override void HandlerAttached(Widget w)
	{
		if (!m_Hint)
		return;
		// change the description of the Hint
		m_Hint.SetDescription("Some Description");
		// change the titel of the hint
		m_Hint.SetName("Some Titel");
		// change the Text of the selected description block
		// index = index of description block in descriptionBlocks array in the Hint
		// text will be formatted to add the needed information like action for the ActionBlock
		m_Hint.SetDescriptionBlockName(0, "Some Input: %1");
		// if the hint is already shown, it can be updated immediately using this
		// if the hint is not currently shown this can be skiped and SCR_HintManagerComponent.ShowHint(m_Hint) can be called when the hint should be shown
		SCR_HintManagerComponent.Refresh();
	}
}
```

## Script Modification

Editing a Hint you previously created in your script is the easiest. Take the variable that holds the just change the values you want.

ⓘ

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.0.0 "Category:Arma Reforger/Version 1.0.0") [1.0.0](/wiki?title=Category:Arma_Reforger/Version_1.0.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.0.0 (page does not exist)") It is currently only possible to change the hint's description and label. Methods to change other attributes will be added later®.

```enforce
class TAG_SomeClass
{
	protected SCR_HintUIInfo TAG_HintInfo;
	// ...
	override void HandlerAttached(Widget w)
	{
		// create the hint as we did in the example earlier.
		TAG_HintInfo = SCR_HintUIInfo.CreateInfo(hintDescription, hintName, duration, type, fieldManualEntry, isTimerVisible);
		// change the description of the Hint.
		TAG_HintInfo.SetDescription("This is a new description");
		// change the label of the Hint.
		TAG_HintInfo.SetName("This is a new name");
		// if the hint is already shown, it can be updated immediately using this
		SCR_HintManagerComponent.Refresh();
	}
}
```

ⓘ

If the hint is already shown and you want to refresh it, you need to use c[SCR\_HintManagerComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintManagerComponent.c;33).Refresh();

## Hint Sequence Creation

To create a Hint sequence, it is recommended to first create the different configs for each hint as shown above. It is also possible to create the different hints inside the sequence config to have less configs.

1. Create a config that inherits from [SCR\_HintSequenceList](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceList.c;1)
2. Insert or create your hints in the Hints array - hints will be displayed in the order of the array, from first to last
3. Create an entity (or use an existing one in your world) and attach the [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15) to it
4. In that component, assign your Sequence config to the Sequence attribute

### Show the Sequence

In script, get a reference to your entity and find its [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15).

```enforce
// SCR_EditorModeEntity is used as example
SCR_EditorModeEntity currentMode = SCR_EditorModeEntity.GetInstance();
if (!currentMode)
return;
// find the SCR_HintSequenceComponent attached to the entity
SCR_HintSequenceComponent hintSequence = SCR_HintSequenceComponent.Cast(currentMode.FindComponent(SCR_HintSequenceComponent));
```

With the reference to the [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15) you can then start/stop the sequence as well as listen to its cOnSequenceChange() invoker.

| Method | Description |
| --- | --- |
| GetOnSequenceActive | Get the event triggered when the hint sequence starts or stops. return Script invoker. |
| IsSequenceActive | Check if the hint sequence is currently active. return True when active. |
| StartSequence | Start the hint sequence defined in the [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15). |
| StopSequence | Stop the hint sequence defined in the [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15). |
| ToggleSequence | Toggle the hint sequence defined in the [SCR\_HintSequenceComponent](enfusion://ScriptEditor/scripts/Game/Components/Hints/SCR_HintSequenceComponent.c;15). Stops the sequence if it is active and starts it if it is not. |
