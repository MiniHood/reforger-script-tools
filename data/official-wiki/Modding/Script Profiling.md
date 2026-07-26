# [Script Profiling](https://community.bistudio.com/wiki/Arma_Reforger:Script_Profiling)

**Script Profiling** allows to profile script performance while it is running.

## In-Game Profiler

Enable the in-game profiler from the [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") > Statistics > Script Profiler - the value can be set to frame or continuous.

## Memory Allocation

It is possible to dump all allocations from script:

* Run the game with the [-checkInstance](/wiki/Arma_Reforger:Startup_Parameters#checkInstance "Arma Reforger:Startup Parameters") startup parameter
* Call the following from script: cDumpInstances(/\* bool csvFormatting = \*/ true);
* This outputs memory allocations in the logs to the CSV format Show example

  ```
  ==== Script allocations (script side) ====
  count,class,file,line,method
  144,array<@AnimSoundEvent>,scripts/3_Game/InventoryItemType.c,16,LoadSoundEvents
  1,ActionEatTetracyclineAntibiotics,scripts/4_World/Classes/UserActionsComponent/ActionConstructor.c,123,ConstructActions
  1,Wetness,scripts/4_World/Classes/PlayerNotifiers/NotifiersManager.c,19,NotifiersManager
  55,map<string,@WidgetCacheObject>,scripts/5_Mission/GUI/WidgetCacheObject.c,16,WidgetCacheObject
  1,Param1<int>,scripts/3_Game/tools/UtilityClasses.c,24,Init
  1,CraftGhillieTop,Scripts/4_World/Classes/Recipes/Recipes/_RecipeList.inc,11,CreateAllRecipes
  1,array<@Param>,scripts/4_World/Plugins/PluginBase/PluginItemDiagnostic.c,23,PluginItemDiagnostic
  861,array<SoundObjectBuilder>,scripts/3_Game/DayZAnimEventMaps.c,34,LoadTable
  ```
