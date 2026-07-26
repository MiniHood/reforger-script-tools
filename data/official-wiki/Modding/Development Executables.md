# [Development Executables](https://community.bistudio.com/wiki/Arma_Reforger:Development_Executables)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

The Workbench now boasts the capabilities to be connected to new special versions of the game client and server executables.
It is now possible to debug not only Workbench Game and/or Peer Tool but also proper Client and Server environments.

⚠

Trying to connect Workbench debugging tools into a non Diag executable is not possible, and non Diag executables can not connect to Diag environments.
This means that a non Diag client can not connect to a Diag server, and non Diag executables can not connect to Workbench debugging tools.

The executables that provide this functionality are suffixed by Diag on their file name, making it clear to understand what executables are compatible for debugging capabilities:

* Renamed executables:
  + ArmaReforgerWorkbenchSteam.exe is now ArmaReforgerWorkbenchSteam**Diag**.exe
* New executables:
  + ArmaReforgerSteam**Diag**.exe which is the same as ArmaReforgerSteam.exe but with debugging capabilities
  + ArmaReforgerServer**Diag**.exe which is the same as ArmaReforgerServer.exe but with debugging capabilities

    ⚠

    This affects Windows and Linux server environments.

The Diag executables provides the ability to connect the Workbench to them but also to use all the developer options for the game that can be found in the Workbench internal game, meaning that the [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") and other tools are available.
