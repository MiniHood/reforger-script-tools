# [Diag Menu](https://community.bistudio.com/wiki/Arma_Reforger:Diag_Menu)

## General Use

The Diag Menu is a menu listing many options used to debug game scripting and assets. It is available in [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools") in any 3D viewport (e.g world or model preview).

[![armareforger debugmenu-main.png](/wikidata/images/5/55/armareforger_debugmenu-main.png)](/wiki/File:armareforger_debugmenu-main.png)

ⓘ

|  |  |
| --- | --- |
| Usage   * To open the Diag Menu, press `⊞ Win` + `Alt` in any 3D viewport   + The `Ctrl` + `⊞ Win` shortcut conflicts with Windows 11's shortcut * To enter a sub-menu, press `→` * To leave a sub-menu, press `⟵ Backspace` * To change a value, use `←` and `→`. | Hints   * `Home` - jump to root menu * `Insert` - reset to default just for options in actual menu * `Del` - reset to default for all options * `⇧ Shift` + `↑` / `↓` - faster movement through the menu * `F1` - save current options to user profile * `F2` - load options from user profile |

It is possible to read & save Diagnostic Menu options from different location via [diagMenu](/wiki/Arma_Reforger:Startup_Parameters#diagMenu "Arma Reforger:Startup Parameters") startup parameter.

## Hotkeys List

List of all diag hotkeys

| Header | Hotkey |
| --- | --- |
| [Disable player damage](#Disable_player_damage) | `Ctrl` + `Alt` + `F1` |
| [Teleport](#Teleport) | `Ctrl` + `Alt` + `F2` |
| [Infinite ammo](#Infinite_ammo) | `Ctrl` + `Alt` + `F3` |
| [Infinite bullets](#Infinite_bullets) | `Ctrl` + `Alt` + `F4` |
| [Commit suicide](#Commit_suicide) | `Ctrl` + `Alt` + `F5` |
| [Execute characters in front](#Execute_characters_in_front) | `Ctrl` + `Alt` + `F6` |
| [Debug manual camera](#Debug_manual_camera) | `Ctrl` + `Alt` + `C` |
| [Reset vehicle](#Reset_vehicle) | `Ctrl` + `Alt` + `R` |
| [Teleport up](#Teleport_up) | `Ctrl` + `Alt` + `T` |
| [FPS](#FPS) | `Ctrl` + `NUM 1` |
| [General](#General) | `Ctrl` + `NUM 2` |
| [Counters](#Counters) | `Ctrl` + `NUM 3` |
| [Reload shaders](#Reload_shaders) | `Ctrl` + `F12` |
| [Postprocess](#Postprocess) | `Ctrl` + `Alt` + `P` |
| [Layers diag mode](#Layers_diag_mode) | `Ctrl` + `Alt` + `L` |
| [Detail texture](#Detail_texture) | `Ctrl` + `Alt` + `S` |
| [Render debug mode](#Render_debug_mode) | `RCtrl` + `AltGr` + `W` |
| [Show AABB](#Show_AABB) | `Alt` + `NUM 5` |
| [Show wind emitters](#Show_wind_emitters) | `Ctrl` + `NUM 8` |
| [Disable wind](#Disable_wind) | `Ctrl` + `NUM 9` |
| [Show bodies](#Show_bodies) | `Alt` + `NUM 6` |
| [Simulation](#Simulation) | `Ctrl` + `NUM 7` |
| [Superscreenshot to file](#Superscreenshot_to_file) | `Ctrl` + `⇧ Shift` + `F7` |
| [Screenshot to clipboard](#Screenshot_to_clipboard) | `⇧ Shift` + `F7` |
| [Screenshot to file](#Screenshot_to_file) | `Ctrl` + `F7` |
| [Copy view link](#Copy_view_link) | `Ctrl` + `⇧ Shift` + `L` |
| [Make a step of AStar](#Make_a_step_of_AStar) | `Ctrl` + `Alt` + `V` |
| [Road network](#Road_network) | `Ctrl` + `Alt` + `N` |
| [Show Road Width](#Show_Road_Width) | `Ctrl` + `Alt` + `W` |
| [Road Id](#Road_Id) | `Ctrl` + `Alt` + `I` |
| [Previous agent](#Previous_agent) | `RCtrl` + `RShift ⇧` + `9` |
| [Next agent](#Next_agent) | `RCtrl` + `RShift ⇧` + `0` |
| [Set start position](#Set_start_position) | `Ctrl` + `Alt` + `NUM 0` |
| [Set end position](#Set_end_position) | `Ctrl` + `Alt` + `NUM 1` |
| [Run pathfinder](#Run_pathfinder) | `Ctrl` + `Alt` + `NUM 3` |
| [Generate Graph](#Generate_Graph) | `Ctrl` + `Alt` + `NUM 2` |
| [Disable AI actions](#Disable_AI_actions) | `Ctrl` + `Alt` + `F9` |
| [Paste view link](#Paste_view_link) | `⇧ Shift` + `Alt` + `L` |

## Modding

ⓘ

See [DiagMenu.c](enfusion://ScriptEditor/scripts/Core/generated/Debug/DiagMenu.c) for DiagMenu API information.

```enforce
enum TAG_EMyModDiagMenu
{
	ModMenu = 0,
	ModMenu_DEBUG,
	ModMenu_SHOW,
	ModMenu_LOG,
}
```

```enforce
// init script, needs to run only once
// DiagMenu.RegisterMenu(int id, string name, string parent)
DiagMenu.RegisterMenu(TAG_EMyModDiagMenu.ModMenu, "CategoryName", "");
{
	// RegisterBool(int id, string shortcut, string name, string parent, bool reverse = false)
	DiagMenu.RegisterBool(TAG_EMyModDiagMenu.ModMenu_DEBUG, "", "Do debug", "CategoryName");
	DiagMenu.RegisterBool(TAG_EMyModDiagMenu.ModMenu_SHOW, "", "Show stuff", "CategoryName");
	DiagMenu.RegisterBool(TAG_EMyModDiagMenu.ModMenu_LOG, "", "Log stuff", "CategoryName");
}
```

```enforce
// somewhere needed
if (DiagMenu.GetBool(TAG_EMyModDiagMenu.ModMenu_DEBUG))
{
	Print("Debug entered through Diag menu");
	/* ... */
}
```

## GameCode

## Damage

### Hit Zones

#### Show hit zone memory

Enable/disable showing of hit zone memory.

**Available Options**: false, true

#### Show hit zones

Enable/disable showing of hit zones.

[![armareforger-diag-menu-show-hitzones-1.jpg](/wikidata/images/thumb/a/a9/armareforger-diag-menu-show-hitzones-1.jpg/600px-armareforger-diag-menu-show-hitzones-1.jpg)](/wiki/File:armareforger-diag-menu-show-hitzones-1.jpg)

When a zone is damaged, underlying fire geometry is also drawn

[![armareforger-diag-menu-hitzones-2.jpg](/wikidata/images/thumb/9/96/armareforger-diag-menu-hitzones-2.jpg/600px-armareforger-diag-menu-hitzones-2.jpg)](/wiki/File:armareforger-diag-menu-hitzones-2.jpg)

**Available Options**: false, true

#### Log damage on hit zones

Enable/disable logging of damage on hit zones.

**Available Options**: false, true

#### Log Rpl registration

Enable/disable logging of Rpl registration.

**Available Options**: false, true

### Destruction

#### Damage all on init

Enable/disable damage all on init.

**Available Options**: false, true

#### Enable Logging

Enable/disable logging of destruction.

**Available Options**: false, true

#### Enable layout logging

Enable/disable logging of destruction layout.

**Available Options**: false, true

#### Rotate permanent debris 90deg

Enable/disable rotating permanent debris.

**Available Options**: false, true

#### Snap to ground (debris)

Enable/disable snapping of debris to ground.

**Available Options**: false, true

#### Allow broken prefabs spawning

Enable/disable spawning of broken prefabs.

**Available Options**: false, true

#### Enable destruction diag

Enable/disable destruction diagnostics.

**Available Options**: false, true

### Type of damage info

Set type of damage info. When activated, small windows appears showing HP (current and maximum) of all individual HitZones

* off - debug is disabled
* trace - displays info for what are you looking at
* self - displays info for controlled character
* both - acts as trace if looking at something, otherwise acts as self

**Available Options**: off, trace, self, both

### DamageEffect info

Set damage effect info type. Information about states like Bleeding, Tourniquets, Saline bags, Morphine, and Health/Resilience/Blood is listed in this diag

**Available Options**: off, Persistent, Damage History, both

### Visualize weapon blast

Set weapon blast visualization mode.

**Available Options**: disabled, hit, all, posDebug, onlyMainBlast, onlyRicochet

## Achievements

### Achievements display

Enable/disable achievements display.

**Available Options**: false, true

### Dump achievement memory data

Enable/disable dumping of achievement memory data.

**Available Options**: false, true

### Request achievement platform data

Enable/disable request for platform achievement data.

**Available Options**: false, true

### Unlock ACH\_COMBAT\_HYGIENE

Unlock ACH\_COMBAT\_HYGIENE.

**Available Options**: false, true

### +50 STAT\_ENEMIES\_NEUTRALIZED

Increase STAT\_ENEMIES\_NEUTRALIZED by 50.

**Available Options**: false, true

## Particle

### Particles manager

Enable/disable particles manager debug.

**Available Options**: false, true

### Distance particle effect

Enable/disable distance particle effect.

**Available Options**: enabled, disabled

## Signals

### Show current SignalsManagerComponent

Show or hide current SignalsManagerComponent.

**Available Options**: false, true

### Log received

Enable/disable logging of received signals.

**Available Options**: false, true

### Log sent

Enable/disable logging of sent signals.

**Available Options**: false, true

### Log signal changes

Set logging mode for signal changes.

**Available Options**: off, all, 0.5, 1, 10, 100

### Signals to dump

Set signals to dump.

**Available Options**: SP, MP, all

### Dump signals

Enable/disable dumping of signals.

**Available Options**: false, true

## Game materials

### Cursor

Shows information about materials being used on collider which is under the cursor. It shows also thickness of the collider. By default it using all [Collision Layers](/wiki/Arma_Reforger:Collision_Layer#InteractionMatrixRow_Setup "Arma Reforger:Collision Layer").

**Available Options**: false, true

### Projectile

Restricts game materials check to [Projectile](/wiki/Arma_Reforger:Collision_Layer#InteractionMatrixRow_Setup "Arma Reforger:Collision Layer") layer. Can be used to check fire geometry

**Available Options**: false, true

## Weapons

### Deployment

#### Debug deploying

Enable/disable debugging of weapon deployment.

**Available Options**: false, true

#### IK diag

Enable/disable IK diagnostics for weapon deployment.

**Available Options**: false, true

#### Aim speed diag

Enable/disable aim speed diagnostics for weapon deployment.

**Available Options**: false, true

#### Animation diag

Enable/disable animation diagnostics for weapon deployment.

**Available Options**: false, true

#### Weapon collision diag

Enable/disable weapon collision diagnostics for deployment.

**Available Options**: false, true

#### Disable Prone deployment

Enable/disable prone deployment for weapons.

**Available Options**: false, true

#### Disable post-phys hand pose

Enable/disable post physics hand pose for weapon.

**Available Options**: false, true

#### ADS limits diag

Enable/disable ADS limits debug.

**Available Options**: false, true

#### Deploying height

Set deploying height.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0.3
* Step: 0.01

#### Max dist wpn-shoulder

Set max distance between weapon and shoulder.

**Range Settings**:

* Min: 0
* Max: 90
* Current Value: 15
* Step: 0.05

#### Disable prone movement

Enable/disable movement while prone deployment.

**Available Options**: true, false

#### Prone deployment heigh limit

Set prone deployment height limit.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0.15
* Step: 0.01

### Show sights points

Enable/disable showing of weapon sight points.

**Available Options**: false, true

### Disable aim modifiers

Enable/disable aim modifiers for weapons.

**Available Options**: false, true

### Disable character aim modifiers

Enable/disable character aim modifiers for weapons.

**Available Options**: false, true

### Disable weapon offset

Enable/disable weapon offset.

**Available Options**: false, true

### Enable sway diagnostics

Enable/disable sway diagnostics.

**Available Options**: false, true

### Disable movement based sway

Enable/disable movement based sway.

**Available Options**: false, true

### Weapon obstruction

Enable/disable weapon obstruction debug. Diag shows various data related to obstruction like i.e. current obstruction status of controlled character.

It is also possible to change **Weapon Move back offset** with this diag.

When debug is activated additional spheres will be rendered with following color coding:

• Green  →  Starting or unobstructed points.

• Red  →  Obstructed or fully blocked locations

• Orange  →  Static reference points for offset calculations.

[![armareforger-diag-menu-weapon-obstruction.gif](/wikidata/images/6/60/armareforger-diag-menu-weapon-obstruction.gif)](/wiki/File:armareforger-diag-menu-weapon-obstruction.gif)

When character is moving, either pink or red orbs are visible - those are obstruction predictions orbs. When no obstruction is predicted their color is pink.

[![armareforger-diag-menu-weapon-obstruction-prediction.gif](/wikidata/images/d/d5/armareforger-diag-menu-weapon-obstruction-prediction.gif)](/wiki/File:armareforger-diag-menu-weapon-obstruction-prediction.gif)

⚠

To prevent flashing behavior of debug windows it is necessary to **[Limit FPS](#Limit_FPS) below 20 frames**.

**Available Options**: false, true

### Weapon handling

Enable/disable weapon handling debug.

**Available Options**: false, true

### Weapon IK

Enable/disable weapon IK debug.

**Available Options**: false, true

### Enable casing endpoints

Enable/disable casing endpoints debug.

**Available Options**: false, true

### Disable partial lower

Enable/disable partial lower disable.

**Available Options**: false, true

### Force zeroing anim value

Set force zeroing animation value.

**Range Settings**:

* Min: -0.001
* Max: 1
* Current Value: -0.001
* Step: 0.001

### Force Recoil Type

Set force recoil type.

**Range Settings**:

* Min: 0
* Max: 3
* Current Value: 0
* Step: 1

### Roll comp. weight

Set roll compensation weight.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0
* Step: 0.01

### Roll comp. gizmo diag

Enable/disable roll compensation gizmo debug.

**Available Options**: false, true

### Correct zeroing data by ZeroGenerator

Enable/disable zeroing data correction.

**Available Options**: false, true

### Adjustable magnification input scaling info

Enable/disable adjustable magnification scaling info.

**Available Options**: false, true

### Adjustable magnification input scaling value

Set adjustable magnification input scaling value.

**Range Settings**:

* Min: -1.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.01

### Show Collimator Debug

Enable/disable collimator debug.

**Available Options**: false, true

### Show Attachment Debug

Enable/disable attachment debug.

**Available Options**: false, true

### TestAimModifier

#### Reset aim mod

Reset aim modifier.

**Available Options**: false, true

### Toggle 2D optics

Toggle 2D optics usage.

**Available Options**: false, true

### Show optics diag

Show or hide optics diagnostics.

**Available Options**: false, true

### Show PIP settings diag

Show or hide PIP settings diagnostics.

**Available Options**: false, true

## User Actions

### Log actions

Enable/disable logging of user actions.

**Available Options**: false, true

### Context position

Enable/disable context position debug.

**Available Options**: false, true

### Context visibility angle

Enable/disable context visibility angle debug.

**Available Options**: false, true

### Draw diags for select ent only

Enable/disable drawing of diags for selected entity only.

**Available Options**: false, true

### Context gizmo scale

Set context gizmo scale.

**Range Settings**:

* Min: 0
* Max: 5
* Current Value: 0.3
* Step: 0.05

### Context name scale

Set context name scale.

**Range Settings**:

* Min: 0
* Max: 50
* Current Value: 10
* Step: 1

### Show context radius gizmo

Enable/disable showing of context radius gizmo.

**Available Options**: false, true

### Enable handler diag

Enable/disable user action handler diagnostics.

**Available Options**: false, true

### Forcedisable Interactions

Enable/disable forcing of interactions.

**Available Options**: false, true

### Enable adv. handler diag

Enable/disable advanced user action handler diagnostics.

**Available Options**: false, true

### Use distance to line

Enable/disable using distance to line for interaction.

**Available Options**: false, true

### Fix square distance

Enable/disable fixing of square distance.

**Available Options**: false, true

### Use predicate cache

Enable/disable predicate cache usage.

**Available Options**: true, false

### Script listeners

Enable/disable script listeners.

**Available Options**: true, false

### SetActionEnabled\_S ON

Enable/disable setting of action enable on client.

**Available Options**: true, false

### New interaction method

Enable/disable new interaction method.

**Available Options**: true, false

### Skip action duration

Enable/disable skipping of interaction duration.

**Available Options**: false, true

## Gamepad

### Show active effects

Enable/disable showing of active gamepad effects.

**Available Options**: false, true

### Gyro Speed Sensitivity Yaw

Set gyro speed sensitivity yaw.

**Range Settings**:

* Min: -100.0
* Max: 100.0
* Current Value: 0.0
* Step: 0.01

### Gyro Speed Sensitivity Pitch

Set gyro speed sensitivity pitch.

**Range Settings**:

* Min: -20.0
* Max: 20.0
* Current Value: 0.0
* Step: 0.01

### Gyro Speed Sensitivity Roll

Set gyro speed sensitivity roll.

**Range Settings**:

* Min: -20.0
* Max: 20.0
* Current Value: 0.0
* Step: 0.01

## Track-IR

### Enable TrackIR freelook

Enable/disable TrackIR freelook.

**Available Options**: true, false

### Enable TrackIR leaning

Enable/disable TrackIR leaning.

**Available Options**: true, false

## Radio

### Transmissions diag

Enable/disable radio transmissions diagnostics.

**Available Options**: false, true

### Show radio ranges

Enable/disable showing of radio ranges.

**Available Options**: false, true

### Force transmit

Enable/disable forcing of VON transmission.

**Available Options**: false, true

### Disable audio filters

Enable/disable audio filters for VON.

**Available Options**: false, true

## Chat

### Log client chat messages

Enable/disable logging of client chat messages.

**Available Options**: false, true

### Show chat diag

Show or hide chat diagnostics.

**Available Options**: false, true

### Disable chat

Enable/disable chat.

**Available Options**: false, true

## Time

### Pause world time

Enable/disable pausing of world time.

**Available Options**: false, true

## Electricity

### Set state (SP & MP)

Enable/disable setting of electricity state.

**Available Options**: false, true

### Delete nearest pole (SP ONLY)

Enable/disable deleting of nearest electricity pole.

**Available Options**: false, true

### Disable nearest pole edges (SP ONLY)

Enable/disable disabling of nearest electricity pole edges.

**Available Options**: false, true

### Switch connection of nearest poles (SP ONLY)

Enable/disable switching of nearest electricity poles connection.

**Available Options**: false, true

## Inventory

### Dump storages content

Enable/disable dumping of inventory storage content.

**Available Options**: false, true

### Show inventory items

Enable/disable showing of inventory items.

**Available Options**: false, true

### Log inventory changes

Enable/disable logging of inventory changes.

**Available Options**: false, true

### Log visibility changes

Enable/disable logging of inventory visibility changes.

**Available Options**: false, true

### Show volume info

Enable/disable showing of inventory volume info.

**Available Options**: false, true

### Debug vicinity

Enable/disable vicinity debug.

**Available Options**: false, true

### Show attributes debug

Enable/disable showing of inventory attributes debug.

**Available Options**: false, true

### Enable item placement in WB

Enable/disable item placement in workbench.

**Available Options**: false, true

### Enable hand slot

Enable/disable hand slot.

**Available Options**: false, true

## Loadout

### Show clothes

Enable/disable showing of loadout clothes.

**Available Options**: false, true

### Show slots

Enable/disable showing of loadout slots.

**Available Options**: false, true

### Unwear all

Unwear all loadout items.

**Available Options**: false, true

### Show only me

Enable/disable showing of only player loadout.

**Available Options**: false, true

### Show only lines without text

Enable/disable showing of only lines without text.

**Available Options**: false, true

## Lights

### Light positions

Enable/disable showing of light positions.

**Available Options**: false, true

## Camera

### Show camera position

Enable/disable showing of camera position.

**Available Options**: false, true

## Animations

### Show animated items bones

Enable/disable showing of animated items bones.

**Available Options**: false, true

## Network

### Network Movement

#### Movement

Set network movement debug mode.

**Available Options**: none, in, out

#### Movement simulation

Enable/disable network movement simulation debug.

**Available Options**: false, true

#### Display Selection

Enable/disable network movement display selection.

**Available Options**: false, true

#### NwkMovement Selection

Set network movement selection.

**Range Settings**:

* Min: 0
* Max: 512
* Current Value: 0
* Step: 1

#### Display buffer state

Enable/disable network movement display buffer state.

**Available Options**: false, true

#### Execution mode

Enable/disable network movement execution mode.

**Available Options**: false, true

#### Phys Simulation

Enable/disable network movement physics simulation.

**Available Options**: false, true

#### Nwk Updates

Enable/disable network movement network updates.

**Available Options**: false, true

### Network Streaming

#### Override Distance

Enable/disable overriding of streaming distance.

**Available Options**: false, true

#### Stream In Grid Distance [#]

Set stream in grid distance.

**Range Settings**:

* Min: 1
* Max: 10
* Current Value: 4
* Step: 1

#### Vehicle Grid Distance [#]

Set vehicle grid distance.

**Range Settings**:

* Min: 1
* Max: 10
* Current Value: 2
* Step: 1

#### Character Grid Distance [#]

Set character grid distance.

**Range Settings**:

* Min: 1
* Max: 10
* Current Value: 3
* Step: 1

### RPL scheduler

#### Server side RPL scheduler

##### Items

Enable/disable basic RPL scheduler debug.

**Available Options**: false, true

##### Item Hierarchy Moves

Enable/disable RPL scheduler item hierarchy moves debug.

**Available Options**: false, true

##### Items Streamed In

Enable/disable RPL scheduler streamed in debug.

**Available Options**: false, true

##### Items Streamed Out

Enable/disable RPL scheduler streamed out debug.

**Available Options**: false, true

##### Items bumpped

Enable/disable RPL scheduler bumpped items debug.

**Available Options**: false, true

##### Spatial map

Enable/disable RPL scheduler spatial map debug.

**Available Options**: false, true

##### Moving list

Enable/disable RPL scheduler moving list debug.

**Available Options**: false, true

##### Disabled ref count

Enable/disable RPL scheduler disabled reference count debug.

**Available Options**: false, true

##### Observers

Enable/disable RPL scheduler observers debug.

**Available Options**: false, true

##### Budgeting

Enable/disable RPL scheduler budgeting debug.

**Available Options**: false, true

##### Ownership

Enable/disable RPL scheduler ownership debug.

**Available Options**: false, true

#### Display Observers

Enable/disable display of observers.

**Available Options**: false, true

#### Virtual Player Id

Enable/disable virtual player ID debug.

**Available Options**: false, true

#### Spatial Map of Player Id

Set spatial map of player ID.

**Range Settings**:

* Min: 0
* Max: 192
* Current Value: -1
* Step: 1

#### Virtual Player X pos

Set virtual player X position.

**Range Settings**:

* Min: 0
* Max: 13000
* Current Value: 0
* Step: 100

#### Virtual Player Y pos

Set virtual player Y position.

**Range Settings**:

* Min: 0
* Max: 13000
* Current Value: 0
* Step: 100

#### Spatial layer

Set spatial layer.

**Available Options**: statics, dynamics, char+veh

#### Spatial type

Set spatial type.

**Available Options**: stationary+moving, stationary, moving

#### Network range

Set network range.

**Range Settings**:

* Min: 1
* Max: 50
* Current Value: 2
* Step: 1

#### Global streaming budget

Set global streaming budget.

**Range Settings**:

* Min: 100
* Max: 50000
* Current Value: 500
* Step: 50

#### Staggering budget

Set staggering budget.

**Range Settings**:

* Min: 1
* Max: 10201
* Current Value: 1
* Step: 1

#### Open streams delta

Set open streams delta.

**Range Settings**:

* Min: 1
* Max: 1000
* Current Value: 1
* Step: 1

#### Trouble causer

Enable/disable trouble causer debug.

**Available Options**: false, true

#### Streaming trouble causer rate

Set streaming trouble causer rate.

**Range Settings**:

* Min: 1
* Max: 60
* Current Value: 0
* Step: 1

### Server FPS

Enable/disable network server FPS display.

**Available Options**: false, true

### Damage synchronization

Enable/disable network damage synchronization debug.

**Available Options**: false, true

### Lobby diagnostics

Enable/disable network lobby diagnostics.

**Available Options**: false, true

### Connection diag

Enable/disable network connection diagnostics.

**Available Options**: false, true

### Log ownership changes

Enable/disable logging of network ownership changes.

**Available Options**: false, true

### Validate loaded prefabs

Enable/disable validation of loaded prefabs.

**Available Options**: false, true

### Server admin

Enable/disable server admin mode.

**Available Options**: disabled, enabled

### RPC MT jobs

Enable/disable RPC multithreaded jobs.

**Available Options**: true, false

### Loading prefabs

Enable/disable prefabs loading debug.

**Available Options**: false, true

## Social Component

### Show

Enable/disable client social component debug.

**Available Options**: false, true

### Restrictions override

Set restriction override mode for social component.

**Available Options**: None, Force-Restriction, Force-Unrestricted

### Player to restrict

Set player to restrict.

**Available Options**: First-Found, Last-Found

### Send update

Enable/disable sending of social component update.

**Available Options**: false, true

### Block Me

Enable/disable blocking of current player.

**Available Options**: false, true

### Unblock Me

Enable/disable unblocking of current player.

**Available Options**: false, true

### Update block list

Enable/disable updating of block list.

**Available Options**: false, true

## Turrets

### Turret armory

Enable/disable turret armory debug.

**Available Options**: false, true

### Disable turret modifiers

Enable/disable aim modifiers for turrets.

**Available Options**: false, true

## HitReg

### Draw Compensated Shots

Set draw mode for compensated shots.

**Available Options**: none, missed, all

### Drawn Shots

Set amount of drawn shots.

**Range Settings**:

* Min: 1
* Max: 20
* Current Value: 5
* Step: 1

### Draw Trajectories

Enable/disable drawing of trajectories.

**Available Options**: false, true

### Draw Collider Histories

Set drawing mode for collider histories.

**Available Options**: none, all, included, excluded

### Use RTT (Old Method)

Enable/disable using old RTT method.

**Available Options**: false, true

### Show Graphs

Enable/disable showing of hit registration graphs.

**Available Options**: false, true

## Projectiles

### Proj. override

#### Mass

Projectile mass override

**Available Options**: default, 0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1

#### Muzzle velocity

Projectile muzzle velocity override

**Available Options**: default, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000

#### Air drag coef

Projectile air drag coefficient override

**Available Options**: default, 0.001, 0.0005, 0.0001, 0.00005, 0.00001, 0.000005, 0.000001, 0.0000005, 0.0000001, 0.00000005, 0.00000001

### Proj. test

#### Run

Projectile test run

**Available Options**: disabled, enabled

#### Type

Projectile test type

**Available Options**: camera, general, firefight, pen-test

#### Repeat

Projectile test repeat mode

**Available Options**: once, counter, unlimited

#### Repeat count

Set projectile test repeat count.

**Range Settings**:

Min: 5

Max: 100

Current Value: 5

Step: 5

#### Repeat freq.

Projectile test repeat frequency

**Available Options**: 1, 2, 5, 10, 30, 60

#### Proj. count

Set projectile test shot count.

**Range Settings**:

Min: 10

Max: 100

Current Value: 10

Step: 10

#### Pen. type

Projectile penetration test type

**Available Options**: disc, sphere, box

#### Matrix type

Projectile test matrix type

**Available Options**: actual, saved

#### Matrix save

Projectile test matrix save

**Available Options**: disabled, enabled

#### Projectile

Projectile test projectile type

**Available Options**: bullet, rpg

#### Kill timer

Projectile test kill timer

**Available Options**: 0, 1, 2, 5

#### Kill

Projectile test kill switch

**Available Options**: disabled, enabled

### Proj. tests

#### Show plot

Projectile test show plot

**Available Options**: disabled, enabled

#### Plot type

Projectile test plot type

**Available Options**: 1dscatter, 2dscatter, 1dhistogram, 2dhistogram, defl\_scatter, defl\_hist

#### Gauss param

Set projectile test gauss parameter.

**Range Settings**:

Min: 0

Max: 5

Current Value: 0

Step: 0.1

### Proj. settings

#### Max concurrent debug lines

Maximum amount of trajectories that are shown at once
**Defalut value: 16** (*can be changed to up 1024 trajectories*)

**Available Options**: 128, 8, 16, 32, 64, 256, 512, 1024

#### Version

Projectile version

**Available Options**: default, experimental

### Proj. ballistics

#### Print Hit Info

Projectile print hit info

**Available Options**: false, true

### Show shell trajectory

Show **projectile trajectory in 3D space**, **name of the flying prefab**, **impact points** and also **additional projectile data** in top left corner (including projectile speed, data from prefab and impact data)

Color of the trace represents speed of the projectile, while green represents 100% of projectile initial speed and red represents speeds close to 0.
[![armareforger-diag-menu-projectile-shell-trajectory.png](/wikidata/images/thumb/5/52/armareforger-diag-menu-projectile-shell-trajectory.png/300px-armareforger-diag-menu-projectile-shell-trajectory.png)](/wiki/File:armareforger-diag-menu-projectile-shell-trajectory.png)

**Available Options**: false, true

### Show stats

Shows statistics regarding current, largest & total amount of different type of projectiles: **Shells, Missiles & Grenades**

[![armareforger-diag-menu-projectile-show-stats.png](/wikidata/images/3/36/armareforger-diag-menu-projectile-show-stats.png)](/wiki/File:armareforger-diag-menu-projectile-show-stats.png)

**Available Options**: false, true

### Show predicted shell trajectory

Show prediction of projectile trajectory. You can still follow bullet path by observing where textbox with prefab name is located.

[![armareforger-diag-menu-projectile-predicted-trajectory.png](/wikidata/images/thumb/1/17/armareforger-diag-menu-projectile-predicted-trajectory.png/688px-armareforger-diag-menu-projectile-predicted-trajectory.png)](/wiki/File:armareforger-diag-menu-projectile-predicted-trajectory.png)

**Available Options**: false, true

### Shell name

Controls whether shell name is visible next to projectile prefab.

[![armareforger-diag-menu-projectile-shell-name.png](/wikidata/images/thumb/a/af/armareforger-diag-menu-projectile-shell-name.png/700px-armareforger-diag-menu-projectile-shell-name.png)](/wiki/File:armareforger-diag-menu-projectile-shell-name.png)

**Available Options**: true, false

### Player shell only

Controls if projectile diag should be applied to only player fired projectiles

**Available Options**: false, true

### Visibility

How many projectiles should be visible

**Available Options**: all, 128, 64, 32, 16, 8, 4, 2, 1

### Persistence

Controls how long **trajectory** is visible after projectile destruction. By default it is set to **15 seconds**

**Available Options**: 15s, 30s, 60s, 120s, permanent, 0s, 5s

### Clear trajectories

Works as a toggle.

When activated, it clears world from all debug trajectories

**Available Options**: disabled, enabled

### Info visible

Controls infobox visibility

**Available Options**: true, false

### Info persistence

Controls how long **infobox** in top left corner is visible after projectile destruction. By default it is set to **0 seconds**

**Available Options**: 0s, 5s, 15s, permanent

### Info segment length

Interval at which speed data in infobox is measured and persisted.

* [![500m interval](/wikidata/images/thumb/6/66/armareforger-diag-menu-projectile-interval-500.png/219px-armareforger-diag-menu-projectile-interval-500.png)](/wiki/File:armareforger-diag-menu-projectile-interval-500.png "500m interval")

  500m interval
* [![50m interval](/wikidata/images/thumb/2/2d/armareforger-diag-menu-projectile-interval-50.png/219px-armareforger-diag-menu-projectile-interval-50.png)](/wiki/File:armareforger-diag-menu-projectile-interval-50.png "50m interval")

  50m interval
* [![100m interval](/wikidata/images/thumb/d/d1/armareforger-diag-menu-projectile-interval-100.png/219px-armareforger-diag-menu-projectile-interval-100.png)](/wiki/File:armareforger-diag-menu-projectile-interval-100.png "100m interval")

  100m interval

**Available Options**: 100, 200, 500, 25, 50

### Hide info

Projectile hide info box

**Available Options**: disabled, enabled

### Dump data

*Doesn't work in retail version of the game*

**Available Options**: disabled, enabled

### Special case

Projectile special case

**Available Options**: none, north, up, down, drop

### Deflection

Controls whether projectiles have a chance to deflect from surface.

**Available Options**: true, false

### Penetration

Controls whether projectiles is able to penetrate other objects.

**Available Options**: true, false

### Deviation

Controls whether projectiles randomly deviates after penetration.

**Available Options**: true, false

### Gravity

Controls whether projectiles is affected by gravity.

**Available Options**: true, false

### Decals

Controls whether projectiles are leaving impact decals

**Available Options**: true, false

### Decals advanced

Projectile advanced decals

**Available Options**: true, false

### Decals use diameter

Projectile decals use diameter

**Available Options**: false, true

### Override material

Projectile material override

**Available Options**: true, false

### Bullet time

Reduces speed to 0.01 when projectile is fired. Time doesn't restore to normal speed unless [DTime Multiplier](/wiki/User:Reyhard/Sandbox/Diag_Menu#DTime_multiplier "User:Reyhard/Sandbox/Diag Menu") in Cheats is turned off manually.

**Available Options**: false, true

### Grenade simulation

Grenade simulation mode

**Available Options**: true, false

## Show actions states

Enable/disable showing of actions states.

**Available Options**: false, true

## Gridmap diags

Enable/disable gridmap diagnostics.

**Available Options**: false, true

## Show explosion debug

Enable/disable showing of explosion debug.

[![armareforger-diag-menu-explosive-damage.jpg](/wikidata/images/thumb/7/7b/armareforger-diag-menu-explosive-damage.jpg/600px-armareforger-diag-menu-explosive-damage.jpg)](/wiki/File:armareforger-diag-menu-explosive-damage.jpg)

When an explosion occurs, "traces" (singular: "trace") are visual lines that represent the paths to each hitzone. These traces provide diagnostic feedback about whether damage was dealt or blocked. Here's how the trace system works

* **Red Sphere** :
  + Represents the explosion's **current damage radius** .
  + The **transparency** of the sphere changes based on the explosion's size:
    - **Less transparent (solid)** : Indicates larger, more impactful explosions.
    - **More transparent (faded)** : Indicates smaller, less significant explosions.
* Red: Normal explosion
* **Yellow Sphere**:
  + Impulse explosion
* **Dark Yellow Sphere**
  + Fragmentation explosion
  + Helps distinguish explosions of varying magnitudes and reduces visual clutter in overlapping scenarios.
* **Blue Lines** :
  + Represent **unblocked traces** , indicating successful paths from the explosion’s origin where damage was applied to targets.
* **Red Lines** :
  + Not visible in this image but would represent **blocked traces** , showing paths where damage was obstructed (e.g., by terrain or obstacles).

* Additionally, each hitzone (a specific area of the target) is represented by a **sphere** at its location:
  + **Blue Sphere** : Indicates a hitzone that successfully took damage (unblocked).
  + **Red Sphere** : Indicates a hitzone where damage was *blocked* and not applied.

**Available Options**: false, true

## Show player events

Enable/disable showing of player events.

**Available Options**: false, true

## Show target events

Enable/disable showing of target events.

**Available Options**: false, true

## Show aim directions

Enable/disable showing of aim directions.

**Available Options**: false, true

## Factions

Enable/disable showing of factions debug.

**Available Options**: false, true

## Contacts

Enable/disable showing of contacts debug.

**Available Options**: false, true

## Smooth steering in vehicle

Enable/disable smooth steering for vehicles.

**Available Options**: false, true

## Character acceleration in vehicle

Set character acceleration debug in vehicle.

**Available Options**: acceleration, gforce

## FOV override

Set field of view override.

**Range Settings**:

Min: 0.0

Max: 89.9

Current Value: 0.0

Step: 0.1

## Time and Weather Diag

Enable/disable time and weather diagnostics.

**Available Options**: false, true

## Enable better interpolation

Enable/disable better interpolation.

**Available Options**: true, false

## Log GameEntity events

Enable/disable logging of GameEntity events.

**Available Options**: false, true

## Log GameEntity events (simple)

Enable/disable logging of GameEntity events (simple).

**Available Options**: false, true

## Log shooting queues

Enable/disable logging of shooting queues.

**Available Options**: false, true

## Log SlotInfo

Set SlotInfo logging mode.

**Available Options**: off, attached, detached, both

## Door ColTest

Enable/disable door collision test.

**Available Options**: false, true

## Use traces for possible volume data

Enable/disable traces for volume data.

**Available Options**: false, true

## Show world volumes collection

Enable/disable showing of world volumes collection.

**Available Options**: false, true

## Enable Decal Slots

Enable/disable decal slots.

**Available Options**: false, true

## Game state diag

Enable/disable game state diagnostics.

**Available Options**: false, true

## Force fog at camera

Force fog at camera.

**Available Options**: false, true

## Override fog

Override fog.

**Available Options**: false, true

## Height bias

Set height bias.

**Range Settings**:

Min: -500

Max: 500

Current Value: 0

Step: 5

## Groups

### Enable groups diag

Enable/disable groups diagnostics.

**Available Options**: false, true

## Show input manager

Set input manager debug mode.

**Available Options**: disabled, active, all

## Log player damage

Enable/disable logging of player damage.

**Available Options**: false, true

## Cheats

## Disable player damage

Enables god mod. Has an instant effect on a currently controlled character that persists on that character even after switching to GM or a different character

**Hotkey**: `Ctrl` + `Alt` + `F1`

**Available Options**: Cheat disabled, Force invulnerable, Force vulnerable

## Teleport

Teleport player.

**Hotkey**: `Ctrl` + `Alt` + `F2`

**Available Options**: false, true

## Infinite ammo

Enable/disable infinite ammo (magazines). Doesn't work in 1.2.1

**Hotkey**: `Ctrl` + `Alt` + `F3`

**Available Options**: false, true

## Infinite bullets

Enable/disable infinite bullets. The bullet in the chamber is never consumed, which means only first bullet in magazine array will be only used (cannot be used for testing mixed mags for instance)

**Hotkey**: `Ctrl` + `Alt` + `F4`

**Available Options**: false, true

## Commit suicide

Commit suicide.

**Hotkey**: `Ctrl` + `Alt` + `F5`

**Available Options**: false, true

## Execute characters in front

Execute characters in front.

**Hotkey**: `Ctrl` + `Alt` + `F6`

**Available Options**: false, true

## DTime multiplier

Toggles time progression multiplication by **Dtime multiplier value** (DTime stands for *dilatation time*, also known as time slice in Enfusion)

**Available Options**: false, true

## DTime multiplier value

Set delta time multiplier value.

**Range Settings**:

Min: 0.01

Max: 20

Current Value: 1.0

Step: 0.01

## Debug manual camera

Enable/disable free manual camera. While the "Debug camera" (aka "Free camera" or "Freecam") is active, you can use the **spacebar** to teleport your character under the cursor

**Hotkey**: `Ctrl` + `Alt` + `C`

**Available Options**: false, true

## ECS

## ECS archetypes

Enable/disable ECS archetypes debug.

**Available Options**: false, true

## ECS registered types

Enable/disable ECS registered types debug.

**Available Options**: false, true

## ECS type matching

Enable/disable ECS type matching debug.

**Available Options**: false, true

## ECS deleted entities

Enable/disable ECS deleted entities debug.

**Available Options**: false, true

## Character

## Character physics

### Prequery

Enable/disable character physics prequery.

**Available Options**: enabled, disabled

### Use idle character optimization

Enable/disable idle character physics optimization.

**Available Options**: enabled, disabled

### Show charcter info

Enable/disable showing of character collision info.

**Available Options**: disabled, enabled

### Always apply collision

Enable/disable always applying character collision.

**Available Options**: disabled, enabled

### Ground offset diag

Enable/disable showing of ground offset.

**Available Options**: disabled, enabled

### Draw character mode

Set draw mode for character physics debug.

**Available Options**: None, Everything, Collision, Movement Path, Surface, Prequery

## Enable desync detection

Enable/disable desync detection.

**Available Options**: false, true

## Show desync detection window

Enable/disable desync detection window.

**Available Options**: false, true

## Interpolation enabled

Enable/disable character interpolation.

**Available Options**: true, false

## Animation LOD

Set animation LOD.

**Range Settings**:

Min: -1

Max: 3

Current Value: -1

Step: 1

## Aiming direction

Enable/disable showing of aiming direction.

**Available Options**: false, true

## Head aiming direction

Enable/disable showing of head aiming direction.

**Available Options**: false, true

## Heading angle

Enable/disable showing of heading angle.

**Available Options**: false, true

## Surface slope

Enable/disable showing of surface slope.

**Available Options**: false, true

## Entity transform

Enable/disable showing of entity transform.

**Available Options**: false, true

## AnimPhysAgent transform

Enable/disable showing of animation physics agent transform.

**Available Options**: false, true

## Foot down anim event

Enable/disable showing of foot down animation event.

**Available Options**: false, true

## Animation events

Enable/disable showing of animation events.

**Available Options**: false, true

## Animation tags

Enable/disable showing of animation tags.

**Available Options**: false, true

## Climb command physics test

Enable/disable climb command physics test.

**Available Options**: false, true

## Show current ladder

Enable/disable showing of current ladder.

**Available Options**: false, true

## Input actions

Enable/disable showing of input actions.

**Available Options**: false, true

## Lock view horizontal

Enable/disable locking of view horizontal.

**Available Options**: false, true

## Lock view vertical

Enable/disable locking of view vertical.

**Available Options**: false, true

## Disable Inertia

Enable/disable character inertia.

**Available Options**: false, true

## Disable Inertia exponential

Enable/disable character exponential inertia.

**Available Options**: false, true

## Show additive handler tasks

Enable/disable showing of active additive tasks.

**Available Options**: false, true

## Stamina

Enable/disable showing of character stamina.

**Available Options**: false, true

## Disable animation from velocity

Enable/disable animation from velocity.

**Available Options**: false, true

## Enable new passive weapon mode (sling)

Enable/disable new passive weapon mode (sling).

**Available Options**: false, true

## Disable anim driven icremental turns

Enable/disable anim driven incremental turns.

**Available Options**: false, true

## Disable hit reaction

Enable/disable hit reaction.

**Available Options**: false, true

## Enable ragdoll debug (melee attack to toggle)

Enable/disable ragdoll debug (melee attack to toggle).

**Available Options**: false, true

## Enable feet IK

Enable/disable feet IK.

**Available Options**: false, true

## Show player identity

Enable/disable showing of player identity.

**Available Options**: false, true

## Show inspection diag

Enable/disable showing of inspection debug.

**Available Options**: false, true

## Show drowning / water level diag

Enable/disable showing of drowning/water level debug.

**Available Options**: false, true

## Inspection keeps orientation

Enable/disable keeping orientation during inspection.

**Available Options**: false, true

## Prone root motion transl

Enable/disable root motion translation for prone turns.

**Available Options**: false, true

## Toggle unconsciousness

Enable/disable toggling of character unconsciousness.

**Available Options**: false, true

## Forced unconsciousness pose

Set forced unconsciousness pose.

**Range Settings**:

Min: -1

Max: 5

Current Value: -1

Step: 1

## Show Frustum

Enable/disable showing of frustum.

**Available Options**: false, true

## Disable velocity DTime

Enable/disable velocity delta time.

**Available Options**: false, true

## Enable turret overriding logging

Enable/disable turret overriding logging.

**Available Options**: false, true

## Skip simulation points prequery

Enable/disable skip simulation points prequery.

**Available Options**: false, true

## Show simulation points

Enable/disable showing of simulation points.

**Available Options**: false, true

## Cycle Debug View

Set character debug view cycle.

**Range Settings**:

Min: 0

Max: 8

Current Value: 0

Step: 1

## Draw Camera Collision Solver

Enable/disable drawing of camera collision solver.

**Available Options**: false, true

## Enable Debug UI

Enable/disable character debug UI shows character movement, stance and camera debugs.

**Available Options**: false, true

## Disable Banking

Enable/disable character banking.

**Available Options**: false, true

## Enable transform info

Enable/disable character transformation info.

**Available Options**: false, true

## Disable recoil cam shake

Enable/disable recoil camera shake.

**Available Options**: false, true

## Disable additional cam shake

Enable/disable additional camera shake.

**Available Options**: false, true

## Show shake diag

Enable/disable showing of camera shake test window.

**Available Options**: false, true

## Inspection LookAt

Enable/disable inspection look at.

**Available Options**: false, true

## ADS Camera

Set ADS camera mode.

**Range Settings**:

Min: 0

Max: 3

Current Value: 0

Step: 1

## Deal damage debug

Enable/disable damage debug for character.

**Available Options**: false, true

## Show Perceived Faction

Enable/disable showing of perceived faction.

**Available Options**: false, true

## AimModifier Debug

Enable/disable aim modifier debug.

**Available Options**: false, true

## Vehicles

## Helicopter

### Draw trajectory

Enable/disable drawing of helicopter trajectory.

**Available Options**: false, true

### Draw velocity

Set helicopter velocity visualization mode.

**Available Options**: none, resultant, res+local, res+world

### Draw HUD

Enable/disable helicopter HUD debug.

**Available Options**: false, true

### Toggle autohover

Enable/disable helicopter autohover.

**Available Options**: disabled, enabled

### Collective

Set helicopter collective debug mode.

**Available Options**: localYOffset, default, localY, worldY, direct

### Flight limits

Enable/disable helicopter flight limits debug.

**Available Options**: false, true

### Yaw/Roll

Set helicopter stick mode.

**Available Options**: LS/RS, RS/LS

## Tracked

### TrackSim

Enable/disable Track simulation.

**Available Options**: true, false

### Draw tracks

Enable/disable drawing of tracks.

**Available Options**: false, true

### Track dynamic

Enable/disable track dynamic.

**Available Options**: true, false

### Track gravity

Enable/disable track gravity.

**Available Options**: true, false

### Reset tracks

Enable/disable resetting of tracks.

**Available Options**: disabled, enabled

## Vehicle Networking and Reconciliation

### Determinism testing

#### Record Moves

Enable/disable recording of vehicle moves.

**Available Options**: false, true

#### Clear Record

Clear recorded vehicle moves.

**Available Options**: false, true

#### (F)Replay Full

Enable/disable full replay of vehicle moves from full state.

**Available Options**: false, true

#### (I)Replay Full

Enable/disable full replay of vehicle moves from init state.

**Available Options**: false, true

#### (F)Replay Unconstrained

Enable/disable unconstrained replay of vehicle moves from full state.

**Available Options**: false, true

#### (I)Replay Unconstrained

Enable/disable unconstrained replay of vehicle moves from init state.

**Available Options**: false, true

#### (U)Replay until Move

Set number of unconstrained replays.

**Range Settings**:

Min: 0

Max: 4096

Current Value: 0

Step: 1

#### Num of Instances

Set number of vehicle instances.

**Range Settings**:

Min: 1

Max: 100

Current Value: 1

Step: 1

#### X axis spread

Set X axis spread for vehicle instances.

**Range Settings**:

Min: 0

Max: 100

Current Value: 0

Step: 1

#### Z axis spread

Set Z axis spread for vehicle instances.

**Range Settings**:

Min: 0

Max: 100

Current Value: 0

Step: 1

#### Num of Columns(X)

Set number of columns for vehicle instances.

**Range Settings**:

Min: 0

Max: 100

Current Value: 0

Step: 1

#### Draw recorded paths

Set draw mode for recorded vehicle paths.

**Available Options**: Off, Client, Server, FullSim(W), FullSim(I), Uncon(W), Uncon(I), All

#### SimState to CSV

Export simulation state to CSV.

**Available Options**: false, true

#### WS of (U) to CSV

Export unconstrained simulation state to CSV.

**Available Options**: false, true

#### InitState to CSV

Export initialization state to CSV.

**Available Options**: false, true

### Prediction options

#### C: Disable Correction

Enable/disable client side vehicle correction.

**Available Options**: false, true

#### C: Disable Prediction

Enable/disable client side vehicle prediction.

**Available Options**: false, true

#### C: (T)Prediction Allowance

Set client side vehicle prediction distance allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 25

Step: 1

#### C: (T)Prediction offset

Set client side vehicle prediction offset.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 150

Step: 1

### C: Lerping options

#### C: Debug Lerping

Enable/disable client side vehicle lerping debug.

**Available Options**: false, true

#### C: Lerping Allowance

Set client side vehicle lerping allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 1000

Step: 1

#### C: Lerping Distance Percentage

Set client side vehicle distance lerping percentage.

**Range Settings**:

Min: 0

Max: 100

Current Value: 35

Step: 1

#### C: Lerping Angle Percentage

Set client side vehicle angle lerping percentage.

**Range Settings**:

Min: 0

Max: 100

Current Value: 50

Step: 1

### C: Allowance options

#### S: Enable allowance

Enable/disable server side vehicle allowance.

**Available Options**: false, true

#### S: Speed Scaled Allowance

Enable/disable speed scaling for server side vehicle allowance.

**Available Options**: true, false

#### S: Distance Allowance

Set server side vehicle distance allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 300

Step: 1

#### S: Angle Allowance

Set server side vehicle angle allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 3

Step: 1

#### S: Angular Velocity Allowance

Set server side vehicle angular velocity allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 25

Step: 1

#### S: Linear Velocity Allowance

Set server side vehicle linear velocity allowance.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 30

Step: 1

### Server Car Overrides

#### S: Override KMH

Enable/disable server side KMH override.

**Available Options**: false, true

#### S: -||- close to 0

Enable/disable server side KMH override close to zero.

**Available Options**: false, true

#### S: Override STEERING

Enable/disable server side steering override.

**Available Options**: false, true

#### S: Override THROTTLE

Enable/disable server side throttle override.

**Available Options**: false, true

#### S: Override BRAKE

Enable/disable server side brake override.

**Available Options**: false, true

#### S: Override GEAR

Enable/disable server side gear override.

**Available Options**: false, true

#### S: Override CLUTCH

Enable/disable server side clutch override.

**Available Options**: false, true

### Vehicle type

Set vehicle type.

**Available Options**: Wheeled, Helicopter

### Heli prefab selection

Select helicopter prefab.

**Available Options**: Mi8, UH1

### Show vehicle id

Enable/disable showing of vehicle ID.

**Available Options**: false, true

### Vehicle selection

Set vehicle selection.

**Range Settings**:

Min: 0

Max: 512

Current Value: 0

Step: 1

### S: Input/Output Diags

Set input/output diagnostic mode.

**Available Options**: Off, Sim Out, Sim In, Controller, All

### Debug path correction

Enable/disable path correction debug.

**Available Options**: false, true

### Debug history

Enable/disable network history debug.

**Available Options**: false, true

### Debug engine flags

Enable/disable engine inputs debug.

**Available Options**: false, true

### Draw Phys Comp Debug

Enable/disable drawing of physics component debug.

**Available Options**: false, true

### C: Resend rate

Set client side vehicle resend rate.

**Range Settings**:

Min: 1

Max: 5

Current Value: 2

Step: 1

### S: Replay delay

Set server side vehicle replay delay.

**Range Settings**:

Min: 0

Max: 300

Current Value: 2

Step: 1

### S: Max Rtt allowed

Set server side vehicle max RTT allowed.

**Range Settings**:

Min: 0

Max: 300

Current Value: 60

Step: 1

## Compartments

### Compartment positions

Enable/disable showing of compartment positions.

**Available Options**: false, true

### Show entry points

Enable/disable showing of compartment entry points.

**Available Options**: false, true

### Show accessibility

Enable/disable showing of compartment accessibility.

**Available Options**: false, true

### Show get out prediction

Enable/disable showing of get out prediction.

**Available Options**: false, true

### Show character obstruction trace during get out

Enable/disable showing of character obstruction trace during get out.

**Available Options**: false, true

## MT simulation

Enable/disable multithreaded vehicle simulation.

**Available Options**: true, false

## Show Controller Diags

Enable/disable showing of vehicle controller diagnostics.

**Available Options**: false, true

## Show Compartment Events Diags

Enable/disable showing of vehicle compartment events diagnostics.

**Available Options**: false, true

## Show stats

Enable/disable showing of vehicle statistics.

**Available Options**: false, true

## Show vehicle debug

Enable/disable showing of vehicle debug.

**Available Options**: false, true

## Player vehicle only

Enable/disable showing of player vehicle only.

**Available Options**: true, false

## Show CoM

Enable/disable showing of vehicle center of mass.

**Available Options**: false, true

## Show inertia

Enable/disable showing of vehicle inertia.

**Available Options**: false, true

## Show forces

Enable/disable showing of vehicle forces.

**Available Options**: false, true

## Show engine

Enable/disable showing of vehicle engine.

**Available Options**: false, true

## Show contacts

Enable/disable showing of vehicle contacts.

**Available Options**: false, true

## Show raycast

Enable/disable showing of vehicle raycast.

**Available Options**: false, true

## Show suspension

Enable/disable showing of vehicle suspension.

**Available Options**: false, true

## Show swaybar

Enable/disable showing of vehicle swaybar.

**Available Options**: false, true

## Show wheels

Enable/disable showing of vehicle wheels.

**Available Options**: false, true

## Show bones

Enable/disable showing of vehicle bones.

**Available Options**: false, true

## Show slope

Enable/disable showing of vehicle slope.

**Available Options**: false, true

## Test raycast

Enable/disable testing of raycast.

**Available Options**: false, true

## Reset vehicle

Reset vehicle.

**Hotkey**: `Ctrl` + `Alt` + `R`

**Available Options**: disabled, enabled

## Teleport up

Teleport vehicle up.

**Hotkey**: `Ctrl` + `Alt` + `T`

**Available Options**: disabled, enabled

## Wheel drag set

Enable/disable overriding of wheel drag.

**Available Options**: false, true

## Wheel drag value

Set wheel drag value.

**Range Settings**:

Min: 0

Max: 1

Current Value: 0

Step: 0.05

## Disable freelook

Enable/disable freelook for vehicles.

**Available Options**: false, true

## Enable debugger trace

Enable/disable debugger trace for vehicles.

**Available Options**: false, true

## wheel drag set

Enable/disable overriding of wheel drag (duplicate).

**Available Options**: false, true

## Fuel consumption

Enable/disable fuel consumption debug.

**Available Options**: false, true

## Visualize push force

Enable/disable visualization of push force.

**Available Options**: false, true

## Show dust materials

Enable/disable showing of vehicle dust materials.

**Available Options**: false, true

## Statistics

## FPS

Display FPS statistics.

**Hotkey**: `Ctrl` + `NUM 1`

**Available Options**: none, basic, detailed, average

## General

Display general statistics.

**Hotkey**: `Ctrl` + `NUM 2`

**Available Options**: disable, basic, detailed, all

## Shape stats

Enable/disable shape statistics.

**Available Options**: disabled, enabled

## MeshObject stats

Display mesh object statistics.

**Available Options**: none, count, mesh, faces, verts

## Decal stats

Display decal statistics.

**Available Options**: none, all

## Resource memory stats

Display resource memory statistics.

**Available Options**: none, memory, age

## Texture memory stats

Display texture memory statistics.

**Available Options**: none, memory, age

## Slow script

Set threshold for slow scripts log.

**Available Options**: off, 1ms, 2ms, 5ms, 10ms

## Verbose script Print

Enable/disable verbose script prints.

**Available Options**: false, true

## Script profiler

Enable/disable script profiler.

**Available Options**: off, frame, average

## Script prof. external

Enable/disable external script profiler.

**Available Options**: false, true

## Test mem-stress

Enable/disable memory stress testing.

**Available Options**: off, once, periodic

## Flush memory

Enable/disable memory flushing.

**Available Options**: disabled, enabled

## Flush audio

Enable/disable audio flushing.

**Available Options**: disabled, enabled

## Memory validation

Enable/disable memory validation.

**Available Options**: none, on, off

## Dump render resources

Enable/disable dumping of render resources.

**Available Options**: disabled, enabled

## Counters

Enable/disable performance counters.

**Hotkey**: `Ctrl` + `NUM 3`

**Available Options**: disabled, enabled

## Slow down

Slow down the game.

**Available Options**: off, 1ms, 5ms, 10ms, 20ms, 50ms, 100ms

## Limit FPS

Limit the game's FPS.

**Available Options**: off, 120, 60, 40, 30, 20, 10, 5, 2

## Widget statistics

Enable/disable widget statistics.

**Available Options**: false, true

## Widget hierarchy to log

Log widget hierarchy.

**Available Options**: off, once

## Widget update to log

Log widget updates.

**Available Options**: none, hierarchy, time

## Loaded particle FX

Enable/disable loaded particle FX statistics.

**Available Options**: false, true

## Resource category stats

Enable/disable resource category statistics.

**Available Options**: disabled, enabled

## Dump resources

Toggle. Dumps all resources.

**Available Options**: disabled, enabled

## Log prefab spawn

Log prefab spawning.

**Available Options**: off, non-instant, all

## Data Collection

### Enable debug menu

Enable/disable data collection debug menu.

**Available Options**: false, true

## Render

## RT

### Main RT format

Override main render target format.

**Available Options**: no override, R8G8B8A8\_SRGB, R8G8B8A8, R10G10B10A2, R11G11B10F, R16G16B16A16F

### Main RT MSAA

Override main render target MSAA settings.

**Available Options**: no override, none, 2x, 4x, 8x

### Force custom resolve

Force custom resolve for render target.

**Available Options**: false, true

### Use resolved RT for PPs

Control post-processing RT usage.

**Available Options**: default, disabled, enabled

## Various

### Reload shaders

Enable/disable shader reloading.

**Hotkey**: `Ctrl` + `F12`

**Available Options**: disabled, enabled

### Offline shaders

Enable/disable offline shaders.

**Available Options**: disabled, enabled

### Scene prep. mode

Set scene preparation mode.

**Available Options**: Multithreaded, Singlethreaded

### Cmd list mode

Set command list mode.

**Available Options**: Multithreaded, Singlethreaded

### Log GPU perf diags

Log GPU performance diagnostics.

**Available Options**: off, once, permanent

### Log CMD lists

Log command lists.

**Available Options**: off, once, permament

### Over-sync barriers (PS5)

Set over-sync barriers.

**Available Options**: none, all, RenderTarget, DepthSurface, StencilSurface, RoTexture, RoBuffer, RwTexture, RwBuffer

### Clear RT with

Clear render target with color.

**Available Options**: none, black, white, red, green, blue

### Clear temporal RTs to

Clear temporal render targets.

**Available Options**: none, black, NaN

### GBuffer

Set G-Buffer mode.

**Available Options**: default, AmbientDepthNear, AmbientNear, AmbientDepthFull, AmbientFull

### GBuffer decals

Enable/disable G-Buffer decals.

**Available Options**: false, true

### Decals

Enable/disable decals.

**Available Options**: true, false

### Disable HTILE (PS4&PS5)

Enable/disable HTILE.

**Available Options**: false, true

### Check GPU commands (PS5)

Enable/disable checking of GPU commands.

**Available Options**: false, true

### VSync imm.treshold

Set VSync immediate threshold.

**Available Options**: 0, 10, 20, 30, 40, 50

### VSync limit

Set VSync limit.

**Available Options**: default, 20, 30, 40, 60

### DX12: log trans. barriers

Enable/disable logging of DX12 transition barriers.

**Available Options**: false, true

### DX12: log state commit

Enable/disable logging of DX12 state commits.

**Available Options**: false, true

### CopyHWDepth use depth

Control if CopyHWDepth should use depth.

**Available Options**: true, false

### CopyHWDepth filter

Set filter mode for CopyHWDepth.

**Available Options**: default, firstSample, min, max

### CB draw calls min limit

Enable/disable command buffer draw calls minimum limit.

**Available Options**: true, false

### Validate Input (DP)

Enable/disable validation of input.

**Available Options**: false, true

## System textures

### IBL

#### log messages

Enable/disable IBL logging.

**Available Options**: false, true

#### Env map num

Select which environment map to display.

**Available Options**: none, GlobalSky, GlobalPlaced, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16

#### Refresh

Set refresh mode for IBL.

**Available Options**: no, yes, always, disable

#### Apply HDRmul

Apply or disable HDR multiplier for IBL.

**Available Options**: true, false

#### DFG specular

Enable/disable DFG specular for IBL.

**Available Options**: false, true

#### DFG diffuse

Enable/disable DFG diffuse for IBL.

**Available Options**: false, true

#### Environment

Enable/disable environment for IBL.

**Available Options**: false, true

#### LD specular

Enable/disable LD specular for IBL.

**Available Options**: true, false

#### LD diffuse

Enable/disable LD diffuse for IBL.

**Available Options**: false, true

### Maximize size

Enable/disable Dynamic Texture size maximization.

**Available Options**: false, true

### Show shadowmap

Display shadowmap.

**Available Options**: disable, full smask, R-smask NdotL, G-smask, B-AO, A-wetAdd

### Show combined rainmask

Display combined rainmask.

**Available Options**: disable, full smask, R-wetness, G-c.wetness, B-rain, A-underwate

### Show light shadowmap

Set light shadowmap to show.

**Range Settings**:

* Min: 0
* Max: 36
* Current Value: 0
* Step: 1

### Show cascades

Show shadow cascades.

**Available Options**: disable, cascade 1, cascade 2, cascade 3, cascade 4

### Show rain mask

Show rain mask.

**Available Options**: disable, cascade 1, cascade 2, cascade 3, cascade 4

### Show wetness mask

Show wetness mask.

**Available Options**: disable, cascade 1, cascade 2, cascade 3, cascade 4

### Show custom wetness mask

Show custom wetness mask.

**Available Options**: disable, cascade 1, cascade 2, cascade 3, cascade 4

### Show wetness add mask

Show wetness add mask.

**Available Options**: disable, cascade 1, cascade 2, cascade 3, cascade 4

### Show depthmap

Enable/disable depthmap visualization.

**Available Options**: disabled, enabled

### Show HW depthmap

Enable/disable HW depthmap visualization.

**Available Options**: disabled, enabled

### Terrain shadow map

Show terrain shadow maps.

**Available Options**: none, terrain shadow, shadow raw, shadow volume high, shadow volume low

### Disable SPD

Disable Screen-Space Particle Depth.

**Available Options**: false, true

### Show GBuffer

Show G-Buffer channels (normals, roughness, metalness, albedo, AO).

**Available Options**: none, normals, roughness, metalness, albedo, AO

### Show GBuffer

Show G-Buffer channels (normals, roughness, metalness).

**Available Options**: none, normals, roughness, metalness

### Show volumetric light rays

Enable/disable volumetric light ray visualization.

**Available Options**: disabled, enabled

### Show reflections

Enable/disable reflection visualization.

**Available Options**: disabled, enabled

### Show user textures

Enable/disable user texture visualization.

**Available Options**: disabled

### Show virtual lights RT

Enable/disable virtual lights RT visualization.

**Available Options**: disabled, enabled

## Textures

### Mipmap bias

Set mipmap bias.

**Available Options**: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, -1, -0.75, 0.5, -0.25

### Max mipmap

Set maximum mipmap level.

**Available Options**: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12

### Defrag

Enable/disable defragmentation of texture resources.

**Available Options**: true, false

### Mode

Set mode of defrag.

**Available Options**: auto, once, continuous

### Defrag heap

Set heap for defrag.

**Available Options**: MA heaps, manTextures, manBuffers

### Defrag alg

Set algorithm of defrag.

**Available Options**: fast, balanced, full

## Scene lighting

### Force PBR light

Force or disable PBR light.

**Available Options**: false, true

### Ambient lighting

Enable/disable ambient lighting.

**Available Options**: true, false

### Directional lighting

Enable/disable directional lighting.

**Available Options**: true, false

### Diffuse lighting

Enable/disable diffuse lighting.

**Available Options**: true, false

### Specular lighting

Enable/disable specular lighting.

**Available Options**: true, false

### Emission lighting

Enable/disable emission lighting.

**Available Options**: true, false

### Desaturate Atmosphere (cubemap)

Set desaturation of atmosphere (cubemap).

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0
* Step: 0.01

### Remove interiors fog

Remove fog in interiors.

**Available Options**: true, false

### Capture with glob.probe

Enable/disable capturing with global probe.

**Available Options**: false, true

### Negative probe ambient

Enable/disable negative probe ambient.

**Available Options**: false, true

### IBL reflection/roughness

Set IBL reflection/roughness power.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0
* Step: 0.01

### IBL multiscattering

Enable/disable IBL multiscattering.

**Available Options**: true, false

### Probe lighting mode

Enable/disable probe lighting mode.

**Available Options**: false, true

### Object probe LV

Set object probe luminance value.

**Range Settings**:

Min: -4

Max: 4

Current Value: 0

Step: 0.01

### Show scene LV

Show scene luminance.

**Available Options**: none, pure, NaN, luminance, max.lum, perc.lum, luminanceHM, maxHM, log2HM

### LV auto

Enable/disable automatic adjustment of scene LV.

**Available Options**: true, false

### manual LV middle

Set manual luminance value.
Options: -10 20 5 0.01

### LV +-Range

Set luminance range.

**Range Settings**:

* Min: 1
* Max: 20
* Current Value: 5
* Step: 0.01

### LV watcher

Set luminance watcher.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0.5
* Step: 0.001

### LV watcher size

Set luminance watcher size.

**Range Settings**:

* Min: 0
* Max: 0.2
* Current Value: 0.025
* Step: 0.001

## Shadows

### Shadows

Enable/disable shadows.

**Available Options**: true, false

### Shadows OOF

Enable/disable out of frustum shadows.

**Available Options**: true, false

### Shadows multileaf

Enable/disable multileaf shadows.

**Available Options**: true, false

### Particle shadows

Set particle shadows.

**Available Options**: default, per pixel, per vertex, disabled

### Particle cascades

Set particle shadow cascades.

**Available Options**: default, 1, 2, 3, 4

## PP effects

### DOF Advanced

#### Enabled

Enable/disable advanced depth of field.

**Available Options**: true, false

#### Format

Set the format for advanced depth of field.

**Available Options**: default, R11G11B10F, R16G16B16A16F

#### Quality

Set the quality level for advanced depth of field.

**Available Options**: default, 0, 1, 2, 3

#### Rings

Set the number of rings for advanced depth of field.

**Available Options**: default, 3, 4, 5

#### Combine FS Blur

Set mode to combine full screen blur for advanced depth of field.

**Available Options**: default, enabled, disabled

#### Gather quality

Set gather quality for advanced depth of field.

**Available Options**: default, low, high

#### Force dilate rad.

Set force dilate radius for advanced depth of field.

**Range Settings**:

* Min: 0
* Max: 16
* Current Value: 0
* Step: 1

#### Debug vis

Set debug visualization mode for advanced depth of field.

**Available Options**: disabled, fields, gather, combine, bg col, bg alpha, fg col, fg alpha, fill col, fill alpha

#### Override near blur

Enable/disable overriding the near blur value for advanced depth of field.

**Available Options**: false, true

#### near blur

Set near blur value for advanced depth of field.
Options: -0.0001 0.04 0.02 0.001

#### Override far blur

Enable/disable overriding the far blur value for advanced depth of field.

**Available Options**: false, true

#### far blur

Set far blur value for advanced depth of field.
Options: -0.0001 0.04 0.02 0.001

#### Override FStop

Enable/disable overriding the FStop value for advanced depth of field.

**Available Options**: false, true

#### FStop

Set FStop value for advanced depth of field.

**Range Settings**:

* Min: 0.1
* Max: 32
* Current Value: 2.0
* Step: 0.1

#### Override focus dist

Enable/disable overriding the focus distance for advanced depth of field.

**Available Options**: false, true

#### focus dist

Set focus distance for advanced depth of field.

**Range Settings**:

* Min: 0.0
* Max: 100
* Current Value: 2.0
* Step: 0.1

#### Override focal length

Enable/disable overriding the focal length for advanced depth of field.

**Available Options**: false, true

#### focal length

Set focal length for advanced depth of field.

**Range Settings**:

* Min: 0.01
* Max: 1
* Current Value: 0.05
* Step: 0.01

### Postprocess

Enable/disable all postprocessing effects.

**Hotkey**: `Ctrl` + `Alt` + `P`

**Available Options**: enabled, disabled

### Rain

Enable/disable rain postprocessing effect.

**Available Options**: enabled, disabled

### Snow

Enable/disable snow postprocessing effect.

**Available Options**: enabled, disabled

### Chrom aber

Enable/disable chromatic aberration postprocessing effect.

**Available Options**: enabled, disabled

### Color grading

Enable/disable color grading postprocessing effect.

**Available Options**: disabled, enabled

### Colors

Enable/disable color correction postprocessing effect.

**Available Options**: enabled, disabled

### Depth of field

Enable/disable standard depth of field postprocessing effect.

**Available Options**: enabled, disabled

### Physical Depth of field

Enable/disable physical bokeh depth of field postprocessing effect.

**Available Options**: enabled, disabled

### Force Quality

Set the quality for physical bokeh depth of field.

**Available Options**: default, low, middle, high

### Physical fast Depth of field

Enable/disable fast physical bokeh depth of field postprocessing effect.

**Available Options**: enabled, disabled

### Force Quality

Set the quality for fast physical bokeh depth of field.

**Available Options**: default, low, middle, high

### Dynamic blur

Enable/disable dynamic blur postprocessing effect.

**Available Options**: disabled, enabled

### Film grain

Enable/disable film grain postprocessing effect.

**Available Options**: enabled, disabled

### FXAA

Enable/disable FXAA postprocessing effect.

**Available Options**: enabled, disabled

### God rays

Enable/disable god rays postprocessing effect.

**Available Options**: enabled, disabled

### HBAO

Enable/disable HBAO postprocessing effect.

**Available Options**: enabled, disabled

### Debug

Enable/disable HBAO debugging visualization.

**Available Options**: disabled, enabled

### DebugFullScreen

Enable/disable HBAO fullscreen debugging visualization.

**Available Options**: disabled, enabled

### NumSteps

Set the number of HBAO steps.

**Available Options**: default, 1, 2, 3, 4, 5, 6, 7, 8

### NumDirections

Set the number of HBAO directions.

**Available Options**: default, 1, 2, 3, 4, 5, 6, 7, 8

### HDR

Enable/disable HDR postprocessing effect.

**Available Options**: enabled, disabled

### Debug

Set HDR debug visualization.

**Available Options**: none, mask, LVBlend

### Bloom

Enable/disable bloom effect for HDR.

**Available Options**: enabled, blured, disabled

### Bloom CS

Enable/disable bloom compute shader for HDR.

**Available Options**: false, true

### Bloom Upsample

Select bloom upsample method for HDR.

**Available Options**: before tonemap, in tonemap

### EV

Set HDR exposure value.

**Range Settings**:

Min: -10.0

Max: 10.0

Current Value: 0.0

Step: 0.1

### Show curves

Show HDR curves.

**Available Options**: disabled, histogram, <0..2>lin filmic, <-6..0>LV filmic, <0.01..10>log filmic

### Disable autoExp

Disable auto exposure for HDR.

**Available Options**: false, true

### New LV adaptation

Enable/disable new luminance adaptation for HDR.

**Available Options**: false, true

### HaightmapAO

Enable/disable heightmap based AO postprocessing effect.

**Available Options**: enabled, disabled

### Gauss filter

Enable/disable gaussian blur postprocessing effect.

**Available Options**: enabled, disabled

### Outlines

Enable/disable outline postprocessing effect.

**Available Options**: enabled, disabled

### Radial blur

Enable/disable radial blur postprocessing effect.

**Available Options**: enabled, disabled

### Rot blur

Enable/disable rotation blur postprocessing effect.

**Available Options**: enabled, disabled

### SMAA

Enable/disable SMAA postprocessing effect.

**Available Options**: enabled, disabled

### Edge detection method override

Set the edge detection method override for SMAA.

**Available Options**: Disabled, Color, Depth, Luma

### Show debug textures

Display SMAA debug textures.

**Available Options**: Disabled, SMAAEdgeDetection, SMAABlendingWeightCalculation

### SSAO

Enable/disable SSAO postprocessing effect.

**Available Options**: enabled, disabled

### Method

Set SSAO method.

**Available Options**: non-linear, linear, mask non-linear, mask linear

### BlurSteps

Set blur steps for SSAO.

**Available Options**: default, 0, 1, 2, 3, 4, 5, 6, 7

### SSDO

Enable/disable SSDO postprocessing effect.

**Available Options**: enabled, disabled

### Debug

Enable/disable SSDO debugging visualization.

**Available Options**: disabled, enabled

### DebugFullScreen

Enable/disable SSDO fullscreen debugging visualization.

**Available Options**: disabled, enabled

### SSR (global)

Enable/disable global SSR postprocessing effect.

**Available Options**: enabled, disabled

### SSR (objects only)

Enable/disable object only SSR postprocessing effect.

**Available Options**: enabled, disabled

### Use FFX SSR

Enable/disable FFX Screen Space Reflection.

**Available Options**: enabled, disabled

### Temporal filtering

Enable/disable temporal filtering for SSR.

**Available Options**: enabled, disabled

### Min fade distance

Set minimum fade distance for SSR.

**Range Settings**:

* Min: 0
* Max: 100
* Current Value: 0.5
* Step: 0.01

### Max fade distance

Set maximum fade distance for SSR.

**Range Settings**:

* Min: 0
* Max: 100
* Current Value: 2
* Step: 0.01

### Behind ambient

Correct reflection with ambient.

**Available Options**: disabled, enabled

### UnderWater

Enable/disable underwater postprocessing effect.

**Available Options**: enabled, disabled

### Wet distort

Enable/disable wet distortion postprocessing effect.

**Available Options**: enabled, disabled

## Terrain menu

### Terrain

Enable/disable terrain rendering.

**Available Options**: enabled, disabled

### LOD mode

Set terrain LOD mode.

**Available Options**: auto, exact lod, frac only

### Force exact LOD

Set force exact LOD.

**Range Settings**:

* Min: 1
* Max: 5
* Current Value: 5
* Step: 0.01

### Layers diag mode

Set terrain layers diagnostic mode.

**Hotkey**: `Ctrl` + `Alt` + `L`

**Available Options**: disabled, layers, projection

### Show tiles error

Enable/disable showing of terrain tile errors.

**Available Options**: false, true

### Tile error modif

Set terrain tile error modification.
Options: -10 10 0 0.01

### Blocks' bounding boxes

Enable/disable bounding boxes for terrain blocks.

**Available Options**: disabled, enabled

### MaxLOD blocks' AABB

Enable/disable AABBs for max LOD terrain blocks.

**Available Options**: disabled, enabled

### Biased blocks' AABB

Enable/disable AABBs for biased terrain blocks.

**Available Options**: disabled, enabled

### Tiles' bounding boxes

Enable/disable bounding boxes for terrain tiles.

**Available Options**: disabled, enabled

### Detail texture

Enable/disable terrain detail texture.

**Hotkey**: `Ctrl` + `Alt` + `S`

**Available Options**: enabled, disabled

### Water LOD bias

Set water LOD bias.

**Range Settings**:

* Min: 0
* Max: 1
* Current Value: 0
* Step: 0.001

### Detail texture detail

Set terrain detail texture level.

**Available Options**: normal, -10, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10

### Reload only curr. shaders

Enable/disable reloading of only current terrain shaders.

**Available Options**: false, true

### Use layer cache

Enable/disable terrain layer cache.

**Available Options**: true, false

### Terrain render info

Enable/disable terrain render information.

**Available Options**: false, true

### Holes debug vis

Enable/disable terrain holes debug visualization.

**Available Options**: false, true

### Disabled block debug vis

Enable/disable visualization of disabled terrain blocks.

**Available Options**: false, true

## Grass/Clutter/Obstacles

### Render clutter

Set clutter render mode.

**Available Options**: full, prepare only, disabled

### Show clutter stats

Set clutter statistics.

**Available Options**: none, general, detailed

### Reload clutter configs

Enable/disable reloading of clutter configurations.

**Available Options**: disabled, enabled

### 3D clutter AABB

Enable/disable 3D AABBs for clutter.

**Available Options**: disabled, enabled

### Clutter LOD debug

Enable/disable clutter LOD debugging.

**Available Options**: disabled, enabled

### Clutter LOD blend size

Set clutter LOD blend size.

**Range Settings**:

* Min: 0.0
* Max: 5.0
* Current Value: 1.5
* Step: 0.1

### Thinning slope

Set clutter thinning slope.

**Range Settings**:

* Min: 0.01
* Max: 1.0
* Current Value: 0.25
* Step: 0.01

### Clutter fadeout size

Set clutter fadeout size.

**Range Settings**:

* Min: 0
* Max: 30
* Current Value: 15
* Step: 0.1

### Shadow casting

Enable/disable shadow casting for clutter.

**Available Options**: enabled, disabled

### Clutter position jitter

Set clutter position jitter.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.5
* Step: 0.01

### Enable obstacles

Enable/disable obstacles on terrain.

**Available Options**: true, false

### Enable clutter mask

Enable/disable clutter masking.

**Available Options**: true, false

### Show clutter occldrs

Enable/disable showing of clutter occluders.

**Available Options**: false, true

### Show obst.map

Set obstacle map visualization.

**Available Options**: none, runtime, perm. depth, perm. mask

### Force params:

Enable/disable forcing of obstacle parameters.

**Available Options**: false, true

### RadiusX

Set obstacle radius X.

**Range Settings**:

* Min: 0.5
* Max: 5
* Current Value: 1
* Step: 0.1

### RadiusZ

Set obstacle radius Z.

**Range Settings**:

* Min: 0.5
* Max: 5
* Current Value: 1
* Step: 0.1

### Offset

Set obstacle offset.
Options: -1.0 1.0 0 0.01

### Render grass

Enable/disable grass rendering.

**Available Options**: enabled, disabled

### Grass log

Enable/disable grass logging.

**Available Options**: false, true

### Basic cluster size (10) mul

Set grass cluster size multiplier.

**Range Settings**:

* Min: 1.5
* Max: 10
* Current Value: 10
* Step: 0.01

### Grass distance <50..150>

Set grass detail distance.
Options: -1 1 0 0.1

### Freeze grass pos

Enable/disable freezing of grass positions.

**Available Options**: disabled, enabled

### Grass clusters' boxes

Enable/disable visualization of grass cluster boxes.

**Available Options**: disabled, enabled

### Force grass lod

Set grass LOD level.

**Available Options**: default, 0, 1, 2, 3

### Force grass blend

Set grass blend mode.

**Available Options**: default, alphatest, edges, ATOC(if msaa)

### Grass sort

Enable/disable sorting of grass.

**Available Options**: true, false

### MT rendering

Enable/disable multi-threaded grass rendering.

**Available Options**: true, false

### MT caching

Enable/disable multi-threaded grass caching.

**Available Options**: true, false

### Grass terrain blend

Enable/disable grass terrain blending.

**Available Options**: false, true

### start dist

Set grass terrain blend start distance.

**Range Settings**:

Min: -100

Max: 150

Current Value: 0

Step: 0.1

### end dist

Set grass terrain blend end distance.

**Range Settings**:

Min: -100

Max: 150

Current Value: 0

Step: 0.1

## Roads menu

### Enable roads

Enable/disable road rendering.

**Available Options**: enabled, disabled

## Materials settings

### Grass/Tree wind

#### Wind override

Enable/disable wind override for grass and trees.

**Available Options**: false, true

#### Grass/tree par override

Enable/disable override of parameters for grass and trees.

**Available Options**: false, true

### Enable MultiMat det.mask

Enable/disable use of multi-material detection mask.

**Available Options**: false, true

### Tree colorization

Set tree colorization mode.

**Available Options**: full, color only, disable

### Tree volume fake

Set tree volume fake mode.

**Available Options**: both, geomOcclElipsoid, elipsoidAO, none

### Tree volume shadow

Set tree volume shadow mode.

**Available Options**: none, geomOcclElipsoid, geomOccl, elipsoidAO

### Override geomOccl distance

Enable/disable overriding of geometric occlusion distance.

**Available Options**: false, true

### Max distance

Set max distance for geometric occlusion.

**Range Settings**:

* Min: 0
* Max: 250
* Current Value: 100
* Step: 1

## Sky

### Sky

Enable/disable sky rendering.

**Available Options**: enabled, disabled

### Fog

Enable/disable fog rendering.

**Available Options**: enabled, disabled

### Fog in env. map

Enable/disable fog in environment map.

**Available Options**: enabled, disabled

### Lens Flares

Enable/disable lens flares.

**Available Options**: enabled, disabled

### Godrays

Enable/disable god rays.

**Available Options**: enabled, disabled

### Show planets

Enable/disable rendering of planets.

**Available Options**: enabled, disabled

### Show stars

Enable/disable rendering of stars.

**Available Options**: enabled, disabled

### Show real stars constellations

Enable/disable rendering of real star constellations.

**Available Options**: enabled, disabled

### Show far layer

Enable/disable rendering of far sky layer.

**Available Options**: enabled, disabled

### Show dynamic layer

Enable/disable rendering of dynamic sky layer.

**Available Options**: enabled, disabled

## Rivers

### Central flow func

Set the central flow function for rivers.

**Available Options**: sine, polynomial

### Debug output

Display debug information of river flow.

**Available Options**: no debug, flow map, flow map x, flow map y, flow noise, flow noise - LF part, flow noise - HF part, phase noise, flow mod map

## LightSourcVis

### Blend

Set blend mode for light sources.

**Available Options**: default, additive, alphablend, max

## Subsurface Scattering

### SSSSS override params

Enable/disable overriding of SSSSS parameters.

**Available Options**: false, true

### Enabled

Enable/disable subsurface scattering.

**Available Options**: false, true

### Scale

Set subsurface scattering scale.

**Range Settings**:

* Min: 0
* Max: 4
* Current Value: 1
* Step: 0.01

### Random rotation

Enable/disable random rotation for subsurface scattering.

**Available Options**: false, true

### Log kernel

Enable/disable logging of subsurface scattering kernel.

**Available Options**: false, true

### Use stencil

Enable/disable stencil buffer for subsurface scattering.

**Available Options**: true, false

### Profile resolve

Set profile resolve mode for subsurface scattering.

**Available Options**: default, first, min, max, avg

### SSSSS quality

Set quality for subsurface scattering.

**Available Options**: default, low, medium, high

## Analytic lights

### Disable far lights

Disable or enable far analytic lights.

**Available Options**: false, true

### Show far lights

Show or hide far analytic lights.

**Available Options**: disabled, show, no clip

### Invert geometry

Enable/disable inverting of geometry.

**Available Options**: false, true

### Enable vol lights

Enable/disable volumetric lights.

**Available Options**: enabled, no blend, disabled

### Show log

Enable/disable logging of far lights.

**Available Options**: false, true

### Farlight intensity

Set far light intensity.
Options: -3 20 0 0.1

### Analytic lights

Enable/disable analytic lights.

**Available Options**: true, false

### Lights shadows

Enable/disable analytic lights shadows.

**Available Options**: true, false

### Override limits

Enable/disable overriding of light shadow limits.

**Available Options**: false, true

### Memory budget

Set memory budget for light shadows.

**Range Settings**:

* Min: 10
* Max: 100
* Current Value: 40
* Step: 2

### Max shadows count

Set max shadow count for lights.

**Range Settings**:

* Min: 4
* Max: 64
* Current Value: 16
* Step: 1

### Basic tex size

Set basic texture size for lights shadows.

**Available Options**: 64, 256, 512, 768, 1024, 1536, 2048, 2560, 3072, 3584, 4096

### Aggr. particle lights

Enable/disable aggregation of particle lights.

**Available Options**: true, false

### Autoshrink part. lights

Enable/disable auto shrinking of particle lights.

**Available Options**: true, false

## Occlusion Queries

### Benchmark

Enable/disable occlusion queries benchmark.

**Available Options**: false, true

### TestSun

Enable/disable occlusion queries test sun.

**Available Options**: false, true

## Snow

### Quantize speed

Enable/disable quantization of snow speed.

**Available Options**: true, false

### Quantize dir

Enable/disable quantization of snow direction.

**Available Options**: true, false

### Fix dir

Enable/disable fixing of snow wind direction.

**Available Options**: false, true

### Enable logs

Enable/disable snow logs.

**Available Options**: false, true

### Use underwater mask

Enable/disable underwater mask for snow.

**Available Options**: true, false

### Macro mask debug

Show macro mask debug for snow.

**Available Options**: disabled, layer 1, layer 2, layer 3

### Override snow

Enable/disable overriding snow.

**Available Options**: false, true

### Density

Set snow density.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 1.0
* Step: 0.01

### Time coef

Set snow time coefficient.

**Range Settings**:

* Min: 0.0
* Max: 10.0
* Current Value: 1.0
* Step: 0.01

## Deferred Decals

### Render

Enable/disable rendering of deferred decals.

**Available Options**: true, false

### HW depth

Enable/disable HW depth usage for deferred decals.

**Available Options**: true, false

### Debug visualization

Enable/disable debugging visualization of deferred decals.

**Available Options**: true, false

## Light portals

### Enable light portals

Enable/disable light portals.

**Available Options**: enabled, disabled

### Probe blending mode

Set probe blending mode for light portals.

**Available Options**: directional, diffuse, disabled

### Specular proj. tex.

Enable/disable specular projection texture.

**Available Options**: disabled, enabled

### Proj. tex. offset fix

Enable/disable projection texture offset fix.

**Available Options**: disabled, enabled

### Portal light mode

Set light mode for light portals.

**Available Options**: none, diffuse+specular, diffuse, specular

### Normalize L by area

Enable/disable normalization of light by area.

**Available Options**: disabled, enabled

### Direct light contrib.

Enable/disable direct light contribution.

**Available Options**: disabled, enabled

### Light volume lerp (m)

Set light volume lerp.

**Available Options**: 0.00001, 0.0001, 0.001, 0.01, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.5, 0.75, 1.0

### Disable light volumes

Disable light volumes.

**Available Options**: false, true

### Show active subscene

Show or hide active subscene.

**Available Options**: disabled, enabled

### Show area rectangles

Show or hide area rectangles.

**Available Options**: disabled, enabled

### Show portal rectangles

Show or hide portal rectangles.

**Available Options**: disabled, enabled

### Show light rectangles

Show or hide light rectangles.

**Available Options**: disabled, enabled

### Show debug info

Show light portals debug info.

**Available Options**: none, portals, areas, all

### Virtual light range

Set virtual light range.

**Range Settings**:

* Min: 5
* Max: 50
* Current Value: 25
* Step: 0.1

### BSP diag

Enable/disable BSP diagnostics.

**Available Options**: disabled, enabled

### Hide mesh

Hide light portal mesh.

**Available Options**: disabled, enabled

### Show src. geometry

Show source geometry for light portals.

**Available Options**: disabled, opaque, transparent

### Show occupancy grid

Show occupancy grid for light portals.

**Available Options**: disabled, enabled

### Freeze camera

Enable/disable freezing of camera for light portals.

**Available Options**: disabled, enabled

### Show BSP tree

Show BSP tree for light portals.

**Available Options**: none, AABB, planes, convex

### Leaf mode

Set leaf mode for BSP tree debug.

**Available Options**: all, opaque, non-opaque

### BSP disp. mode

Set display mode for BSP debug.

**Available Options**: normal, at depth, active, connectivity

### Wanted depth

Set wanted depth for BSP debug.

**Range Settings**:

* Min: 0
* Max: 64
* Current Value: 0
* Step: 1

### Show leaf split plane

Show leaf split plane.

**Available Options**: disabled, enabled

### Show areas

Show light portal areas.

**Available Options**: none, AABB, convex

### Area disp. mode

Set display mode for light portal areas.

**Available Options**: all (excl. out), all, single, active

### Area ID

Set area ID for light portals.

**Range Settings**:

* Min: 0
* Max: 128
* Current Value: 0
* Step: 1

### Show portals

Show light portals.

**Available Options**: none, all, single, per area, active

### Portal ID

Set portal ID for light portals.

**Range Settings**:

* Min: 0
* Max: 512
* Current Value: 0
* Step: 1

### Portal disp. mode

Set display mode for light portals.

**Available Options**: rect, AABB, rect no clip, AABB no clip

### Show portal volumes

Show light portal volumes.

**Available Options**: disabled, enabled

### Show virtual lights

Show virtual lights.

**Available Options**: disabled, active all, active single

### Light disp. mode

Set display mode for virtual lights.

**Available Options**: rect, rect+cone, rect no clip, rect+cone no clip

## Terrain shadows

### Shadow cascades

#### Terrain num cascades

Set number of terrain shadow cascades.

**Available Options**: default, 0, 1, 2

#### Terrain near type

Set near cascade type for terrain.

**Available Options**: default, movable, static

#### Terrain near m/px

Set near cascade meters per pixel for terrain.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### Terrain far type

Set far cascade type for terrain.

**Available Options**: default, movable, static

#### Terrain far m/px

Set far cascade meters per pixel for terrain.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### Objects num cascades

Set number of object shadow cascades.

**Available Options**: default, 0, 1, 2

#### Objects near type

Set near cascade type for objects.

**Available Options**: default, movable, static

#### Objects near m/px

Set near cascade meters per pixel for objects.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### Objects far type

Set far cascade type for objects.

**Available Options**: default, movable, static

#### Objects far m/px

Set far cascade meters per pixel for objects.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### AO num cascades

Set number of AO shadow cascades.

**Available Options**: default, 0, 1, 2

#### AO near type

Set near cascade type for AO.

**Available Options**: default, movable, static

#### AO near m/px

Set near cascade meters per pixel for AO.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### AO far type

Set far cascade type for AO.

**Available Options**: default, movable, static

#### AO far m/px

Set far cascade meters per pixel for AO.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

### Objects heightmap

#### Obj. heightmap movable

Set object heightmap movability.

**Available Options**: movable, static

#### Draw volumetric info

Draw volumetric debug info.

**Available Options**: false, true

#### Draw object heightmap

Draw object heightmap.

**Available Options**: disabled, set regions, set bits, all bits

#### Obj.height.full upd

Enable/disable full object heightmap update.

**Available Options**: false, true

#### Obj.height. dbg region

Enable/disable object heightmap region debug.

**Available Options**: false, true

#### Obj.height.reset

Reset object heightmap.

**Available Options**: false, true

#### Obj.height.reset stats

Reset object heightmap statistics.

**Available Options**: false, true

#### Obj.height.print stats

Print object heightmap statistics.

**Available Options**: false, true

#### Obj.height.use prepare

Enable/disable prepare for object heightmap.

**Available Options**: true, false

#### Obj.height.use CB

Enable/disable command buffer for object heightmap.

**Available Options**: false, true

#### Obj.height.max merge

Set maximum merge for object heightmap.

**Available Options**: unlimited, 1, 4, 8, 16

#### Obj.height. subdiv

Enable/disable object heightmap subdivision.

**Available Options**: false, true

#### Obj.height. sync draw

Set object heightmap draw call synchronization.

**Available Options**: default, sync, no sync

#### Obj. heightmap tex size

Set object heightmap texture size.

**Available Options**: no override, 1024, 2048, 4096, 8192

#### Obj. heightmap m/pixel

Set object heightmap meters per pixel.

**Available Options**: default, 0.5, 1, 1.5, 2, 3, 4, 5, 6, 7, 8

#### Min entities per frame

Set min entities per frame for heightmap.

**Available Options**: default, 512, 1K, 2K, 4K, 8K, 16K, 32K, 64K, 128K, unlimited

#### Max entities per frame

Set max entities per frame for heightmap.

**Available Options**: default, 512, 1K, 2K, 4K, 8K, 16K, 32K, 64K, 128K, unlimited

### Terrain shadows

Set terrain shadows mode.

**Available Options**: default, on (slice), on (full), no update, disabled

### Freeze update

Freeze terrain shadow update.

**Available Options**: false, true

### Quality profile override

Override terrain shadow quality profile.

**Available Options**: no override, disabled, low, medium, high

### Diag messages

Enable/disable terrain shadows debug messages.

**Available Options**: false, true

### Texture size

Override terrain shadow texture size.

**Available Options**: no override, 1024, 2048, 4096, 8192

### Blur override

Override terrain shadow blur.

**Available Options**: no override, 0, 3, 5, 7, 9

### Hierarchical raytrace

Enable/disable hierarchical raytrace for terrain shadows.

**Available Options**: true, false

### Force mipmap rebuild

Enable/disable forcing of mipmap rebuild for terrain shadows.

**Available Options**: false, true

### Herarchical mip1

Set hierarchical mip 1 level.

**Available Options**: default, none, 0, 1, 2, 3, 4, 5, 6

### Herarchical mip2

Set hierarchical mip 2 level.

**Available Options**: default, none, 0, 1, 2, 3, 4, 5, 6

### Herarchical mip3

Set hierarchical mip 3 level.

**Available Options**: default, none, 0, 1, 2, 3, 4, 5, 6

### Herarchical mip4

Set hierarchical mip 4 level.

**Available Options**: default, none, 0, 1, 2, 3, 4, 5, 6

### Override Angle bias

Enable/disable overriding of angle bias.

**Available Options**: false, true

### Angle bias (deg)

Set angle bias for terrain shadows.

**Range Settings**:

Min: -10

Max: 10

Current Value: -0.5

Step: 0.1

### Override Height bias

Enable/disable overriding of height bias.

**Available Options**: false, true

### Height bias (m)

Set height bias for terrain shadows.

**Range Settings**:

Min: 0

Max: 10

Current Value: 0.5

Step: 0.1

### Override Trace bias

Enable/disable overriding of trace bias.

**Available Options**: false, true

### Trace bias

Set trace bias for terrain shadows.

**Range Settings**:

Min: 0

Max: 10

Current Value: 0.0

Step: 0.1

### Override Trace bias obj

Enable/disable overriding of object trace bias.

**Available Options**: false, true

### Trace bias obj min

Set object trace bias minimum.

**Range Settings**:

Min: 0

Max: 10

Current Value: 0.1

Step: 0.1

### Trace bias obj max

Set object trace bias maximum.

**Range Settings**:

Min: 0

Max: 10

Current Value: 1.5

Step: 0.1

### Object shadows

Set object shadows mode.

**Available Options**: default, force off, force on

### Object shadow type

Set object shadow type.

**Available Options**: no override, Physics, Physics+AAB, AAB, AABScaled, Volume

### Occ grid runtime

Enable/disable runtime creation of occupation grid.

**Available Options**: false, true

### Blend mode

Set blend mode for terrain shadows.

**Available Options**: blend, min

### Border size

Set border size for terrain shadows.

**Available Options**: default, 0, 0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5

### Sun move test

Set sun move test mode.

**Available Options**: off, fwd slow, fwd medium, bwd fast, bwd slow, bwd medium, bwd fast

### Min pixels per frame

Set min pixels per frame for terrain shadows.

**Available Options**: default, 16K, 32K, 64K, 128K, 256K, 512K, unlimited

### Max pixels per frame

Set max pixels per frame for terrain shadows.

**Available Options**: default, 16K, 32K, 64K, 128K, 256K, 512K, unlimited

### Obj shadow blend dist(m)

Set object shadow blend distance.

**Range Settings**:

Min: 0

Max: 20

Current Value: 10.0

Step: 0.1

### Terrain shadow blend dist(m)

Set terrain shadow blend distance.

**Range Settings**:

Min: 0

Max: 20

Current Value: 3.0

Step: 0.1

### AO mode override

Set AO mode override for terrain shadows.

**Available Options**: no override, disabled, PP half, PP mixed, PP full, force precomputed, presomputed+runtime, force runtime

### Override AO coef

Enable/disable overriding of AO coefficient.

**Available Options**: false, true

### AO coef

Set AO coefficient.

**Range Settings**:

* Min: 0
* Max: 1.0
* Current Value: 0.3
* Step: 0.02

### AO radius

Set AO radius.

**Range Settings**:

* Min: 0
* Max: 10.0
* Current Value: 3.0
* Step: 0.2

### AO blend distance

Set AO blend distance.

**Range Settings**:

* Min: 0
* Max: 10.0
* Current Value: 5.0
* Step: 0.2

### AO PP mode

Set AO post-processing mode.

**Available Options**: default, full res, half res, half res + reconst

### Terr. light angle limit

Set light angle limit for terrain shadows.

**Available Options**: default, user

### Limit angle

Set light limit angle.

**Range Settings**:

* Min: 1
* Max: 60
* Current Value: 15
* Step: 0.1

## FSR

### Resolution scale

Set resolution scale for FSR.

**Available Options**: default, manual, 100%, FSR Ultra (77%), FSR Quality (67%), FSR Balanced (59%), FSR Perf (50%)

### Manual scale

Set manual scale for FSR.

**Range Settings**:

* Min: 0.1
* Max: 2.0
* Current Value: 1.0
* Step: 0.01

### Use pixel shader

Enable/disable pixel shader for FSR.

**Available Options**: false, true

### Override FSR settings

Enable/disable overriding of FSR settings.

**Available Options**: false, true

### FSR enabled

Enable/disable FSR.

**Available Options**: false, true

### FSR sharpen

Enable/disable FSR sharpening.

**Available Options**: true, false

### FSR sharpen att.

Set FSR sharpen amount.

**Range Settings**:

* Min: 0.0
* Max: 2.0
* Current Value: 0.5
* Step: 0.01

### FSR dither

Set FSR dither amount.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 1.0
* Step: 0.01

### FSR RT format

Set render target format for FSR.

**Available Options**: default, R8G8B8A8, R10G10B10A2, R11G11B10F, R16G16B16A16F

### FSR mip bias

Set mip bias for FSR.

**Available Options**: disabled, automatic, manual

### manual mip bias

Set manual mip bias for FSR.
Options: -4.0 0.0 0.0 0.01

## CAS

### Shader

Set shader type for CAS.

**Available Options**: default, force PS, force CS

### Override CAS settings

Enable/disable overriding of CAS settings.

**Available Options**: false, true

### enabled

**Available Options**: false, true

### sharpness

Set CAS sharpness.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.8
* Step: 0.01

## Rain

### Wetness

#### Override wetness

Enable/disable overriding of rain wetness.

**Available Options**: false, true

#### Wetness

Set rain wetness.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.01

#### Flood cracks

Set rain flood level for cracks.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.01

#### Flood puddles

Set rain flood level for puddles.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.01

#### Tree scale

Set rain tree scale.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.01

#### Wetness simulation

Enable/disable wetness simulation.

**Available Options**: false, true

#### Simulation value

Set wetness simulation value.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.002

#### Override wetness render params

Enable/disable overriding of wetness render params.

**Available Options**: false, true

### Wind

#### Override rain wind

Enable/disable overriding of rain wind.

**Available Options**: false, true

#### Mask operator

Set mask operator for rain wind.

**Available Options**: add, mul

#### Override splash wind

Enable/disable overriding of splash wind.

**Available Options**: false, true

#### Mask operator

Set mask operator for splash wind.

**Available Options**: add, mul

### Downsample

Set rain color downsample method.

**Available Options**: default, simple, blur, none

### Blend method

Set rain blend method.

**Available Options**: default, standard, noRefraction

### Diag refr. tex

Enable/disable rain refraction texture diagnostic.

**Available Options**: false, true

### Stop time

Enable/disable stopping of rain simulation time.

**Available Options**: false, true

### Use underwater mask

Enable/disable underwater mask for rain.

**Available Options**: true, false

### Override rain density

Enable/disable overriding of rain density.

**Available Options**: false, true

### Rain density

Set rain density.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.0
* Step: 0.02

### Override direction

Enable/disable overriding of rain direction.

**Available Options**: false, true

### Override dirX

Set rain direction X.
Options: -1.0 1.0 0.0 0.05

### Override dirY

Set rain direction Y.
Options: -1.0 1.0 -1.0 0.05

### Override dirZ

Set rain direction Z.
Options: -1.0 1.0 0.0 0.05

### Distant noise mask debug

Enable/disable rain distant noise mask debug.

**Available Options**: false, true

### Override ripple tex size

Override rain ripple texture size.

**Available Options**: default, 128, 256, 512, 1024

### Sliding drops obj space

Enable/disable sliding drops object space.

**Available Options**: false, true

### Override ripples

Enable/disable overriding of rain ripples.

**Available Options**: false, true

### Ripples count min

Set rain ripples count minimum.

**Range Settings**:

* Min: 1.0
* Max: 100.0
* Current Value: 10.0
* Step: 1.0

### Ripples count max

Set rain ripples count maximum.

**Range Settings**:

* Min: 1.0
* Max: 100.0
* Current Value: 50.0
* Step: 1.0

### Ripples size min

Set rain ripples size minimum.

**Range Settings**:

* Min: 1.0
* Max: 20.0
* Current Value: 1.0
* Step: 1.0

### Ripples size max

Set rain ripples size maximum.

**Range Settings**:

* Min: 1.0
* Max: 20.0
* Current Value: 5.0
* Step: 1.0

### Ripples height min

Set rain ripples height minimum.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 0.5
* Step: 0.01

### Ripples height max

Set rain ripples height maximum.

**Range Settings**:

* Min: 0.0
* Max: 1.0
* Current Value: 1.0
* Step: 0.01

### Ripples speed

Set rain ripples speed.

**Range Settings**:

* Min: 1.0
* Max: 2000.0
* Current Value: 500.0
* Step: 10.0

### Ripples damping

Set rain ripples damping.

**Range Settings**:

* Min: 0.01
* Max: 1.0
* Current Value: 0.04
* Step: 0.01

### Ripples normal strength

Set rain ripples normal strength.

**Range Settings**:

* Min: 0.1
* Max: 20.0
* Current Value: 8.0
* Step: 0.1

### Ripples reset

Enable/disable resetting of rain ripples.

**Available Options**: false, true

## Shore Wetness

### Shore wetness enabled

Enable/disable shore wetness.

**Available Options**: default, enable, disable

### Shore wetness override

Enable/disable overriding of shore wetness.

**Available Options**: false, true

### Shore wet height above

Set shore wet height above.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### Shore wet fade above

Set shore wet fade above.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### Shore wet height below

Set shore wet height below.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### Shore wet fade below

Set shore wet fade below.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### Shore wet fade coef

Set shore wet fade coefficient.

**Range Settings**:

Min: 0.001

Max: 10.0

Current Value: 1.0

Step: 0.01

### Shore wet rough fade start

Set shore wet rough fade start.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.0

Step: 0.01

### Shore wet rough fade size

Set shore wet rough fade size.

**Range Settings**:

Min: 0.001

Max: 10.0

Current Value: 1.0

Step: 0.01

### Shore wet rough coef

Set shore wet rough coefficient.

**Range Settings**:

Min: 0.001

Max: 10.0

Current Value: 1.0

Step: 0.01

### River wetness enabled

Enable/disable river wetness.

**Available Options**: default, enable, disable

### River wetness override

Enable/disable overriding of river wetness.

**Available Options**: false, true

### River wet height above

Set river wet height above.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### River wet fade above

Set river wet fade above.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### River wet height below

Set river wet height below.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### River wet fade below

Set river wet fade below.

**Range Settings**:

Min: 0.0

Max: 10.0

Current Value: 0.5

Step: 0.01

### River wet fade coef

Set river wet fade coefficient.

**Range Settings**:

Min: 0.001

Max: 10.0

Current Value: 1.0

Step: 0.01

### River wet rough coef

Set river wet rough coefficient.

**Range Settings**:

Min: 0.001

Max: 10.0

Current Value: 1.0

Step: 0.01

## LensFlares

### Debug render

Enable/disable lens flare debug rendering.

**Available Options**: false, true

### HalfRes render

Set half resolution render for lens flares.

**Available Options**: no override, false, true

### Force flare dirlight

Set force flare dirlight.
Options: -1 100 -1 1

### Force flare pointlight

Set force flare pointlight.
Options: -1 100 -1 1

### Force flare spotlight

Set force flare spotlight.
Options: -1 100 -1 1

### Override flare set

Override lens flare set.

**Available Options**: no override, none, 1st person, 3rd person

### Override offset spot

Enable/disable overriding of offset spot for lens flares.

**Available Options**: false, true

### Offset spot

Set offset spot for lens flares.
Options: -2 2 0 0.01

### Override offset point

Enable/disable overriding of offset point for lens flares.

**Available Options**: false, true

### Offset point

Set offset point for lens flares.
Options: -2 2 0 0.01

### Use new occl.system

Set new occlusion system for lens flares.

**Available Options**: no override, false, true

### Use intens.occ.for all

Enable/disable intensity occlusion for all lens flares.

**Available Options**: no override, false, true

## Ocean & Water

### Shore

#### Shore enabled

Enable/disable shore rendering.

**Available Options**: true, false

#### Debug visualize

Show debug visualization of shore.

**Available Options**: disabled, waves, DF distance, DF direction, DF near, peaks tex, noise onShore, noise foam start, noise foam, damp wave, damp peak, damp depth, on shore coef, custom foam, mask

#### Debug displacement

Enable/disable shore displacement debugging.

**Available Options**: false, true

#### Debug height

Show shore height debug.

**Available Options**: none, final, shore, final+shore

#### Use height lerp

Enable/disable height lerp for shore.

**Available Options**: false, true

#### Force live preview each frame

Enable/disable forcing live preview for shore.

**Available Options**: false, true

### WaterErase

#### Water erase enabled

Enable/disable water erase.

**Available Options**: true, false

#### Debug visualize

Show water erase debug visualization.

**Available Options**: disabled, front faces, back faces, front & back

### Ocean render

Set ocean render mode.

**Available Options**: enabled, disable simulation, disabled

### Show ocean tex

Display ocean texture.

**Available Options**: disable, height, displaceX, displaceZ, normal, foam, UW topdown, UW topdown depth, UW geom depth

### Tex size

Set ocean debug texture size.

**Available Options**: 1, 2, 3, 4, 0.25, 0.33, 0.5

### Water bodies

Show water bodies.

**Available Options**: invisible, camera, all

### Ocean foam

Set ocean foam mode.

**Available Options**: enabled, mask, foam

## Atmosphere

### Show Sky LUTs

Enable/disable showing of sky LUTs.

**Available Options**: false, true

### Show CameraVolume

Enable/disable showing of CameraVolume.

**Available Options**: false, true

### FullScreen

Enable/disable fullscreen view for CameraVolume.

**Available Options**: false, true

### Haze source

Set haze source for atmosphere.

**Available Options**: User(CloudsCycle), Atmosphere, GlobalProbe

### Enable a.fog shadows

Enable/disable analytical fog shadows.

**Available Options**: false, true

### Use SkyAmbient tex.

Enable/disable use of sky ambient texture.

**Available Options**: true, false

### Probe fake

Set probe ambient for atmosphere.

**Available Options**: probe lut, sky lut, original

### Overcast fix.

Set overcast fix mode for atmosphere.

**Available Options**: enabled, disabled

### Original lighting

Set original lighting mode for atmosphere.

**Available Options**: enabled, disabled

### Final output brightness mod.

Enable/disable final brightness modification for atmosphere.

**Available Options**: disabled, enabled

### Show Fog Debug.

Enable/disable fog debugging visualization.

**Available Options**: disabled, enabled

### Show constellations

Enable/disable rendering of constellations.

**Available Options**: false, true

## Volumetric Clouds

### Use New

Use the new volumetric cloud system.

**Available Options**: false, true

### Render clouds

Enable/disable volumetric cloud rendering.

**Available Options**: enabled, disabled

### Render Mode

Set volumetric cloud render mode.

**Available Options**: Quarter, Full, Experimental

### Output Optimization

Set volumetric cloud output optimization.

**Available Options**: default, none, vertexShader, CPU

### Show Output Optim Diag

Show output optimization diagnostic for clouds.

**Available Options**: false, true

### Apply cycle values

Apply cycle values for clouds.

**Available Options**: true, false

### Sky light mode

Set sky light mode for clouds.

**Available Options**: Default(Artistic), New(Probe+Atmo), Gray, PBR(OriginalTechnique)

### Disable direct light

Disable direct light for clouds.

**Available Options**: false, true

### Enable occlusion map

Enable/disable occlusion map for clouds.

**Available Options**: enabled, disabled

### Disable wind

Enable/disable wind for clouds.

**Available Options**: false, true

### Stop wind

Stop wind for clouds.

**Available Options**: false, true

### Show cloud lighting

Show or hide cloud lighting visualization.

**Available Options**: disabled, enabled

### Force noise recompute

Force recomputation of cloud noise.

**Available Options**: disabled, enabled

### Show detail noise

Enable/disable detail noise visualization for clouds.

**Available Options**: disabled, enabled

### Show noise tiling

Set noise tiling visualization for clouds.

**Available Options**: disabled, base, middle, detail, newBase\_R, newBase\_G, newBase\_B, newBase\_A, newDetail\_R, newDetail\_G, newDetail\_B, newDetail\_A

### Show noise tiling

Set noise tiling visualization for clouds (base, middle, detail).

**Available Options**: disabled, base, middle, detail

### Show height detail noise

Enable/disable height detail noise texture visualization.

**Available Options**: disabled, enabled

### Detail noise resolution

Set resolution of detail noise for clouds.

**Available Options**: default, 16, 32, 64, 128, 256, 512

### Weather map resolution

Set weather map resolution for clouds.

**Available Options**: default, 256, 512, 1024, 2048

### Temporal multiplier

Set temporal multiplier for clouds.

**Range Settings**:

Min: 0

Max: 5

Current Value: 0.0

Step: 0.01

### Max. Num. samples

Set max number of samples for clouds.

**Range Settings**:

Min: 0

Max: 256

Current Value: 0.0

Step: 1.0

### HeightMin

Set minimum height for clouds.

**Range Settings**:

Min: 0

Max: 20000

Current Value: 0.0

Step: 10.0

### Show shadowmap

Enable/disable showing of shadow map for clouds.

**Available Options**: disabled, enabled

### Shadowmap resolution

Set shadow map resolution for clouds.

**Range Settings**:

Min: 0

Max: 256

Current Value: 0

Step: 1

### Shadowmap dispatches p. frame

Set shadow map dispatches per frame for clouds.

**Range Settings**:

Min: 0

Max: 4

Current Value: 0

Step: 1

### Debug shape

Set debug shape visualization for clouds.

**Available Options**: none, cylinders, spheres

### haze value

Set haze value for clouds.

**Range Settings**:

Min: 0.0001

Max: 0.1

Current Value: 0.018

Step: 0.001

### Enable dist. fog

Enable/disable distance fog for clouds.

**Available Options**: enabled, disabled

### dist. fog range

Set distance fog range for clouds.

**Range Settings**:

Min: 0

Max: 400000

Current Value: 65000

Step: 1000

### debug detail noise weights

Enable/disable main shape noise weights debug.

**Available Options**: false, true

### disable noise(middle,micro)

Enable/disable noise (middle, micro) for clouds.

**Available Options**: false, true

### offet X

Set offset X for clouds.

**Range Settings**:

Min: 0

Max: 1000000

Current Value: 0

Step: 10

### offet Z

Set offset Z for clouds.

**Range Settings**:

Min: 0

Max: 1000000

Current Value: 0

Step: 10

## No render

Disable rendering.

**Available Options**: false, true

## Shape instancing

Enable/disable shape instancing.

**Available Options**: enabled, disabled

## Render debug mode

Set render debug mode.

**Hotkey**: `RCtrl` + `AltGr` + `W`

**Available Options**: none, wire, wire only, ovedraw, overdrawZ, albedo, world norm, roughness, rough.scene, metalness, mask, ao, vert.color, sss, detail light, diffuse, iblDiffuse, reflection, iblReflection, full diffuse, full reflection, emissive, albedo lin, wetness, porosity, vertex norms, vfx, global layer

## Widgets

Set widget rendering mode.

**Available Options**: on, onlyRT, off

## Active GPU wait

Enable/disable active GPU wait.

**Available Options**: false, true

## Allocator stats

Set allocator statistics display.

**Available Options**: none, show, dump

## Show VRAM resources

Enable/disable showing of VRAM resources.

**Available Options**: false, true

## Scene

## Filter

### MeshObject

Enable/disable MeshObject filter.

**Available Options**: enabled, disabled

### Particle

Enable/disable Particle filter.

**Available Options**: enabled, disabled

## Occluders

### Occluders

Enable/disable occluders.

**Available Options**: true, false

### Occlude entities

Enable/disable occluding of entities.

**Available Options**: true, false

### Show active occluders

Show or hide active occluders.

**Available Options**: false, true

### Show occluded

Show or hide occluded objects.

**Available Options**: false, true

### Show occluder volumes

Show or hide occluder volumes.

**Available Options**: false, true

## Terrain Occlusion

### Terrain occlusion culling

Enable/disable terrain occlusion culling.

**Available Options**: true, false

### Occlusion mode

Set terrain occlusion mode.

**Available Options**: Voxelization, Hybridvoxel

### Cull dynamic objects

Enable/disable culling of dynamic objects.

**Available Options**: true, false

### Cull individual objects

Enable/disable culling of individual objects.

**Available Options**: true, false

### Cull entity lists

Enable/disable culling of entity lists.

**Available Options**: true, false

### Cull kd-tree leafs

Enable/disable culling of kd-tree leafs.

**Available Options**: false, true

### Cull terrain

Enable/disable culling of terrain.

**Available Options**: false, true

### Cull kd-tree nodes

Enable/disable culling of kd-tree nodes.

**Available Options**: true, false

### # voxel render jobs

Set number of voxel render jobs.

**Range Settings**:

* Min: 1
* Max: 8
* Current Value: 8
* Step: 1

### Show depth texture

Enable/disable depth texture visualization.

**Available Options**: false, true

### Show FOV deb shapes

Enable/disable showing of FOV debug shapes.

**Available Options**: false, true

### Voxel height scale

Set voxel height scale.

**Range Settings**:

* Min: 0.25
* Max: 32
* Current Value: 1
* Step: 0.01

### Voxel depth LOD step

Set voxel depth LOD step.

**Range Settings**:

* Min: 0.01
* Max: 10
* Current Value: 0.25
* Step: 0.01

### Depth buffer fraction

Set depth buffer fraction.

**Range Settings**:

* Min: 1
* Max: 100
* Current Value: 16
* Step: 1

### Voxel near plane

Set voxel near plane.

**Range Settings**:

* Min: 1
* Max: 100
* Current Value: 20
* Step: 1

### Override far plane

Enable/disable overriding of far plane.

**Available Options**: false, true

### Voxel far plane

Set voxel far plane.

**Range Settings**:

* Min: 1
* Max: 5000
* Current Value: 2000
* Step: 50

### Show AAB of occluded

Show AAB of occluded objects.

**Available Options**: None, Occluded, Visible, All

### Voxel occlusion check

Set voxel occlusion check method.

**Available Options**: Hi-Z, Full-Z raster

### Rasterize AABB

Enable/disable rasterization of AABBs.

**Available Options**: false, true

### Show rasterized AABB

Enable/disable showing of rasterized AABBs.

**Available Options**: false, true

### Draw silhouette edges

Enable/disable drawing of silhouette edges.

**Available Options**: false, true

### Cull beyond frustum edges

Enable/disable culling beyond frustum edges.

**Available Options**: false, true

### # frames reuse voxel buffer

Set number of frames to reuse voxel buffer.

**Range Settings**:

* Min: 0
* Max: 60
* Current Value: 0
* Step: 1

### Raster size threshold

Set raster size threshold.

**Range Settings**:

* Min: 0
* Max: 1024
* Current Value: 64
* Step: 2

## Render world

Enable/disable world rendering.

**Available Options**: enabled, disabled

## Force vertical FOV

Force vertical field of view.

**Available Options**: default, 65, 70, 75, 80, 85, 90, 95, 100, 105, 110, 120, 2.5f, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55

## Render world env

Enable/disable rendering of world environment.

**Available Options**: enabled, disabled

## Show clips

Show or hide scene clips.

**Available Options**: disabled, enabled

## LOD multiplier

Set LOD multiplier.

**Available Options**: 1, 1.5, 2, 3, 4, 5, 10, 0.001, 0.01, 0.1, 0.25, 0.5

## Size multiplier

Set size multiplier.

**Available Options**: 1, 1.5, 2, 3, 4, 5, 10, 0.001, 0.01, 0.1, 0.25, 0.5

## Override object far plane

Set object far plane override.

**Range Settings**:

* Min: 0
* Max: 50000
* Current Value: 0
* Step: 100

## Force clear visent

Force clear of visible entities.

**Available Options**: false, true

## Render world ents

Enable/disable rendering of world entities.

**Available Options**: enabled, disabled

## Render world leafs

Enable/disable rendering of world leafs.

**Available Options**: enabled, disabled

## RenderEnts

Enable/disable entities rendering.

**Available Options**: enabled, disabled

## Fast way

Enable/disable fast rendering path.

**Available Options**: false, true

## Show light

Set global light to show.
Options: -1 32 -1 1

## Show active PPs

Show or hide active post processes.

**Available Options**: false, true

## Cube map size

Set cubemap size.

**Available Options**: 32, 16, 32, 64, 128, 256, 512, 1024

## Capture cube map

Capture cube map.

**Available Options**: no, capture, capture&insert

## Override far plane

Set far plane override.

**Range Settings**:

* Min: 0
* Max: 50000
* Current Value: 0
* Step: 100

## Force linear depth

Enable/disable forcing of linear depth.

**Available Options**: false, true

## Linear depth

Set linear depth.

**Range Settings**:

* Min: 50
* Max: 5000
* Current Value: 250
* Step: 10

## Visualize groups

Enable/disable group visualization.

**Available Options**: false, true

## Colorize objects LODs

Colorize objects by LOD.

**Available Options**: disabled, colorize, LOD0 only, +LOD1, +LOD2, +LOD3, +LOD4, +LOD5, +LOD6, +LOD7

## World

## Animation

Enable/disable apply pose in animation.

**Available Options**: true, false

## Streaming

Enable/disable world streaming.

**Available Options**: true, false

## Trace stresstest

Set trace stress test mode.

**Available Options**: none, nearest, any, all, nearestVis, anyVis, allVis

## Query stresstest

Enable/disable query stress test.

**Available Options**: disabled, enabled

## Update stresstest

Enable/disable update stress test.

**Available Options**: disabled, enabled

## Update MT stresstest

Enable/disable multithreaded update stress test.

**Available Options**: disabled, enabled

## Show scene tree

Show scene tree.

**Available Options**: None, KDTree, QuadTree

## Show entity bounds

Show or hide entity bounds.

**Available Options**: disabled, enabled

## Async traces

Enable/disable asynchronous traces.

**Available Options**: enabled, disabled

## Async queries

Enable/disable asynchronous queries.

**Available Options**: enabled, disabled

## Use quadtree

Enable/disable quadtree usage.

**Available Options**: enabled, disabled

## Show AABB

Show AABB (Axis-Aligned Bounding Box). When you enable "Show AABB" in the Diag Menu, the engine will render these bounding boxes around objects, making them visible to help with debugging. This can be extremely useful for:

* Verifying collision boundaries
* Checking if physics objects have appropriate bounds
* Debugging spatial issues with entities
* Understanding why certain objects might be colliding unexpectedly

**Hotkey**: `Alt` + `NUM 5`

**Available Options**: disabled, real, quantized, rendered

## Show multileaf ents.

Show or hide multileaf entities.

**Available Options**: disabled, all, single, multi

## Show active

Show or hide active entities.

**Available Options**: enabled, disabled

## Show bones

Show or hide bones.

**Available Options**: disabled, enabled

## Show wind emitters

Show or hide wind emitters.

**Hotkey**: `Ctrl` + `NUM 8`

**Available Options**: disabled, enabled

## Disable wind

Disable wind.

**Hotkey**: `Ctrl` + `NUM 9`

**Available Options**: disabled, enabled

## Game update

Enable/disable game update.

**Available Options**: enabled, disabled

## MT entity sim

Enable/disable multithreaded entity simulation.

**Available Options**: enabled, disabled

## MT entity update

Enable/disable multithreaded entity update.

**Available Options**: enabled, disabled

## Entity diag

Enable/disable entity diagnostics.

**Available Options**: false, true

## Entity hierarchy changes

Enable/disable entity hierarchy change diagnostics.

**Available Options**: false, true

## Debug prefabs

Enable/disable debug prefabs.

**Available Options**: disabled, enabled

## New skeleton

Enable/disable new skeleton.

**Available Options**: false, true

## JobManager Callstack

Set JobManager callstack logging mode.

**Available Options**: off, ext, all

## JobManager Print CS

Set JobManager print callstack mode.

**Available Options**: off, once, always

## Ambient Life

Enable/disable ambient life.

**Available Options**: false, true

## Animation

## Enable debugger

Enable/disable animation debugger.

**Available Options**: disabled, enabled

## Root motion always enabled

Enable/disable root motion always enabled.

**Available Options**: false, true

## Force animation LOD

Set force animation LOD.
Options: -1 4 -1 1

## Show transforms before IK

Enable/disable showing of transforms before IK.

**Available Options**: false, true

## Show transforms after IK

Enable/disable showing of transforms after IK.

**Available Options**: false, true

## Show IK offsets

Enable/disable showing of IK offsets.

**Available Options**: false, true

## Show IK solver details

Enable/disable showing of IK solver details.

**Available Options**: false, true

## Show procedural solver details

Enable/disable showing of procedural solver details.

**Available Options**: false, true

## Show cached MS transforms

Enable/disable showing of model space cache.

**Available Options**: false, true

## Show RBF details

Enable/disable showing of RBF details.

**Available Options**: false, true

## Record animations

Enable/disable animation recording.

**Available Options**: false, true

## Record local player

Set animation recording entities mode.

**Available Options**: Only characters, All entities

## Animation traffic

Enable/disable animation traffic logging.

**Available Options**: false, true

## Min IK blend time

Set minimum IK blend time.

**Range Settings**:

* Min: 0.01
* Max: 1
* Current Value: 0.01
* Step: 0.01

## Physics

## Settings

### Update rate

Set physics update rate.

**Range Settings**:

Min: 20

Max: 1000

Current Value: 60

Step: 10

### Max fixed steps

Set maximum fixed steps for physics.

**Range Settings**:

Min: 1

Max: 60

Current Value: 10

Step: 1

### Apply

Enable/disable physics settings apply.

**Available Options**: disabled, enabled

## Bullet

### Max. Collider Distance

Set maximum distance for Bullet collider.

**Available Options**: 0, 1, 2, 5, 10, 20, 50, 100, 200, 500

### Draw Bullet shape

Enable/disable drawing of Bullet shape.

**Available Options**: enabled, disabled

### Draw Bullet wireframe

Enable/disable drawing of Bullet shape wireframe.

**Available Options**: enabled, disabled

### Draw Bullet shape AABB

Enable/disable drawing of Bullet shape AABB.

**Available Options**: disabled, enabled

### Draw obj center of mass

Enable/disable drawing of object center of mass.

**Available Options**: disabled, enabled

### Draw Bullet contacts

Enable/disable drawing of Bullet contacts.

**Available Options**: enabled, disabled

### Force sleep Bullet

Enable/disable forcing of Bullet to sleep.

**Available Options**: disabled, enabled

### Profile Bullet

Set threshold for Bullet profiling.

**Available Options**: disabled, >1ms, >5ms, >10ms, >20ms, >50ms

### Bullet MT

Enable/disable multithreaded Bullet simulation.

**Available Options**: true, false

## Dump contacts on server

Enable/disable server side physics contacts dump.

**Available Options**: false, true

## Show bodies

Show or hide physics bodies. Visible colliders can be controlled with Show layer

**Hotkey**: `Alt` + `NUM 6`

**Available Options**: disabled, flat-mesh, transp-mesh, flat+mesh, transp+mesh

## Show layer

Show physics bodies by layer. Example layers available:

* **Filtered** - default mode, shows all the most important layers, most of the time there is no need to switch away from it
* **Fire geometry** - "realistic" resolution colliders used for hit detection

[![armareforger-diag-menu-show-layers-firegeo.jpg](/wikidata/images/thumb/6/62/armareforger-diag-menu-show-layers-firegeo.jpg/600px-armareforger-diag-menu-show-layers-firegeo.jpg)](/wiki/File:armareforger-diag-menu-show-layers-firegeo.jpg)

* **View geometry** - used by AIs, acts as solid wall

[![armareforger-diag-menu-show-layers-viewgeo.jpg](/wikidata/images/thumb/4/4c/armareforger-diag-menu-show-layers-viewgeo.jpg/600px-armareforger-diag-menu-show-layers-viewgeo.jpg)](/wiki/File:armareforger-diag-menu-show-layers-viewgeo.jpg)

* **Foliage** - used by AIs, slows down detection time (tree crowns, bushes)

[![armareforger-diag-menu-show-layers-foliage.jpg](/wikidata/images/thumb/9/97/armareforger-diag-menu-show-layers-foliage.jpg/600px-armareforger-diag-menu-show-layers-foliage.jpg)](/wiki/File:armareforger-diag-menu-show-layers-foliage.jpg)

* **Interaction layer** - used by UserAction together with fire geometry, when fire geometry is not enough for pleasant user action experience (used on stuff like wire fence doors)

[![armareforger-diag-menu-show-layers-interaction-1.jpg](/wikidata/images/thumb/7/71/armareforger-diag-menu-show-layers-interaction-1.jpg/300px-armareforger-diag-menu-show-layers-interaction-1.jpg)](/wiki/File:armareforger-diag-menu-show-layers-interaction-1.jpg)[![armareforger-diag-menu-show-layers-interaction-2.jpg](/wikidata/images/thumb/8/85/armareforger-diag-menu-show-layers-interaction-2.jpg/300px-armareforger-diag-menu-show-layers-interaction-2.jpg)](/wiki/File:armareforger-diag-menu-show-layers-interaction-2.jpg)

* **Mine layer** - mines and objects that are meant to collide with mines, such as wheels of vehicles

[![armareforger-diag-menu-show-layers-mine.jpg](/wikidata/images/thumb/a/a0/armareforger-diag-menu-show-layers-mine.jpg/597px-armareforger-diag-menu-show-layers-mine.jpg)](/wiki/File:armareforger-diag-menu-show-layers-mine.jpg)

* **Char collider** - used for collision between character and terrain

[![armareforger-diag-menu-show-layers-character.jpg](/wikidata/images/thumb/8/87/armareforger-diag-menu-show-layers-character.jpg/600px-armareforger-diag-menu-show-layers-character.jpg)](/wiki/File:armareforger-diag-menu-show-layers-character.jpg)

* **Vehicle simple** - used for collision between body of the car and non-characters

[![armareforger-diag-menu-show-layers-vehicle-simple.jpg](/wikidata/images/thumb/f/f8/armareforger-diag-menu-show-layers-vehicle-simple.jpg/600px-armareforger-diag-menu-show-layers-vehicle-simple.jpg)](/wiki/File:armareforger-diag-menu-show-layers-vehicle-simple.jpg)

**Available Options**: [*See Collision Layer page for all options*](/wiki/Arma_Reforger:Collision_Layer#Layers_Setup "Arma Reforger:Collision Layer")

## Show collider type

Show physics bodies by collider type. See c[ollider shape](/wiki/Arma_Reforger:FBX_Import#Collider_shape "Arma Reforger:FBX Import") pargraph on FBX Import page

**Available Options**: all, sphere, box, capsule, cylinder, convex, trimesh

## Show simulation state

Show physics bodies by simulation state.

**Available Options**: col+sim, none, collision, simulation, all

## Show body type

Show physics bodies by body type.

**Available Options**: all, static, dynamic, kinematic

## Color by

Set physics bodies color mode.

**Available Options**: collider type, activation state, body type, by entity

## Show COM

Show or hide physics center of mass.

**Available Options**: false, true

## Simulation

Enable/disable physics simulation.

**Hotkey**: `Ctrl` + `NUM 7`

**Available Options**: enabled, disabled

## Dump non-static bodies

Enable/disable dumping of non-static physics bodies.

**Available Options**: disabled, enabled

## Dump contacts

Enable/disable dumping of physics contacts.

**Available Options**: false, true

## Show stats

Set physics statistics display mode.

**Available Options**: disabled, basic, detailed, all

## Show Bullet

Enable/disable showing of Bullet physics debug.

**Available Options**: disabled, enabled

## Scripts

### Show interior bounding box

Show or hide interior bounding box.

**Available Options**: false, true

### Show helper trace

Show or hide physics helper trace.

**Available Options**: false, true

## Game

## Weather

### Force weather update (WB edit mode)

Enable/disable forcing of weather update.

**Available Options**: false, true

### Debug time

Enable/disable weather debug time.

**Available Options**: disabled, enabled

### time

Set weather debug time.

**Range Settings**:

Min: 0

Max: 1

Current Value: 0.0

Step: 0.0001

### Show diag.

Show or hide weather diagnostics.

**Available Options**: false, true

### Show diag. values

Show or hide weather diagnostic values.

**Available Options**: false, true

### Override wet speeds

Enable/disable overriding wetness speeds.

**Available Options**: false, true

### - Wetting speed

Set current wetting speed.

**Range Settings**:

* Min: 0
* Max: 100
* Current Value: 1.0
* Step: 0.1

### - Drying speed

Set current drying speed.

**Range Settings**:

* Min: 0
* Max: 100
* Current Value: 1.0
* Step: 0.1

### Override states

Enable/disable overriding weather states.

**Available Options**: false, true

### - State source

Set current weather state source.

**Range Settings**:

* Min: 0
* Max: 10
* Current Value: 0
* Step: 1

### - State destination

Set current weather state destination.

**Range Settings**:

* Min: 0
* Max: 10
* Current Value: 0
* Step: 1

### Override variants

Enable/disable overriding weather variants.

**Available Options**: false, true

### - Variant source

Set current weather variant source.

**Range Settings**:

* Min: 0
* Max: 10
* Current Value: 0
* Step: 1

### - Variant destination

Set current weather variant destination.

**Range Settings**:

* Min: 0
* Max: 10
* Current Value: 0
* Step: 1

## Dialogue

### Demo

Enable/disable dialogue demo mode.

**Available Options**: false, true

## Superscreenshot to file

Save super screenshot to file.

**Hotkey**: `Ctrl` + `⇧ Shift` + `F7`

**Available Options**: false, true

## Screenshot to clipboard

Save screenshot to clipboard.

**Hotkey**: `⇧ Shift` + `F7`

**Available Options**: false, true

## Screenshot to file

Save screenshot to file.

**Hotkey**: `Ctrl` + `F7`

**Available Options**: false, true

## Deployable SpawnPoints

### Show Exclusion Zones

Show or hide deployable spawn points exclusion zones.

**Available Options**: false, true

## Voting

### Enable All Vote Types

Enable/disable all vote types.

**Available Options**: false, true

## Show bounds overlap target info

Show or hide bounds overlap target info.

**Available Options**: false, true

## Show cursor target info

Show or hide cursor target info containing information about prefab you are looking at and its full path to find it in the Workbench

[![armareforger-diag-menu-show-layers-cursor-target-info.jpg](/wikidata/images/thumb/c/c6/armareforger-diag-menu-show-layers-cursor-target-info.jpg/600px-armareforger-diag-menu-show-layers-cursor-target-info.jpg)](/wiki/File:armareforger-diag-menu-show-layers-cursor-target-info.jpg)

**Available Options**: false, true

## Copy view link

Create [World Editor view link](/wiki/Arma_Reforger:Workbench_Links#World_Editor "Arma Reforger:Workbench Links") using hotkey (LCtrl+LShift+L).

**Hotkey**: `Ctrl` + `⇧ Shift` + `L`

**Available Options**: false, true

## Sounds

## Audio Debug Context

### Template Debug Context

Select audio debug context.

**Available Options**:

### Signal info

Enable/disable showing of signal info.

**Available Options**: false, true

### Samples

Enable/disable showing of audio samples.

**Available Options**: false, true

## VON

### Play recorder VON

Enable/disable playing of recorded VON.

**Available Options**: false, true

### Replace VON with 440Hz

Enable/disable replacement of VON with 440Hz tone.

**Available Options**: false, true

### Bypass loudness

Enable/disable loudness bypass for VON.

**Available Options**: false, true

### Log starving

Enable/disable logging of VON starving.

**Available Options**: false, true

## Sound World

### Sound map

Set sound map type.

**Available Options**: none, terrain, water

### Sound map range

Set sound map range.

**Range Settings**:

Min: 25

Max: 250

Current Value: 50

Step: 25

### Sound map create

Set sound map creation mode.

**Available Options**: none, create, create+save

## Sound Geometry

### Sound Geometry Map

#### Shadowed

Enable/disable showing of shadowed areas.

**Available Options**: false, true

#### Visible

Enable/disable showing of visible areas.

**Available Options**: false, true

#### Obstacle

Enable/disable showing of obstacles.

**Available Options**: false, true

#### Edge

Enable/disable showing of edges.

**Available Options**: false, true

#### Diffract

Enable/disable showing of diffraction.

**Available Options**: false, true

#### Tile Type

Set tile type display for sound geometry.

**Available Options**: Square, Value

#### Sample Visibility

Enable/disable showing of sound sample visibility.

**Available Options**: false, true

#### Set Sample Pos

Enable/disable setting of sound sample position.

**Available Options**: false, true

### Enable Diag

Enable/disable sound geometry diagnostics.

**Available Options**: disabled, enabled

### System Info

Enable/disable showing of sound geometry system info.

**Available Options**: disabled, enabled

### Interior Cache Info

Enable/disable showing of sound geometry interior cache info.

**Available Options**: disabled, enabled

### Show Update Range

Enable/disable showing of sound geometry update range.

**Available Options**: disabled, enabled

### Object Info

Enable/disable showing of sound geometry object info.

**Available Options**: disabled, enabled

### Show Building Portals

Show building portals.

**Available Options**: none, all, single, per area, active

### Show Buildings Rect

Show or hide buildings rectangles.

**Available Options**: disabled, enabled

### Hide Building Mesh

Hide building mesh.

**Available Options**: disabled, enabled

### Obstruction Pos

Enable/disable obstruction position debug.

**Available Options**: disabled, enabled

### Freeze Positions

Enable/disable freezing of positions for sound geometry.

**Available Options**: disabled, enabled

## Interior Handler

### InteriorHandler

Select interior handler debug.

**Available Options**:

### Old interior detection

Enable/disable old interior detection.

**Available Options**: false, true

### Enable Debug

Enable/disable interior handler debug.

**Available Options**: false, true

### Queries

Enable/disable interior queries debug.

**Available Options**: false, true

## SoundMap

### SoundMap

Select sound map debug.

**Available Options**:

### Show values

Enable/disable showing of sound map values.

**Available Options**: false, true

### Draw Regions

Enable/disable drawing of sound map regions.

**Available Options**: false, true

### Layer to draw

Select sound map layer to draw.

**Available Options**: Terrain, Coast, Freshwater

### Show memory allocated

Enable/disable showing of sound map memory allocated.

**Available Options**: false, true

### Force generation

Enable/disable forcing of sound map generation.

**Available Options**: false, true

### Force default SoundMap

Enable/disable forcing of default sound map.

**Available Options**: false, true

## Chimera Sounds

### Signals Debug

Enable/disable chimera sound signals debug.

**Available Options**: false, true

### Signals Debug by Entity

Enable/disable chimera sound signals debug by entity.

**Available Options**: false, true

## Ambients

Set ambients debug mode.

**Available Options**: None, River, Coast, Lake

## Freeze cam position

Enable/disable freezing of camera position for ambients debug.

**Available Options**: false, true

## Debug Com.Sound

Enable/disable communication sound debug.

**Available Options**: false, true

## MM - debug info

Enable/disable music manager debug info.

**Available Options**: false, true

## Debug RadioBroadcast

Enable/disable radio broadcast debug.

**Available Options**: false, true

## Show invalid event

Enable/disable showing of invalid sound events.

**Available Options**: false, true

## Show system info

Set sound system info display.

**Available Options**: none, generic, cmds, alloc

## Resources

Set sound resources display.

**Available Options**: none, all, events

## Show categories

Enable/disable showing of sound categories.

**Available Options**: false, true

## Show sounds

Enable/disable showing of playing sounds.

**Available Options**: false, true

## Show sounds 3D

Set 3D sound visualization.

**Available Options**: none, all, waiting, playing, fading

## Log source info

Enable/disable logging of sound source info.

**Available Options**: false, true

## Com debug

Enable/disable communication debug.

**Available Options**: false, true

## Enable ITD

Enable/disable ITD.

**Available Options**: true, false

## Play recorded VON

Enable/disable playing of recorded VON.

**Available Options**: false, true

## Print Melee Surface

Enable/disable printing of melee surface.

**Available Options**: false, true

## Reload AmbientSounds Conf

Enable/disable reloading of ambient sound configurations.

**Available Options**: false, true

## Random Ambient Sounds

Enable/disable random positional ambient sounds.

**Available Options**: false, true

## Show MPD Impulse Values

Enable/disable showing of MPD impulse values.

**Available Options**: false, true

## Sound Manager

Enable/disable sound manager debug.

**Available Options**: false, true

## Global Variables

Enable/disable sound global variables debug.

**Available Options**: false, true

## Particles

## Disable particle render

Enable/disable particle rendering.

**Available Options**: false, true

## Disable particle update

Enable/disable particle update.

**Available Options**: false, true

## Show particle FX LOD

Enable/disable showing of particle FX LOD.

**Available Options**: disabled, enabled

## Show 3D rotation spaces

Set scale for showing of particle rotation spaces.

**Available Options**: off, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0

## Overdraw optimization

Set overdraw optimization mode.

**Available Options**: off, overhead (prerender), overhead (complete), on

## Render

Set particle render mode.

**Available Options**: full, background only, no background

## Depth write mode

Set particle depth write mode.

**Available Options**: full-screen tri, particle draw

## Prerender mask

Set particle prerender mask mode.

**Available Options**: on, off, overhead

## Copy from main RT

Enable/disable copying frommain render target.

**Available Options**: enabled, disabled

## Show internal texture

Show particle internal texture.

**Available Options**: none, depth ranges, alpha pre, alpha full

## Show Particle Contacts

Enable/disable showing of particle contacts.

**Available Options**: false, true

## Input

## Sink Input

Enable/disable sink input.

**Available Options**: false, true

## Debug Contexts

Set input context debug mode.

**Available Options**: disabled, active, all

## Show Cursor

Enable/disable showing of input cursor.

**Available Options**: false, true

## UI

## Disable Loc

Enable/disable localization.

**Available Options**: false, true

## Show UI Diag

Enable/disable UI diagnostics.

**Available Options**: false, true

## Log Construction

Enable/disable logging of UI construction.

**Available Options**: false, true

## refactored

### Override config

Override radial menu config.

**Available Options**: true")", false, true

### Close on open

Close radial menu on open.

**Available Options**: true")", false, true

### Perform on close

Perform action on radial menu close.

**Available Options**: true")", false, true

### Close on perform

Close radial menu on perform.

**Available Options**: true")", false, true

### Delect in center

Deselect radial menu in center.

**Available Options**: true")", false, true

### Large size

Use large size for radial menu.

**Available Options**: true")", false, true

### Override size

Override radial menu custom size.

**Available Options**: true")", false, true

### Sustom size

Set custom radial menu size.

**Range Settings**:

Min: 100

Max: 1000

Current Value: 100

Step: 1

### Entries setup

Enable/disable custom entry count for radial menu.

**Available Options**: true")", false, true

### Entries

Set custom entry count for radial menu.

**Range Settings**:

Min: 2

Max: 32

Current Value: 2

Step: 1

## Close All Menus

Close all menus.

**Available Options**: false, true

## Open Main Menu

Open main menu.

**Available Options**: false, true

## Log widgets under cursor

Enable/disable logging of widgets under cursor.

**Available Options**: false, true

## Ignore hint persistency

Enable/disable ignoring of hint persistency.

**Available Options**: false, true

## Enable map debug menu

Enable/disable map debug menu.

**Available Options**: false, true

## Show all settings

Enable/disable showing of all UI settings.

**Available Options**: false, true

## AI

## Performance

### Show IsAIActivated

Enable/disable showing of IsAIActivated flag.

**Available Options**: false, true

### AILib counters

Enable/disable AI library counters.

**Available Options**: false, true

### Load balancing

Enable/disable AI load balancing.

**Available Options**: true, false

### Forced AI simulation LOD

Set forced AI simulation LOD.

**Range Settings**:

Min: -1

Max: 10

Current Value: -1

Step: 1

### BT holder

Enable/disable BT holder debug.

**Available Options**: false, true

### AI Limits

Enable/disable AI limits debug.

**Available Options**: false, true

## Navmesh

### Show NavMesh

Enable/disable showing of NavMesh.

**Available Options**: false, true

### Generate without navlinks

Enable/disable generation of NavMesh without navlinks.

**Available Options**: true, false

### Show links between polygons

Enable/disable showing of links between polygons.

**Available Options**: false, true

### Navlinks Validation

Enable/disable Navlinks validation.

**Available Options**: false, true

### Navlink Active Direction

Enable/disable showing of Navlink active direction.

**Available Options**: false, true

### Show Navmesh Project Index

Set Navmesh project index to show.

**Range Settings**:

* Min: 0
* Max: 1000
* Current Value: 0
* Step: 1

### Ignore area costs

Enable/disable ignoring of area costs for pathfinding.

**Available Options**: false, true

### Path smoothing (on/off)

Set path smoothing mode.

**Available Options**: default, on, off

## Movement

### Path

Enable/disable path debug.

**Available Options**: false, true

### Path index

Enable/disable showing of current path index.

**Available Options**: false, true

### Obstacle Avoidance diag

Enable/disable obstacle avoidance debug.

**Available Options**: false, true

### Current Navlink

Enable/disable showing of current Navlink.

**Available Options**: false, true

### Request parameters

Enable/disable showing of request parameters.

**Available Options**: false, true

### Movement parameters

Enable/disable showing of movement parameters.

**Available Options**: false, true

### Request Target

Enable/disable showing of request target.

**Available Options**: false, true

### Movement Target

Enable/disable showing of movement target.

**Available Options**: false, true

### Orientation Target

Enable/disable showing of orientation target.

**Available Options**: false, true

### Formation offsets

Set formation offsets debug mode.

**Available Options**: off, projected, default, both

### Formation Center

Enable/disable showing of formation center.

**Available Options**: false, true

### Move Handlers

Enable/disable showing of move handlers.

**Available Options**: false, true

### Stuck Teleport

Enable/disable teleport if stuck.

**Available Options**: false, true

### Out of Navmesh Teleport

Enable/disable teleport if out of NavMesh.

**Available Options**: false, true

### Memory

Enable/disable showing of movement memory.

**Available Options**: false, true

## Pathfinding

### Debug of AStar

Enable/disable AStar debug.

**Available Options**: false, true

### Make a step of AStar

Step AStar pathfinding.

**Hotkey**: `Ctrl` + `Alt` + `V`

**Available Options**: false, true

### Pathfinding component

Enable/disable pathfinding component debug.

**Available Options**: false, true

### Road network

Set road network debug mode.

**Hotkey**: `Ctrl` + `Alt` + `N`

**Available Options**: off, active, roads, children, bridges, offRoad, links, all

### Show Road Width

Enable/disable showing of road widths.

**Hotkey**: `Ctrl` + `Alt` + `W`

**Available Options**: false, true

### Road network component

Set road network component debug.

**Range Settings**:

Min: -1

Max: 300

Current Value: -1

Step: 1

### Road Id

Enable/disable showing of road IDs.

**Hotkey**: `Ctrl` + `Alt` + `I`

**Available Options**: false, true

### RoadPart Min Max Id

Enable/disable showing of road part min max ids.

**Available Options**: false, true

### Pathfinding errors

Enable/disable reporting of pathfinding errors.

**Available Options**: false, true

### Link on bridge Id

Set link on bridge ID.

**Range Settings**:

Min: 0

Max: 1000

Current Value: 0

Step: 1

## Aiming

### Aiming component

Enable/disable aiming component debug.

**Available Options**: false, true

### Aiming component PID

Enable/disable aiming component PID debug.

**Available Options**: false, true

### Head aiming component

Enable/disable head aiming component debug.

**Available Options**: false, true

### Use aim at version of ai aiming

Enable/disable usage of aim at version of AI aiming.

**Available Options**: false, true

### Show target aimpoint

Show or hide target aimpoint.

**Available Options**: false, true

### Show target projected size

Show or hide projected target size.

**Available Options**: false, true

## AI Navigator

### Enabled

Enable/disable AI navigator debug.

**Available Options**: false, true

### Previous agent

Select previous AI navigator agent.

**Hotkey**: `RCtrl` + `RShift ⇧` + `9`

**Available Options**: false, true

### Next agent

Select next AI navigator agent.

**Hotkey**: `RCtrl` + `RShift ⇧` + `0`

**Available Options**: false, true

## Driving

### Show character detection

Enable/disable character detection debug.

**Available Options**: false, true

### Vehicle driving diag

Enable/disable vehicle driving diagnostics.

**Available Options**: false, true

### AI Forbid Reversing

Enable/disable AI reversing.

**Available Options**: false, true

### AI No Steer Adjustments

Enable/disable AI steering adjustments.

**Available Options**: false, true

### Curve Circumference

Enable/disable curve circumference debug.

**Available Options**: false, true

### PID Display

Enable/disable PID debug.

**Available Options**: false, true

### Show avoidance

Enable/disable vehicle avoidance debug.

**Available Options**: false, true

## Perception

### Perception targets

Enable/disable perception targets debug.

**Available Options**: false, true

### Perception sensors eyes

Enable/disable perception sensors eyes debug.

**Available Options**: false, true

### Perception sensors ears

Enable/disable perception sensors ears debug.

**Available Options**: false, true

### Perception recognition

Enable/disable perception recognition factors debug.

**Available Options**: false, true

### Cached perceiveble positions

Enable/disable showing of cached perceived positions.

**Available Options**: false, true

## Animals

### Show animals debug info

Enable/disable animals debug.

**Available Options**: false, true

### Manager to debug

Select animal manager to debug.

**Available Options**: None, Birds

### Show current Animal positions

Enable/disable showing of current animal positions.

**Available Options**: false, true

### Draw tile boundaries

Enable/disable drawing of animal tile boundaries.

**Available Options**: false, true

### Draw Animal despawn range

Enable/disable drawing of animal despawn range.

**Available Options**: false, true

### Draw Animal to Animal minimum spawn range

Enable/disable drawing of animal to animal minimum spawn range.

**Available Options**: false, true

### Turn correction debug

Enable/disable turn correction debug.

**Available Options**: false, true

## POV Pathfinder

### Extend of querry box

Set extend of query box.

**Range Settings**:

Min: 1

Max: 20

Current Value: 10

Step: 1

### Number of iterations

Set number of iterations.

**Range Settings**:

Min: 1

Max: 10

Current Value: 5

Step: 1

### Fail on hitting origin

Enable/disable pathfinding failure on hitting origin.

**Available Options**: false, true

### Set start position

Set pathfinder start position.

**Hotkey**: `Ctrl` + `Alt` + `NUM 0`

**Available Options**: false, true

### Set end position

Set pathfinder end position.

**Hotkey**: `Ctrl` + `Alt` + `NUM 1`

**Available Options**: false, true

### Run pathfinder

Run pathfinder.

**Hotkey**: `Ctrl` + `Alt` + `NUM 3`

**Available Options**: false, true

### Generate Graph

Generate pathfinder graph.

**Hotkey**: `Ctrl` + `Alt` + `NUM 2`

**Available Options**: false, true

## Show Cover Query

Enable/disable showing of cover query.

**Available Options**: false, true

## Disable AI damage

Set AI damage mode.

**Available Options**: Cheat disabled, Force invulnerable, Force vulnerable

## Disable AI actions

Enable/disable AI action processing.

**Hotkey**: `Ctrl` + `Alt` + `F9`

**Available Options**: false, true

## Smart actions usage

Enable/disable smart actions debug.

**Available Options**: false, true

## Group members

Enable/disable showing of AI group members.

**Available Options**: false, true

## Waypoints

Set waypoint visualization.

**Available Options**: off, cylinder, sphere

## AI Units

Enable/disable AI units debug.

**Available Options**: false, true

## Messages

Enable/disable AI messages debug.

**Available Options**: false, true

## Show target AI context

Enable/disable showing of target AI context.

**Available Options**: false, true

## AIScript

### AI Debug Category

Set AI debug category.

**Available Options**: MISSING CATALOGS!

### Set BT Breakpoint

Enable/disable BT breakpoint.

**Available Options**: false, true

### Print debug from BTs

Enable/disable printing of debug from BTs.

**Available Options**: false, true

### Print init of groups

Enable/disable printing of group init info.

**Available Options**: false, true

### Show fireteams

Enable/disable showing of fireteams.

**Available Options**: false, true

### Show subgroups

Enable/disable showing of subgroups.

**Available Options**: false, true

### Show target clusters

Enable/disable showing of target clusters.

**Available Options**: false, true

### Print stats for aiming

Enable/disable printing of aiming statistics.

**Available Options**: false, true

### Print new activity

Enable/disable printing of new AI activity.

**Available Options**: false, true

### Show debug shapes from BTs

Enable/disable showing of debug shapes from BTs.

**Available Options**: false, true

### Show fiendly in aim

Enable/disable showing of friendly units in aim.

**Available Options**: false, true

### Show target last seen

Enable/disable showing of target last seen position.

**Available Options**: false, true

### Select fixed AIAgent

Enable/disable selection of fixed AI agent.

**Available Options**: false, true

### Debug cover search

Enable/disable cover search debug.

**Available Options**: false, true

### Show Comms Handlers

Enable/disable showing of comms handlers.

**Available Options**: false, true

### Show perception panel

Enable/disable showing of perception panel.

**Available Options**: false, true

### Show grenade positions

Enable/disable showing of grenade positions.

**Available Options**: false, true

### Show suppression debug

Enable/disable showing of suppression debug.

**Available Options**: false, true

### Show illum flares positions

Enable/disable showing of illum flares positions.

**Available Options**: false, true

### Show aim leading debug

Enable/disable showing of aim leading debug.

**Available Options**: false, true

### Show sector threat filter

Enable/disable showing of sector threat filter.

**Available Options**: false, true

### Show search and destroy points of interest

Enable/disable showing of search and destroy points of interest.

**Available Options**: false, true

### Show Send Message Menu

Enable/disable showing of send message menu.

**Available Options**: false, true

### Open Debug Panel

Enable/disable opening of AI debug panel.

**Available Options**: false, true

### Show Grenade Trace

Enable/disable showing of grenade trace.

**Available Options**: false, true

## Systems

## DynSim Distance Diag

Enable/disable Dynamic Simulation distance diagnostics.

**Available Options**: false, true

## DynSim Visibility Diag

Enable/disable Dynamic Simulation visibility diagnostics.

**Available Options**: false, true

## ProcAnim Registration Diag

Enable/disable procedural animation registration diagnostics.

**Available Options**: false, true

## HandleControls Registration Diag

Enable/disable handle controls registration diagnostics.

**Available Options**: false, true

## NwkMovement Registration Diag

Enable/disable network movement registration diagnostics.

**Available Options**: false, true

## Sound Registration Diag

Enable/disable sound registration diagnostics.

**Available Options**: false, true

## Backend Diag

Enable/disable backend diagnostics.

**Available Options**: false, true

## AnimKeyFrame Fake Time

Set animation keyframe fake time.

**Range Settings**:

Min: 0

Max: 1.0

Current Value: 0.0

Step: 0.05

## Systems diag

Set systems diagnostics display.

**Available Options**: none, basic, full

## Show diag system stats

Enable/disable showing of system statistics.

**Available Options**: false, true

## Platform

## PlayStation Network matches

### Show match status

Enable/disable showing of match status.

**Available Options**: false, true

### Create dummy match

Enable/disable creation of dummy match.

**Available Options**: false, true

### Start match

Enable/disable start of match.

**Available Options**: false, true

### Cancel match

Enable/disable cancelation of match.

**Available Options**: false, true

### Finish match (submit result)

Enable/disable finishing of match (submit result).

**Available Options**: false, true

### \*Join match (dummy player)

Enable/disable joining of match (dummy player).

**Available Options**: false, true

### \*Leave match (dummy player)

Enable/disable leaving of match (dummy player).

**Available Options**: false, true

### Add player via update (dummy player)

Enable/disable adding of player via update (dummy player).

**Available Options**: false, true

## Pick user

Enable/disable picking of user.

**Available Options**: false, true

## Simulate Sign-out

Simulate user sign out.

**Available Options**: false, true

## Obtain save data

Obtain save data.

**Available Options**: false, true

## Commit save data

Commit save data.

**Available Options**: false, true

## Open URL

Open URL in browser.

**Available Options**: false, true

## Change user name

Change user name.

**Available Options**: false, true

## Show local player profile

Show local player profile.

**Available Options**: false, true

## Request & log privileges

Request and log privileges.

**Available Options**: false, true

## Revoke privilege: Text comm

Revoke text communication privilege.

**Available Options**: false, true

## Revoke privilege: Voice comm

Revoke voice communication privilege.

**Available Options**: false, true

## Revoke privilege: Multiplayer

Revoke multiplayer privilege.

**Available Options**: false, true

## Revoke privilege: Lobby

Revoke lobby privilege.

**Available Options**: false, true

## Revoke privilege: UGC

Revoke UGC privilege.

**Available Options**: false, true

## Revoke privilege: Shared sessions

Revoke shared sessions privilege.

**Available Options**: false, true

## Revoke privilege: Social network sharing

Revoke social network sharing privilege.

**Available Options**: false, true

## Accept dummy invitation

Accept dummy invitation.

**Available Options**: false, true

## Invite using platform GUI

Invite using platform GUI.

**Available Options**: false, true

## Log list of achievements

Log list of achievements.

**Available Options**: false, true

## Serialization

## Enable log reports

Enable/disable serialization log reports.

**Available Options**: none, errors, all

## Manual Camera

## Show debug menu

Enable/disable manual camera debug menu.

**Available Options**: false, true

## Control player

Enable/disable control player using manual camera. Option is only available when [Debug manual camera](#Debug_manual_camera) is enabled

**Available Options**: false, true

## Speed boost up debug

Enable/disable manual camera speed boost debug.

**Available Options**: false, true

## Paste view link

Paste view link using hotkey (LShift+LAlt+L).

**Hotkey**: `⇧ Shift` + `Alt` + `L`

**Available Options**: false, true

## Unlimited Movement

Enable/disable unlimited manual camera movement.

**Available Options**: false, true

## Editor

## Is Opened

Enable/disable editor opened flag debug.

**Available Options**: false, true

## Can Open

Enable/disable editor can open flag debug.

**Available Options**: false, true

## Can Close

Enable/disable editor can close flag debug.

**Available Options**: false, true

## Show Debug

Enable/disable editor debug.

**Available Options**: false, true

## Force Limited

Enable/disable forcing of limited editor.

**Available Options**: false, true

## Log Async Load

Enable/disable logging of async loading for editor.

**Available Options**: false, true

## Show Screenshot

Enable/disable showing of screenshot for editor.

**Available Options**: false, true

## Simulated Network Delay

Set simulated network delay.

**Range Settings**:

* Min: 0
* Max: 1000
* Current Value: 0
* Step: 100

## GameMode

## Game Mode

Enable/disable showing of game mode debug.

**Available Options**: false, true

## Show kill log

Enable/disable showing of kill log.

**Available Options**: false, true

## Respawn Component

Enable/disable respawn component debug.

**Available Options**: false, true

## Respawn Time Measures

Enable/disable respawn time measurement debug.

**Available Options**: false, true

## Respawn Timer

Enable/disable respawn timer component debug.

**Available Options**: false, true

## Scoring System

Enable/disable showing of scoring system debug.

**Available Options**: false, true

## Player Factions

Enable/disable showing of player factions debug.

**Available Options**: false, true

## Player Loadouts

Enable/disable showing of player loadouts debug.

**Available Options**: false, true

## Conflict

## Instant composition spawning

Enable/disable instant composition spawning.

**Available Options**: false, true

## Promotion

Promotes player to higher rank.

**Available Options**: false, true

## Demotion

Demotes player to lower rank

**Available Options**: false, true

## Singleplayer

## Disable Game Over

Enable/disable game over.

**Available Options**: false, true

## Tutorial

## Course locations

### OBSTACLE COURSE

Enable obstacle course location.

**Available Options**: false, true

### FIRING RANGE

Enable firing range location.

**Available Options**: false, true

### COMBAT ENGINEERING

Enable combat engineering location.

**Available Options**: false, true

### SEIZING

Enable seizing location.

**Available Options**: false, true

### FIRST AID

Enable first aid location.

**Available Options**: false, true

### DRIVING

Enable driving location.

**Available Options**: false, true

### NAVIGATION

Enable navigation location.

**Available Options**: false, true

### HELICOPTER PILOTING

Enable helicopter piloting location.

**Available Options**: false, true

### SPECIAL WEAPONS

Enable special weapons location.

**Available Options**: false, true

### LONG-DISTANCE SHOOTING

Enable long range shooting location.

**Available Options**: false, true

### FIRE SUPPORT COURSE

Enable fire support course location.

**Available Options**: false, true

### SQUAD LEADERSHIP

Enable squad leadership location.

**Available Options**: false, true

### VEHICLE MAINTENANCE

Enable vehicle maintenance location.

**Available Options**: false, true

## Disable course reqs

Enable/disable tutorial course requirements.

**Available Options**: false, true

## Skip intro sequence

Enable/disable skip intro sequence.

**Available Options**: false, true

## Regenerate course tasks

Enable/disable regeneration of course tasks.

**Available Options**: false, true

## Skip AI Driving

Enable/disable skipping AI driving.

**Available Options**: false, true

## Finish current stage

Enable/disable finishing of current stage.

**Available Options**: false, true

## Move to waypoint

Enable/disable moving to waypoint.

**Available Options**: false, true

## Set evening time

Enable/disable setting evening time.

**Available Options**: false, true

## Dump stages into log

Enable/disable dumping of stages into log.

**Available Options**: false, true

## Finish all courses

Enable/disable finishing of all courses.

**Available Options**: false, true

## Draw Area Restrictions

Enable/disable drawing of area restrictions.

**Available Options**: false, true

## ScenarioFramework

## Tasks

Enable/disable scenario framework tasks debug.

**Available Options**: false, true

## Registered Areas

Enable/disable scenario framework registered areas debug.

**Available Options**: false, true

## Debug Areas

Enable/disable scenario framework debug areas.

**Available Options**: false, true

## Layer Inspector

Enable/disable scenario framework layer inspector.

**Available Options**: false, true

## Action Inspector

Enable/disable scenario framework action inspector.

**Available Options**: false, true

## Logic Inspector

Enable/disable scenario framework logic inspector.

**Available Options**: false, true

## Plugin Inspector

Enable/disable scenario framework plugin inspector.

**Available Options**: false, true

## Condition Inspector

Enable/disable scenario framework condition inspector.

**Available Options**: false, true

## Tasks

## Execute tasks

Enable/disable execution of tasks.

**Available Options**: true, false

## Print all tasks

Enable/disable printing of all tasks.

**Available Options**: false, true

## Radial Menu

## Enable diagnostics

Enable/disable radial menu diagnostics.

**Available Options**: false, true

## Enable logging

Enable/disable radial menu logging.

**Available Options**: false, true

ⓘ

It is possible to hover over debug setting titles in this page to see their access path in the Debug Menu.
