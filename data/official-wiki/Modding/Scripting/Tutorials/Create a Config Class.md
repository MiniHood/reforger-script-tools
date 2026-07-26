# [Create a Config Class](https://community.bistudio.com/wiki/Arma_Reforger:Create_a_Config_Class)

Creating a class used as .conf root is as easy as creating a standard class, with a few additions.

ⓘ

Be sure to read [Config Editor documentation](/wiki/Arma_Reforger:Resource_Manager:_Config_Editor "Arma Reforger:Resource Manager: Config Editor") as well as the [Config Object guidelines](/wiki/Arma_Reforger:Scripting:_Config_Object "Arma Reforger:Scripting: Config Object") before following this tutorial.

## Creation

### Class

Let's create a class with the wanted properties, using the [Resource Manager: Config Editor](/wiki/Arma_Reforger:Resource_Manager:_Config_Editor "Arma Reforger:Resource Manager: Config Editor") documentation:

```enforce
class TAG_SuperConfig
{
	[Attribute(defvalue: "DEFAULT VALUE", category: "Personal Details")]
	string m_sName; // member variables are not protected as this class is considered a databag
	// let's not clutter it with getters
	[Attribute(defvalue: "100", params: "1 500", category: "Damage")]
	int m_iTotalHealth;
	bool m_bOtherValue; // other properties without [Attribute()] will be ignored by the .conf UI but are still accessible by script
}
```

### Make It Root

The class exists, now to make it appear as an option in [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") > Create > Config file; all it takes is this class decorator:

```enforce
[BaseContainerProps(configRoot: true)]
```

making the end result look like:

```enforce
[BaseContainerProps(configRoot: true)]
class TAG_SuperConfig
{
	[Attribute(defvalue: "DEFAULT VALUE", category: "Personal Details")]
	string m_sName;
	[Attribute(defvalue: "100", params: "1 500", category: "Damage")]
	int m_iTotalHealth;
}
```

Now, the class can be used as config root.

## Config File Creation

1. In [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager")'s **Resource Browser**, browse to the directory in which to create the config file; then click on the **Create** button and select the **Config File** option
2. Enter the new file's name in the appearing dialog
3. Look for the wanted "BaseContainerProp" class in the next dialog
4. After clicking the classname once, the file is created from this model in the selected directory.  
   Double-click on it to open it and edit it.

## Example

See a broad SCR\_ConfigExample class:

Show Example

```enforce
[BaseContainerProps(configRoot: true)]
class SCR_ConfigExample
{
	//----------------------------------------------------------------------------------
	// basic
	//----------------------------------------------------------------------------------
	[Attribute(defvalue: "1", desc: "This is a Bool value")]
	bool m_bBoolValue;
	[Attribute(defvalue: "-1", desc: "This is an Int value")]
	int m_iIntValue;
	[Attribute(defvalue: "0.5", desc: "This is a Float value")]
	float m_fFloatValue;
	[Attribute(defvalue: "-", desc: "This is a String value")]
	string m_sStringValue;
	[Attribute(defvalue: "0 1 0", desc: "This is a Vector value")]
	vector m_vVectorValue;
	[Attribute(defvalue: "1", uiwidget: UIWidgets.CheckBox, desc: "This is a Bool Array value")]
	ref array<bool> m_aBoolArrayValue;
	[Attribute(defvalue: "String default value", desc: "This is a String Array value")]
	ref array<string> m_aStringArrayValue;
	[Attribute(defvalue: "0.008 0.365 0 1", desc: "This is a Colour value")]
	ref Color m_ColourValue;
	// does not show up - no Attribute defined
	int m_iIntValueNotShown;
	// does not show up - no set<> UI exists
	[Attribute(defvalue: "", desc: "This is a String Set value")]
	ref set<string> m_SetValue;
	// does not show up - no map<> UI exists
	[Attribute(defvalue: "", desc: "This is an Int-String Map value")]
	ref map<int, string> m_mMapValue;
	//----------------------------------------------------------------------------------
	// advanced
	//----------------------------------------------------------------------------------
	[Attribute(defvalue: "50", desc: "This is an Int value in a 0..100 range with steps of 5", params: "0 100 5")]
	int m_iIntValueMinMax;
	[Attribute(defvalue: "50", uiwidget: UIWidgets.Slider, desc: "This is an Int value using a 0..100 slider with steps of 5", params: "0 100 5")]
	int m_iIntValueSlider;
	[Attribute(defvalue: "1", uiwidget: UIWidgets.ComboBox, desc: "This is an Int value using a combobox", enums: { ParamEnum("Zero", "0"), ParamEnum("One", "1"), ParamEnum("Two", "2") })]
	int m_iIntValueComboBox;
	[Attribute(defvalue: "", uiwidget: UIWidgets.ResourcePickerThumbnail, desc: "This is an Image Resource value", params: "edds imageset")]
	ResourceName m_sImageResourcePickerThumbnail;
	[Attribute(defvalue: "", uiwidget: UIWidgets.ResourcePickerThumbnail, desc: "This is a ResourceName selection that only sees Prefabs (.et files)", params: "et")]
	ResourceName m_sFilteredResourceName;
	[Attribute(defvalue: "", uiwidget: UIWidgets.ResourcePickerThumbnail, desc: "This is a ResourceName selection that only sees Prefabs with the Vehicle parent class", params: "et class=Vehicle")]
	ResourceName m_sFilteredResourceNameByClass;
}
```

[↑ Back to spoiler's top](#bikisp6a631b73de763)
