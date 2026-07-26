# [Resource Manager: Layout Editor](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Layout_Editor)

## Top Bar

### Snap To Grid while dragging

Snap the moved widget to the nearest grid node.

### Draw grid

Draw the position helper grid.

### Deselect

Deselect the currently selected widget(s).

### Live Preview

Start a preview window allowing to check resolution/ratio compatibility.

### Borders

Draw an outline border around all controls. An outline will still appear on mouse hover and selection if this option is disabled.

### Grid Size

Define the grid size in pixels - the grid can only be made of squares.

### Anchor Grid Size

Set the step of Anchor's manual resize - e.g a step of **25%** will allow for a resize to 100%, 75%, 50%, 25% and 0%.

### Zoom

Click to reset zoom to 100%. Zoom can be set with **Scrollwheel Up/Down**.

### Root size

Define the base screen's resolution. Minimum officially supported resolution is 1K (1920 × 1080).

### From slot

Set the **Root size** from the slot of the root widget.

### Root DPI scaling

Allow to see how the layout looks on a different resolution.

### Language

Select a language to see if the UI fits the translated version of the text.

### Compact Mode

Set the Hierarchy view on top of Details/Import Settings tabs.

When deselected Hierarchy will appear on the left of the viewport, in its own panel.

This setting is per layout.

## Hierarchy

This panel lists widgets and their children (subwidgets).

It sports a search bar that allows to look for widgets by their name.

To rename a widget, double-click on it or use the contextual menu.

### Contextual Menu

#### Convert into…

Allow to convert the current widget to another one, keeping its children and its place in hierarchy.

#### Wrap into…

Allow to have the current widget as the child of another one, keeping its children hierarchy.

#### Expand hierarchy

Expand/unroll selected widget(s) and all their children.

#### Collapse hierarchy

Collapse/shrink selected widget(s) and all their children.

#### Copy

Shortcut: `Ctrl` + `C`.

#### Paste

Shortcut: `Ctrl` + `V`.

#### Duplicate

Shortcut: `Ctrl` + `D`.

#### Delete

Delete selected widget(s) and their children. Shortcut: `Del`.

#### Rename

Shortcut: `F2`.

#### Open prefab

Open the selected prefab widget in a new tab.

#### Save as prefab

Save selected widget as a prefab.

#### Deselect

Deselect current widget.

## Properties Tab

The properties Tab sports:

* a Search field to search by properties name,
* a widget's Name field to rename the currently selected widget,
* a Property Grid Options button.

### Property Grid Options

#### Show only modified properties

This option hide all properties that have their default value.

#### Expand all groups

Expand all property groups (not the properties themselves).

#### Collapse all groups

Collapse all property groups (not the properties themselves).

#### Copy all properties

Copy all of the widget properties.

#### Paste properties

Paste copied properties.

* Not all property groups will appear for one widget; the following documented ones are the most commonly encountered.
* It is possible to select and edit multiple widgets at once; if done so, only the properties common to all widgets will appear in the common property groups.

### Property Group Options

Right-clicking on a property group will display its contextual menu.

#### Copy group properties

Copy all properties from this group.

#### Paste properties

Paste the properties previously copied.

#### Paste properties here

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated.

### Transform

#### Slot

Can be one of:

* AlignableSlot
* ButtonWidgetSlot
* FrameWidgetSlot
* GridWidgetSlot
* LayoutSlot
* OverlayWidgetSlot
* UniformGridWidgetSlot
* WidgetSlot
* WrapLayoutWidgetSlot

##### Anchor

Set widget's behaviour on screen "resizing" - from 16/9 to 48/9 (triple screen) for example.

##### Size To Content

Tell the widget to shrink size around its actual content.

#### Z Order

Set the widget's z-index: in case of overlapping widgets, the higher z-index goes "above" or "in front of" the others.

#### Rotation

Set the clockwise angle of rotation, in degrees - usual range 0..360.

#### Pivot

Set the pivot position: default is 0.5, 0.5, the widget's centre.

### Behavior

#### Is Enabled

Set the widget enabled or not. A text widget will appear greyed out and cannot be selected, a checkbox cannot be checked, etc.

#### Is Visible

Set widget's and its children's visibility.

#### Is Pixel Perfect

Define if the widget must be drawn with pixel perfection. More expensive, but more beautiful.

#### Opacity

Set widget's and its children's opacity: 0 means fully transparent (invisible but still active), 1 means fully opaque.

#### Clipping

Set widget's clipping mode:

* **True** - Performs an intersection of this widget's rectangle with the current clipping rectangle and uses it to clip self and children.
* **False** - No clipping is performed. This widget's rectangle does not contribute to clipping at all.
* **Ancestor** - Performs clipping by current clipping rectangle, but does not add current widget's rectangle to the clipping intersection.
* **Inherit** - Take clipping from the parent. If a widget is a root (has no parent), this setting is the same as **True**.

#### Tooltip

Add a mouse-hover text tooltip.

#### Ignore Cursor

Make the selected widget(s) unresponsive to mouse actions.

### Appearance

#### Color

Set widget content's colour and opacity (RGBA).

#### Inherit Color

Set widget content's colour inheritance from the widget's parent. If activated, this setting overrides widget's **Color**.

#### Text

Set widget's text.

#### Font Size

Set widget's font size.

#### Min Font Size

Set widget's *minimum* font size - for text-resizing widgets.

### Widget

#### Style

Set widget's style between default and debugUI.

#### No focus

Make the widget unable to get focus (e.g by tab navigation).

### Script

Add scripts (Components) to the widget.

### Navigation

Define widget children's navigation.

See Widget navigation.

### Events

Add widget events, from the following:

|  |  |  |
| --- | --- | --- |
| * OnMouseEnter * OnMouseLeave * OnMouseMove * OnMouseButtonDown * OnMouseButtonup * OnMouseWheel * OnMouseDoubleClick | * OnKeyDown * OnKeyUp * OnKeyPress * OnController * OnDrag * OnDrop * OnDropReceived * OnDragging * OnDraggingOver | * OnResize * OnChildAdd * OnChildRemove * OnUpdate * OnModalResult * OnModalClickOut * OnFocus * OnFocusLost |

## Classes Tab

Choose from Basic/Layout/Special widgets and drag and drop in the viewport or in the hierarchy to create such widget.

## Last Used Prefabs Tab

Pick from the last used prefabs.
