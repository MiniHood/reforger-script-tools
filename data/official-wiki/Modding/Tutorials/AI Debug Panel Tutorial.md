# [AI Debug Panel Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:AI_Debug_Panel_Tutorial)

The AI Debug Panel is an important tool for AI debug, allowing to see in real time an AI's condition.

## Usage

Use the [scrDefine](/wiki/Arma_Reforger:Startup_Parameters#scrDefine "Arma Reforger:Startup Parameters") startup parameter:

```
-scrDefine AI_DEBUG
```

Select a Character or a Group with [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master"). Open the [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu"), select **AI → AI Script → Open Debug Panel**. It will create a debug panel for this AI Agent.

## Debug Panel

A debug panel is assigned to only one AI Group/Character and shows data about it.

### Available Data

| Data | Available for | |
| --- | --- | --- |
| Groups | Characters |
| Call sign | Checked | Checked |
| Current action | Checked | Checked |
| List of available actions and their priority | Checked | Checked |
| Threat level | Unchecked | Checked |
| Unit roles | Unchecked | Checked |
| Unit states | Unchecked | Checked |

### Available Buttons

#### Dump Debug Msgs

Dumps all events of this AI. You can also select a time interval.

#### Breakpoint

[Script Editor](/wiki/Arma_Reforger:Script_Editor "Arma Reforger:Script Editor") will hit a breakpoint on this AI's next update.

#### Locate

*Character only*

Press to locate the selected AI in the world.

## Add Debug Panel Information

To add more information to the debug panel, mod the [SCR\_AIAgentDebugPanel](enfusion://ScriptEditor/scripts/Game/AI/SCR_AIAgentDebugPanel.c;4) class.

Please note that the same class is used for both Groups and Characters!
