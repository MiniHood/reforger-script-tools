# [Input Manager](https://community.bistudio.com/wiki/Arma_Reforger:Input_Manager)

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** See all TODO entries.

**Input Manager**, a sub-class of Action Manager, both holds and updates actions by input controllers (mouse, keyboard, gamepad, joystick etc).

## Configuration File

[chimeraInputCommon.conf](enfusion://ResourceManager/~ArmaReforger:Configs/System/chimeraInputCommon.conf) is the file used by Arma Reforger to setup controls.

It holds **[Actions](#Action)** and **[Contexts](#Context)**, described below.

## Action

In gameplay code **actions** are used in order to never have any direct reference to control inputs themselves (such as keyboard key or mouse click).

An action is named (e.g CharacterFire) and see inputs assigned to it (e.g mouse:button0 and gamepad0:right\_trigger both in an InputSum's list).
Each action has:

* a **name**
* a **type**

Action type tells Input Manager how the action's input value should be used as well as how to deal with different types of inputs. An action can be of one of these four types:

* Digital - e.g keyboard, mouse buttons etc.
* Analog - e.g thumb/trigger controller on gamepad etc.
* AnalogRelative - e.g mouse wheel, relative mouse pos etc.
* Motion - e.g motion controllers for VR, absolute mouse position etc.

Various combinations of Action type vs Input type behave differently:

|  | Input: Digital | Input: Analog | Input: AnalogRelative | Input: Motion |
| --- | --- | --- | --- | --- |
| Action: Digital | value | value | **sum** | **diff** |
| Action: Analog | value | value | **sum** | **diff** |
| Action: AnalogRelative | value | value | value | **diff** |
| Action: Motion | **sum** | **sum** | **sum** | value |

### InputSource

Each action has an **InputSource** defining from which input(s) (used in a certain way) the action is triggered.

Each input source has the **Visible** flag as well as the **Filter preset** entry.
The **Visible** flag makes the input source visible (active? TODO) or not.
The **Filter preset** applies the provided filter preset to the input (TODO).

There are several source classes with different behaviours.

#### InputSourceValue

Simply provides value, parameters for only one possible key.

* Input - any possible input (keyboard key, mouse click or movement, gamepad button, etc)
* Filter - see [InputFilter](#InputFilter) below

#### InputSourceSum

Sums values from any number of children input sources (when more than one key is valid for one action)

* Sources: an array of InputSource items from which to sum return values

#### InputSourceCombo

Provides "combo" functionality - all of children sources must be activated for activating InputSourceCombo.
When the first input from combo is activated, all other inputs in combo are blocked for other Input Sources (to distinguish between e.g `Ctrl` + `A` and `A`)

* Sources: an array of InputSource items from which to sum return values

#### InputSourceEmpty

* Device Type - TODO

### Input Type

The input type is either a specific key/control (e.g keyboard:KC\_RETURN), a positive/negative value (e.g gamepad0:left\_thumb\_horizontal), or a more generic input (e.g joystick0:any\_button).
The prefix can be one of:

* keyboard - e.g keyboard:KC\_F5, keyboard:KC\_NUMPAD0
* mouse - e.g mouse:button0, mouse:*any*\_button, mouse:x\_abs
* gamepad0 - e.g gamepad0:pad\_down, gamepad0:right\_trigger, gamepad0:a
* joystick0 - e.g joystick0:button4, joystick0:axis0
* trackir - e.g trackir:yaw, trackir:z

### InputFilter

A filter transforms raw value from input to output value, for example filters just rising edge of signal (key down), transform input by curve, reacts just when input is active for some time (key hold), etc.
Same as source class, it is supposed to extend functionality by overriding this class. The following filters are available:

* InputFilterValue redirect normalised input value to output
* InputFilterPressed returns 1.0 when input value is above threshold, otherwise 0.0
* InputFilterDown returns 1.0 only in frames where the input value just hit raising edge (key DOWN)
* InputFilterDoubleClick returns 1.0 only in frames where input value just hit raising edge (key DOWN) 2nd time in short time period (DOUBLE\_CLICK\_DURATION`)
* InputFilterHold returns 1.0 when input value is above threshold for minimal amount of time HOLD\_DURATION (value is rising gradually from 0.0 to 1.0 in time of HOLD\_DURATION)
* InputFilterHoldOnce returns 1.0 once when input value is above threshold for minimal amount of time HOLD\_DURATION (value is rising gradually from 0.0 to 1.0 in time of HOLD\_DURATION)
* InputFilterToggle has value 0.0 or 1.0 switched by rising edge (key DOWN) of input value
* InputFilterUp returns 1.0 only in frames where input value just hit lowering edge (key UP)
* InputFilterClick returns 1.0 only in frames where input value just hit lowering edge (key UP) AND duration between high and low edge is less than HOLD\_DURATION
* InputFilterSingleClick returns 1.0 only in frames where input value just hit lowering edge (key UP) AND duration between high and low edge is less than HOLD\_DURATION
* InputFilterRepeat similar to InputFilterDown returns 1.0 only in frames where input value just hit raising edge (key DONW), and then after INITIAL\_INTERVAL, every REPEAT\_INTERVAL returns 1.0 for just one frame, until the value is above threshold

[![armareforger inputmanager-wave-diagram.png](/wikidata/images/thumb/6/66/armareforger_inputmanager-wave-diagram.png/300px-armareforger_inputmanager-wave-diagram.png)](/wiki/File:armareforger_inputmanager-wave-diagram.png)

## Context

All actions are organised in **contexts**. A context is a group of actions belonging together. One action can be in multiple contexts.
Because many actions are sharing same inputs, but naturally not all actions are used at the same time, the game must activate pertinent contexts on each frame or activate/deactivate it once for some amount of time.
Each context has **priority** and **flags**.
Input Manager during update iterate trough contexts and if context was activated by game in this frame, and there is not any active context with higher priority, processes its actions.
Otherwise values of actions are zeroed or unchanged (depends on action type). By setting context flag **Overlay** we let activate event context with lower priority.
Flag **CursorVisible** tells Input Manager that the context is using mouse cursor.

### Flags

* Overlay - TODO
* Cursor Visible - shows the game cursor
* Exclusive - disables other contexts that might have been activated. If two Exclusive contexts are active at the same time, TODO
* Force Cursor - forces the game cursor to be shown
* Capture Cursor - TODO

### Example

See the following four contexts:

| Name | Description | Priority | Flags |
| --- | --- | --- | --- |
| CharacterLookContext | mouse look actions | 1 |  |
| CharacterMovementContext | `W``A``S``D` movement actions | 2 | Overlay |
| InventoryContext | mouse UI interaction actions | 2 | CursorVisible |
| InGameMenuContext | mouse UI interaction actions | 3 | CursorVisible |

* During regular gameplay, CharacterLookContext and CharacterMovementContext are activating by gameplay code/script. CharacterMovementContext has priority 2, but it has overlay flag, so it let to update also CharacterLookContext with lower priority.  
  Result: character can both walk and look around, cursor is not visible
* When the inventory is open, InventoryContext gets activated too. It has priority 2, but no **Overlay** flag, so only contexts with priority 2 or higher are updated - here InventoryContext and CharacterMovementContext.  
  Result: character can still walk, but mouse movement is used by for interaction with UI in inventory, cursor is now visible because of InventoryContext has the **CursorVisible** flag
* In-Game menu is open: InGameMenuContext is activating by the game. It has priority 3, so the first three contexts are inactive.  
  Result: character do not react to any input, player can just interact with in-game menu, cursor is now visible because of the **CursorVisible** flag on InGameMenuContext
