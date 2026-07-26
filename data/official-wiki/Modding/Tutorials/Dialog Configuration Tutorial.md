# [Dialog Configuration Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Dialog_Configuration_Tutorial)

The Configurable Dialog system allows to easily make a custom dialog given a config file.

## Dialog Creation

* Make a layout if needed.
* Inherit from [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) for a custom dialog handler, if needed.
* Make a [SCR\_ConfigurableDialogUiPresets](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;802) config file or choose an existing one.
* Add entries of [SCR\_ConfigurableDialogUiPreset](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;762) in the config: these are your dialogs. Give them exclusive tags within the config file that holds them.
* In scripts, when you want to spawn a dialog, you need to:
  + Instantiate the custom handler (if created)
  + Call the static method c[SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16).CreateFromPreset(). The parameters you need to provide are:
    - **presetsResourceName**: ResourceName (GUID) of the config file that holds the dialog preset.
    - **tag**: unique identifier string to target the dialog to be spawned.
    - **customDialogObj**: optional handler class inheriting from [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) - if not provided the base SCR\_ConfigurableDialogUi will be used instead.

The cCreateFromPreset() method returns the instance of the dialog handler that has been made for the newly spawned dialog, so you can cast to your custom handler type to perform further operations.

### Layouts

Arma Reforger generally uses three layouts: ConfigurableDialog, ConfigurableDialog\_Medium, ConfigurableDialog\_Big.



The structure is:

* a Size widget that defines the dimensions of the whole dialog
* a Header with title and icon
* a Message text
* a Content area
* a Footer for the buttons.

The content of complex dialogs should be its own layout, which the system will dynamically add to the base.

Still, nothing stops anyone from creating their own "complete" layout by inheriting from one of the bases and filling the ContentLayoutContainer widget manually with whatever needed content.

It is recommended to have the content be its own layout; however for ease of maintenance, future changes to the base hierarchy might cause the loss of content that is not protected in its own prefab layout.
Same goes with buttons: they either get added through the .conf files or in the manually created layout.

### SCR\_ConfigurableDialogUi

The base dialog handler is a [ScriptedWidgetComponent](enfusion://ScriptEditor/scripts/Core/generated/UI/ScriptedWidgetComponent.c;12) that will be instantiated by the dialog creation process and attached to the dialog layout.

It provides handling of the Title, Message, Content and Footer as defined in the presets, and includes methods and invokers for confirming, canceling and closing.

The dialog itself is technically a menu,, since the Configurable layout is inserted into a proxy menu created with the enfusion Menu Manager, and thus has also access to menu events.

By inheriting from [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) you can freely extend its functionality.

### SCR\_ConfigurableDialogUiPresets

These config files allow us to spread the large amount of dialogs needed by the game into multiple, smaller lists.

This avoids unnecessary clutter in chimeraMenus.conf, and allows to organize dialogs in their own .confs and scripts.

A dialog config file can contain multiple presets, identified by tag. The standard Configurable dialog preset should already have all the attributes necessary for your dialog, but you can always inherit from [SCR\_ConfigurableDialogUiPreset](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;762) and extend it.

**The Tag is used to know which dialog to create**. As such, you need unique tags in a single .conf file.

The system will assemble the dialog, adding the content and buttons to the base.

It is possible to create prefab .conf files for single dialogs and buttons, and then use those in the general dialog .conf files.
An example is [MessageOkCancel.conf](enfusion://ResourceManager/~ArmaReforger:Configs/ConfigurableDialogs/DialogPrefabs/MessageOkCancel.conf).

### Related Classes

#### Configuration Classes

* [SCR\_ConfigurableDialogUiPresets](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;802) - collection of dialog presets. This is a config root.
* [SCR\_ConfigurableDialogUiPreset](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;762) - class of one preset configuration. A preset contains some properties of a dialog (dialog tag, style, message, title, buttons configuration)
* [SCR\_ConfigurableDialogUiButtonPreset](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;735) - class of one button configuration (button tag, name, label, alignment).

#### Main Classes

* [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) - the main class which represents a configurable dialog. You can override some methods in inherited classes for custom functionality. This class inherits from [ScriptedWidgetComponent](enfusion://ScriptEditor/scripts/Core/generated/UI/ScriptedWidgetComponent.c;12), and in the end gets attached to the dialog's root widget as a component. There are several ways to attach it to the widget:
  1. call cCreateByPreset() with customDialogObj left null. In this case the system creates a new c[SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) object and attaches it to the widget.
  2. call cCreateByPreset() and provide it with a customDialogObj, in this case the provided object will be attached to the widget.
  3. attach a [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16)-inherited object directly to the layout's root widget, in the layout file.
* [SCR\_ConfigurableDialogUiProxy](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;641) - this is the parent menu which holds the actual dialog widgets.

## Initialisation Sequence

The whole dialog creation and initialisation sequence is done in c[SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16).CreateFromPreset() and cCreateByPreset methods.

* cCreateFromPreset([ResourceName](enfusion://ScriptEditor/scripts/Core/generated/Types/ResourceName.c;12) presetsResourceName, [string](enfusion://ScriptEditor/scripts/Core/generated/Types/string.c;12) tag, [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16) customDialogObj = null) is called.
* the config file provided in presetsResourceName is loaded, the preset object is found by tag.
* the proxy dialog in game's MenuManager is created.
* the actual dialog widgets are created inside proxy dialog's root widget. The actual dialog layout is taken from preset's m\_sLayout property.
* a new customDialogObj is created and attached to the dialog's root, or the provided customDialogObj is used, or the one which is already attached to the layout.
* cDialog.Init is called - here we apply title, message, add buttons.
* cDialog.OnMenuOpen is called - here we expect derived class to perform the custom initialisation, if needed.

## Examples

### Basic Usage

We create a simple dialog with message. If we want to do something when a button is pressed, we can subscribe to one of the available events.

```enforce
const ResourceName DIALOGS_CONFIG = "{814FCA3CB7851F6B}Configs/Dialogs/CommonDialogs.conf";
SCR_ConfigurableDialogUi dialog = SCR_ConfigurableDialogUi.CreateFromPreset(DIALOGS_CONFIG, "timeout_ok"); // "timeout_ok" is the Tag property
// we can execute our code when dialog is closed
// useful for a common dialog which is called from many places,
// but in each place we want to run different code on dialog closure
dialog.m_OnClose.Insert(OnDialogClose);
// we can also use m_OnConfirm or m_OnCancel events
```

### Standard Usage

The dialog has some code associated only with itself, and we want that code to run regardless from where the dialog was invoked.

The takeaway here is the call signature of cCreateFromPreset:

```enforce
static SCR_ConfigurableDialogUi CreateFromPreset(ResourceName presetsResourceName, string tag, SCR_ConfigurableDialogUi customDialogObj = null)
```

the last argument is customDialogObj, which lets us tie an object to a dialog if needed.

```enforce
class SCR_ExitGameWhileDownloadingDialog : SCR_ConfigurableDialogUi
{
	//---------------------------------------------------------------------------------------------
	void SCR_ExitGameWhileDownloadingDialog()
	{
		// note that we pass 'this' into CreateFromPreset call!
		SCR_ConfigurableDialogUi.CreateFromPreset(SCR_CommonDialogs.DIALOGS_CONFIG, "exit_game_while_downloading", this);
	}
	//---------------------------------------------------------------------------------------------
	override void OnConfirm()
	{
		// try to terminate all current downloads
		SCR_DownloadManager dlManager = SCR_DownloadManager.GetInstance();
		if (dlManager)
		dlManager.EndAllDownloads();
		// exit the game
		GetGame().RequestClose();
		SCR_AllFilterSetsStorage.ResetAllToDefault();
	}
}
void SomeOtherMethod()
{
	new SCR_ExitGameWhileDownloadingDialog();
}
```

### Advanced Usage

We can create a dialog's more advanced content

```enforce
class SCR_AddonListDialog : SCR_ConfigurableDialogUi
{
	array<ref SCR_WorkshopItem> m_aItems = {};
	protected ref array<SCR_DownloadManager_AddonDownloadLine> m_aDownloadLines = {};
	protected ResourceName ADDON_LINE_LAYOUT = "{BB5AEDDA3C4134FD}UI/layouts/Menus/ContentBrowser/DownloadManager/DownloadManager_AddonDownloadLineConfirmation.layout";
	//------------------------------------------------------------------------------------------------
	void SCR_AddonListDialog(array<ref SCR_WorkshopItem> items, string preset)
	{
		foreach (SCR_WorkshopItem i : items)
		{
			m_aItems.Insert(i);
		}
		if (!preset.IsEmpty())
		SCR_ConfigurableDialogUi.CreateFromPreset(SCR_WorkshopUiCommon.DIALOGS_CONFIG, preset, this);
	}
	//------------------------------------------------------------------------------------------------
	override void OnMenuOpen(SCR_ConfigurableDialogUiPreset preset)
	{
		VerticalLayoutWidget layout = VerticalLayoutWidget.Cast(GetRootWidget().FindAnyWidget("AddonList")); // the widget we have added
		WorkspaceWidget workspace = getGame().GetWorkspace();
		// Create a line for each entry in m_aItems
		Widget w;
		SCR_DownloadManager_AddonDownloadLine comp;
		foreach (SCR_WorkshopItem item : m_aItems)
		{
			w = workspace.CreateWidgets(ADDON_LINE_LAYOUT, layout);

			component = SCR_DownloadManager_AddonDownloadLine.Cast(w.FindHandler(SCR_DownloadManager_AddonDownloadLine));
			component.InitForWorkshopItem(item, string.Empty, false);
			m_aDownloadLines.Insert(component);
		}
	}
}
```

### Common ConfigurableDialog

1. Game exit popup - MainMenuUI:  
   Binds the OnBack function to the "Back" button click

   ENFORCECODEMARKER

   ```
   // subscribe to buttons
   SCR_InputButtonComponent back = SCR_InputButtonComponent.GetInputButtonComponent("Back", footer);
   if (back)
   back.m_OnActivated.Insert(OnBack);
   ```
2. cOnBack() calls cTryExitGame():

   ENFORCECODEMARKER

   ```
   protected static void TryExitGame()
   {
   	int numCompleted, numTotal;
   	SCR_DownloadManager dlManager = SCR_DownloadManager.GetInstance();
   	if (dlManager)
   	dlManager.GetDownloadQueueState(numCompleted, numTotal);
   	if (numTotal > 0)
   	new SCR_ExitGameWhileDownloadingDialog();
   	else
   	new SCR_ExitGameDialog();
   }
   ```
3. [SCR\_ExitGameWhileDownloadingDialog](enfusion://ScriptEditor/scripts/Game/UI/Menu/CommonDialogs.c;98) and [SCR\_ExitGameDialog](enfusion://ScriptEditor/scripts/Game/UI/Menu/CommonDialogs.c;82) are defined in CommonDialogs.c, they inherit from [SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16).

   ENFORCECODEMARKER

   ```
   class SCR_ExitGameDialog : SCR_ConfigurableDialogUi
   {
   	//---------------------------------------------------------------------------------------------
   	void SCR_ExitGameDialog()
   	{
   		SCR_ConfigurableDialogUi.CreateFromPreset(SCR_CommonDialogs.DIALOGS_CONFIG, "exit_game", this);
   	}
   	//---------------------------------------------------------------------------------------------
   	override void OnConfirm()
   	{
   		GetGame().RequestClose();
   		SCR_AllFilterSetsStorage.ResetAllToDefault();
   	}
   }
   ```
4. DIALOGS\_CONFIG is the config file CommonDialogs.conf.  
   "exit\_game" is a tag in CommonDialogs that cCreateFromPreset() uses to find the right preset. cCreateFromPreset() then calls cCreateByPreset() to initialise the dialog and bind events to the buttons.

   ⓘ

   The actual widget created is ConfigurableDialogProxy, an empty overlay widget which is referenced in chimeraMenus.conf, and the new ConfigurableDialog preset is then created inside it.

   ENFORCECODEMARKER

   ```
   static SCR_ConfigurableDialogUi CreateByPreset(SCR_ConfigurableDialogUiPreset preset, SCR_ConfigurableDialogUi customDialogObj = null)
   {
   	// Open the proxy dialog
   	SCR_ConfigurableDialogUiProxy proxyComp = SCR_ConfigurableDialogUiProxy.Cast(GetGame().GetMenuManager().OpenDialog(ChimeraMenuPreset.ConfigurableDialog));
   	// Create the actual layout inside proxy
   	Widget internalWidget = GetGame().GetWorkspace().CreateWidgets(preset.m_sLayout, proxyComp.GetRootWidget());
   	if (!internalWidget)
   	{
   		Print(string.Format("[SCR_ConfigurableDialogUi] internalWidget wans't created"), LogLevel.ERROR);
   		return null;
   	}
   	SCR_ConfigurableDialogUi dialog = SCR_ConfigurableDialogUi.Cast(internalWidget.FindHandler(SCR_ConfigurableDialogUi));
   	// Create a new dialog object, or apply the provided one, if the dialog obj was not found in the layout.
   	if (!dialog)
   	{
   		if (customDialogObj)
   		dialog = customDialogObj;
   		else
   		dialog = new SCR_ConfigurableDialogUi();

   		dialog.InitAttributedVariables();
   		internalWidget.AddHandler(dialog);
   	}

   	dialog.Init(internalWidget, preset, proxyComp);
   	proxyComp.Init(dialog);
   	// Set action context
   	if (!preset.m_sActionContext.IsEmpty())
   	proxyComp.SetActionContext(preset.m_sActionContext);
   	// Call dialog's events manually
   	dialog.OnMenuOpen(preset);
   	return dialog;
   }
   ```

### Accessing the content layout

```enforce
VerticalLayoutWidget layout = VerticalLayoutWidget.Cast(GetContentLayoutRoot(GetRootWidget()).FindAnyWidget("AddonList"));
```

c[SCR\_ConfigurableDialogUi](enfusion://ScriptEditor/scripts/Game/UI/Menu/SCR_ConfigurableDialogUI.c;16).GetContentLayoutRoot() will return the first widget of the content layout. Useful for those dynamically generated dialogs.
