# [Game Master: Entity Property Creation](https://community.bistudio.com/wiki/Arma_Reforger:Game_Master:_Entity_Property_Creation)

**Entity Properties** are properties the [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master") can modify by using the Edit Properties option on an entity; the properties can be fuel, health, bleeding etc.
This guide will help in setting up these Properties and how to create a custom property layout.

## Preparation

There are a few questions to be considered regarding properties:

What will the property affect/edit
:   Properties always need a replicated target. In most cases this are entities such as Characters, Vehicles and so on. In some cases for more global usage the Game Mode is the target (or the respawn component for Game End Attributes).

:   If a property targets a specific entity then it generally edits that entity's properties (like character health), whereas more global properties that use the Game Mode as a target tend to use it to make sure that all the properties are in one place but then get other components (like the [TimeAndWeatherManagerEntity](enfusion://ScriptEditor/scripts/GameCode/World/TimeAndWeatherManagerEntity.c;25) or the [RespawnComponent](enfusion://ScriptEditor/scripts/Game/generated/GameMode/RespawnComponent.c;21)) to actually change settings (so this is outside of the game mode rather then the gamemode itself).

In which mode(s) will the property be available
:   In general properties are specific to an editor mode. Like changing a player character's health can only be done in Edit mode whereas changing the player's access to modes can only be done in Admin mode.

The type of layout
:   Will the property modification UI be a drop-down, checkbox, slider or maybe even something more custom.

Have all these questions been addressed? Good! This guide will now explain the creation of a simple property as well as a custom property layout.

ⓘ

In game files, **Properties** are called **Attributes**.

## Script Creation

### Base Classes

Attribute scripts are located in Data\scripts\Game\Editor\Containers\Attributes.

| Class | Description |
| --- | --- |
| [SCR\_RespawnEnabledEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_RespawnEnabledEditorAttribute.c;5) | * Global attribute * Uses a Bool |
| [SCR\_AmbientSoundToggleEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_AmbientSoundToggleEditorAttribute.c;4) | * Global attribute * Uses a Bool |
| [SCR\_FuelEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_FuelEditorAttribute.c;4) | * Entity attribute * Uses a Float |
| [SCR\_PlayableFactionsEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_PlayableFactionsEditorAttribute.c;2) | * Button attribute * Slightly more advanced * Uses an Int as flag |

### Methods

| Method | Description | Usage |
| --- | --- | --- |
| cReadVariable() | This is where the attribute variable is created, somewhat equal to an init combined with the cShouldShow method. The attributeVar is the variable that the Attribute layout will get and set. Event though there are type-dedicated static methods (cCreateInt, cCreateBool, cCreateFloat and cCreateVector), data will always be saved as a Vector. | 1. Check if the Managed item is the item your attribute uses    * If not return null 2. Check if any other scripts that the attributes need to exist; e.g editorManager or Respawn component    * If not return null 3. If it passes the tests above then you will create your AttributeVar. Do this by using one of the following methods: ENFORCECODEMARKER      ```    return SCR_BaseEditorAttributeVar.CreateInt(YOUR_INT);    return SCR_BaseEditorAttributeVar.CreateBool(YOUR_BOOL);    return SCR_BaseEditorAttributeVar.CreateFloat(YOUR_FLOAT);    return SCR_BaseEditorAttributeVar.CreateVector(YOUR_VECTOR);    ``` 4. The attribute will now be displayed as it will return the [SCR\_BaseEditorAttributeVar](enfusion://ScriptEditor/scripts/Game/Editor/Containers/AttributeVariables/SCR_BaseEditorAttributeVar.c;1) instead of null. |
| cWriteVariable() | Similar to cReadVariable() but instead this is where you actually get the changed [SCR\_BaseEditorAttributeVar](enfusion://ScriptEditor/scripts/Game/Editor/Containers/AttributeVariables/SCR_BaseEditorAttributeVar.c;1) to set it again. Note that AttributeVars that were not changed by the user will never get their cWriteVariable() called. The system will handle this. | 1. First add a safety if the var exists. ENFORCECODEMARKER      ```    if (!var)    return;    ``` 2. Same as the checks for any scripts that this attribute needs which you checked in cReadVariable()    * If you needed editorManager then once again check if it exists else return 3. Once that is done you can use the AttributeVar to get the get variable again you set in a similar way: ENFORCECODEMARKER      ```    var.GetInt();    var.GetBool();    var.GetFloat();    var.GetVector();    ``` 4. Use these var to change anything in the component/entity you want |
| cGetEntries() | Use this to send over any additional data for the attribute. Think of the buttons that need to be displayed or the min, max and steps of the slider. Note that this is not replicated even if using the IsServer value (See [Config Modding](#Config_Modding) below) | Most existing scripts already handle this data as they inherit from:  * [SCR\_BasePresetsEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BasePresetsEditorAttribute.c;4) - used for buttons * [SCR\_BaseMultiSelectPresetsEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseMultiSelectPresetsEditorAttribute.c;3) - used for multi select buttons * [SCR\_BaseFloatValueHolderEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseFloatValueHolderEditorAttribute.c;2) - used for drop downs and Spin boxes * [SCR\_BaseValueListEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseValueListEditorAttribute.c;4) - used for sliders |
| cPreviewVariable() | Is called every time the variable is set via the user input; use this if you want to display previews | See [SCR\_WeatherInstantEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_WeatherInstantEditorAttribute.c;4) how it is used there |

## Layout Selection

Most of the times, the default layouts will be enough to cover an attribute.
Attribute layouts are located in UI\layouts\Editor\Attributes\AttributePrefabs.

| Layout | Variable Type | Description |
| --- | --- | --- |
| [AttributePrefab\_Checkbox.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_Checkbox.layout) | Bool | A simple true/false checkbox |
| [AttributePrefab\_Slider.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_Slider.layout) | Float | A slider which sets a float |
| [AttributePrefab\_Dropdown.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_Dropdown.layout) | Int | Gets an array of elements and uses int as index and only one element can be selected |
| [AttributePrefab\_Spinbox.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_Spinbox.layout) | Int | Gets an array of elements and uses int as index and only one element can be selected |
| [AttributePrefab\_ButtonBox\_Selection.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_ButtonBox_Selection.layout) | Int | Gets an array of elements and uses int as index and only one element can be selected |
| [AttributePrefab\_ButtonBox\_MultiSelection.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/AttributePrefabs/AttributePrefab_ButtonBox_MultiSelection.layout) | Vector | A bit more complicated as it uses flags to allow multiselection of elements (buttons) |

### Layout Creation

1. Create a new layout
2. Drag in [Attributes.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/Attributes.layout) as a child. This takes care of all the sizing, descriptions, locking and unlocking attributes when multi editing entities and so on.
3. Locate the AttributeHolder widget within the Hierarchy. This is where we will be adding the actual unique attribute.
4. The attribute system uses the widget library, so the [WLib\_Slider.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/WidgetLibrary/WLib_Slider.layout) slider will be used for the example. Drag this to the new attribute layout and set it as a child of the AttributeHolder Widget.

   ⓘ

   It is also possible to create a more custom layout; check [Date.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Editor/Attributes/Date/Date.layout) for a very custom attribute, although it still uses the widget library for the elements within it.
5. It is time to add the layout component to the widget's root - the Slider one already exists and we can add it: [SCR\_SliderEditorAttributeUIComponent](enfusion://ScriptEditor/scripts/Game/Editor/UI/Components/Attributes/SCR_SliderEditorAttributeUIComponent.c;3). Note that each attribute variable type will have its layout script and you might want to have your own custom script here.
6. Lastly we will need to fill in the variables. Some of them are already filled in.

:   :   | Variable | Description |
        | --- | --- |
        | UI Component Name | This is the name of the widget added to the Attribute Holder - "SliderRoot0" in the case of our slider |
        | Conflicting Attribute UI Info | Set the Class by pressing the button and fill it in the same as the image or use the drop down below to fill in the the variables of the UIInfo. The system will complain if these fields are not filled in as well as if it is not a global attribute. Copy the following to the UI info to make sure it is filled in correctly. You can also check the existing attributes as examples.   * **Description**: #AR-Editor\_AttributeConflicting\_Description * **Icon**: [Attribute\_Locked.edds](enfusion://ResourceManager/~ArmaReforger:UI/Textures/Editor/Attributes/Attribute_Locked.edds)   ⓘ  This system may be refactored, changed or removed in the future as it needs to be filled for every new attribute and it is prone to mistakes. |

## Layout Scripting

We will go over some methods of the layout script. Each of the above layouts have their own scripts that inherit from [SCR\_BaseEditorAttributeUIComponent](enfusion://ScriptEditor/scripts/Game/Editor/UI/Components/Attributes/SCR_BaseEditorAttributeUIComponent.c;3).

| Method | Description |
| --- | --- |
| cInit() | This is where the system can get any data sent *via* the cGetEntries() method |
| cSetVariableToDefaultValue() | This is the value displayed if multi selecting entities and those entities have conflicting values for the same attribute |
| cSetFromVar() | This is called on init and reset and sets the attribute UI to the actual value of the AttributeVar |
| cOnChange() | Use this method to get and then set the AttributeVar eg: cvar.SetBool(x); |

## Config Modding

### Add to Config

With the knowledge above you can create a basic Bool attribute.
In Script Editor, create a new attribute script which inherits from [SCR\_BaseEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseEditorAttribute.c;2).
Take [SCR\_RespawnEnabledEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_RespawnEnabledEditorAttribute.c;5) as an example (cReadVariable() and cWriteVariable() methods only).
Once the attribute is made, locate the config to which you want to add the attribute.
Locate the correct config to which to add the attribute; attribute configs are located in Data\Configs\Editor\AttributeLists.

| Config | Description |
| --- | --- |
| [Edit.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/AttributeLists/Edit.conf) | The default editor mode a.k.a [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master") mode. This is typically the mode to which you want to add most of the new attributes. The fact that any player can technically become Game Master must be considered. |
| [Photo.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/AttributeLists/Photo.conf) | This is the photomode all players (by default) have access to. It typically has only global attributes. |
| [Admin.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Editor/AttributeLists/Edit.conf) | This is the admin mode to which only server admins (host or config-declared admins) have access. Here should be added any attribute that should be handled with care, e.g setting mode access for all players. |

Open the config, click the **+** button and select the wanted attribute class.

### Fill UI Info

Press the "Set Class" button to create the UI Info for the attribute.

| Variable | Description |
| --- | --- |
| Name | The name of the attribute |
| Description | Displayed when focusing on the attribute to explain what the attribute does |
| Icon | Only used if using the cOverrideDescription(), this will display an icon in front of the description |
| Description Icon Color | Used in combination with Icon and sets the color of said icon when displayed |

### Additional Properties

The following are some additional attribute variables that are required as well as some more advanced attributes and their variables.

| Variable | Description |
| --- | --- |
| Is Server | If the cReadVariable() and cWriteVariable() are only called on server. Take note that it does not affect most other methods, those are still called on client like cGetEntries() and cPreviewVariable(). |
| Category Config | This determents which tap in the attribute UI the attribute is displayed. Currently all attribute categories are listed in Data\Configs\Editor\AttributeCategories |
| Layout | This is the layout as described in the steps above |

#### Slider

Inherit from [SCR\_BaseValueListEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseValueListEditorAttribute.c;4) to easily use sliders - this will provide the following variables:

| Variable | Description |
| --- | --- |
| Slider Value formating | How the slider value will be displayed, %1 being the float value |
| Min | The minimum value of the slider |
| Max | The maximum value of the slider |
| Steps | Each step of the slider will add/remove this amount |
| Decimals | When displaying the value how many decimals are shown |

Note that theses values can be set within the cGetEntries() so you do not need to fill them in the config.

#### Dropdown/Spinbox

Inherit from [SCR\_BaseFloatValueHolderEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseFloatValueHolderEditorAttribute.c;2) for Dropdowns and Spinboxes.
This will give you a Values array. Each element in the array correspond with an element in the Dropdown/Spinbox.
As with all these you do not need to fill in the array within the config but instead dynamically fill it in within the cGetEntries() method.

| Variable | Description |
| --- | --- |
| Icon | Not supported by default for dropdowns and spinboxes |
| Entry name | Name of element as displayed in dropdown/spinbox |
| Entry Description | Not supported by default for spinboxes and dropdowns |
| Float value | This value can be obtained within the cWriteVariable() using the cAttributeVar.GetInt as index |

ⓘ

Check the cGetEntries() method in [SCR\_WindDirectionEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_WindDirectionEditorAttribute.c;4) to see a dynamically filled-in array, and check [SCR\_KillfeedTypeEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_KillfeedTypeEditorAttribute.c;1) to see how to set up an Enum array that does not count up from 0 as by default the attribute works with indexes and not enum values.

#### Selection Button

A selection button only allows for one button to be selected at the time. Inherent from SCR\_BasePresetsEditorAttribute to use this.

| Variable | Description |
| --- | --- |
| Values | See [Dropdown/Spinbox](#Dropdown/Spinbox) as it is mostly the same. It is highly advised to fill this dynamically. Use cCreatePresets() for this. |
| Values > Icon | Icon of element within the button if "Use Icon" (below) is true |
| Values > Description | If hovering over the button this is the description given. This means the user can get some more information about what each button does. Only shown if Has "Button Description:  (below) is true. And will be %1 within the "Button Description" string (below) |
| Buttons On Row | How many buttons will be displayed on one row |
| Has Randomise Button | Will have a randomise button that if selected will select any of the other buttons at random |
| Randomise button Icon | The icon used for the randomise button |
| Use Icon | It will use the Icon used in the Values array if true. Else it will add the Name as text |
| Button Description | The description shown when hovering over buttons. %1 param that is filled in with the Element Description |
| Button Height | Height of the buttons |

#### Multiselection Button

This is the most complicated of the attribute types as it uses flags to set values, though there is some support for this.
Inherit from [SCR\_BaseMultiSelectPresetsEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_BaseMultiSelectPresetsEditorAttribute.c;3) which has methods to convert an array of bools to flags and converting the flags back to a readable array of bools.
Check [SCR\_PlayableFactionsEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_PlayableFactionsEditorAttribute.c;2) for a use case. The most important part is that csuper.ReadVariable() and csuper.WriteVariable() need to be called at the right locations.

You can also use custom flags instead of an array of bools. See [SCR\_ModesEditorAttribute](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/SCR_ModesEditorAttribute.c;4) and the layout component [SCR\_OverrideModesEditorAttributeUIComponent](enfusion://ScriptEditor/scripts/Game/Editor/UI/Components/Attributes/SCR_OverrideModesEditorAttributeUIComponent.c;3) for a use case.

We will not go into further details, just know that the variables in the config are the same as [Selection Button](#Selection_Button) - only the layout is different.

### Dynamic Description

The Dynamic Description system changes the attribute's description depending on its value.
It is great if you want to warn the player that something major will happen if the attribute changes are applied, like setting a character's blood to 0 will kill the character.
It is possible to have multiple descriptions.

#### Variables

Some base variables to highlight:

| Variable | Description |
| --- | --- |
| Description Display info | This is similar to the attribute UI info and the variables: description, Icon and color are used to display the description. Only Description is required. |
| Enabled | If false, the description will never show. |
| Only Show if Condition Changed | This is available for most base attributes (though not always available for custom) If true it will only set the description if the value changed. If you open the attributes and the Dynamic description is valid to display it will not if this is set to true, only when it is set to it. |

#### Classes

|  |  |
| --- | --- |
| [SCR\_BoolAttributeDynamicDescription](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/DynamicDescriptions/SCR_BoolAttributeDynamicDescription.c;4) | Shows a custom description if the bool value is met (either true or false) and mainly used for checkboxes. |
| [SCR\_ValueAttributeDynamicDescription](enfusion://ScriptEditor/scripts/Game/Editor/Containers/Attributes/DynamicDescriptions/SCR_ValueAttributeDynamicDescription.c;4) | Shows a custom description if a specific value is met. This could be anywhere to if the value equals, greater than, less than etc. |
