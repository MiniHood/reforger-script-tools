# [Scripting: Preprocessor Macros](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Preprocessor_Macros)

Preprocessor macros provide helpful context information, especially useful in debug.

ⓘ

See also [Scripting: Preprocessor Directives](/wiki/Arma_Reforger:Scripting:_Preprocessor_Directives "Arma Reforger:Scripting: Preprocessor Directives").

| Macro | Description | Example |
| --- | --- | --- |
| \_\_FILE\_\_ | Is replaced by a string containing the current file's relative path. | ENFORCECODEMARKER   ``` Print(__FILE__, LogLevel.NORMAL); // ends as e.g Print("scripts/WorkbenchGame/ScriptEditor/TAG_MyTestPlugin.c", LogLevel.NORMAL); // this is absolutely valid string absPath; Workbench.GetAbsolutePath(__FILE__, absPath, true); Print(absPath); // e.g "D:/MyMods/TAG_MyMod/scripts/WorkbenchGame/ScriptEditor/TAG_MyTestPlugin.c" ``` |
| \_\_LINE\_\_ | Is replaced by a **string** containing the current file's line number. | ENFORCECODEMARKER   ``` Print(__LINE__ + 2, LogLevel.NORMAL); // ends as Print("4" + 2, LogLevel.NORMAL); // "42" ``` |
