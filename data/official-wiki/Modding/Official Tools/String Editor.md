# [String Editor](https://community.bistudio.com/wiki/Arma_Reforger:String_Editor)

## Open String Editor

* In Resource Manager, select Editors → String Editor. In the newly open window select File → Open and in the newly open dialog navigate to the wanted \*.st file.
* alternatively, search for the \*.st file in Resource Manager's Resource Browser tab and open it by double-clicking it.

## Translation Workflow

| Status | Operation | Editable | If the string is correct | If the string is incorrect |
| --- | --- | --- | --- | --- |
| **DEVELOPMENT\_PENDING** | Fill **Target\_en\_us** as translation reference | Checked | Change the status to **DEVELOPMENT\_DONE** | Fix it directly in the **Target\_en\_us** field |
| **DEVELOPMENT\_DONE** | Feature development end, translation requirement confirmation | Checked | Change the status to **PROOFREADING\_PENDING** | Set the status back to **DEVELOPMENT\_PENDING** and edit the **Target\_en\_us** field |
| **PROOFREADING\_PENDING** | Proofreaders acknowledgement | Unchecked | Change the status to **PROOFREADING\_DONE** | **Unlock** the string, set the status back to **DEVELOPMENT\_PENDING** and edit the **Target\_en\_us\_edited** field |
| **PROOFREADING\_DONE** | Proofreaders validation | Unchecked | Change the status to **TRANSLATION\_PENDING** | **Unlock** the string, set the status back to **DEVELOPMENT\_PENDING** and edit the **Target\_en\_us\_edited** field |
| **TRANSLATION\_PENDING** | Translators acknowledgement | Unchecked | Change the status to **TRANSLATION\_DONE** | **Unlock** the string, set the status back to **DEVELOPMENT\_PENDING** and edit the **Target\_en\_us\_edited** field |
| **TRANSLATION\_DONE** | Translators validation | Unchecked | Nothing | **Unlock** the string, set the status back to **DEVELOPMENT\_PENDING** and edit the **Target\_en\_us** field |

ⓘ

This workflow used in a professional environment to synchronise all the actors of the translation process.  

In the scope of a custom, one-man mod this is bound to change and be adapted.

Its behaviour is defined in the LocStatusPlugin and can be disabled, making the workflow optional and customisable to one's needs.

## ID Convention

A translation id must follow a certain convention. Its name is constituted of specific parts in a specific order, and these parts are separated with a **dash** ( - ):

| Name | Description |
| --- | --- |
| Game/Product | Game or Project prefix: **AR** for **A**rma **R**eforger, **ENF** for **Enf**usion |
| Game Project | Optional, subsequential game project code (e.g a DLC, etc) |
| **Name** | PascalCased string name - e.g MyNewWeapon, SplendidActionOnTruck Name prefixes/suffixes can exist through the usage of **underscores** ( **\_** ):   * MainMenu\_Options * MainMenu\_Options\_Video   Iterators are part of the name section too, e.g AOSettings\_**1**, AOSettings\_**2**, AOSettings\_**3** |
| Case | Optional, defines if the string MUST remain uppercase or lowercase. Can be one of:   * **UC** for **U**pper**C**ase * **LC** for **L**ower**C**ase |
| Platform | Used to determine a string that is only present on a certain platform, as opposed to a string that is present on PC. Can be one of:   * **XB** for Microsoft **XB**ox * **PS** for Sony **P**lay**S**tation * **SWC** for Nintendo **Sw**it**c**h |

### Naming Convention

* use **meta** naming; the id must **not** be a 1:1 of the original text
  + e.g: **#AR-ExitConfirmation** and not **#AR-DoYouWantToQuit**
* do **not** use any of:
  + trademarks (use a generic name)
  + profanities (for obvious reasons)
* write in English

### Examples

* ENF-Settings
* AR-**MainMenu**
* AR-**MainMenu\_Options**
* AR-OFP-**MainMenu\_Options\_RefreshRate**-UC-PS

## Writing Convention

A translation text must follow some rules and advices to be valid:

The in-game text should **always** refer to a stringtable and **never** be written directly in UI/data.

Do **not** hardcode:

* text in UI
* text in data (config)
* text in code
* text in image (unless it is part of the lore)
* numeric format (e.g **1000** could be written **1,000** or **1 000** in another language)
* date format (2001-12-01, 01/12/2001, 12/01/2021 etc)
* percentage values (25%, 25 %, %25 etc)

### Tips

To use arguments (formatted) text, use **%1**, **%2**, etc to include argument 1, 2, etc in the text. to display the percentage sign itself (%), use **%%**. Composed strings should be restricted to the bare minimum and used only when necessary for translation reasons.

**Localised string**

```enforce
// formatted text example unlocalised
string testName = "Peter";
PrintFormat("Hello %1, how are you?", testName); // displays "Hello Peter, how are you?"
// #AR-Test is the translation container's id
string testName = "Peter";
PrintFormat("#AR-Test", testName); // displays "Hello Peter, how are you?" as well
```

⚠

If a string contains any **%1** arguments, they **must** be documented in the **Comment** field!

Do **not** assemble multiple parts to make a sentence, as such construction might be invalid in another language:

| Bad Example | Good Example | Explanation |
| --- | --- | --- |
| Hold Space to Take weapon  * Hold %1 to %2 * Space * Take weapon * Sleep | Hold Space to take weapon  * Hold %1 to take weapon * Hold %1 to sleep * Space | The translator does not always know that the value is to be used in a composition The translation itself might not have been thought for this usage in the first place |
| Remaining time: 30 seconds  * Remaining time: %1 %2 * 30 (game value) * seconds | Remaining time: 30 seconds  * Remaining time: %1 second(s) * Remaining time: %1 minute(s) * 30 (game value) |  |
| Heavy Machinegun  * %1 %2 * Heavy * Machinegun | Heavy Machinegun  * Heavy Machinegun | Some languages place adjectives in a different order; such translation must then be done in one piece |

## Create a Stringtable File

Follow these steps to create a new \*.st file:

1. In Resource browser select a folder where you want to create a new \*.st file
2. Use right mouse button to open an option and here select **GUI String table**
3. Set the name of the file
4. Choose **StringTable** class and confirm
5. A new \*.st was created - all there is to do now is to assign this \*.st file to the project
6. In Workbench select Options
7. In newly open window select Game project
8. In the Configuration dropbox, select the desired platform (Headless, PC, Xbox, PS5, etc)
9. Navigate to Widget Manager Settings → String Tables → String Table Definition and here:
   * String Table Source - select the newly created file
   * Languages - press the "+" button to add all desired languages.

## Add a New Localised String

1. Open String Editor
2. Write an Id (stringtable entry key) into the **insert section**
3. Press **insert**, the new container is created
4. Select the new container in the **list of containers**
5. In the **Details** section, add the original text in the **Target\_en\_us** field.
6. Fall all other necessary details like Comment, Max Length of the text, all matching labels and so on. Details about those items are in this section of tutorial.
7. Save changes by using File → Save (or `Ctrl` + `S`)
8. Go to Table → Build Runtime Table (or `Ctrl` + `B`). **Only** after the following confirmation is the text ready to be used in game.

📖

**Recommended read:** Detailed tutorial on creating localisation in mod can be found on [**Mod Localisation**](/wiki/Arma_Reforger:Mod_Localisation "Arma Reforger:Mod Localisation") page.
