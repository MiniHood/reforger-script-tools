# [Scripting Modding](https://community.bistudio.com/wiki/Arma_Reforger:Scripting_Modding)

Before starting working on modified scripts, we need to prepare the basic structure for our new version of the scoring system. Therefore we will create:

* Basic folder structure (see [Directory Structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure"))
* Empty script files ready for our new code

In this tutorial, the scoring system will be used as an example of script modding and following things will be changed:

Permanently changing scoring coefficients for death & suicide

* Playing a sound upon character suicide
* Those changes are fairly simple and should be a good showcase about how to proceed when modding files.

⚠

Scripting modding can only happen in **Modules**, defined in Arma Reforger's .gproj (Arma Reforger > Script Project Manager Settings > Modules):

| Module | core | gameLib | game | workbench | workbenchGame |
| --- | --- | --- | --- | --- | --- |
| Directories | * Core | * GameLib | * Game * GameCode | * Workbench | * WorkbenchGame |

Scripts placed **outside of those folders** will be simply **ignored!**

## File structure

[![armareforger-scripting-modding-find-symbol.png](/wikidata/images/e/e6/armareforger-scripting-modding-find-symbol.png)](/wiki/File:armareforger-scripting-modding-find-symbol.png)

Before writing any code, let's start with investigating which files we need to modify and then, prepare structure for our modded files.

Since we want to modify the scoring system, we can begin by searching for terms related to it.

By typing scoring into the **Find Symbol** search field, we should see **SCR\_ScoringSystemComponent.c** on the first place.

Double clicking ![Double Left Mouse Button](/wikidata/images/thumb/a/af/mouse-button-left-double.png/32px-mouse-button-left-double.png "Double Left Mouse Button") opens the file containing that class and reveals its location in the file structure.

Next to it is **SCR\_BaseScoringSystemComponent.c** and those two files should be enough to achieve the goals stated above.

Note that these two files are located in the Scripts/Game/GameMode/Scoring directory and contain most of the scoreboard-related functionality.

We will be interested in changing the behaviour of:

```enforce
// SCR_BaseScoringSystemComponent.c
void AddSuicide(int playerID) // method which increases suicides & deaths count in score system
```

```enforce
// SCR_ScoringSystemComponent.c
int CalculateScore(SCR_ScoreInfo info) // method used to calculate total score
```

```
SampleMod_ModdedScript/Scripts/Game/GameMode/Scoring/Modded
```

Once the directory is prepared, create two new script files inside the **Modded** folder.

To do so, right-click on the Resource Browser field to open the context menu.

From there, select "Script" to create a new script file. From here, it is time to create actual code!

1. Create a new Script File: In Resources Manager, click the Create button then "Script" to create a script file
2. Name the new Script File: the files should have the same name as the modded ones; namely SCR\_BaseScoringSystemComponent.c and SCR\_ScoringSystemComponent.c

ⓘ

It is also recommended to replace **SCR\_ prefix with your own [tag](/wiki/Scripting_Tags "Scripting Tags")** - this will ensure good inter-compatibility with other scripts by preventing naming conflicts.For instance, instead of naming file *SCR\_ScoringSystemComponent.c* you would call it *YOURTAG\_ScoringSystemComponent.c*

It is usually a good habit to keep the original script and file structures;
for the purpose of this tutorial, it assumed that following addon structure is used:
[![armareforger-scripting-modding-file-structure.png](/wikidata/images/9/93/armareforger-scripting-modding-file-structure.png)](/wiki/File:armareforger-scripting-modding-file-structure.png)

## Create a Modified Script

### Syntax

It is possible to modify already existing scripts by using some of special keyword:

* **modded** - keyword used to modify existing scripting class
* **override** - keyword to override methods in modded classes
* **super** - allows to invoke content of overridden method

We will use all these three words to create modded variants of **SCR\_ScoringSystemComponent**.

ⓘ

For more elaborate information please refer to the [Arma\_Reforger:Object\_Oriented\_Programming\_Basics](/wiki/Arma_Reforger:Object_Oriented_Programming_Basics "Arma Reforger:Object Oriented Programming Basics"), where more details can be found about how these keywords are working as well as simple examples.

### Writing

ⓘ

Please note that:

* this is a proof of concept used for this tutorial and there are other ways to achieve the same effect
* this particularly method is only going to work when the scoring system is present in the mission.

First, we will begin with the **modded** keyword:

```enforce
modded class SCR_ScoringSystemComponent // declares which class is being modded
{
}
```

Next, we can proceed with replacing the CalculateScore method by using the override keyword:

```enforce
modded class SCR_ScoringSystemComponent
{
	override int CalculateScore(SCR_ScoreInfo info) // declares a method replacing an existing one
	{
	}
}
```

As there is no intention to modify regular kill, team kills or objective score, lines containing:

* ENFORCECODEMARKER

  ```
  info.m_iKills * m_iKillScoreMultiplier
  ```
* ENFORCECODEMARKER

  ```
  info.m_iTeamKills * m_iTeamKillScoreMultiplier
  ```
* ENFORCECODEMARKER

  ```
  info.m_iObjectives * m_iObjectiveScoreMultiplier
  ```

are left untouched.

```enforce
int score =	info.m_iKills * m_iKillScoreMultiplier +
info.m_iTeamKills * m_iTeamKillScoreMultiplier +
info.m_iDeaths * m_iDeathScoreMultiplier +
info.m_iSuicides * m_iSuicideScoreMultiplier +
info.m_iObjectives * m_iObjectiveScoreMultiplier;
```

We are replacing their modifiers which would normally be provided via parameters

* **m\_iDeathScoreMultiplier** is replaced by 10
* **m\_iSuicideScoreMultiplier** is also replaced by 10

```enforce
int score =	info.m_iKills * m_iKillScoreMultiplier +
info.m_iTeamKills * m_iTeamKillScoreMultiplier +
info.m_iDeaths * 10 +
info.m_iSuicides * 10 +
info.m_iObjectives * m_iObjectiveScoreMultiplier;
```

This translates into what we want: for every death or suicide, ten points are obtained.

Original code - calculated score is only returned if its above 0, otherwise method returns 0.

```enforce
if (score < 0)
return 0;
return score;
```

### Super

[![](/wikidata/images/thumb/7/7e/armareforger-scripting-modding-get-resource-name.png/300px-armareforger-scripting-modding-get-resource-name.png)](/wiki/File:armareforger-scripting-modding-get-resource-name.png)

**Resource Name** can be easily obtained via **Copy Resource Name(s)** function available in file context menu. Resource name obtained this way contains GUID and relative path to the file.

Same as with **SCR\_ScoringSystemComponent**, we will use the **modded** keyword on **SCR\_BaseScoringSystemComponent** to modify the content of that class.

The **AddSuicide** method is called every time the player commits suicide and adds score according to previously defined modifiers.

Since we don't want to change that part and instead want to add a new behaviour to it, we will use the **super** keyword to call the overridden method's code.

Once this is done, we can use the **PlaySound** method from the **AudioSystem** class - this takes one argument, **ResourceName** of a sound file - and plays a 2D, non spatial, sound.

```enforce
modded class SCR_BaseScoringSystemComponent
{
	override void AddSuicide(int playerId, int count = 1)
	{
		super.AddSuicide(playerId, count); // calls the original method
		AudioSystem.PlaySound("{E89D9A1F4BA63CDC}Sounds/Props/Furniture/Piano/Samples/Props_Piano_Jingle_1.wav"); // plays a sound - hardcoded here for example purpose
	}
}
```

A more mod-friendly way would be the following:

```enforce
modded class SCR_BaseScoringSystemComponent
{
	[Attribute(defvalue: "{E89D9A1F4BA63CDC}Sounds/Props/Furniture/Piano/Samples/Props_Piano_Jingle_1.wav", params: "wav")]
	protected ResourceName m_sSuicideSound; // configurable from Workbench!
	override void AddSuicide(int playerId, int count = 1)
	{
		super.AddSuicide(playerId, count); // calls the original method
		AudioSystem.PlaySound(m_sSuicideSound); // plays the sound
	}
}
```

The code is now ready to compile (`⇧ Shift` + `F7`) and the result can be tested in-game.

## Mod Test

### Terrain Preparation

ⓘ

Below prefabs are using **Enfusion link** which directly loads Workbench and points you to a proper resource.

This functionality has to be **manually enabled** in **Resource Manager** options though!

For more details see [**Resource Manager: Options page**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options").

[![](/wikidata/images/thumb/4/4f/armareforger-scripting-modding-hierarchy-prefabs.png/600px-armareforger-scripting-modding-hierarchy-prefabs.png)](/wiki/File:armareforger-scripting-modding-hierarchy-prefabs.png)

Prefabs in World Editor [Hierarchy tab](/wiki/Arma_Reforger:World_Editor#Hierarchy "Arma Reforger:World Editor")

At minimum, a new test scenario built in **World Editor** requires the following prefabs:

* [GameMode\_Deathmatch\_Automatic.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Modes/Deathmatch/GameMode_Deathmatch_Automatic.et) - this prefab contains **Deathmatch** game mode configuration
* [FactionManager\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Factions/FactionManager_FFA.et) - prefab defining which factions are participating in the game. **FFA** means represents **F**ree **F**or **A**ll, meaning there is only one faction
* [SpawnPoint\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Spawning/SpawnPoint_FFA.et) - spawn point with **FFA** specific configuration
* [LoadoutManager\_FFA.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/MP/Managers/Loadouts/LoadoutManager_FFA.et) - respawn loadout manager - the **FFA** variants contains all available loadouts for both USSR & US characters

All those prefabs can be placed in **World Editor**'s viewport by drag and dropping them from the **Resource Browser**.

[![armareforger-scripting-modding-adding-prefabs.gif](/wikidata/images/0/05/armareforger-scripting-modding-adding-prefabs.gif)](/wiki/File:armareforger-scripting-modding-adding-prefabs.gif)

### Debug Process

While testing scripts, built-in debugging options such as **Breakpoints**, **Console** and **Watch** features - see [Script Editor - Debugging](/wiki/Arma_Reforger:Script_Editor#Debugging "Arma Reforger:Script Editor") for more information.

[![armareforger-scripting-modding-adding-breakpoints.gif](/wikidata/images/f/f5/armareforger-scripting-modding-adding-breakpoints.gif)](/wiki/File:armareforger-scripting-modding-adding-breakpoints.gif)
