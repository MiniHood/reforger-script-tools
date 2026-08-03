# [Script Editor: Class Renaming Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_Class_Renaming_Plugin)

| Class Renaming |
| --- |
| [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") plugin |
| A plugin to rename a class in scripts and Prefabs |
| **File:** [SCR\_ClassRenamingPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_ClassRenamingPlugin.c) |

**Class Renaming** is a plugin that replaces a string by another in scripts and Prefabs (not terrain layers *yet*).

⚠

This plugin can break addons if used carelessly, be sure to make a backup before using it!

## Usage

Use it with Plugins > Class Renaming Plugin; a window appears allowing to set replacement values and the concerned directories.

## Parameters

### Renaming

* **Only Rename Existing Editable Classes** - only rename classes that exist in script files and that can be modified; if unchecked, replace all words that match criteria
* **Class Must Start With A Capital Letter** - only rename words/classes that begin with a capital letter
* **Process Script Files** - process .c files in the provided directories
* **Process Prefab Files** - process .et files in the provided directories
* **Demo Mode** - read-only mode that goes through all the steps without overwriting files
* **Parameters** - a list of "from X to Y" replacement rules {{Feature|informative|Only the first matching rule applies
  + The **From** field supports prefix and suffix wildcards:
    - word means exact match of "word"
    - word\* means "word" as a prefix (e.g "**word**ing")
    - \*word means "word" as a suffix (e.g "pass**word**")
    - \*word\* means inside a word (e.g "pass**word**ing" - does **not** match "word")
    - any other combination is **not** supported (e.g wor\*d to try and match "wor**l**d")

### Directories

* **Script Directories** - in which directories renaming can happen in script files (.c)
* **Prefab Directories** - in which directories renaming can happen in Prefab files (.et)
