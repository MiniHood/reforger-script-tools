# [Scripting: Preprocessor Directives](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Preprocessor_Directives)

Preprocessor directives allow to determine preprocessor behaviour, e.g ignoring blocks of code depending on certain conditions.

For use cases, see e.g [Scripting Temporary Feature](/wiki/Arma_Reforger:Scripting_Temporary_Feature "Arma Reforger:Scripting Temporary Feature").

ⓘ

See also [Scripting: Preprocessor Macros](/wiki/Arma_Reforger:Scripting:_Preprocessor_Macros "Arma Reforger:Scripting: Preprocessor Macros").

| Directive | Description | Example |
| --- | --- | --- |
| #define | Define a flag. A flag is either defined or not. ⓘ  A flag can be determined outside of the code; see [Startup Parameters - scrDefine](/wiki/Arma_Reforger:Startup_Parameters#scrDefine "Arma Reforger:Startup Parameters"). | ENFORCECODEMARKER   ``` #define MY_FLAG ``` |
| #ifdef | Open a preprocessor scope that is considered if the provided flag is defined. The scope must be ended by #endif (see below). | ENFORCECODEMARKER   ``` #define MY_FLAG #ifdef MY_FLAG Print("Flag is defined"); #endif ``` |
| #ifndef | Open a preprocessor scope that is considered if the provided flag is **not** defined. The scope must be ended by #endif (see below). | ENFORCECODEMARKER   ``` #define MY_FLAG #ifndef MY_FLAG Print("Flag is not defined"); #endif ``` |
| #else | Add a preprocessor scope that is of the opposite condition of the current #ifdef/#ifndef. The scope must be ended by #endif (see below). | ENFORCECODEMARKER   ``` #ifdef MY_FLAG Print("Flag is defined"); #else Print("Flag is not defined"); #endif ``` |
| #endif | Close a preprocessor scope - see #ifdef and #ifndef above. | See #ifdef and #ifndef above. |
| #include | Include another file. The effect is as if the other file's content was copy-pasted at this exact #include location. | ENFORCECODEMARKER   ``` // FileToInclude.c protected static const string MY_PRINT = "Hello there"; ```     ENFORCECODEMARKER   ``` class SCR_ScriptedClass { 	#include "scripts/Game/FileToInclude.c" 	void ShowMessage() 	{ 		Print(MY_PRINT); 	} } ``` |
