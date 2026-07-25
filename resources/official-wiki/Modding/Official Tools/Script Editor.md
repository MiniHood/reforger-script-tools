# [Script Editor](https://community.bistudio.com/wiki/Arma_Reforger:Script_Editor)

Welcome to the **Enfusion Script Editor!**

The **Script Editor** is a text editor module with integrated debugger, available from within the Enfusion Workbench that allows the user to **edit, manage and debug** scripts.
It also provides many tools and utilities for improved efficiency like **syntax highlighting**, script **auto-completion**, fast script **validation** and numerous different methods of **searching** files, scripts and **symbols** within the project.

For a Script Editor newcomer's quickstart see the [**Getting Started**](#Getting_Started) section.

To learn more about the features of the Script Editor, see [**Script Editor Features**](#Script_Editor_Features).

## Script Editor Features

The Script Editor provides many features that make writing and managing code easier.

There are numerous ways of searching through the code, namely **Find File**, **Text Search** or **Find Symbol** in addition to the **Outline**, **Find In Entire Solution** and **Goto Declaration** functionalities.

To learn more about navigating the Script Editor and the codebase, see [**Navigate and find code**](#Navigate_and_Find_Code). To learn more about script syntax error checking and validation, see [**Compilation and Errors**](#Compilation_and_Errors).

Run-time Debugging features include - but are not limited to - the **Debugger**, **Breakpoints**, **Callstack** and **Watch**. For more information regarding debugging, see [**Debugging**](#Debugging).

### Editor Features

| Feature | Description |
| --- | --- |
| Syntax Highlighting | Highlighting of language specific keywords, functions and matching braces. |
| Auto Complete | Auto-completion/suggestion of code via known symbols. See [**Auto Complete**](#Auto_Complete). |
| Script Validation and Error Checking | Syntax checking, for more information see [**Compilation and Errors.**](#Compilation_and_Errors) |
| Searching | Full-text search, symbol search, search for files and more. See [**Searching**](#Searching). |
| Line Numbers | Script file line numbers display in the [**Text Editor**](#Text_Editor). |
| Debugger | Code can be debugged step by step via the usage of the debugger. See [**Debugging**](#Debugging). |
| Undo and Redo | Changes made by the user can be undone and redone at will. |

### Navigate and Find Code

Moving around in the code can be done in several different ways, including navigating backwards and forwards to the last insertion points. Navigation can also be done *via* [**Goto Declaration**](#Goto_Declaration).

Search for and replace text in single or multiple files can be done using the [**Find in Files**](#Find_in_Files).

ⓘ

For more information regarding code navigation, see [**Navigation**](#Navigation).  
For more information regarding searching, see [**Searching**](#Searching).

### Editor Personalization

The layout of the Script Editor is composed from many different windows and panels. They can be resizes, undocked from their position or docked to a preference.
To learn more refer to [**Script Editor Windows**](#Script_Editor_Windows).
In addition, the Script Editor can also be customised to a certain degree by changing the font and font size it uses by default - find more information in [**Preferences**](#Preferences).

### Editor Shortcuts

Opposed to finding particular options from within the Editor windows manually, there is also the option to use some of (or all of) the available keyboard shortcuts - learning to use them might drastically improve the speed and efficiency of the workflow.
For more information about keyboard shortcuts, see [**Keyboard Shortcuts**](#Keyboard_Shortcuts).

## Getting Started

In this quick introduction to the Script Editor we'll take a tour of some of the windows, tools and other features.
For a more in-depth look at features of the Script Editor see [**Script Editor Features**](#Script_Editor_Features).

### Opening Workbench

The Script Editor can be opened from the main *Enfusion Workbench window* at any time. There can only be one instance of the Script Editor running, but it can be opened and closed at free will.

The Script Editor can be opened from the ***Workbench menu bar (1)*** by selecting the ***Script Editor*** ***(2)*** option in the **Editors** drop-down menu, or by clicking the **Script Editor** button ***(3)*** in the Resource Manager's welcome page.

[![armareforger-scripteditor-launch.jpg](/wikidata/images/thumb/0/09/armareforger-scripteditor-launch.jpg/800px-armareforger-scripteditor-launch.jpg)](/wiki/File:armareforger-scripteditor-launch.jpg)

### Default Layout

The default (initial) layout of the Script Editor is composed of the following windows and panels, listed in counter clockwise order as seen on the image below.

In order to learn more about individual windows, please refer to individual pages. For information about working with the windows in general, see **Script Editor Windows**.

1. [**Menu Bar**](#Menu_Bar)
2. [**Debug**](#Debug)
3. [**Outline**](#Outline) and [**Projects**](#Projects)
4. [**Watch**](#Watch), [**Errors**](#Errors) and [**Find Results**](#Find_Results)
5. [**Callstack**](#Callstack), [**Breakpoints**](#Breakpoints) and [**Output**](#Output)
6. [**Console**](#Console) and [**Find in Files**](#Find_in_Files)
7. [**Find Entity**](#Find_Entity), [**Find Symbol**](#Find_Symbol), [**Find File**](#Find_File) and [**Bookmarks**](#Bookmarks)
8. [**Text Editor**](#Text_Editor)

[![armareforger-scripteditor layout.png](/wikidata/images/thumb/3/32/armareforger-scripteditor_layout.png/800px-armareforger-scripteditor_layout.png)](/wiki/File:armareforger-scripteditor_layout.png)

### Compilation and Errors

For error management, see [**Debugging**](#Debugging).

Let's start by creating and validating our first script.

Navigate to the ***Projects (1)*** window and open (or create and open) the ***selected file (2)***.

[![armareforger-scripteditor helloworld 00 openfile.png](/wikidata/images/thumb/7/73/armareforger-scripteditor_helloworld_00_openfile.png/800px-armareforger-scripteditor_helloworld_00_openfile.png)](/wiki/File:armareforger-scripteditor_helloworld_00_openfile.png)

Write the wanted code *via* the ***Text Editor (1).*** In our case we have a ***class Welcomer*** that provides us with a void method ***SayHello***.

Pay close attention to the syntax ***error on line 11 (2)**.* We are missing a closing bracket and a semicolon at the end of the line.

[![armareforger-scripteditor helloworld 01 scripterror.png](/wikidata/images/thumb/b/b2/armareforger-scripteditor_helloworld_01_scripterror.png/800px-armareforger-scripteditor_helloworld_01_scripterror.png)](/wiki/File:armareforger-scripteditor_helloworld_01_scripterror.png)

Use the **Menu Bar** to access the ***Validate Scripts (2)*** option in the **Build** tab.

[![armareforger-scripteditor helloworld 02 validate.png](/wikidata/images/thumb/6/63/armareforger-scripteditor_helloworld_02_validate.png/800px-armareforger-scripteditor_helloworld_02_validate.png)](/wiki/File:armareforger-scripteditor_helloworld_02_validate.png)

#### Errors

The script validation has resulted in multiple errors that can be seen in the ***Errors (1)*** window of the Script Editor.

**Double Click** ![Double Left Mouse Button](/wikidata/images/thumb/a/af/mouse-button-left-double.png/32px-mouse-button-left-double.png "Double Left Mouse Button") on any error to navigate to its location in the **Text Editor**.

[![armareforger-scripteditor helloworld 03 errors.png](/wikidata/images/thumb/d/df/armareforger-scripteditor_helloworld_03_errors.png/800px-armareforger-scripteditor_helloworld_03_errors.png)](/wiki/File:armareforger-scripteditor_helloworld_03_errors.png)

After fixing errors and validating the scripts once again, no problems are found. The [***Output (1)***](#Output) window then shows that the compilation finished successfully*.*

[![armareforger-scripteditor helloworld 04 success.png](/wikidata/images/thumb/9/91/armareforger-scripteditor_helloworld_04_success.png/800px-armareforger-scripteditor_helloworld_04_success.png)](/wiki/File:armareforger-scripteditor_helloworld_04_success.png)

## Preferences

The Script Editor can see some settings adjusted like the font family type and the font size in the Script Editor tab of the [**Workbench Options**](/wiki/Arma_Reforger:Resource_Manager:_Options#Font_Family "Arma Reforger:Resource Manager: Options").

The ***Workbench menu bar (1)*** in the main [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") window allows access to the options by selecting the ***Workbench** → **Options (2)*** entry in the **Workbench** drop down menu.

[![armareforger-scripteditor wb options.png](/wikidata/images/thumb/7/79/armareforger-scripteditor_wb_options.png/800px-armareforger-scripteditor_wb_options.png)](/wiki/File:armareforger-scripteditor_wb_options.png)

The settings can then be personalised in the ***Options (1)*** window by editing the properties available in the ***Script Editor (2)*** tab.

[![armareforger-scripteditor wb options window.png](/wikidata/images/thumb/f/fc/armareforger-scripteditor_wb_options_window.png/800px-armareforger-scripteditor_wb_options_window.png)](/wiki/File:armareforger-scripteditor_wb_options_window.png)

## Navigation

## General

### Menu Bar

The menu bar allows access to some of the most sought after **features** of the Script Editor. The main tabs are:

* **File** → *Provides options of working with files like* **Save All**
* **Edit** → *Provides editing options like* **Jump To Line**, **Navigate Back** *or* **Duplicate Line**
* **Build →** *Provides build options like* **Validate Scripts** *or* **Compile and Reload Scripts**
* **Debug** → *Provides debugging options like* **Debug Client**, *or* **Insert Breakpoint**
* **Windows** → *Provides the option to open windows like the* **Projects***,* **Outline** *or* **Callstack**
* **Bookmarks** → *Provides bookmark options like* **Toggle Bookmark** *or* **Next Bookmark**
* **Plugins** → *Provides and general and individual* **(options per plugin)**

[![armareforger-scripteditor main panel.png](/wikidata/images/thumb/4/46/armareforger-scripteditor_main_panel.png/800px-armareforger-scripteditor_main_panel.png)](/wiki/File:armareforger-scripteditor_main_panel.png)

### Script Editor Windows

#### Opening and Closing

The Script Editor works with windows which all contribute to the layout of the editor.
Windows can be opened and closed by the user at any given time and moved or docked to a certain position within the editor.

A window can always be closed by using ***Click*** on the icon in the window title bar.
If the window needs to be re-opened, it can be done so *via* the ***Menu Bar** → **Windows (1)*** option.

[![armareforger-scripteditor windows tab.png](/wikidata/images/thumb/f/f0/armareforger-scripteditor_windows_tab.png/800px-armareforger-scripteditor_windows_tab.png)](/wiki/File:armareforger-scripteditor_windows_tab.png)

#### Docking and Undocking

To undock a window from its position, either ***Click*** on the button or hover the mouse cursor over the window title bar and hold the ***Left Mouse Button*** while dragging the window out.

To dock a window into a different position, keep dragging the **windows title** by holding the ***Left Mouse Button*** .
Valid positions where the window can be docked will be visualised by the **highlighted area.** Release the ***Left Mouse Button***  and the window will dock itself.

[![armareforger-scripteditor wb windows docking.gif](/wikidata/images/f/fc/armareforger-scripteditor_wb_windows_docking.gif)](/wiki/File:armareforger-scripteditor_wb_windows_docking.gif)

### Projects

#### Summary

A project is a collection of code files that exists within a particular module.

To search for files, use the ***Search** **(1)*** panel that can be found within the ***Projects** **(2)*** window.

***Left-click*** to select any entry from the projects window.

***Right-Click*** on any entry to expand a ***contextual menu (3)*** that will provide additional options like removal or navigation to file via Explorer.

***Double Click*** on a selected Script File to open it in the **Text Editor**.

[![armareforger-scripteditor outline navigation.png](/wikidata/images/thumb/3/36/armareforger-scripteditor_outline_navigation.png/800px-armareforger-scripteditor_outline_navigation.png)](/wiki/File:armareforger-scripteditor_outline_navigation.png)

#### Icons

All entries visible within the Projects browser use a specific icon representing their type:

| Icon | Description |
| --- | --- |
|  | Project |
|  | Directory |
|  | Script File |

### Text Editor

#### Summary

The ***text editor (1)*** will allow us to navigate through script files and edit them as desired. The ***navigation bar (2)*** can be used to switch between open script files.

***Click*** on any of the open tabs to select them as active or on the icon to close an open tab. Additionally use the ***Scroll Wheel*** to cycle between open tabs.

***Right Click*** on any of the open tabs to expand a contextual menu that will provide additional options such as closing multiple files.

[![armareforger-scripteditor texteditor navigation.png](/wikidata/images/thumb/0/0c/armareforger-scripteditor_texteditor_navigation.png/800px-armareforger-scripteditor_texteditor_navigation.png)](/wiki/File:armareforger-scripteditor_texteditor_navigation.png)

Files may be marked as **read-only**. Such script files can still be opened and read freely, but cannot be modified and saved. An example of such files may be core game data.

[![armareforger-scripteditor readonly file.png](/wikidata/images/thumb/f/fd/armareforger-scripteditor_readonly_file.png/800px-armareforger-scripteditor_readonly_file.png)](/wiki/File:armareforger-scripteditor_readonly_file.png)

#### Icons

The following icons may appear during Text Editor usage:

| Icon | Description | Information |
| --- | --- | --- |
|  | Script Error | The script will not compile. |
|  | Script Warning | The script will compile, but might yield unexpected results. |
|  | Breakpoint *(enabled)* | User-placed breakpoint in an enabled state. |
|  | Breakpoint *(disabled)* | User-placed breakpoint in a disabled state. |
|  | Breakpoint *(invalid)* | User-placed breakpoint that was invalidated. |
|  | Breakpoint Trigger Arrow | Current execution step in the debugger. |
|  | Bookmark | User-placed bookmark. |

#### Auto Complete

The Script Editor can help fill in code through symbols suggestion with its **Auto Completion** feature. Simply start writing any code and the ***Auto Completion Dialog (1)*** will appear.

Its dialog can browsed through using the **Up** and **Down** arrows, confirming selection using **Enter** or simply by using mouse **Click** on the desired option.
The ***Auto Completion Dialog (1) can*** reappear by using the **Left Control + Spacebar** ( **+** ) key combination.

[![armareforger-scripteditor autocompletion.png](/wikidata/images/thumb/2/2e/armareforger-scripteditor_autocompletion.png/800px-armareforger-scripteditor_autocompletion.png)](/wiki/File:armareforger-scripteditor_autocompletion.png)

### Bookmarks

***Right Click*** right next to the ***Line Number (1)*** panel in the Text Editor to place or remove a bookmark in code. Placed bookmarks will be visible in the ***Bookmarks (2)*** window.

***Double Click***  on any bookmark from within the ***Bookmarks (2)*** window to quickly navigate to its location.

### Outline

#### Summary

**Outline** can be used to see and navigate through symbols (i.e. variables, methods, classes etc.) in the currently open document.
To search for symbols in the whole solution, use **Find Symbol**.

***Click*** on any of the symbols in the ***Outline (1)*** window to quickly navigate to them in the **Text Editor**.

[![armareforger-scripteditor outline.png](/wikidata/images/thumb/b/b2/armareforger-scripteditor_outline.png/800px-armareforger-scripteditor_outline.png)](/wiki/File:armareforger-scripteditor_outline.png)

### Output

The **Output** window is especially useful while debugging.
All scripts can send diagnostic or other messages to the **Output** window via the ***Print*** method. In addition text can be selected and copied at will.

[![armareforger-scripteditor output.png](/wikidata/images/thumb/6/62/armareforger-scripteditor_output.png/800px-armareforger-scripteditor_output.png)](/wiki/File:armareforger-scripteditor_output.png)

## Searching

The Script Editor also provides numerous ways of searching through the codebase.

### Find File

Use **Find File** to find a script file by name. To search for text in files, refer to **Find in Files** instead**.**

[![armareforger-scripteditor findfile navigation.png](/wikidata/images/thumb/2/24/armareforger-scripteditor_findfile_navigation.png/800px-armareforger-scripteditor_findfile_navigation.png)](/wiki/File:armareforger-scripteditor_findfile_navigation.png)

To search for a file start by typing into the ***search bar (1)***. The results are visible in the ***search results (2)***.

***Double Click*** on any of the files in the ***search results (2)*** to open and navigate to them in the **Text Editor**.

[![armareforger-scripteditor findfile.png](/wikidata/images/thumb/5/5e/armareforger-scripteditor_findfile.png/800px-armareforger-scripteditor_findfile.png)](/wiki/File:armareforger-scripteditor_findfile.png)

### Find Symbol

Use the **Find Symbol** to find a particular symbol located within any of the script files. To search for text in files, refer to **Find in Files** instead**.**

[![armareforger-scripteditor findsymbol navigation.png](/wikidata/images/thumb/3/32/armareforger-scripteditor_findsymbol_navigation.png/800px-armareforger-scripteditor_findsymbol_navigation.png)](/wiki/File:armareforger-scripteditor_findsymbol_navigation.png)

To search for a symbol, type into the ***search bar (1)***. The results are visible in the ***search results (2)**.*

***Double Click*** on any of the symbols in the ***search results (2)*** to open and navigate to them in the [**Text Editor**](#Text_Editor).

[![armareforger-scripteditor findsymbol.png](/wikidata/images/thumb/9/98/armareforger-scripteditor_findsymbol.png/800px-armareforger-scripteditor_findsymbol.png)](/wiki/File:armareforger-scripteditor_findsymbol.png)

### Find in Files

Use **Find in Files** to search for a text match within any of the script files. To search for symbols, refer to **Find Symbol** instead.
For the search results of the Text Search window see [**Find Results**](#Find_Results).

[![armareforger-scripteditor search fulltext navigation.png](/wikidata/images/thumb/c/c2/armareforger-scripteditor_search_fulltext_navigation.png/800px-armareforger-scripteditor_search_fulltext_navigation.png)](/wiki/File:armareforger-scripteditor_search_fulltext_navigation.png)

Type into the ***search bar (1). Click*** on the ***Find All (2)*** button to search. This should open the ***Find Results (3)*** window with results.

***Click*** on any of the found results visible in the ***Find Results (3)*** window to open and navigate to them in the **Text Editor**.

[![armareforger-scripteditor search fulltext.png](/wikidata/images/thumb/3/35/armareforger-scripteditor_search_fulltext.png/800px-armareforger-scripteditor_search_fulltext.png)](/wiki/File:armareforger-scripteditor_search_fulltext.png)

### Goto Declaration

The **Goto Declaration** functionality is another navigation tool available from within the **Text Editor**:

***Right Click*** on any of the ***symbols (1)*** in the currently open script file to expand a contextual menu.

***Click***  on the ***Goto Declaration (2)*** option in the contextual menu. This will navigate to the ***symbol declaration (3)***.

In addition, use **Left Control + Click** key combination on desired symbol to navigate to the Declaration.

[![armareforger-scripteditor goto declaration.png](/wikidata/images/thumb/e/e0/armareforger-scripteditor_goto_declaration.png/800px-armareforger-scripteditor_goto_declaration.png)](/wiki/File:armareforger-scripteditor_goto_declaration.png)

### Find in Entire Solution

Navigate the code using the **Find in Entire Solution** functionality from within the **Text Editor**.

***Right Click*** on any of the ***symbols (1)*** in the currently open script file to expand a contextual menu.

***Click*** on the ***Find In Entire Solution (2)*** option in the contextual menu. This will perform a project-wide search.
For more information regarding searching and finding results, see **Find Results**.

[![armareforger-scripteditor findallreferences.png](/wikidata/images/thumb/3/39/armareforger-scripteditor_findallreferences.png/800px-armareforger-scripteditor_findallreferences.png)](/wiki/File:armareforger-scripteditor_findallreferences.png)

### Find Results

Find Results is a window that provide us with all found results when performing any search *via* the Script Editor search functionality. See **Searching** for more information.

***Double*** **Click** on any of the entries within the ***Find Results (1)*** window to navigate to them and open them in the **Text Editor**.

[![armareforger-scripteditor findresults.png](/wikidata/images/thumb/d/da/armareforger-scripteditor_findresults.png/800px-armareforger-scripteditor_findresults.png)](/wiki/File:armareforger-scripteditor_findresults.png)

### Find Entity

Some entities in the world can be named and have a script associated with them. Such entity scripts can be found *via* the **Find Entity** window.

To be able to use the **Find Entity** option, open a world in the **World Editor** module of the **Enfusion Workbench**.

Then, filter for a particular entity by typing into the ***Search field (2)*** of the Find Entity window.

***Double*** **Click** on any of the entries within the ***Find Entity (1)*** window to navigate to them and open them in the **Text Editor**.

[![armareforger-scripteditor findentity.png](/wikidata/images/thumb/a/a2/armareforger-scripteditor_findentity.png/800px-armareforger-scripteditor_findentity.png)](/wiki/File:armareforger-scripteditor_findentity.png)

## Keyboard Shortcuts

Shortcuts are useful to improve the speed and/or efficiency of the Script Editor usage's workflow:

| Shortcut | Function |
| --- | --- |
| `Ctrl` + `S` | Save All |
| `Ctrl` + `O` | Open File |
|  |  |
| `Ctrl` + `F` | Find in current file |
| `Ctrl` + `⇧ Shift` + `F`} | Find in all files |
| `Alt` + `⇧ Shift` + `O` | Find Files |
| `Alt` + `⇧ Shift` + `S` | Find Symbols |
| `F3` | Find Next |
| `⇧ Shift` + `F3` | Find Previous |
| `Ctrl` + `G` | Jump To Line |
| `Alt` + `G` | Find Stack Trace |
| `Ctrl` + Left Mouse Button on symbol | Find in Symbols, Goto Declaration |
| Double Left Mouse Button | Highlight selection |
|  |  |
| `F7` | Compile |
| `Ctrl` + `F7` | Compile current file on host (game) |
| `F5` | Debug: Continue |
| `F10` | Debug: Step Over |
| `F11` | Debug: Step Into |
| `⇧ Shift` + `F11` | Debug: Step Out |
| `F9` | Insert Breakpoint |
| `Ctrl` + `F2` | Toggle Bookmark |
|  |  |
| `Ctrl` + `D` | Duplicate Line / Selected Text |
| `Ctrl` + `Page Up` | Previous Tab |
| `Ctrl` + `Page Down` | Next Tab |
| `Ctrl` + `↹ Tab` | Cycle Tabs |
| `Ctrl` + `W` | Close Current Tab |
| `Alt` + `←` | Navigate Back |
| `F2` | Next Bookmark |
| `⇧ Shift` + `F7` | Previous Bookmark |
| `Ctrl` + `Spacebar` | Show code completion suggestions |
| `Ctrl` + `⇧ Shift` + `Space` | Show tooltip for current cursor position |

## Debugging

### Breakpoints

To place or remove a breakpointinto/from code, ***Click*** right next to the ***Line Number (1)*** panel in the Text Editor or press **F9**.

The placed breakpoint will then be visible in the ***Breakpoints (2)*** window*.* Breakpoints can be added, removed or toggled at any time, even during run-time.

The checkboxes (/) can be used from within the ***Breakpoints (2)*** window to enable or disable given breakpoint.

Disabled breakpoints will not be hit and will simply be ignored.

Breakpoints shown as are invalid and will not be hit by the debugger.

Breakpoints are invalidated if placed in code that does not match the current code run by the connected game.

[![armareforger-scripteditor debugging breakpoint.png](/wikidata/images/thumb/2/2f/armareforger-scripteditor_debugging_breakpoint.png/800px-armareforger-scripteditor_debugging_breakpoint.png)](/wiki/File:armareforger-scripteditor_debugging_breakpoint.png)

After placing a breakpoint in our code, let's run the game. With an assumption that the code will be executed, we should see theicon pop up over the first encountered breakpoint.

The icon shows the current execution step, as seen on the ***current line (1)**.* The ***Debug (2)*** window can then be used to go through the code execution step by step.

[![armareforger-scripteditor debugging breakpoint hit debugger.png](/wikidata/images/thumb/a/ac/armareforger-scripteditor_debugging_breakpoint_hit_debugger.png/800px-armareforger-scripteditor_debugging_breakpoint_hit_debugger.png)](/wiki/File:armareforger-scripteditor_debugging_breakpoint_hit_debugger.png)

### Debug

The ***Debug*** window then provides us with the following options: **Continue**, **Step Over**, **Step Into**, **Step Out** and **Stop**.

* **Continue:** `F5` Continue standard code execution until next breakpoint is hit / run game mode in World Editor (as `F5` does in World Editor).
* **Step Over:** `F10` Advance the debugger without stepping into functions or methods.
* **Step Into:** `F11` Advance the debugger one statement at a time.
* **Step Out:** `⇧ Shift` + `F11` Advance the debugger until the current function returns.
* **Stop:** `⇧ Shift` + `F5` Stop the playmode and current debugging session.

[![armareforger-scripteditor debugger descriptors.png](/wikidata/images/thumb/d/d1/armareforger-scripteditor_debugger_descriptors.png/800px-armareforger-scripteditor_debugger_descriptors.png)](/wiki/File:armareforger-scripteditor_debugger_descriptors.png)

Hovering the mouse cursor over a variable displays its current value, in the addition to **Watch** usage, explained below.

[![armareforger-scripteditor debugging steps.gif](/wikidata/images/c/c6/armareforger-scripteditor_debugging_steps.gif)](/wiki/File:armareforger-scripteditor_debugging_steps.gif)

### Connecting the debugger

Opening the Script Editor while the game is running automatically attaches the debugger *via* the default port, displaying the following ***pop up message (1)***.

[![armareforger-scripteditor debugging connectedpopup.png](/wikidata/images/thumb/3/36/armareforger-scripteditor_debugging_connectedpopup.png/800px-armareforger-scripteditor_debugging_connectedpopup.png)](/wiki/File:armareforger-scripteditor_debugging_connectedpopup.png)

In addition, a debugger can be connected to a different port *via* the **Debug** options from the [**Menu Bar**](#Menu_Bar).

One particularly interesting option is the **Debug → Debug Custom**, which allows us to connect to a user provided port.

Selecting the **Debug → Debug Custom** will make the following ***window (1)*** appear in which the debugger's port can be set.

[![armareforger-scripteditor debugging customport.png](/wikidata/images/thumb/6/6f/armareforger-scripteditor_debugging_customport.png/800px-armareforger-scripteditor_debugging_customport.png)](/wiki/File:armareforger-scripteditor_debugging_customport.png)

To launch the Enfusion Workbench or the game itself on a specific port, use the **-debuggerPort** command line argument.

In the following image, the ***Properties (1)*** of a Workbench shortcut is set to port 1234 via the argument **-debuggerPort 1234** as seen in the ***Target (2)*** field.

[![armareforger-scripteditor windows properties scriptdebugger port.png](/wikidata/images/thumb/7/78/armareforger-scripteditor_windows_properties_scriptdebugger_port.png/800px-armareforger-scripteditor_windows_properties_scriptdebugger_port.png)](/wiki/File:armareforger-scripteditor_windows_properties_scriptdebugger_port.png)

### Watch

The **Watch** feature can be used to learn about how code changes in run-time. This allows the user to see the current values of variables while navigating through the code via the debugger.

[![armareforger-scripteditor debugger watch.png](/wikidata/images/thumb/1/18/armareforger-scripteditor_debugger_watch.png/800px-armareforger-scripteditor_debugger_watch.png)](/wiki/File:armareforger-scripteditor_debugger_watch.png)

### Callstack

To learn about the code flow in run-time the **Callstack** feature can be used. This feature shows us the methods' call order.

It must be read from the bottom up. In this case, the depicted call originated in: **MyComponent:EOnInit →** **MyComponent::DoWelcome → Welcomer::SayHello**.

[![armareforger-scripteditor debugger callstack.png](/wikidata/images/thumb/9/93/armareforger-scripteditor_debugger_callstack.png/800px-armareforger-scripteditor_debugger_callstack.png)](/wiki/File:armareforger-scripteditor_debugger_callstack.png)

### Console

In addition to the **Callstack** and **Breakpoints**, the **Console** is of great debugging assistance.
The **Console** can be used in **run-time** to execute user scripts within the current stack.

[![armareforger-scripteditor console.png](/wikidata/images/thumb/3/37/armareforger-scripteditor_console.png/800px-armareforger-scripteditor_console.png)](/wiki/File:armareforger-scripteditor_console.png)

Being in playmode is mandatory to use the console. Code which will be global to the current file can then be run.
The console can also be used when a breakpoint is hit while debugging and the run code will be local to the current instance.
For more information regarding breakpoints see [**Breakpoints**](#Breakpoints).

Write code into the console and press the ***Run (1)*** button. Local variables can be used; for more information see **Watch**.

Results will be visible in the [***Output (2)***](#Output) window.

[![armareforger-scripteditor console run.png](/wikidata/images/thumb/5/5a/armareforger-scripteditor_console_run.png/800px-armareforger-scripteditor_console_run.png)](/wiki/File:armareforger-scripteditor_console_run.png)

### Virtual Machine Exceptions

From time to time, unhandled code may be encountered - like in this example below, where we are trying to call a method *via* a null reference. In such cases a **Virtual Machine Exception** will be raised.

Press **Stop** to halt the code execution immediately, ignore single or all cases *via* **Ignore** and **Ignore All**, or navigate straight to the problem using the **Debug** button.

[![armareforger-scripteditor debugging virtualmachineexception.gif](/wikidata/images/5/56/armareforger-scripteditor_debugging_virtualmachineexception.gif)](/wiki/File:armareforger-scripteditor_debugging_virtualmachineexception.gif)
