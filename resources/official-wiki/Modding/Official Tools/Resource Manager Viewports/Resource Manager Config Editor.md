# [Resource Manager: Config Editor](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Config_Editor)

ⓘ

See [Scripting: Config Object](/wiki/Arma_Reforger:Scripting:_Config_Object "Arma Reforger:Scripting: Config Object") and [Create a Config Class](/wiki/Arma_Reforger:Create_a_Config_Class "Arma Reforger:Create a Config Class") to see how to create and configure a config class.

## Main Interface

### Search Field

Search through value **names** (not values themselves).

### Class

Remind the used Config class.

### Parent

A button leading to the parent Config file, in the case of an inherited Config file.

### Values

Below these fields are the values name/UI lines. An entry that is not the default value will be **bolded**.

An entry can be reset to its default value using the arrow  that appears then to the right of the field.

## Value Interface

The value interface is composed of two columns; on the left, the value name (extracted from the property's name) and on the right, the UI to edit it - either deduced by the property's type or forced by the Config designer (see Attribute).

See uiwidget for all possible interfaces and how to interact with them.

### Array

a "+/-" interface displaying the current amount of elements between parentheses.

Press "+" to add an element, "-" to remove the highlighted one - if none is selected, the last array element is removed.

To edit the array in a more advanced way, right-click on an item's name to use contextual menu's additional options.

### Slider

Click and drag to set the value. It is still possible to click on the slider and manually enter the desired value.

## Advanced Usage

### Config Prefab from Config

Creating a Prefab Config from a config object is doable by drag and dropping the object into the Resource Browser window.

This will display the dialog to name the new file; once filled the file is created in the selected directory.

### Inherited Config File

An inherited config file is a config that will use another config file as its default values model, and obviously uses the same config class.

In order to create one, right-click on the config file wanted as model in Resource Browser, then click "Create Inherited File" - a dialog will ask to type the inherited config file name.

The created file will now have the (default) values of the model.

### Filling by Config

If a Config already exists and is compatible with the target entry, dragging and dropping the config on the field/entry will fill it with Config's values.
This is shown with the blue circle as well as the .conf button present in the value name, as shown below:

Editing from the current Config will only change and override the sub-Config values in the current Config.

To edit the sub-Config values directly, press the .conf button to open said Config.

Changes made in one tab are immediately broadcast to other tabs if such sub-Config is used.
