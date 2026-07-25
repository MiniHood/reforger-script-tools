# [World Editor: Navmesh Tool Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Navmesh_Tool_Tutorial)

This is a tutorial for [World Editor: Navmesh Tool](/wiki/Arma_Reforger:World_Editor:_Navmesh_Tool "Arma Reforger:World Editor: Navmesh Tool").

## Setup

The loaded world must have an AIWorld entity placed, holding a NavmeshWorldComponent. This NavmeshWorldComponent should have **World Settings**, **Navmesh Settings** and **Server Config** initialised - pointing to a navmesh file (.nmn).

### Connect

Click connect to "connect" to a Navmesh Generation server. the "Connect" window appears, allowing to choose which Navmesh "server" to use.

### Generate

Pressing the **Generate** button will bring the **Recast params** settings window. Click OK to confirm and it will open the **Generator area** settings window. Click OK again to apply the default settings and start navmesh generation.

While generating, the black square preview will fill with current terrain's navmesh preview.

### Save

If **Autosave when done** was not ticked, it is important to click **Save** or **Save As** to properly save the generated data. Otherwise, the generated navmesh will be lost!
