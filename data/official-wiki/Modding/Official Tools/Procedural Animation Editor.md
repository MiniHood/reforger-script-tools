# [Procedural Animation Editor](https://community.bistudio.com/wiki/Arma_Reforger:Procedural_Animation_Editor)

The **Procedural Animation Editor** is used for the processing of engine signals and transforming those values to the animations. Every project consists of two files. \*.siga and \*.pap. A project can represent one animation or a logical group of animations i.e. vehicle dashboard, gear, etc.

* .siga file - This file handles the processing of the signal and all of the required math operations.
* .pap file - Links the \*.siga outputs to model bones and handle its movement in desired directions and range.

ⓘ

Some definitions:

* Signal - Any value (number) sent by the engine to the Procedural Animation Editor.
* Bone - Similar to a bone in the human body. It is a solid piece of model that can be animated and it interacts with the rest of the model.

## Editor Panels

1. **Nodes palette:** This palette contains all available functions in the Procedural Animation Editor.
2. **Resource browser:** Serves to navigate throw folders hierarchy.
3. **Item detail:** All available parameters of selected node, or parameters of inputs (in .siga file) if none is selected.
4. **View:** Preview with currently selected model.
5. **Console:** Used for printing information related to the project.
6. **Main window:** In this space you can build the chain of nodes to process an engine signal.

Nodes in the Procedural Animation editor can be chained together. This is done by pressing and holding ![Left Mouse Button](/wikidata/images/thumb/b/b9/mouse-button-left.png/32px-mouse-button-left.png "Left Mouse Button") on the connection point of node and dragging the line to another connection point.

## Nodes

See [Procedural Animation Editor: Nodes](/wiki/Arma_Reforger:Procedural_Animation_Editor:_Nodes "Arma Reforger:Procedural Animation Editor: Nodes").
