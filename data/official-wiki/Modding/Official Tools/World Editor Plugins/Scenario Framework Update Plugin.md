# [Scenario Framework Update Plugin](https://community.bistudio.com/wiki/Arma_Reforger:Scenario_Framework_Update_Plugin)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

With the 1.1.0 update, [huge changes](/wiki/Arma_Reforger:Scenario_Framework#1.1.0_Changes "Arma Reforger:Scenario Framework") were introduced about how AIs and their Waypoints are handled by the Scenario Framework.  
This results in a backward compatibility impossibility for the scenarios created with a previous version.

This [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin does the conversion in an automated way, provided its usage steps are respected.

## Usage

1. Back up your work before loading your world (It is good practice to regularly back up your work whenever you can so you can always go back/fix things when something breaks)
2. Load the Scenario Framework world to be updated
3. Navigate to the top bar → Plugins → Update/1.0.0 to 1.1.0 and run phase 1
4. Save the world
5. **Load the World again** (important!)
6. Navigate to the top bar → Plugins → Update/1.0.0 to 1.1.0 and run phase 2
7. Save the world again to apply all changes.

The scenario is now 1.1.0-compatible and can be worked on again.

⚠

Running the plugin on a 1.1.0-compatible scenario is not recommended and its behaviour is undocumented (although it *should not* be an issue, no guarantees are offered).
