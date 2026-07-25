# [Audio Editor: Getting Started Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Audio_Editor:_Getting_Started_Tutorial)

In this tutorial, we will guide the reader through the steps to replace an existing sound of a game entity (in this case the footsteps of a character) with custom ones created in the [Arma Reforger:Audio Editor](/wiki/Arma_Reforger:Audio_Editor "Arma Reforger:Audio Editor").
The purpose of this tutorial is to demonstrate how to use the key features of the audio editor and how files created in the audio editor can be connected to an Enfusion-based game.
In particular, we will cover the following topics:

* Creating a new audio project (.acp) file
* Using nodes to create sounds
* Auditioning sounds in the editor
* Connecting sounds to the game
* Using signals to control node parameters
* Debugging sounds

## Audio Project Creation

When it comes to modding sounds, the user can start with one of two approaches:

1. Modifying an existing .acp
2. Creating a new .acp from scratch

For the purpose of this tutorial, we will start by creating a new .acp.

From the Enfusion Workbench main window, we start by opening the audio editor by selecting **Editors→Audio Editor**.

From the Audio Editor main window, we create a new project via **File→New→Project**.
A new window should pop up prompting the user to define the name and location of the new file.
We will name our file *Footsteps\_Demo.acp* and place it in a new folder called *Demo*.

*Creating a new project in the Audio Editor*

Upon pressing *Ok*, we should see a new tab show up with the name of our file accompanied by a blank design canvas beneath it.
This is the area where we can create our sound. We will start by creating a simple sound for testing purposes, then expand on it later after we confirm that it works as intended.

## Work with Nodes

As detailed in [Arma Reforger:Audio Editor: Nodes](/wiki/Arma_Reforger:Audio_Editor:_Nodes "Arma Reforger:Audio Editor: Nodes"), the simplest possible sound we can create must contain the following fundamental nodes:

* a node that outputs an audio waveform: either a Bank or a Generator
* a Sound node

A Bank requires that we have some external audio samples to work with, so for the sake of simplicity, we will use a Generator as our audio source for now.

We start by placing a Generator node on the design canvas by locating its name in the nodes palette and clicking it.
We should see a new instance of the node appear on the canvas with a default name assigned (*Generator 1*).
We repeat the same step for the Sound node.

Now we need to connect the two nodes together. We first re-arrange the nodes on the canvas so that the *Out* port of the Generator is approximately lined up with the *In* port of the Sound.
We then create a connection between the two ports by clicking on the *Out* port and dragging to the *In* port (or vice-versa).

*Simple audio signal chain*

Our simple audio chain is almost complete - now we just need to configure the parameters of each node before we proceed to test it.

Since we're prototyping a footstep sound, we want the sound to be short so that we can clearly hear each individual footstep and that there is no overlapping of sounds.
For this reason, we will select the Generator node and inside the **Item detail panel**, modify the value of **Time** to 100.
This means that the sound produced by the generator (by default, a 440Hz sine wave) will have a duration of 100ms.
We will also reduce the value of **Volume** to a more comfortable level of -12dB. We will leave the rest of the parameters as their defaults.
The same goes for the Sound node.

## Sound Check

Now we're ready to play the sound. To do this, we select one of the nodes and press the space bar (or alternatively, the play button in the **Playlist** panel).
If everything was configured correctly, we should hear a brief tone through our audio playback device accompanied by some debug message in the **Log Console** indicating that the sound was successfully played.
If we turn our attention to the **Output tracker** panel, we will see the waveform of the played sound. Zooming in, we can verify that the duration of the sound was 100 ms and that the shape of the waveform is a sine, as intended.
If we look at the **Master level monitor** panel, we can see after playing the sound again that the level peaks at -12dB.

*View during playback of the sound*

Now that we have verified that the sound plays as intended, we're almost ready to connect it to the game, save for one important step - setting the correct name for the Sound node.
The name of the Sound node corresponds to the name of the sound event called from gamecode or script - if they don't match exactly then sound won't be played!
Therefore, our next task is to find out the name of the sound event for footsteps.
The easiest way to go about this is to identify an existing game entity that makes footstep sounds and look into the .acp(s) attached to that entity's **SoundComponent**.

## Sound Event Name

In order to identify the sound event name for footsteps, we first need to get the list of all .acps attached to the target entity, of which one will contain the sound for footsteps.
We can obtain this list via the entity's **SoundComponent**.
To view an entity's SoundComponent, we need to open it inside the **World Editor**.

From the Enfusion Workbench main window, we can open the World Editor by selecting **Editors→World Editor**.
We need to load some world (any world is fine) and place our target entity in the world.
For this tutorial, we're working with Arma Reforger, where our target entity is called *Character\_Base.et*.
Note that the name of the prefab may differ according to which game the user is working with - the search function in the **Resource Browser** panel can be helpful in this case.

Next we must find the target entity in the world. If it is not already present we can place a new instance of it by locating the prefab file in the **Resource Browser** and dragging it into the world.
Now we select the entity in the world.
We will see the **Object Properties** panel get populated with a list of components on the entity. We can quickly find the SoundComponent, if present, by typing "sound" in the **Filter components** search bar.
In our case we will see a special type of SoundComponent made specifically for character sounds, called **CharacterSoundComponent**.
After clicking on this component, we will see the parameters of the component appear below, including the list of .acp files under **Filenames**.
We're interested in the file that contains footstep sounds, which is appropriately titled *Character\_Footsteps.acp*.
We proceed to open this file in the audio editor by clicking the *Open File* icon next to the filename.

*Locating the SoundComponent on the target entity*

Now with the .acp opened inside the audio editor we want to find the Sound node associated with footstep sounds. We can locate it quickly with the help of the **Item explorer** panel.
This panel contains a list of all nodes grouped by type, so we can scroll down the list until we get to the *Sound* category to see a list of all sound names in the file.
Out of the sound names listed, we can identify the one for footsteps as SOUND\_CHAR\_MOVEMENT\_FOOT. To confirm that this is the correct sound we can double-click on the sound name to select that node on the design canvas.
We can then play the sound to confirm that what we hear played back sounds like footsteps.

*Locating the sound event name for footsteps*

Now that we have the sound event name, we will note it down and return to our file *Footsteps\_Demo.acp*, where we will simply change the name of the Sound node to SOUND\_CHAR\_MOVEMENT\_FOOT.
After saving the file, everything will have been prepared and we're now ready to connect the sound to the game and test how it behaves as the character moves around.

## Game-Sound Connection

In this step, we will replace the existing footstep sounds on the target entity with the one we created in the previous steps.
As in the previous step, we will start by locating the SoundComponent (or CharacterSoundComponent) on the target entity.
Under the **Filenames** parameter of the CharacterSoundComponent, we will locate the existing .acp containing footsteps and remove it.
We can do this by selecting the file *Character\_Footsteps.acp* in the list and clicking the "-" button.
Now we will add our new file to the list - we can do this by clicking the "+" button next to Filenames, then setting the filename to *Footsteps\_Demo.acp* using the ".." button.

All that's left to do now is save the world and run the game. Note that when we save the world, any changes made to the target entity will only apply to that one instance of the target entity in the world.
If we want to apply the changes to all instances of the prefab, we can do that with the "Apply to prefab" button.

If we configured everything correctly up to this point, we should hear our simple sound play every time the character takes a step.

## Signal Chain Addition

At this point in the tutorial, we have successfully created a simple sound in the audio editor and connected it to a game.
Our sound is far from ideal though - it is missing some key behavior to make it sound anywhere near convincing.
First of all, the sound is not *spacialised* at all - that is to say that the position of listener/camera in the game world with respect to the character has no effect on the sound.
Each time the sound is played, it sounds exactly the same no matter where the sound is played from in the world.
In the real world, we perceive sounds differently based on where they are located; nearby sounds will sound louder than distant ones, and direction is also reflected in the balance between what we hear in each ear.
These so-called *auditory cues* are simulated by the **Shader** node.

The Shader should typically come at the final stage of the signal chain, just before the Sound node, so we will place it there, between the existing Generator and Sound nodes.
We will also need to connect an **Amplitude** node to the Shader as well for it to have any affect. Finally we will set the parameters of the new nodes.
For the Shader, we will set the **Spatial Factor** parameter to 1 - this controls the degree to which spacial processing is applied.
For the Amplitude node, we will set the **Curve** parameter to *1/r* - this defines how the volume changes with distance.

*Signal chain with a Shader inserted*

We can observe the resulting effect of the Shader by playing the sound while changing the spatial relationship between the listener and sound source. This can be accomplished with the help of the **Listener setup** panel.
This is an interactive panel that depicts a simple world where we can see an abstract shape (red sphere) that represents the audio source. We can imagine that this shape is producing all the sounds played in the audio editor.
By interacting with this panel, we can adjust where the source is located in space with respect to us, the listener.
For example, scrolling up or down with the mouse wheel takes us closer or farther away from the source, and dragging with the left/right mouse button allows us to rotate the perspective of the source and listener.
Try playing the sound while interacting with this window to hear the differences for yourself.

### Signals

Another key feature of the audio engine is the ability to control various node parameters with *signals* (a time-varying parameter with an associated name) from the game.
For example, say we want the sound of the footsteps to change based on how fast the character is moving.
We can accomplish this by creating two or more Banks, each containing samples corresponding to a speed range (fast, slow, etc.), and selecting one of the banks for playback based on the speed signal.
The **Selector** node is the key that allows us to implement this behavior.

For this tutorial we will define three speed ranges:

* Walk: [0, 2) m/s
* Run: [2, 4) m/s
* Sprint: 4+ m/s

Note that a square bracket indicates that its value is included in the range while a rounded bracket indicates that its value is not included in the range.
For example, the range [2, 4) includes all values between 2 and 3.999...
This is to eliminate ambiguity in case the value of speed is exactly on the edge between two ranges.

For the sake of simplicity, instead of using Banks with unique samples for each range, we will add additional Generators with different frequency settings for each range so that we can quickly hear a difference.
We will use the existing Generator for the "Run" range and add two more for the remaining ranges.
The new Generators should have the same parameters as the existing one, with the only difference being the **Frequency**: we will set it to 220Hz for walking and 880Hz for sprinting.

Now we need to insert a **Selector** into the chain and connect all the Generators to it in the correct order. Upon creating a new selector, we need to add three input ports to it, corresponding to the "Walk", "Run", and "Sprint" ranges.
We can do this by clicking the "+" button next to the **Ports** property in the **Item detail** window. Upon adding a new port, the user will be prompted to give a name to the port (we will start with "Walk").
Next we need to define the range of the port via the **Min** and **Max** parameters.
If the Selector's control signal falls in this range then whatever is connected to the corresponding port will be selected for playback and anything connected to the other ports will be ignored.
As defined above, the "Walk" port will correspond to the range [0, 2) m/s, so we set **Min** to 0 and **Max** to 2. We repeat the same procedure for the "Run" and "Sprint" ports.
For the Max parameter of the Sprint port, we can set the value to something high (99 is fine).

At this point, our signal chain should look something like the one below, save for the **Signal** node, which we will cover next.

*Signal chain with a Selector inserted*

The final step is configuring the Signal node. This node will provide us with the value of the character's speed that we can use to control nodes such as the Selector.

We start by creating a new instance of a Signal node. A Signal is a resource-type node, so we will have to specify a new name and location to save it at.
When placed on the canvas, the Signal node has no inputs or outputs by default, so we will have to add some.
We can do that by double-clicking on the Signal node to open it for editing in the Signal Editor.

The Signal Editor is a sub-part of the audio editor where the user can create signal chains, similar to the ones created in the main audio editor, that transform one or more signal inputs into one or more signal outputs.
In our case, we want the output of the signal node to be the speed, with no further transformations applied to it, so our signal chain will simply be a single input connected to a single output.
We can construct the signal chain the same way we would in the main audio editor, by creating instances of an **Input** node, an **Output** node, and connecting them together on the canvas.
The last thing that we need to do inside the Signal Editor is to assign the appropriate name to the input the appropriate name of the signal from the game.
This is an important step because if the signal input name doesn't match the one in the game, the value assigned to that input will remain as 0 for the lifetime of the game.
Following the same logic as in the Getting the sound event name step, we can deduce what the signal name should be by looking into the existing .acps attached to the target entity.
In our case, the speed signal is appropriately named "Speed".

*Internal configuration of the speed signal node as viewed from the Signal Editor*

After we save the file and return to *Footsteps\_Demo.acp*, we should see the new input and output ports appear on the Signal node that we had placed earlier.
We just need to connect the output to the Selector's "Signal" input and our chain is complete.

## Sound Debug

Before moving on to testing the sound in-game, we would like to make a quick note of a useful feature of the audio editor - debugging. This feature can be useful when a sound isn't behaving as intended.
Often times the problem is related to signal values, so the debugging feature allows us to observe the values of the signal inputs to each node when it is enabled.

With a project open inside the audio editor, debugging mode can be toggled via the F5 key (or selecting **Debug→Start Debugging**). Now whenever a sound is played, the signal value at every signal input on a node is displayed.
We can try it with *Footsteps\_Demo.acp* for varying values of the speed signal.

*Playing the sound in debug mode*

Note that the same feature is available inside the Signal Editor.

## Final Test In-Game

The last step of this tutorial is to test how the final sound behaves in-game.
With the .acp attached to the target entity as described in the Getting the sound event name step, we run the game and control the target entity's movement in the world.
As we modify the speed of the character (using the Ctrl and Shift keys), we should be able to hear the pitch of the character's footstep sounds change depending on how fast the character is moving.

## Conclusion

In this tutorial, we have covered the basics of using the audio editor to modify sounds in an Enfusion-based game. We have only scratched the surface in terms of the possible sounds that can be created in the editor.
For a detailed reference of the all of the available node types and their uses, see the [Arma Reforger:Audio Editor: Nodes](/wiki/Arma_Reforger:Audio_Editor:_Nodes "Arma Reforger:Audio Editor: Nodes") page.
