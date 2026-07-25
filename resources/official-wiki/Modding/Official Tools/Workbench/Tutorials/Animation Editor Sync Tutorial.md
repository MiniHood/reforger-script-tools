# [Animation Editor: Sync Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Sync_Tutorial)

This article will explain in detail how you can set up animations to sync with each other, primarily focusing on locomotion.

## Overview

The **Event Table** keeps the data about what events exist in the **Workspace**, the **Sync Table** defines when these events happen inside a specific cycle.
Walk, run, and sprint all have different **cycles** but **share the same events.** The events defined in the **Sync Table** are then placed in the animations at the right time, and the **Source Sync Node** is assigned the proper **Sync Line.**

If you wish to transition from one animation with a different **Sync Line** than another animation, **it will still work,** assuming you are **passing the right time**.

## Event tables

Every workspace has an **Event Table**, and this table defines the events that can be placed in animations.
The basic events are the planting of the feet in any walk, run, or sprint animation.

Right clicking on the **Event Table** line in the **Workspace** menu will allow you to **Create New Event Table** which when opened will allow you to add events, shown below.

These will then be used in something called **Sync Tables.**

## Sync Table

Similarly, a **Sync Table** is needed in order for animations to properly sync across multiple transitions or changes in movement speed, and you can create one similar to the **Event Table:**

A **Sync Table** defines **where in animation time the events will occur.** To create multiple **Sync Lines**, press the **+** button in the top left after opening the **Sync Table.**
**The time it is using is** Normalised, **which means that they are per cycle, explained in the next paragraph.**
For example, the order of events in a walk cycle are *generally* as following: Right Foot Down, Left Foot Up, Left Foot Down, Right Foot Up, repeating as many times as there are loops in that animation.

You would only have one of each event in one **Sync Line;** which would be the **first loop** of this animation.
If you have a walk animation that is **96** frames and there are **three** loops (the right foot is planted 3 times), the **first** loop of this animation would be approximately **32 frames**.
This is important to note down, as the **Sync Line** only accounts for the first loop and then assumes that the rest of the loops in that **Sync** share a similar pattern.
This way, if you want to blend to another animation that has the same **Sync Line** as this one, but has a **different length,** the system can still approximate the syncing between them.

So, in the DayZ character's Walk Forward, we can look at the frame that has the first occurrence of **Right Foot Down**.
The animation is just **32** frames and the right footfall is around frame **8**.
So dividing the footfall frame with the total frames, we can get the **Normalised time at which the footfall would normally occur.** 8 / 32 = 0.25, which means that **eight** frames out of **32** is 25% through the animation, that makes it 0.25 in **Normalised Time**.
Here it is applied to the **Walk Sync Line** inside of the **Sync Table:**

Repeating this for the rest of the events (Left Foot Up, Left Foot Down, Right Foot Up), then we get following:

## Events in Animations

Now that we have the proper **Sync Line** for an animation, you need to open up an animation in the workbench, then open the **Anim Editor Preview**, click on Snap to make your scroll bar snap on frames, and then click on **Edit**:

When clicking on **Edit,** you will get another button to **Save** any changes you made to the animation.
By clicking on **Edit,** it allows you to add **Events** to the animation at the specific frame you are on.
If you go to the 8th frame, as per your **Sync Line,** and click on the **+** button in the **Properties** window, you will see that a blank event has been added in the **Properties** window as well as in your animation**:**

Pressing on **Event,** you will get a drop down list of the events that can be placed on that frame.
As per the **Sync Line** we created before, the first one should be RFootDown.
If you kept notes of the other footfalls, you should be able to place the other events at the appropriate frames.

After you have done this, the animation is done and it is ready to be added to the graph.

## Sync Node

Once you have created the **Sync Line** and filled it out, you can create a **Source Sync Node** and assign it the **Walk Sync Line.** once you have done this, you want to take another animation and create the proper events

## Syncing between multiple animations

Now that we have synced the animation in this particular clip, we need to actually sync between animations in order for all of this to actually matter.
For you to understand the following steps, it would be best if you read the article on **State Machines**.

If you set up a simple state machine that looks something like this, and create a condition that will be able to transition you between the two states like so:

However, there's a problem. Each time you transition between two states, the time is reset and the animation skips back to the beginning of the clip.
This is due to the fact that each state takes care of their own time due to performance optimization.
See this video:

You will need to use a **function** in the **transition** that will allow you to take the time that the previous state was in and hand it to the new state, and this function is called **GetLowerTime()**.
In order to add this, open the transition between two states that have sync animations in them and add it under **Start Time:**

You will need to tick the **Post Eval** checkbox, since you are dealing with time, and this is explained more in detail in the [Animation Editor: Nodes - State Machine Transition](/wiki/Arma_Reforger:Animation_Editor:_Nodes#State_Machine_Transition "Arma Reforger:Animation Editor: Nodes") article.
The **GetLowerTime()** function will then take the time that was in the previous state before the transition, pass it to the new state and play the animation from that time.
Now, you'll get something that looks a bit more natural:

With the passing of time, the feet accurately step in the right order instead of going out of sync. These are the principles of **Syncing** and **Events.**
