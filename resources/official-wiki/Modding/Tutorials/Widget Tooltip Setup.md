# [Widget Tooltip Setup](https://community.bistudio.com/wiki/Arma_Reforger:Widget_Tooltip_Setup)

ScriptedWidgetTooltips is an Enfusion API for tooltips that is currently employed in core menus.
It is expanded in scripts by [SCR\_ScriptedWidgetTooltip](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/SCR_ScriptedWidgetTooltip.c;15), and allows setting up a customisable tooltip triggered by the mouse hovering a particular widget.

Most widgets in .layout files support it, which can be set in their **Behavior** section. Choose SCR\_ScriptedWidegtTooltip as the Tooltip class.

[![armar-widget tooltip setup ui.png](/wikidata/images/3/3b/armar-widget_tooltip_setup_ui.png)](/wiki/File:armar-widget_tooltip_setup_ui.png)

Once the class is set, there are two fields to provide: a .conf file and a tag. The config file must be of [SCR\_ScriptedWidgetTooltipPresets](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/SCR_ScriptedWidgetTooltip.c;527) type.

Similarly to Configurable Dialogs, the tooltip config provides modular options for tooltip's appearance and behaviour. Override the Content class to provide further customisation.

[![armar-widget tooltip config ui.png](/wikidata/images/a/a0/armar-widget_tooltip_config_ui.png)](/wiki/File:armar-widget_tooltip_config_ui.png)

The tooltips are automatically displayed on mouse hover and gamepad focus.

[SCR\_ScriptedWidgetTooltip](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/SCR_ScriptedWidgetTooltip.c;15) provides [invokers](enfusion://ScriptEditor/scripts/Game/UI/Components/WidgetLibrary/SCR_ScriptedWidgetTooltip.c;41) for different stages of a tooltip's lifetime.

```enforce
// Invokers
// Reliance on static invokers is a bit of a bandaid solution but there is no other way for other scripts to access the tooltip class
// If possible, only bind on hover/focus gained and make sure to unbind on lost.
protected static ref ScriptInvokerTooltip m_OnTooltipShowInit; // Called before creating the content widget
protected static ref ScriptInvokerTooltip m_OnTooltipShow; // Called after creating the content widget
protected static ref ScriptInvokerTooltip m_OnTooltipHide; // Called after removing the content widget
```

These are static, so make sure to check that the tooltip firing an invoker is the one your are actually interested in!
You can use the cIsValid() method, which checks for tags and optionally for hoverWidget and config file.
