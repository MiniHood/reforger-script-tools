# [Field Manual Modding](https://community.bistudio.com/wiki/Arma_Reforger:Field_Manual_Modding)

The **Field Manual** is the helpful in-game manual available at all time from either the main menu or the pause menu.

Here we will see how to add our own entries.

## Config Modding

ⓘ

* the actual structure is: Root Config > Categories > SubCategories > Entries > Pieces.
* the structure itself can support an infinite amount of levels, but the UI does not.
* a Category and a SubCategory are the same [SCR\_FieldManualConfigCategory](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/SCR_FieldManualConfigCategory.c;1) object - they only differ in their hierarchy level.

### Root Config

The Field Manual's root config can be found here: [FieldManualConfigRoot.conf](enfusion://ResourceManager/~ArmaReforger:Configs/FieldManual/FieldManualConfigRoot.conf) (in Data\Configs\FieldManual).

It holds:

* the Field Manual's main title
* the tab names
* various layouts
* categories (other .conf)
* tile backgrounds from which it will select randomly

### Add a Category

A category holds:

* its title
* sub-categories

ⓘ

* A sub-category is identical to a category ([SCR\_FieldManualConfigCategory](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/SCR_FieldManualConfigCategory.c;1)) - they only differ in hierarchy level.
* Even if first-level categories can hold *entries*, they should not for the time being (they will be ignored as the UI does not support them).
* Even if second-level categories can hold *categories*, they should not for the time being (they will be ignored as the UI does not support them).

| Variable | Description |
| --- | --- |
| Enabled | This allows to enable/disable (show/hide) the category. |
| Title | This is the category's title - it will also be shown in the config entry name |
| Categories | Sub-categories to this category |
| Entries | Sub-category entries to display (the "tiles" and their content) |
| Category Layout | [FieldManual\_MenuCategory.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/MenuParts/FieldManual_MenuCategory.layout) by default |
| Sub Category Layout | [FieldManual\_MenuSubCategory.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/MenuParts/FieldManual_MenuSubCategory.layout) by default |

### Add a Sub-Category

Repeat the same operation as above in a Category instead of the config root.

### Add an Entry

The following entry types are available:

* ![Unchecked](/wikidata/images/thumb/f/f6/Ico_none.png/24px-Ico_none.png "Unchecked") [SCR\_FieldManualConfigEntry](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry.c;1) - the parent type, not to be used directly
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualConfigEntry\_Standard](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Standard.c;1) - the standard type, full width page
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualConfigEntry\_Weapon](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Weapon.c;1) - an unused weapon type with an automatically-filled weapon statistics' side panel (with rate of fire, weight, etc)

| Variable | Description |
| --- | --- |
| Enabled | This allows to enable/disable (show/hide) the entry. |
| Id | This ID is an [EFieldManualEntryId](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EFieldManualEntryId.c;2) enum constant - required only if this entry is to be opened from the outside (e.g a hint) through c[SCR\_FieldManualUI](enfusion://ScriptEditor/scripts/Game/UI/Menu/FieldManual/SCR_FieldManualUI.c;1).Open(enumValue); |
| Title | This is the entry's title, both for the tile and for the opened entry - it will also be shown in the config entry name |
| Image | The tile content. Recommended definition: 400×300. |
| Content | A list of [SCR\_FieldManualPiece](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece.c;1) used to define and setup the entry's content. |
| Weapon Entity Path | [SCR\_FieldManualConfigEntry\_Weapon](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Weapon.c;1) only: the path to the weapon's prefab (e.g [Rifle\_AK74.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Weapons/Rifles/AK74/Rifle_AK74.et)) |
| Layout | * [FieldManual\_Entry\_Standard.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Entries/FieldManual_Entry_Standard.layout) by default for [SCR\_FieldManualConfigEntry\_Standard](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Standard.c;1) * [FieldManual\_Entry\_Weapon.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Entries/FieldManual_Entry_Weapon.layout) by default for [SCR\_FieldManualConfigEntry\_Weapon](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Weapon.c;1) |

### Add Content

An Entry's content is made out of multiple pieces. All pieces are vertically set, one after another - there are no "image to the left, scroll to the right" elements.

The available piece types are as follow:

**Field Manual Entry Pieces**

* ![Unchecked](/wikidata/images/thumb/f/f6/Ico_none.png/24px-Ico_none.png "Unchecked") [SCR\_FieldManualPiece](#SCR_FieldManualPiece)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_ConfigEntry](#SCR_FieldManualPiece_ConfigEntry)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_ConfigEntryList](#SCR_FieldManualPiece_ConfigEntryList)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_Header](#SCR_FieldManualPiece_Header)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_Image](#SCR_FieldManualPiece_Image)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_ImageGallery](#SCR_FieldManualPiece_ImageGallery)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_Keybind](#SCR_FieldManualPiece_Keybind)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_KeybindList](#SCR_FieldManualPiece_KeybindList)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_LineBreak](#SCR_FieldManualPiece_LineBreak)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_Separator](#SCR_FieldManualPiece_Separator)
* ![Checked](/wikidata/images/thumb/c/c0/Ico_ok.png/24px-Ico_ok.png "Checked") [SCR\_FieldManualPiece\_Text](#SCR_FieldManualPiece_Text)

#### SCR\_FieldManualPiece

[SCR\_FieldManualPiece](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece.c;1) - the parent type, not to be used directly.

#### SCR\_FieldManualPiece\_ConfigEntry

[SCR\_FieldManualPiece\_ConfigEntry](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_ConfigEntry.c;1).

| Variable | Description |
| --- | --- |
| Display Name | The config entry text |
| Config Path | the .conf/.et file to read |
| Entry Path | the config path, in format path/to/entry (case-sensitive!) |
| Value Format | A specific format can be used along with %1 |
| Layout | [FieldManual\_Piece\_ConfigEntry.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_ConfigEntry.layout) by default |
| Decimal Move | Move the comma left (negative) or right (positive), range -9..+9, default 0 |
| Fixed Decimals | Define the amount of number's decimals, range -1..5, default -1 |

#### SCR\_FieldManualPiece\_ConfigEntryList

[SCR\_FieldManualPiece\_ConfigEntryList](#SCR_FieldManualPiece_ConfigEntryList) - a list of [SCR\_FieldManualPiece\_ConfigEntry](#SCR_FieldManualPiece_ConfigEntry).

| Variable | Description |
| --- | --- |
| Config Entries | A list of [SCR\_FieldManualPiece\_ConfigEntry](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_ConfigEntry.c;1) - see [SCR\_FieldManualPiece\_ConfigEntry](#SCR_FieldManualPiece_ConfigEntry) above |
| Layout | [FieldManual\_Piece\_ConfigEntryList.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_ConfigEntryList.layout) by default |

#### SCR\_FieldManualPiece\_Header

[SCR\_FieldManualPiece\_Header](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Header.c;1) - a text "title".

| Variable | Description |
| --- | --- |
| Image Path | Can be an .edds file or an .imageset |
| Image Set Name | If an image set has been used in **Image Path**, set the image name here |
| Text | The header's text |
| Layout | [FieldManual\_Piece\_Header.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_Header.layout) by default |

#### SCR\_FieldManualPiece\_Image

[SCR\_FieldManualPiece\_Image](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Image.c;1) - a simple image.

| Variable | Description |
| --- | --- |
| Image Path | Can be an .edds file or an .imageset |
| Image Set Name | If an image set has been used in **Image Path**, set the image name here |
| Image Color | Can be used to colour an image with a colour filter |
| Caption | The text associated to the image; be wary as some layouts cannot take a long text |
| Layout | [FieldManual\_Piece\_Image.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_Image.layout) by default |

#### SCR\_FieldManualPiece\_ImageGallery

[SCR\_FieldManualPiece\_ImageGallery](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_ImageGallery.c;1) - a gallery of [SCR\_FieldManualPiece\_Image](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Image.c;1).

| Variable | Description |
| --- | --- |
| Type | One of [SCR\_EFieldManual\_ImageGalleryType](enfusion://ScriptEditor/scripts/Game/UI/Components/SCR_FieldManual_ImageGalleryComponent.c;1), can be either ICONS\_VERTICAL, ICONS\_LIST, GALLERY\_HORIZONTAL or GALLERY\_VERTICAL |
| Text | The text displayed along the gallery |
| Images | A list of [SCR\_FieldManualPiece\_Image](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Image.c;1) - see [SCR\_FieldManualPiece\_Image](#SCR_FieldManualPiece_Image) above |
| Layout | [FieldManual\_Piece\_ImageGallery.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_ImageGallery.layout) by default |

#### SCR\_FieldManualPiece\_Keybind

[SCR\_FieldManualPiece\_Keybind](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Keybind.c;1) - a keybind information.

| Variable | Description |
| --- | --- |
| Keybind | The keybind in format `<action name="ActionName"/>` |
| Description | The keybind's description |
| Input Display Condition | One of [SCR\_EInputTypeCondition](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_EInputTypeCondition.c;1), can be either ALL\_INPUTS, KEYBOARD\_ONLY or GAMEPAD\_ONLY:  * ALL\_INPUTS: the keybind is always displayed * KEYBOARD\_ONLY: the keybind is only displayed if keyboard/mouse are currently used * GAMEPAD\_ONLY: the keybind is only displayed if gamepad is currently used   ⓘ  The UI will automatically show/hide the appropriate keybinds when controls change. |
| Layout | [FieldManual\_Piece\_Keybind.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_Keybind.layout) by default |

#### SCR\_FieldManualPiece\_KeybindList

[SCR\_FieldManualPiece\_KeybindList](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_KeybindList.c;1) - a list of [SCR\_FieldManualPiece\_Keybind](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Keybind.c;1).

| Variable | Description |
| --- | --- |
| Keybinds | A list of [SCR\_FieldManualPiece\_Keybind](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Keybind.c;1) keybinds - see [SCR\_FieldManualPiece\_Keybind](#SCR_FieldManualPiece_Keybind) above |
| Layout | [FieldManual\_Piece\_KeybindList.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_KeybindList.layout) by default |

#### SCR\_FieldManualPiece\_LineBreak

[SCR\_FieldManualPiece\_LineBreak](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_LineBreak.c;1) an empty space, used to separate elements where a bigger gap is needed.

No variables.

#### SCR\_FieldManualPiece\_Separator

[SCR\_FieldManualPiece\_Separator](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Separator.c;1) - a Reforger-themed horizontal separator

| Variable | Description |
| --- | --- |
| Padding Top | Top padding, range 0..100, default 20 |
| Width Percentage | Total parent's width, range 0..100, default 75 |
| Thickness | "Height" of the separator, range 0..10, default 2 |
| Padding Bottom | Bottom padding, range 0..100, default 20 |
| Layout | [FieldManual\_Piece\_Separator.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_Separator.layout) by default |

#### SCR\_FieldManualPiece\_Text

[SCR\_FieldManualPiece\_Text](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece_Text.c;1) - a block of text.

| Variable | Description |
| --- | --- |
| Text | The paragraph's content |
| Layout | [FieldManual\_Piece\_Text.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/Menus/FieldManual/Pieces/FieldManual_Piece_Text.layout) by default |

## Script Modding

The concerned classes are:

* **[SCR\_FieldManualUI](enfusion://ScriptEditor/scripts/Game/UI/Menu/FieldManual/SCR_FieldManualUI.c;1)** for the UI management
* [SCR\_FieldManualConfigRoot](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/SCR_FieldManualConfigRoot.c;1) for the root config
* [SCR\_FieldManualConfigCategory](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/SCR_FieldManualConfigCategory.c;1) for categories
* [SCR\_FieldManualConfigEntry](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry.c;1) for entries
  + [SCR\_FieldManualConfigEntry\_Standard](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Standard.c;1)
  + [SCR\_FieldManualConfigEntry\_Weapon](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EntryTypes/SCR_FieldManualConfigEntry_Weapon.c;1)
* [SCR\_FieldManualPiece](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Pieces/SCR_FieldManualPiece.c;1) and children for the content pieces
* [EFieldManualEntryId](enfusion://ScriptEditor/scripts/Game/FieldManual/Models/Config/EFieldManualEntryId.c;2) for the entry ID

### Open Field Manual

```enforce
EFieldManualEntryId entryId = EFieldManualEntryId.CONFLICT_OVERVIEW;
SCR_FieldManualUI.Open(entryId);
```
