# [Object Import Tool](https://community.bistudio.com/wiki/Arma_Reforger:Object_Import_Tool)

| CSV Object Import |
| --- |
| [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin |
| A plugin to mass-import terrain objects |
| **File:** [SCR\_ObjectImportPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/WorldEditor/SCR_ObjectImportPlugin.c) |

The **CSV Object Import** [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin allows users to parse a CSV file to mass-import objects in the currently-selected world layer.

## CSV File Format

CSV lines must be formatted the following way:

```
resourceHash posX posY posZ quatX quatY quatZ quatW scale
```

where:

* resourceHash: the ResourceName (or {GUID}) to be placed
* posX posY posZ: the relative or absolute (depending on Relative Y setting)
* quatX quatY quatZ quatW: a [quaternion](https://en.wikipedia.org/wiki/Quaternions_and_spatial_rotation) determining Prefab's angles
* scale: the wanted Prefab's scale

⚠

* Prefab resource name MUST be in double quotes. Single quotes or no quotes **will** cause a parsing error.
* As of [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)"), only **spaces** are supported as data field separation.
* As of [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)"), the parser expects **exactly** nine fields.
