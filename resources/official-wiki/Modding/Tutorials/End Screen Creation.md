# [End Screen Creation](https://community.bistudio.com/wiki/Arma_Reforger:End_Screen_Creation)

The End Screen (interchangeably called Game Over screen) are screens shown when the game ends. This guide will go through each part and how to set up your own game over screen.

The end screen consists out of various parts:

| Element | Type | Description |
| --- | --- | --- |
| cGameMode.EndGameMode() (from [SCR\_BaseGameMode.c](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_BaseGameMode.c)) | Method | The end screen is called by means of the cGameMode.EndGameMode() method and uses the data provided, combined with the local player data to decide what to display. |
| [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4) | Data | Data used in the cEndGameMode method combined with wining player(s) and winning faction(s). See the script itself as well as the c[SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4).Create() and c[SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4).CreateSimple() to get more information as to how to call end game. |
| [EndScreen.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/HUD/GameOver/EndScreen/EndScreen.layout) | Layout | This is the base layout which fades in when game over is called. The specific game over widgets are spawned within this layout. |
| [GameOverScreensConfig.conf](enfusion://ResourceManager/~ArmaReforger:Configs/GameOverScreen/GameOverScreensConfig.conf) | Config | This holds all the gameover screens and is referenced to by various scripts to get the specific information that needs to be shown such as but not limited to: The specific layout spawned within the [EndScreen.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/HUD/GameOver/EndScreen/EndScreen.layout), the localised title, subtitle, briefing, the icon shown, audio played as well as various optional parameters. |
| [EndScreenContent\_Default.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/HUD/GameOver/EndScreen/EndScreenContent_Default.layout)  [EndScreen\_NoImage.layout](enfusion://ResourceManager/~ArmaReforger:UI/layouts/HUD/GameOver/EndScreen/EndScreenContent_NoImage.layout) | Layout | These are two standard layouts used in the End Screens in the config. These are spawned within the EndScreen.layout and hold the actual visuals. (It is possible that there are more layouts by the time you are reading this). Use these as an example if you want to display any specific information like: Score or some custom screen. **Create your own if you want to modify the end screen** (Keep in mind that changing the basic ones will change it for all game modes). Do take note that some widgets like text widgets and the image will need specific naming, though leaving them out will not cause any issues. |
| [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1) | Script | This is the main widget component found on the EndScreen\_Default and EndScreen\_NoImage layout. If you want to do something special with the layout then it would be advised to inherit from this component and override the methods. |
| [EGameOverTypes.c](enfusion://ScriptEditor/scripts/Game/GameOverScreen/EGameOverTypes.c) | Enum | This hold the enums used as identifiers within the config. Config endscreens mostly use this as an identifier for developers but any editor or custom endscreen will need a specific unique enum. |

## Config Modding

### Add New End Screen

* Open [GameOverScreensConfig.conf](enfusion://ResourceManager/~ArmaReforger:Configs/GameOverScreen/GameOverScreensConfig.conf)
* Press the '+' to start creating a new End Screen. Here you will be presented with a few options.  
  These are the options by the time of writing this guide:

:   | Class | Description |
    | --- | --- |
    | [SCR\_BaseGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_BaseGameOverScreenInfo.c;1) | Basic game over screen to set a title, subtitle, briefing and image. If you create any new End Screens this is the base you will inherent from |
    | [SCR\_FactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionGameOverScreenInfo.c;1) | If the end condition contains one or more factions then this allows you to get the faction flag for the image and faction name as parameter in the Subtitle |
    | [SCR\_FactionVictoryGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionVictoryGameOverScreenInfo.c;1) | Similar to [SCR\_FactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionGameOverScreenInfo.c;1) but also sets the Vignette color to the winning faction |
    | [SCR\_EditorFactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_EditorFactionGameOverScreenInfo.c;1) | Similar to the above but supports multiple winning factions which by default it does not. |
    | [SCR\_DeathMatchGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_DeathMatchGameOverScreenInfo.c;1) | Used to get the wining player name(s) and show it as params in the Subtitle. Note that by the time of writing, this is not used. |

    Chose any of the above scripts, though if you want to make it fully custom simply create your own or use [SCR\_BaseGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_BaseGameOverScreenInfo.c;1) and handle any other logic by inheriting and overriding the [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1).

### Main Variables

The following variables must be filled in:

| Variable | Description |
| --- | --- |
| Game Over Content Layout | This is the actual layout spawned within the game over screen. You can use [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1) (or an inherited version). More information about the layout in the steps below. |
| Game Over Screen Id | The ID of the game over screen. Make sure it is an unique enum which you added to the [EGameOverTypes](enfusion://ScriptEditor/scripts/Game/GameOverScreen/EGameOverTypes.c;1) enum. |

{{Feature|informative|Some of the End screen types such as [SCR\_EditorFactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_EditorFactionGameOverScreenInfo.c;1) have additional variables (see [Optional Variables](#Optional_Variables) and [Optional Game Master Variables](#Optional_Game_Master_Variables) below). As usual, hover over the description to read what they mean.

### Optional Variables

The following are optional parameters which affect various base UI elements. To use this Press the **Optional Params**' set class button.

ⓘ

Note that in order to use most of these optional variables, the layout you used as "Game Over Content Layout" will need to have the Text/Image components with the correct name as set in the [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1).

| Variable | Description |
| --- | --- |
| Title | This is the localised string for the title. |
| Subtitle | The localised string for the subtitle. |
| Image Texture | What image will be shown. Note that using [SCR\_FactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionGameOverScreenInfo.c;1) will show the players faction, or in case of [SCR\_FactionVictoryGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionVictoryGameOverScreenInfo.c;1) will show the faction that won. |
| Debriefing | The localised string for debriefing text |
| Audio One Shot | The audio music one shot that will be played upon the opening of the end screen. |
| Title Param | %1 param shown in Title |
| Subtitle Param | %1 param shown in Subtitle. This is the player faction's name if using the [SCR\_FactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionGameOverScreenInfo.c;1) or the winning faction's name if [SCR\_FactionVictoryGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionVictoryGameOverScreenInfo.c;1). |  |
| Debriefing Param | %1 for debriefing param |

ⓘ

In most cases the Title, Subtitle and Debriefing params are actually set by script (such as [SCR\_FactionGameOverScreenInfo](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_FactionGameOverScreenInfo.c;1)) instead of being set directly in the config.

### Optional Game Master Variables

In most cases you would want to make sure your end screen is compatible with [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master"); one thing you need to keep in mind is that there can be more than one winning faction in Game Master.

Similarly to the [Optional Variables](#Optional_Variables) above press the set class button to add Game Master compatibility. If you leave this null then the game will never allow Game Masters to access this particular End Screen.

| Variable | Description |
| --- | --- |
| Can Be Set By Game Master | Set to true if you want the Game Master to specifically be able to set this End Screen. You can leave this false, see the 'Mirrored State' variable below for more information. |
| Needs Playable Factions | Some End screens such as FactionVictory will need factions to be set, while others like EndSession do not need any factions. If your end screen checks factions then make sure this is true. Only relevant if 'Can Be Set By Game Master' is true. |
| Display Name | This is the localised string name seen by the Game Master when selecting the End Screen. Only relevant if 'Can Be Set By Game Master' is true. |
| Description | This is the localised string description the player sees when selecting the End Screen. Only relevant if 'Can Be Set By Game Master' is true. |
| Description Param1 and Description Param2 | %1 and %2 of Description. Generally this is set through script rather then in the config. Only relevant if 'Can Be Set By Game Master' is true. |
| Mirrored State | In some cases you don't want the Game Master to be able to choose the End Screen directly. Think of Defeat End Screen if there is already a Victory End Screen. In this cause you set 'Can Be Set By Game Master' true for victory and false for defeat and set the Mirrored state of Victory to Defeat and *vice versa*.  If the Victory End screen is chosen then any faction that is not selected will automatically get the Defeat End Screen.  ⓘ  Currently mirrored states are only supported with factions. Any player of the faction that is not selected will get the mirrored state end screen. This is mainly because only factions are supported in end screens. |

## Layout

The End screen layout is the actual layout shown when the end screen is called and **needs to be set to the 'Game Over Content Layout' variable in the config**. Most things are already explained in the steps above. Here though you can go in the most details for your custom End Screen.

The following Table will show all the Default widgets which are affected by the config element we set up before. Do not worry though, you can delete the widgets you do not need as the system will check if they exist. Also the widget names are the default names and can be changed in the [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1) class.

| Widget Name | Widget Type | Target | Description |
| --- | --- | --- | --- |
| GameOver\_Image | ImageWidget | Image Texture | The image shown in the end screen |
| GameOver\_State | TexWidget | Title | The title shown in the end screen |
| GameOver\_Condition | TextWidget | Subtitle | The Subtitle Shown in the end screen |
| GameOver\_Description | TextWidget | Debrief | The Debrief shown in the end screen |

## Scripting

### Layout Component

If you create a new End screen layout then do make sure it has the [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1) or an inherited version of it.

The [SCR\_GameOverScreenContentUIComponent](enfusion://ScriptEditor/scripts/Game/UI/GameOverScreen/SCR_GameOverScreenContentUIComponent.c;1) is the base component used to initialise the end screen. If you want to make a fully custom end screen simply inherent from this script. Simply override the cInitContent() method to initialise the end screen. This method will give you the [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4) as well as all the data set in the config as mentioned above. You can always use csuper.InitContent() in the method if you want to keep the base functionality.

### Screen Manager Component

The [SCR\_GameOverScreenManagerComponent](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_GameOverScreenManagerComponent.c;7) handles things like displaying the right endscreen depending on local player faction and winning faction but you can also handle this within the layout script as well.
You will need to set up the End screen in the [SCR\_GameOverScreenManagerComponent](enfusion://ScriptEditor/scripts/Game/GameOverScreen/SCR_GameOverScreenManagerComponent.c;7) specifically in the cGetGameOverType() method. It is a bit convoluted as the GameOverData and the Game Over Screens are not very compatible in some causes. It mostly is about factions (or winning player Id) and if the local player should get the victory or defeat game end screen.

Either use the existing code as a guide or use the code below to ignore this section altogether:

```enforce
else if (endReason == EGameOverTypes.YOUR_ENUM)
{
	// your logic
	return endReason;
}
```

ⓘ

This system may change in the future.

### End the Game

The last step is to actually make sure to call your end game.

Use the cGameMode.EndGameMode() to actually end the game. This method takes a new [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4) which you can create with the c[SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4).CreateSimple() and c[SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4).Create() static methods.

Create Simple method params:

| Variable | Description |
| --- | --- |
| int reason | The end reason. Generally if using a custom end screen you will be using the [EGameOverTypes](enfusion://ScriptEditor/scripts/Game/GameOverScreen/EGameOverTypes.c;1) enum you created as the reason. Check the [SCR\_GameModeEndData](enfusion://ScriptEditor/scripts/Game/GameMode/SCR_GameModeEndData.c;4) class for more in-depth information. |
| int winnerId | The winner player ID. Leave -1 if no winning player |
| int winnerFactionId | The winning faction index. Leave -1 if no winning faction |

Create method params:

| Variable | Description |
| --- | --- |
| int reason | Same as Create Simple |
| array<int> winnerIds | An array of winning players. Leave null if no winning players. |
| array<int> winnerFactionIds | An array of wining factions. Leave null if no winning factions. |

Example of calling end game in script:

```enforce
SCR_BaseGameMode gamemode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
if (!gamemode)
return;

// No winning player nor faction
gamemode.EndGameMode(SCR_GameModeEndData.CreateSimple(EGameOverTypes.YOUR_ENUM));

// Winning player
gamemode.EndGameMode(SCR_GameModeEndData.CreateSimple(EGameOverTypes.YOUR_ENUM, PLAYER_ID));

// Winning Faction
gamemode.EndGameMode(SCR_GameModeEndData.CreateSimple(EGameOverTypes.YOUR_ENUM, -1, FACTION_INDEX));

// Winning players
gamemode.EndGameMode(SCR_GameModeEndData.Create(EGameOverTypes.YOUR_ENUM, PLAYER_ID_ARRAY));

// Winning Factions
gamemode.EndGameMode(SCR_GameModeEndData.Create(EGameOverTypes.YOUR_ENUM, null, FACTION_INDEX_ARRAY));
```
