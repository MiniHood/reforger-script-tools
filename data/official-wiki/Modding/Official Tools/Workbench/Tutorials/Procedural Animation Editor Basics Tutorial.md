# [Procedural Animation Editor Basics Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Procedural_Animation_Editor_Basics_Tutorial)

For the demonstration of how Procedural Animation Editor works, let's see the example of vehicle exhaust, the end of which should be shaking a bit when the engine of the vehicle is on.

This shake should be also affected by vehicle RPM and also be a bit bigger when engine goes on or off. Let's go into it.

ⓘ

See [Procedural Animation Editor](/wiki/Arma_Reforger:Procedural_Animation_Editor "Arma Reforger:Procedural Animation Editor") documentation.

## Create Files

* Create a new .pap and .siga file:
  + File > New > Project for the .pap file (or `Ctrl` + `⇧ Shift` + `N`)
  + File > New > Signal for the .siga file

## Input and Output

1. In the .siga file, create two new **Inputs** and name them **EngineRPM** and **EngineOn**. - these are this animation's two parameters. In the same file, create an **Output** and name it **EngineShake** (the naming here is up to the modder).

## Project Setup

* In the .pap file, click on **Signal**
* In the new window, navigate to the existing .siga file and press OK
* Create two new nodes **RotationMake** and **RotationSet**
* Connect the **Signal**'s EngineShake output to both X and Y axis inputs of the **RotationMake** node
* Connect **RotationMake**'s Out to **RotationSet**'s Rotation input
* In the view window, select the vehicle:
  + Click on the **SelectModel** button
  + Search for UAZ469\_base.xob and select the .xob file
* Select on the model the bone to be used. Click on the exhaust bone (should be v\_exhaust and select **Add bone** - this will create a **Bone** node
* Connect this new Bone node to **RotationSet**'s In input

[![armareforger proceduralanimationeditor-tutorial-pap-result.png](/wikidata/images/f/fa/armareforger_proceduralanimationeditor-tutorial-pap-result.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-pap-result.png)

## Signal Processing

Now that the .pap project file has been set up, it is time to work with the .siga file, in order to process the signal from the engine.
The following picture represents the wanted end result and is described in the next steps.

[![armareforger proceduralanimationeditor-tutorial-siga-first-step.png](/wikidata/images/4/41/armareforger_proceduralanimationeditor-tutorial-siga-first-step.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-siga-first-step.png)

The whole process can be divided into two branches:

### Engine RPM

Let's handle the shaking of exhaust based on engine RPM:

* Create a **Value** node and set its value to 7500
* Create a **Div** node in order to divide engineRPM (value in range 0..~5000) by the **Value**'s set value (7500)
* Connect the **Input** *engineRPM*'s Out to **Div**'s In input
* Connect the **Value**'s Out to **Div**'s Divisor input

ⓘ

The value **7500** could also have been set directly in the **Div** node.

* Create a **Random** node to generate a random value
  + Set its min value to -0.6 and max value to 0.6 to make it generate a random number in -0.6..+0.6 range
* Create a **Mul** node to multiply the result of the division output by the random value
* Connect the **Div**'s Out and the **Random**'s Out to **Mul**'s In
* Create a **Smoother** node
  + Set its Fade-in and Fade-out values to 300 (milliseconds)
  + Keep "Interpolate First" ticked
* Connect the **Mul**'s Out to **Smoother**'s In
* Connect the **Smoother**'s Out to **Output** engineShake's In

[![armareforger proceduralanimationeditor-tutorial-engine-rpm-result.png](/wikidata/images/0/0d/armareforger_proceduralanimationeditor-tutorial-engine-rpm-result.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-engine-rpm-result.png)

The exhaust now shakes according to engine's RPM as well as a little randomization.

### Engine On

Let's handle a shake of exhaust on start and stop of the engine:

* Create a **Smoother** node, keep its default values
* Connect the **Input** *engineOn*'s Out to this **Smooker**'s In
* Create a **Convertor[sic](/wiki/Template:sic "Template:sic")** node
  + Set this node's Intervals' values (min/max/out):  
    [0] 0.100/0.300/0.500  
    [1] 0.301/0.600/1.000  
    [2] 0.601/0.900/0.500  
    [3] 0.901/1.000/0.000

ⓘ

These values will create the effect of a stronger shake on engine start / stop (smoothed later).

* Connect the **Smoother**'s Out to the **Convertor[sic](/wiki/Template:sic "Template:sic")**'s In (**not** Default)
* Create another **Smoother** node
  + Set its Fade-in value to 300 (milliseconds) and this time set its Fade-out value to **0**
* Connect the **Convertor[sic](/wiki/Template:sic "Template:sic")**'s Out to the **Smoother**'s In
* Create a **Random** node
* Create a **Mul** node to multiply **Smoother**'s and **Random**'s Out values
* Connect the **Smoother**'s Out and the **Random**'s Out to **Mul**'s In

[![armareforger proceduralanimationeditor-tutorial-engine-on-result.png](/wikidata/images/d/d1/armareforger_proceduralanimationeditor-tutorial-engine-on-result.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-engine-on-result.png)

The whole Signal file is not completed! There are still "dangling" nodes.

[![armareforger proceduralanimationeditor-tutorial-engine-intermediate-result.png](/wikidata/images/f/fd/armareforger_proceduralanimationeditor-tutorial-engine-intermediate-result.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-engine-intermediate-result.png)

Now, in order to merge (by sum) engineRPM and engineOn's results, we will change engineRPM's flow:

* Disconnect **Output** *engineShake* from the **Smoother** node

  ⓘ

  In order to do so, click on the node link and press `Del`.
* Create a **Sum** node
* Connect the two open ends to it: the last **Smoother** node from *engineRPM* and the last **Mul** node from *engineOn*
* Connect the **Sum**'s Out to *engineShake*

The final result should look like the following image.
[![armareforger proceduralanimationeditor-tutorial-engine-final-result.png](/wikidata/images/5/5e/armareforger_proceduralanimationeditor-tutorial-engine-final-result.png)](/wiki/File:armareforger_proceduralanimationeditor-tutorial-engine-final-result.png)

## Vehicle Setup

Now, the last thing to be done is to set up the vehicle. For that, a [CarProcAnimComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/CarProcAnimComponent.c;16) has to exist on the vehicle:

Hit the "**+**" button in its Parameters section and add a new **ProcAnimParams**.
As a **ResourceName** navigate to the created .pap file, in **BoneNames** click on "**+**" button and select the bone to be animated.

Index of added bones on ProcAnimComponent has to be same as the order in .pap file.

Same goes for inputs in .siga file, or the following could happen:

3 new input boxes were created in .siga file in order Sig1, Sig3, Sig2. In script, using SignalManager's FindSignal method:

| FindSignal | Expected index | Actual index |
| --- | --- | --- |
| ENFORCECODEMARKER   ``` int sigInd1 = m_SignalManager.FindSignal("Sig1"); ``` | 0 | 0 |
| ENFORCECODEMARKER   ``` int sigInd2 = m_SignalManager.FindSignal("Sig2"); ``` | 1 | 2 |
| ENFORCECODEMARKER   ``` int sigInd3 = m_SignalManager.FindSignal("Sig3"); ``` | 2 | 1 |
