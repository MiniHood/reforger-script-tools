# [Script Editor: SVN Plugins](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor:_SVN_Plugins)

| SVN Plugins |
| --- |
| This plugin is available in:  * [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") * [Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") |
| |  |  | | --- | --- | | SVN Blame | `Alt` + `⇧ Shift` + `B` *[Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") only* | | SVN Diff | `Alt` + `⇧ Shift` + `I` *[Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") only* | | SVN Log | `Alt` + `⇧ Shift` + `L` | |
| VCS-related shortcut commands |
| **File:** [SCR\_SVNPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_SVNPlugin.c) |

SVN plugins are keyboard shortcuts to SVN commands - more precisely [TortoiseSVN](https://tortoisesvn.net/) commands by default.
They are defined in [SCR\_SVNPlugin.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ScriptEditor/SCR_SVNPlugin.c).

ⓘ

The plugins are named [SVN](https://subversion.apache.org/) but can absolutely be used with another VCS software like [Git](https://git-scm.com/) (e.g [Git Extensions](https://gitextensions.github.io/)), provided commands are replaced in the plugins's options.  
They will be renamed to **VCS** plugins in [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.3.0 "Category:Arma Reforger/Version 1.3.0") [1.3.0](/wiki?title=Category:Arma_Reforger/Version_1.3.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.3.0 (page does not exist)").

## Commands

Commands can be anything and have two possible parameters:

* $path: replaced by the absolute file path between double quotes ("), e.g "C:\Users\John Bob\MyFile.c".
* $line: replaced by the current cursor position's line number.

| Name | Shortcut | Description | Command |
| --- | --- | --- | --- |
| *SVN* Blame | `Alt` + `⇧ Shift` + `B` | List the last author of each line, to find the culprit | ``` TortoiseProc /command:blame /path:$path /startrev:1 /endrev:-1 /ignoreeol /ignoreallspaces /line:$line ```  ``` gitex blame $path ``` |
| *SVN* Diff | `Alt` + `⇧ Shift` + `I` | List the changes between the repository and the local file | ``` TortoiseProc /command:diff /path:$path ```  ``` gitex difftool $path ``` |
| *SVN* Log | `Alt` + `⇧ Shift` + `L` | Show the file's changes commit history | ``` TortoiseProc /command:log /path:$path ```  ``` gitex filehistory $path ``` |

## See Also

* [SVN](https://subversion.apache.org/)
  + [TortoiseSVN](https://tortoisesvn.net/)
* [Git](https://git-scm.com/)
  + [Git Extensions](https://gitextensions.github.io/)
  + [TortoiseGit](https://tortoisegit.org/)
