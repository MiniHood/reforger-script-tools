# [Animation Editor: Live Debug Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Live_Debug_Tutorial)

Live animation debugging can be useful to observe and record in-game animation evaluation.

Here is how to use it in few steps:

* Start the scenario in **World Editor** or in the game itself.
  + [MpTest world](enfusion://ResourceManager/~ArmaReforger:worlds/MP/MpTest.ent) is a good candidate for isolated tests
* Open the **Animation Editor** and load animation workspace you would like to debug.
  + If you are trying to debug some **entity** *(like weapon)*, make sure that it is present in the world
* In **Live Debug**, press **Fetch**. This will list all available instances and then you can **Attach** to one of the returned.
* The editor should now record whatever happens to animation in-game. You can pause and rewind, check the set variables, commands and returned events and tags.

## Troubleshooting

* The animation debugging can be disabled. You can jumpstart it in [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") (win+alt) under **Animation > Start Debugger**. Triggering the variable starts the debugger and the variable returns to false.
* Game is not updating when alt-tabbed to editor. This way it cannot submit any reports to the editor. Make sure that the game runs with [forced updates on](/wiki/Arma_Reforger:Startup_Parameters#forceUpdate "Arma Reforger:Startup Parameters").
