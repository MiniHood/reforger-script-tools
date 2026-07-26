# [Resource Manager: Data To Spreadsheet](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Data_To_Spreadsheet)

| Data To Spreadsheet plugin |
| --- |
| [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") plugin |
| Import/export of data to spreadsheets |
| **File:** [SCR\_DataToSpreadsheet.c;4](enfusion://ScriptEditor/Scripts/WorkbenchGame/ResourceManager/SCR_DataToSpreadsheet.c;4) |

## Description

Data to spreadsheet is a **Resource Manager** plugin which allows to export selected parameters to clipboard, which then can be pasted into various spreadsheets software like Excel, Libre Calc or Google Docs.

## Usage

## Interface

[![armareforger-resourcemanager-data-to-spreadsheet-interface.png](/wikidata/images/a/a6/armareforger-resourcemanager-data-to-spreadsheet-interface.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-interface.png)

### Prefab Config

Link to config file containing spreadsheet parser data. **Required field**

### Print To Console

When this option is enabled, some debug output will be printed to the Log Console.

### Comma As Delimiter

When enabled, importer & exporter will use comma instead of dot as delimiter. Depending on locale of PC, Excel might use one or another.

### Use Selection From Config

When enabled, Data To Spreadsheet parser will use Prefabs Array list defined in the config, instead of currently selected assets in Resource Browser

### Import

Initializes data import from clipboard to Workbench

### Export

Initializes data export from Workbench to clipboard

## Spreadsheet Parser

### Prefabs Array

List of **prefabs** or **configs** which should be parsed by the plugin. Only used if **Use Selection From Config** option is enabled

### Attributes Array

List of attributes which should be extracted. This array supports nested entries, where elements can also define sub-arrays, enabling you to reach a specified depth

#### List of attributes

##### SCR\_DataToSpreadsheetTemplatesAttribute

Extracts attribute of given name. This entry cannot have sub-entries

##### SCR\_DataToSpreadsheetTemplatesComponent

Finds Component of given name.

Example: *WeaponComponent*

##### SCR\_DataToSpreadsheetTemplatesObject

Finds Object of given name

##### SCR\_DataToSpreadsheetTemplatesObjectIndex

Finds Object by given index. In some cases you might encounter array where objects don't have name

##### SCR\_DataToSpreadsheetTemplatesObjectArray

Finds & retrieves ObjectArray

Example: *components*

### Working with Spreadsheet Parser

New spreadsheet parser config can be created by creating new config file and selecting **SCR\_DataToSpreadsheetTemplates** config. Once its created, open it in **Resource Manager** config viewer. After that, it is possible to start filling config with data

Parameters visible in Workbench are beautified in the interface so what you see in the Workbench is not how its stored or referenced by engine

[![armareforger-resourcemanager-data-to-spreadsheet-copy-params.png](/wikidata/images/3/3f/armareforger-resourcemanager-data-to-spreadsheet-copy-params.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-copy-params.png)

Use **Copy property path** to get name of property useable by **Data To Spreadsheet** plugin

#### Example: Getting Ammo data

In case you would like to grab some basic data from ammo prefabs, first you need to decide what you want to grab. In our case, parameters like **Init Speed**, **Mass**, **Air Drag** or **Diameter** are located in **ShellMoveComponent.**

Then follow below steps

* Add new **SCR\_DataToSpreadsheetTemplatesComponent** element to root **Attributes Array**
  + Type in **Component Name** field name of the component **-** in this case it is **ShellMoveComponent**
* Add new **SCR\_DataToSpreadsheetTemplatesAttribute** element to **Attributes Array** inside **ShellMoveComponent**
  + In **Attribute Name** field fill in engine name of variable (*see tool tip above about **property path***)
* Repeat those steps until list is filled with all attributes that should be parsed

*In most cases default Float attribute type is good enough - even if value is int - this is due how excel might treat it afterwards.*

[![armareforger-resourcemanager-data-to-spreadsheet-example-config.png](/wikidata/images/7/7b/armareforger-resourcemanager-data-to-spreadsheet-example-config.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-example-config.png)

#### Example: Getting Ammo data from Object Array

[![](/wikidata/images/thumb/d/d7/armareforger-resourcemanager-data-to-spreadsheet-index-retrival.png/300px-armareforger-resourcemanager-data-to-spreadsheet-index-retrival.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-index-retrival.png)

Index retrieval

In scenario where you would wish to extract i.e. **Damage** Value from **ProjectileDamage** object inside **Projectile Effects** of bullet prefab, some extra steps are necessary.

* Add new **SCR\_DataToSpreadsheetTemplatesComponent** element to root **Attributes Array**
  + Fill in **Component Name** as before (*ShellMoveComponent in this case)*
* Add **SCR\_DataToSpreadsheetTemplatesObjectArray** element to **ShellMoveComponent Attributes Array**
  + Type in **Object Name** name of array - in this case it is **ProjectileEffects**
* Add **SCR\_DataToSpreadsheetTemplatesObjectIndex** to **Attributes Array** of **ProjectileEffects**
  + Find the index of element in array you want to retrive - you can do it by for instance hovering mouse over the property. In particular case index of that class is 4
* Add **SCR\_DataToSpreadsheetTemplatesAttribute** for all relevant parameters that are supposed to be retrieved to  **Object Index: 4** element added in previous step

As a result, you should get something like this

[![armareforger-resourcemanager-data-to-spreadsheet-object-index-example.png](/wikidata/images/b/be/armareforger-resourcemanager-data-to-spreadsheet-object-index-example.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-object-index-example.png)

#### Example: Getting Ammo data from Object Array

Getting data from child components - like on picture below -  is again bit different than other methods.

[![armareforger-resourcemanager-data-to-spreadsheet-nested-component.png](/wikidata/images/9/93/armareforger-resourcemanager-data-to-spreadsheet-nested-component.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-nested-component.png)

**SCR\_DataToSpreadsheetTemplatesComponent doesn't support finding child components** so it is necessary to use different method in this case. Instead of it, it is possible to use **SCR\_DataToSpreadsheetTemplatesObjectArray & SCR\_DataToSpreadsheetTemplatesObject** classes to retrieve desired data.

* Add **SCR\_DataToSpreadsheetTemplatesObjectArray** to root **Attributes Array**
  + Type in **Object Name "components"** - that's the object array containing components under the hood. You can check the structure by opening prefab in text editor of your choice
* Add **SCR\_DataToSpreadsheetTemplatesObject** to **Attributes Array** of **components**
  + Write inside **WeaponComponent** - this way you will get reference to **WeaponComponent**
* Add **SCR\_DataToSpreadsheetTemplatesObjectArray** to **WeaponComponent**
  + Type in "**components"** again
* Add once again **SCR\_DataToSpreadsheetTemplatesObject -** this time to **components** class under **WeaponComponent**
  + Write name of component you were looking for **- MuzzleComponent**
* Add as many **SCR\_DataToSpreadsheetTemplatesAttribute** to **Attributes Array** of **MuzzleComponent** as you wish in order to extract desired parameters

*End result below*

[![armareforger-resourcemanager-data-to-spreadsheet-nested-component-example.png](/wikidata/images/b/b1/armareforger-resourcemanager-data-to-spreadsheet-nested-component-example.png)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-nested-component-example.png)

## Exporting to Spreadsheet

[![armareforger-resourcemanager-data-to-spreadsheet-toSpreadsheet.gif](/wikidata/images/b/bc/armareforger-resourcemanager-data-to-spreadsheet-toSpreadsheet.gif)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-toSpreadsheet.gif)

## Importing from Spreadsheet

[![armareforger-resourcemanager-data-to-spreadsheet-fromSpreadsheet.gif](/wikidata/images/6/69/armareforger-resourcemanager-data-to-spreadsheet-fromSpreadsheet.gif)](/wiki/File:armareforger-resourcemanager-data-to-spreadsheet-fromSpreadsheet.gif)
