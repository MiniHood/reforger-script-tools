# [Scripting: Config Object](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Config_Object)

## Config Base Container Class

A BaseContainerProps decorator tells the Workbench that this class serves as a blueprint for the Config Editor.

### BaseContainerProps

The usage of a BaseContainerProps decorator is required for the class to be visible and editable in the Config Editor - see the examples below.

The minimum required decorator is c[[BaseContainerProps](enfusion://ScriptEditor/scripts/Core/generated/Attributes/BaseContainerProps.c;50)(configRoot: true)] for the base object Config to be visible in the Config creation process.

A class inheriting from a decorated class must be decorated as well in order to be usable.

It accepts the following parameters:

```enforce
BaseContainerProps(
string category = "",
string description = "",
string color = "255 0 0 255",
bool visible = true,
bool insertable = true,
bool configRoot = false,
string icon = "",
NamingConvention namingConvention = NamingConvention.NC_MUST_HAVE_GUID)
```

A class name display is defined as follow:

* if the class uses a name instead of a GUID (see NC\_MUST\_HAVE\_NAME), the name is displayed - otherwise, the classname is
* the class can use a custom name with the usage of an additional decorator such as c[BaseContainerCustomTitle](enfusion://ScriptEditor/scripts/Core/generated/Attributes/BaseContainerCustomTitle.c;34) or one of its children

Parameters Description

| Parameter | Type | Description |
| --- | --- | --- |
| category | string | The class's category - only useful in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugins definition. |
| description | string | The class's description - only useful in World Editor plugins definition. |
| color | string | The class's colour in RGBA format (Red Green Blue Alpha) in a 0..255 range - only useful in World Editor plugins definition. |
| visible | bool | The class's visibility - only useful in World Editor plugins definition. |
| insertable | bool |  |
| configRoot | bool | Define if the class is selectable when creating a new Config. |
| icon | string | The class's icon - only useful in World Editor plugins definition. |
| namingConvention | enum | See the [NamingConvention](enfusion://ScriptEditor/scripts/Core/attributes.c;268) enum. Possible values:   * NC\_CAN\_HAVE\_NAME: Internal use. * NC\_MUST\_HAVE\_NAME: Force providing a name on object creation (whether a class or an array of them) instead of having an automatically generated GUID. The displayed object name is then the entered name. * NC\_MUST\_HAVE\_GUID: The default value - the Workbench generates a GUID for this config entry. The displayed object name is then the classname. |

### Attribute

The usage of an Attribute member variable decorator is required for the value to be visible and editable in the Config Editor - see the examples below.

The minimum required decorator is c[[Attribute](enfusion://ScriptEditor/scripts/Core/generated/Attributes/Attribute.c;12)()].

A variable's display name is deduced as follows:

m\_iMyVariable is split by removing m\_ (conventional name prefix for a member variable), i (optional, additional prefix for variable type - see [Values - Types](/wiki/Arma_Reforger:Scripting:_Values#Types "Arma Reforger:Scripting: Values")) and splitting the remaining part using the capital letters: here My Variable.

It accepts the following parameters:

```enforce
Attribute(
string defvalue = "",
string uiwidget = "auto",
string desc = "",
string params = "",
ParamEnumArray enums = null,
string category = "",
int precision = 3,
typename enumType = void
bool prefabbed = false)
```

Parameters Description

| Parameter | Type | Description |
| --- | --- | --- |
| defvalue | string | The default value. It is always a string, even in the case of a boolean ("0", "1", **not** "false"/"true"), a number ("5678", "12.34") or a vector ("0 0 0"). ⓘ  * There is **no** way to set the default value of an object * There is **no** way to set the default value of an array, only of its new item. |
| uiwidget | string | The Widget type to be used. It can be forced to whatever wanted. Note that not all widgets fit all variable types. See the [UIWidgets](enfusion://ScriptEditor/scripts/Core/attributes.c;39) class. ⓘ  In the case of an array, the value will define the UI for the items, not the array itself.   | Value | Description | Compatible With | | | | | | --- | --- | --- | --- | --- | --- | --- | | bool | float | int | string | vector | | Auto | | Attribute Type | Default UI Widget | | --- | --- | | bool | Checkbox | | int | SpinBox | | float | EditBox (numerical version) | | string | EditBox (text version) | | vector | Coords | | array | Array Widget and item type Widget | | Color | ColorPicker | | ResourceName | ResourceNamePicker | | Checked | Checked | Checked | Checked | Checked | | Hidden | Does not display the field. | N/A | | | | | | None | Identical to Hidden. | N/A | | | | | | ColorPicker | A colour modification UI. Clicking the colour rectangle will display the colour picker, allowing to set values in many ways. | c[Color](enfusion://ScriptEditor/scripts/Core/generated/Visual/Color.c;12) | | | | | | ResourceNamePicker | Allows referring to a resource of any kind. | c[ResourceName](enfusion://ScriptEditor/scripts/Core/generated/Types/ResourceName.c;12) | | | | | | ResourcePickerThumbnail | Allows previewing images and textures - to be combined with edds and/or imageset params Attribute parameter. | c[ResourceName](enfusion://ScriptEditor/scripts/Core/generated/Types/ResourceName.c;12) | | | | | | FileNamePicker | Internal use. | N/A | | | | | | ResourceAssignArray | Internal use. | N/A | | | | | | Date | Range 01/01/2000 00:00 → 31/12/2063 23:59 **UTC Time** (e.g 01/01/2000 **01:00** in Western Europe) | Unchecked | Unchecked | Checked | Unchecked | Unchecked | | Graph | Internal use. | N/A | | | | | | Font | Internal use. | N/A | | | | | | FileEditBox | Internal use. | N/A | | | | | | SpinBox | A numerical, editable up-down field. | Unchecked | Checked | Checked | Unchecked | Unchecked | | ComboBox | A combo box that opens and allows to pick one value among a selection. | Unchecked | Checked | Checked | Checked | Unchecked | | EditComboBox |  | Unchecked | Checked | Checked | Checked | Unchecked | | SearchComboBox |  | Unchecked | Checked | Checked | Checked | Unchecked | | LocaleEditBox | Internal use. | N/A | | | | | | EditBox |  | Unchecked | Checked | Checked | Checked | Unchecked | | Checkbox | Checked for true, unchecked for false. | Checked | Unchecked | Unchecked | Unchecked | Unchecked | | Slider | A click and drag allows to set the value on the slider; a click on the slider itself allows to type in the wanted value. | Unchecked | Checked | Checked | Unchecked | Unchecked | | Flags | Offers checkboxes for each enums provided. Power of two values are recommended, e.g:   ENFORCECODEMARKER   ``` enums: { 	ParamEnum("First flag", "1"), 	ParamEnum("Second flag", "2"), 	ParamEnum("Third flag", "4"), 	ParamEnum("Fourth flag", "8"), // etc } ``` | Unchecked | Unchecked | Checked | Unchecked | Unchecked | | Button | Internal use. | N/A | | | | | | Script | Internal use. | N/A | | | | | | EditBoxWithButton | Internal use. | N/A | | | | | | LODFactorsEdit | Internal use. | N/A | | | | | | Object | Clicking the "set class" button creates an instance allowing to set its values (if any). | All objects | | | | | | Coords | Clicking an axis letter and dragging left or right changes the value. | Unchecked | Unchecked | Unchecked | Unchecked | Checked | | Range | Internal only. | N/A | | | | | | Callback | Pick a callback method. ⚠  A callback method **must** be decorated with c[CallbackMethod()] and have the following signature: cvoid MethodName([Managed](enfusion://ScriptEditor/scripts/Core/proto/Types.c;112) eventParams). | cEvent | | | | | | TopLevelObject | Internal use. | N/A | | | | | | GraphDialog | A graph modification UI. Double-clicking on the graph creates a new point, click/dragging on a point moves it (no further than its surrounding ones on the X-axis), clicking on a point focuses it, and pressing `Del` removes that point. | c[Curve](enfusion://ScriptEditor/scripts/Game/Utilities/Math.c;21) | | | | | | BoundingVolume | Internal use. | N/A | | | | | |
| desc | string | The variable's description; this provides a **tooltip** when hovering the variable name and does not replace it. |
| params | string | Used to define UI-specific settings, e.g a numerical range.   * Array: "MaxSize=10" to limit the amount of possible elements * Number: "minValue maxValue step" (e.g "0 100 5" for a 0..100 range with steps of 5) * Curve: "rangeX rangeY offsetX offsetY" (e.g "50.0 80.0 10.0 20.0" for curve axes to go X:10..60 (10 + 50) and Y:20..100 (20 + 80)) * Vector: "minValue maxValue step purpose=purposeType space=spaceType anglesVar=anglesVariableName coordsVar=coordsVariableName" (e.g "0 100 5" for a 0..100 range with steps of 5) for each vector value. purpose and space are optional; using them makes step optional and allows to set the vector value in World Editor using the transformation gizmo.  ⚠  minValue and maxValue are **mandatory** in order to define purpose and space (e.g params: "inf inf purpose=coords space=entity").    + purpose possible values:     - coords - the vector property represents coordinates     - angles - the vector property represents rotations   + space possible values:     - entity - the vector represents a value in the entity's local space (relative coordinates)     - world - the vector is an absolute world position     - custom - any custom space defined by the entity   + anglesVar/coordsVar is the direct variable name, e.g m\_vAngles/m\_vRelativePos   + examples:     - inf inf purpose=coords space=entity     - -90 +90 5 purpose=angles space=world * ResourcePicker: "extension1 extension2 class=classname inheritedClasses=false"   + extension1 extension2 file extensions, e.g "dds edds imageset png" for an Image Resource Picker   + class is the Entity or config class type from which the selection must inherit, e.g "et class=AIWaypoint" for AIWaypoint entities   + inheritedClasses set to false to only have a selection from the exact class defined with class, e.g "conf class=TAG\_MyClass inheritedClasses=false" for .conf files of TAG\_MyClass class and only this class   ⓘ  * Note that setting a step for an integer does not prevent the user to type in the value manually. * It is possible to use inf (for "infinity") to specify a non-restricted range - e.g "inf inf 0". |
| enums | cParamsEnumArray | An array of [ParamEnum](enfusion://ScriptEditor/scripts/Core/attributes.c;4). Used to define ComboBox's items. It can be defined as follows:   ENFORCECODEMARKER   ``` { ParamEnum("Text 1", "Value 1", "Description"), ParamEnum("Text 2", "Value 2") } ```     The description is optional and is to fill a combobox's entry tooltip. As many as needed ParamEnum entries can exist. |
| category | string | The variable's category - only useful in World Editor plugins to categorise properties. |
| precision | int | The wanted floating point precision. Restricts a floating point field to a certain amount of decimals. |
| enumType | enum typename | The enum typename to use for the enum ComboBox/Flags UI; prefer this to c[ParamEnumArray](enfusion://ScriptEditor/scripts/Core/attributes.c;18).FromEnum(). |
| prefabbed | bool | Internal use. |
